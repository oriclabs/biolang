//! Lossless variable export shared by browser and native front ends.
//!
//! Browser callers write through a hard-capped buffer. Native callers stream
//! directly to a temporary file beside the destination and rename it only
//! after serialization succeeds.

use bl_core::value::Value;
use std::io::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueExportFormat {
    Json,
    Csv,
    Tsv,
    Text,
}

impl ValueExportFormat {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            "tsv" | "tab" => Ok(Self::Tsv),
            "txt" | "text" => Ok(Self::Text),
            other => Err(format!("unsupported export format '{other}'")),
        }
    }

    pub fn from_path(path: &std::path::Path) -> Result<Self, String> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        Self::parse(extension)
    }

    pub fn media_type(self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::Csv => "text/csv",
            Self::Tsv => "text/tab-separated-values",
            Self::Text => "text/plain",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Csv => "csv",
            Self::Tsv => "tsv",
            Self::Text => "txt",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueExportSummary {
    pub bytes: u64,
}

pub fn export_value_capped(
    value: &Value,
    format: ValueExportFormat,
    maximum_bytes: usize,
) -> Result<Vec<u8>, String> {
    let mut output = CappedWriter::new(maximum_bytes);
    export_value_to_writer(value, format, &mut output).map_err(|error| {
        if output.exceeded {
            format!("export exceeds the browser limit of {maximum_bytes} bytes")
        } else {
            error.to_string()
        }
    })?;
    Ok(output.bytes)
}

pub fn export_value_to_writer(
    value: &Value,
    format: ValueExportFormat,
    writer: &mut impl Write,
) -> io::Result<ValueExportSummary> {
    let mut counted = CountingWriter::new(writer);
    match format {
        ValueExportFormat::Json => write_json_value(&mut counted, value)?,
        ValueExportFormat::Csv => write_delimited_value(&mut counted, value, b',')?,
        ValueExportFormat::Tsv => write_delimited_value(&mut counted, value, b'\t')?,
        ValueExportFormat::Text => write_text_value(&mut counted, value)?,
    }
    counted.flush()?;
    Ok(ValueExportSummary {
        bytes: counted.bytes,
    })
}

#[cfg(feature = "native")]
pub fn export_value_to_path(
    value: &Value,
    destination: &std::path::Path,
    format: ValueExportFormat,
) -> Result<ValueExportSummary, String> {
    use std::fs::{self, OpenOptions};
    use std::io::BufWriter;
    use std::time::{SystemTime, UNIX_EPOCH};

    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let filename = destination
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "export destination must include a valid filename".to_string())?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);
    let temporary = parent.join(format!(".{filename}.part-{}-{nonce}", std::process::id()));

    let result = (|| {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("cannot create '{}': {error}", temporary.display()))?;
        let mut writer = BufWriter::new(file);
        let summary = export_value_to_writer(value, format, &mut writer)
            .map_err(|error| error.to_string())?;
        writer.flush().map_err(|error| error.to_string())?;
        writer
            .get_ref()
            .sync_all()
            .map_err(|error| error.to_string())?;
        drop(writer);
        fs::rename(&temporary, destination)
            .map_err(|error| format!("cannot replace '{}': {error}", destination.display()))?;
        Ok(summary)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

struct CappedWriter {
    bytes: Vec<u8>,
    maximum: usize,
    exceeded: bool,
}

impl CappedWriter {
    fn new(maximum: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(maximum.min(64 * 1024)),
            maximum,
            exceeded: false,
        }
    }
}

impl Write for CappedWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if buffer.len() > self.maximum.saturating_sub(self.bytes.len()) {
            self.exceeded = true;
            return Err(io::Error::other("export byte limit exceeded"));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct CountingWriter<'a, W> {
    inner: &'a mut W,
    bytes: u64,
}

impl<'a, W> CountingWriter<'a, W> {
    fn new(inner: &'a mut W) -> Self {
        Self { inner, bytes: 0 }
    }
}

impl<W: Write> Write for CountingWriter<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buffer)?;
        self.bytes = self.bytes.saturating_add(written as u64);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn write_json_value(writer: &mut impl Write, value: &Value) -> io::Result<()> {
    match value {
        Value::Nil => writer.write_all(b"null"),
        Value::Bool(value) => write!(writer, "{value}"),
        Value::Int(value) => write!(writer, "{value}"),
        Value::Float(value) => write_json_float(writer, *value),
        Value::Str(value) => serde_json::to_writer(writer, value).map_err(io::Error::other),
        Value::List(values) => write_json_array(writer, values.iter()),
        Value::Set(values) | Value::Tuple(values) => write_json_array(writer, values.iter()),
        Value::Map(values) | Value::Record(values) => {
            writer.write_all(b"{")?;
            for (index, (key, value)) in values.iter().enumerate() {
                if index > 0 {
                    writer.write_all(b",")?;
                }
                serde_json::to_writer(&mut *writer, key).map_err(io::Error::other)?;
                writer.write_all(b":")?;
                write_json_value(writer, value)?;
            }
            writer.write_all(b"}")
        }
        Value::Table(table) => {
            writer.write_all(b"[")?;
            for (row_index, row) in table.rows.iter().enumerate() {
                if row_index > 0 {
                    writer.write_all(b",")?;
                }
                writer.write_all(b"{")?;
                for (column, name) in table.columns.iter().enumerate() {
                    if column > 0 {
                        writer.write_all(b",")?;
                    }
                    serde_json::to_writer(&mut *writer, name).map_err(io::Error::other)?;
                    writer.write_all(b":")?;
                    write_json_value(writer, row.get(column).unwrap_or(&Value::Nil))?;
                }
                writer.write_all(b"}")?;
            }
            writer.write_all(b"]")
        }
        Value::Matrix(matrix) => {
            writer.write_all(b"[")?;
            for row in 0..matrix.nrow {
                if row > 0 {
                    writer.write_all(b",")?;
                }
                writer.write_all(b"[")?;
                for column in 0..matrix.ncol {
                    if column > 0 {
                        writer.write_all(b",")?;
                    }
                    write_json_float(writer, matrix.get(row, column))?;
                }
                writer.write_all(b"]")?;
            }
            writer.write_all(b"]")
        }
        Value::SparseMatrix(matrix) => {
            writer.write_all(b"{\"format\":\"csr\",\"rows\":")?;
            write!(
                writer,
                "{},\"columns\":{},\"indptr\":",
                matrix.nrow, matrix.ncol
            )?;
            write_json_usize_array(writer, &matrix.indptr)?;
            writer.write_all(b",\"indices\":")?;
            write_json_usize_array(writer, &matrix.indices)?;
            writer.write_all(b",\"data\":[")?;
            for (index, value) in matrix.data.iter().enumerate() {
                if index > 0 {
                    writer.write_all(b",")?;
                }
                write_json_float(writer, *value)?;
            }
            writer.write_all(b"]}")
        }
        Value::DNA(sequence) | Value::RNA(sequence) | Value::Protein(sequence) => {
            serde_json::to_writer(writer, &sequence.data).map_err(io::Error::other)
        }
        Value::Quality(values) => {
            writer.write_all(b"[")?;
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    writer.write_all(b",")?;
                }
                write!(writer, "{value}")?;
            }
            writer.write_all(b"]")
        }
        Value::Range {
            start,
            end,
            inclusive,
        } => {
            write!(
                writer,
                "{{\"start\":{start},\"end\":{end},\"inclusive\":{inclusive}}}"
            )
        }
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "JSON export does not yet support {}; choose a type-specific format",
                other.type_of()
            ),
        )),
    }
}

fn write_json_float(writer: &mut impl Write, value: f64) -> io::Result<()> {
    if !value.is_finite() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "JSON cannot represent NaN or infinite numbers without changing the value",
        ));
    }
    write!(writer, "{value}")
}

fn write_json_array<'a>(
    writer: &mut impl Write,
    values: impl Iterator<Item = &'a Value>,
) -> io::Result<()> {
    writer.write_all(b"[")?;
    for (index, value) in values.enumerate() {
        if index > 0 {
            writer.write_all(b",")?;
        }
        write_json_value(writer, value)?;
    }
    writer.write_all(b"]")
}

fn write_json_usize_array(writer: &mut impl Write, values: &[usize]) -> io::Result<()> {
    writer.write_all(b"[")?;
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            writer.write_all(b",")?;
        }
        write!(writer, "{value}")?;
    }
    writer.write_all(b"]")
}

fn write_delimited_value(writer: &mut impl Write, value: &Value, separator: u8) -> io::Result<()> {
    match value {
        Value::Table(table) => {
            write_delimited_row(writer, table.columns.iter().map(String::as_str), separator)?;
            for row in &table.rows {
                write_delimited_cells(writer, row.iter(), separator)?;
            }
            Ok(())
        }
        Value::Matrix(matrix) => {
            write_matrix_header(writer, matrix.ncol, matrix.col_names.as_ref(), separator)?;
            for row in 0..matrix.nrow {
                let label = matrix
                    .row_names
                    .as_ref()
                    .and_then(|names| names.get(row))
                    .cloned()
                    .unwrap_or_else(|| (row + 1).to_string());
                write_delimited_text(writer, &label, separator)?;
                for column in 0..matrix.ncol {
                    writer.write_all(&[separator])?;
                    write!(writer, "{}", matrix.get(row, column))?;
                }
                writer.write_all(b"\n")?;
            }
            Ok(())
        }
        Value::SparseMatrix(matrix) => {
            write_matrix_header(writer, matrix.ncol, matrix.col_names.as_ref(), separator)?;
            for row in 0..matrix.nrow {
                let label = matrix
                    .row_names
                    .as_ref()
                    .and_then(|names| names.get(row))
                    .cloned()
                    .unwrap_or_else(|| (row + 1).to_string());
                write_delimited_text(writer, &label, separator)?;
                for column in 0..matrix.ncol {
                    writer.write_all(&[separator])?;
                    write!(writer, "{}", matrix.get(row, column))?;
                }
                writer.write_all(b"\n")?;
            }
            Ok(())
        }
        Value::Quality(values) => {
            writer.write_all(if separator == b'\t' {
                b"position\tscore\n"
            } else {
                b"position,score\n"
            })?;
            for (index, score) in values.iter().enumerate() {
                write!(writer, "{}{}{}\n", index + 1, separator as char, score)?;
            }
            Ok(())
        }
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} export requires Table, Matrix, SparseMatrix, or Quality; got {}",
                if separator == b'\t' { "TSV" } else { "CSV" },
                other.type_of()
            ),
        )),
    }
}

fn write_matrix_header(
    writer: &mut impl Write,
    columns: usize,
    names: Option<&Vec<String>>,
    separator: u8,
) -> io::Result<()> {
    writer.write_all(b"row")?;
    for column in 0..columns {
        writer.write_all(&[separator])?;
        if let Some(name) = names.and_then(|values| values.get(column)) {
            write_delimited_text(writer, name, separator)?;
        } else {
            write!(writer, "{}", column + 1)?;
        }
    }
    writer.write_all(b"\n")
}

fn write_delimited_cells<'a>(
    writer: &mut impl Write,
    values: impl Iterator<Item = &'a Value>,
    separator: u8,
) -> io::Result<()> {
    for (index, value) in values.enumerate() {
        if index > 0 {
            writer.write_all(&[separator])?;
        }
        match value {
            Value::Nil => {}
            Value::Str(value) => write_delimited_text(writer, value, separator)?,
            Value::Bool(value) => write!(writer, "{value}")?,
            Value::Int(value) => write!(writer, "{value}")?,
            Value::Float(value) => write!(writer, "{value}")?,
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "delimited export does not support nested {} table cells",
                        other.type_of()
                    ),
                ))
            }
        }
    }
    writer.write_all(b"\n")
}

fn write_delimited_row<'a>(
    writer: &mut impl Write,
    values: impl Iterator<Item = &'a str>,
    separator: u8,
) -> io::Result<()> {
    for (index, value) in values.enumerate() {
        if index > 0 {
            writer.write_all(&[separator])?;
        }
        write_delimited_text(writer, value, separator)?;
    }
    writer.write_all(b"\n")
}

fn write_delimited_text(writer: &mut impl Write, value: &str, separator: u8) -> io::Result<()> {
    let quoted = value.as_bytes().contains(&separator) || value.contains(['"', '\n', '\r']);
    if !quoted {
        return writer.write_all(value.as_bytes());
    }
    writer.write_all(b"\"")?;
    for byte in value.as_bytes() {
        if *byte == b'"' {
            writer.write_all(b"\"\"")?;
        } else {
            writer.write_all(&[*byte])?;
        }
    }
    writer.write_all(b"\"")
}

fn write_text_value(writer: &mut impl Write, value: &Value) -> io::Result<()> {
    match value {
        Value::Nil => writer.write_all(b"nil"),
        Value::Bool(value) => write!(writer, "{value}"),
        Value::Int(value) => write!(writer, "{value}"),
        Value::Float(value) => write!(writer, "{value}"),
        Value::Str(value) => writer.write_all(value.as_bytes()),
        Value::DNA(sequence) | Value::RNA(sequence) | Value::Protein(sequence) => {
            writer.write_all(sequence.data.as_bytes())
        }
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "text export does not support {}; use JSON, CSV, or TSV",
                other.type_of()
            ),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bl_core::value::Table;

    #[test]
    fn capped_json_is_exact_and_rejects_the_next_byte() {
        let value = Value::List(vec![Value::Str("A\"B".into()), Value::Int(7)].into());
        let expected = br#"["A\"B",7]"#;
        assert_eq!(
            export_value_capped(&value, ValueExportFormat::Json, expected.len()).unwrap(),
            expected
        );
        assert!(export_value_capped(&value, ValueExportFormat::Json, expected.len() - 1).is_err());
        assert!(
            export_value_capped(&Value::Float(f64::NAN), ValueExportFormat::Json, 1024)
                .unwrap_err()
                .contains("cannot represent NaN")
        );
    }

    #[test]
    fn csv_quotes_cells_without_building_a_second_table() {
        let value = Value::Table(Table::new(
            vec!["gene".into(), "note".into()],
            vec![vec![Value::Str("TP53".into()), Value::Str("a,b\"c".into())]],
        ));
        let bytes = export_value_capped(&value, ValueExportFormat::Csv, 1024).unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "gene,note\nTP53,\"a,b\"\"c\"\n"
        );
    }

    #[cfg(feature = "native")]
    #[test]
    fn native_export_replaces_only_after_a_complete_write() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("result.csv");
        std::fs::write(&destination, "previous").unwrap();
        let unsupported = Value::Table(Table::new(
            vec!["nested".into()],
            vec![vec![Value::List(vec![Value::Int(1)].into())]],
        ));
        assert!(export_value_to_path(&unsupported, &destination, ValueExportFormat::Csv).is_err());
        assert_eq!(std::fs::read_to_string(&destination).unwrap(), "previous");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);

        let value = Value::Table(Table::new(vec!["x".into()], vec![vec![Value::Int(2)]]));
        export_value_to_path(&value, &destination, ValueExportFormat::Csv).unwrap();
        assert_eq!(std::fs::read_to_string(destination).unwrap(), "x\n2\n");
    }
}
