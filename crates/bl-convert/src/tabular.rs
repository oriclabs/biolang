use crate::streams::open_reader;
use crate::{context, ConversionStats, ConvertError, Format};
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashSet};
use std::io::{Read, Write};
use std::path::Path;

pub fn validate(input: &Path, format: Format) -> Result<u64, ConvertError> {
    match format {
        Format::Csv | Format::Tsv => {
            let delimiter = if format == Format::Csv { b',' } else { b'\t' };
            let mut reader = csv::ReaderBuilder::new()
                .delimiter(delimiter)
                .from_reader(open_reader(input)?);
            reader.headers()?;
            let mut count = 0u64;
            for row in reader.records() {
                row?;
                count += 1;
            }
            Ok(count)
        }
        Format::Json => {
            let value: Value = serde_json::from_reader(open_reader(input)?)?;
            Ok(value.as_array().map_or(1, |rows| rows.len()) as u64)
        }
        _ => unreachable!(),
    }
}

pub fn convert(
    input: &Path,
    from: Format,
    to: Format,
    writer: &mut dyn Write,
    stats: &mut ConversionStats,
) -> Result<(), ConvertError> {
    match from {
        Format::Csv | Format::Tsv => delimited_to(input, from, to, writer, stats),
        Format::Json => json_to(input, to, writer, stats),
        _ => unreachable!(),
    }
}

fn delimited_to(
    input: &Path,
    from: Format,
    to: Format,
    writer: &mut dyn Write,
    stats: &mut ConversionStats,
) -> Result<(), ConvertError> {
    let delimiter = if from == Format::Csv { b',' } else { b'\t' };
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .from_reader(open_reader(input)?);
    let headers = reader
        .headers()
        .map_err(|error| context(error, "cannot read delimited header"))?
        .clone();

    match to {
        Format::Csv | Format::Tsv => {
            let output_delimiter = if to == Format::Csv { b',' } else { b'\t' };
            let mut output = csv::WriterBuilder::new()
                .delimiter(output_delimiter)
                .from_writer(writer);
            output.write_record(&headers)?;
            for row in reader.records() {
                let row = row.map_err(|error| context(error, "cannot read delimited row"))?;
                stats.records_read += 1;
                output.write_record(&row)?;
                stats.records_written += 1;
            }
            output.flush()?;
        }
        Format::Json => {
            let mut seen = HashSet::new();
            if let Some(duplicate) = headers.iter().find(|name| !seen.insert(*name)) {
                return Err(ConvertError::new(format!(
                    "cannot convert delimited data to JSON: duplicate header '{duplicate}' would overwrite a JSON object key"
                )));
            }
            stats.warnings.push(
                "Delimited fields are emitted as JSON strings so identifiers such as 001 are not silently changed."
                    .into(),
            );
            writer.write_all(b"[\n")?;
            let mut first = true;
            for row in reader.records() {
                let row = row.map_err(|error| context(error, "cannot read delimited row"))?;
                stats.records_read += 1;
                let object = headers
                    .iter()
                    .enumerate()
                    .map(|(index, name)| {
                        (
                            name.to_string(),
                            Value::String(row.get(index).unwrap_or("").to_string()),
                        )
                    })
                    .collect::<Map<_, _>>();
                if !first {
                    writer.write_all(b",\n")?;
                }
                first = false;
                writer.write_all(b"  ")?;
                serde_json::to_writer(&mut *writer, &Value::Object(object))?;
                stats.records_written += 1;
            }
            writer.write_all(b"\n]\n")?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn json_to(
    input: &Path,
    to: Format,
    writer: &mut dyn Write,
    stats: &mut ConversionStats,
) -> Result<(), ConvertError> {
    let mut reader = open_reader(input)?;
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| context(error, format!("invalid JSON in '{}'", input.display())))?;

    if to == Format::Json {
        stats.records_read = value.as_array().map_or(1, |values| values.len()) as u64;
        stats.records_written = stats.records_read;
        serde_json::to_writer_pretty(&mut *writer, &value)?;
        writer.write_all(b"\n")?;
        return Ok(());
    }

    let rows = value.as_array().ok_or_else(|| {
        ConvertError::new("JSON -> CSV/TSV requires a top-level array of objects")
    })?;
    let mut headers = BTreeSet::new();
    for (index, row) in rows.iter().enumerate() {
        let object = row.as_object().ok_or_else(|| {
            ConvertError::new(format!(
                "JSON row {} is not an object; CSV/TSV needs named fields",
                index + 1
            ))
        })?;
        headers.extend(object.keys().cloned());
    }
    let headers = headers.into_iter().collect::<Vec<_>>();
    let delimiter = if to == Format::Csv { b',' } else { b'\t' };
    let mut output = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .from_writer(writer);
    output.write_record(&headers)?;
    let mut nested = false;
    for row in rows {
        let object = row.as_object().expect("validated above");
        let fields = headers
            .iter()
            .map(|header| match object.get(header).unwrap_or(&Value::Null) {
                Value::Null => String::new(),
                Value::String(value) => value.clone(),
                Value::Bool(value) => value.to_string(),
                Value::Number(value) => value.to_string(),
                value => {
                    nested = true;
                    serde_json::to_string(value).unwrap_or_default()
                }
            })
            .collect::<Vec<_>>();
        stats.records_read += 1;
        output.write_record(fields)?;
        stats.records_written += 1;
    }
    output.flush()?;
    stats.lossy = true;
    stats.warnings.push(
        "JSON scalar types and null-versus-empty distinctions are represented as CSV/TSV text fields."
            .into(),
    );
    if nested {
        stats.warnings.push(
            "Nested JSON values were serialized into individual text fields; the tabular output does not retain nested column types."
                .into(),
        );
    }
    stats.warnings.push(
        "JSON object keys are written in stable sorted order; original key order is not a tabular property."
            .into(),
    );
    Ok(())
}
