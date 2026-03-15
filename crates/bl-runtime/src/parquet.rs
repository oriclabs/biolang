use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub fn parquet_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("read_parquet", Arity::Exact(1)),
        ("write_parquet", Arity::Range(2, 3)),
    ]
}

pub fn is_parquet_builtin(name: &str) -> bool {
    matches!(name, "read_parquet" | "write_parquet")
}

pub fn call_parquet_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "read_parquet" => builtin_read_parquet(args),
        "write_parquet" => builtin_write_parquet(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown parquet builtin '{name}'"),
            None,
        )),
    }
}

// ── read_parquet(path) ──────────────────────────────────────────

fn builtin_read_parquet(args: Vec<Value>) -> Result<Value> {
    use parquet::file::reader::{FileReader, SerializedFileReader};

    let path = match &args[0] {
        Value::Str(s) => s.as_str(),
        other => {
            return Err(BioLangError::type_error(
                format!("read_parquet() requires Str path, got {}", other.type_of()),
                None,
            ))
        }
    };

    let file = std::fs::File::open(path).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("read_parquet: cannot open '{path}': {e}"),
            None,
        )
    })?;

    let reader = SerializedFileReader::new(file).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("read_parquet: invalid parquet file '{path}': {e}"),
            None,
        )
    })?;

    // Extract column names from schema
    let metadata = reader.metadata();
    let schema = metadata.file_metadata().schema();
    let col_names: Vec<String> = schema
        .get_fields()
        .iter()
        .map(|f| f.name().to_string())
        .collect();
    let ncols = col_names.len();

    // Read all row groups
    let mut rows: Vec<Vec<Value>> = Vec::new();
    let mut row_iter = reader.get_row_iter(None).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("read_parquet: cannot iterate rows in '{path}': {e}"),
            None,
        )
    })?;

    while let Some(row_result) = row_iter.next() {
        let row = row_result.map_err(|e| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("read_parquet: row read error: {e}"),
                None,
            )
        })?;
        let mut values = Vec::with_capacity(ncols);
        for (_name, field) in row.get_column_iter() {
            values.push(field_to_value(field));
        }
        rows.push(values);
    }

    let table = Table::new(col_names, rows);
    Ok(Value::Table(table))
}

/// Convert a Parquet Field to a BioLang Value.
fn field_to_value(field: &parquet::record::Field) -> Value {
    use parquet::record::Field;

    match field {
        Field::Null => Value::Nil,
        Field::Bool(b) => Value::Bool(*b),
        Field::Byte(b) => Value::Int(*b as i64),
        Field::Short(s) => Value::Int(*s as i64),
        Field::Int(i) => Value::Int(*i as i64),
        Field::Long(l) => Value::Int(*l),
        Field::UByte(b) => Value::Int(*b as i64),
        Field::UShort(s) => Value::Int(*s as i64),
        Field::UInt(i) => Value::Int(*i as i64),
        Field::ULong(l) => {
            // ULong may overflow i64; fall back to Float if needed
            if *l <= i64::MAX as u64 {
                Value::Int(*l as i64)
            } else {
                Value::Float(*l as f64)
            }
        }
        Field::Float(f) => Value::Float(*f as f64),
        Field::Double(d) => Value::Float(*d),
        Field::Str(s) => Value::Str(s.clone()),
        Field::Bytes(b) => Value::Str(format!("<bytes:{}>", b.len())),
        Field::Date(d) => {
            // Days since Unix epoch
            Value::Int(*d as i64)
        }
        Field::TimestampMillis(ts) => Value::Int(*ts as i64),
        Field::TimestampMicros(ts) => Value::Int(*ts as i64),
        Field::Decimal(d) => Value::Str(format!("{d:?}")),
        Field::Float16(f) => Value::Float(f64::from(f32::from(*f))),
        // Nested types: flatten to string representation
        Field::Group(row) => {
            let mut map = HashMap::new();
            for (name, f) in row.get_column_iter() {
                map.insert(name.to_string(), field_to_value(f));
            }
            Value::Record(map)
        }
        Field::ListInternal(list) => {
            let elements: Vec<Value> = list.elements().iter().map(field_to_value).collect();
            Value::List(elements)
        }
        Field::MapInternal(map) => {
            let mut result = HashMap::new();
            for (k, v) in map.entries() {
                let key_str = match field_to_value(k) {
                    Value::Str(s) => s,
                    other => format!("{other}"),
                };
                result.insert(key_str, field_to_value(v));
            }
            Value::Record(result)
        }
    }
}

// ── write_parquet(table, path, [options]) ───────────────────────

fn builtin_write_parquet(args: Vec<Value>) -> Result<Value> {
    let table = match &args[0] {
        Value::Table(t) => t,
        Value::List(items) => {
            // Convert list of records to table
            return write_parquet_from_records(items, &args[1..]);
        }
        other => {
            return Err(BioLangError::type_error(
                format!("write_parquet() requires Table or List, got {}", other.type_of()),
                None,
            ))
        }
    };

    let path = match &args[1] {
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("write_parquet() path must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };

    // Parse options (optional 3rd arg)
    let compression = if args.len() > 2 {
        extract_compression(&args[2])?
    } else {
        parquet::basic::Compression::SNAPPY
    };

    write_table_as_parquet(table, &path, compression)
}

fn extract_compression(opts: &Value) -> Result<parquet::basic::Compression> {
    use parquet::basic::Compression;

    match opts {
        Value::Record(m) | Value::Map(m) => {
            let comp_str = m
                .get("compression")
                .and_then(|v| match v {
                    Value::Str(s) => Some(s.as_str()),
                    _ => None,
                })
                .unwrap_or("snappy");
            match comp_str.to_lowercase().as_str() {
                "snappy" | "snap" => Ok(Compression::SNAPPY),
                "gzip" | "gz" => Ok(Compression::GZIP(Default::default())),
                "none" | "uncompressed" => Ok(Compression::UNCOMPRESSED),
                "zstd" => Ok(Compression::ZSTD(Default::default())),
                "lz4" => Ok(Compression::LZ4_RAW),
                other => Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("write_parquet: unknown compression '{other}', expected: snappy, gzip, zstd, lz4, none"),
                    None,
                )),
            }
        }
        Value::Str(s) => {
            // Allow passing compression as a plain string
            let mut map = HashMap::new();
            map.insert("compression".to_string(), Value::Str(s.clone()));
            extract_compression(&Value::Record(map))
        }
        _ => Err(BioLangError::type_error(
            "write_parquet() options must be a record like {compression: \"snappy\"}",
            None,
        )),
    }
}

/// Detect the Parquet physical type for a column based on sampling row values.
fn detect_column_type(table: &Table, col_idx: usize) -> ColType {
    let nrows = table.num_rows();
    // Sample up to 100 rows to detect type
    let sample_count = nrows.min(100);

    let mut has_int = false;
    let mut has_float = false;
    let mut has_bool = false;
    let mut has_str = false;

    for row_idx in 0..sample_count {
        if col_idx < table.rows[row_idx].len() {
            match &table.rows[row_idx][col_idx] {
                Value::Int(_) => has_int = true,
                Value::Float(_) => has_float = true,
                Value::Bool(_) => has_bool = true,
                Value::Nil => {} // skip nulls
                _ => has_str = true,
            }
        }
    }

    if has_str {
        ColType::Str
    } else if has_float {
        ColType::Float
    } else if has_int {
        ColType::Int
    } else if has_bool {
        ColType::Bool
    } else {
        ColType::Str // all nil or empty => default to string
    }
}

#[derive(Debug, Clone, Copy)]
enum ColType {
    Int,
    Float,
    Bool,
    Str,
}

fn write_table_as_parquet(
    table: &Table,
    path: &str,
    compression: parquet::basic::Compression,
) -> Result<Value> {
    use parquet::basic::{Repetition, Type as PhysicalType};
    use parquet::data_type::ByteArray;
    use parquet::file::properties::WriterProperties;
    use parquet::file::writer::SerializedFileWriter;
    use parquet::schema::types::Type as SchemaType;

    let ncols = table.columns.len();
    let nrows = table.num_rows();

    // Detect column types
    let col_types: Vec<ColType> = (0..ncols).map(|i| detect_column_type(table, i)).collect();

    // Build Parquet schema
    let mut fields: Vec<Arc<SchemaType>> = Vec::with_capacity(ncols);
    for (i, col_name) in table.columns.iter().enumerate() {
        let physical_type = match col_types[i] {
            ColType::Int => PhysicalType::INT64,
            ColType::Float => PhysicalType::DOUBLE,
            ColType::Bool => PhysicalType::BOOLEAN,
            ColType::Str => PhysicalType::BYTE_ARRAY,
        };
        let field = SchemaType::primitive_type_builder(col_name, physical_type)
            .with_repetition(Repetition::OPTIONAL)
            .build()
            .map_err(|e| {
                BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("write_parquet: schema error for column '{col_name}': {e}"),
                    None,
                )
            })?;
        fields.push(Arc::new(field));
    }

    let schema = SchemaType::group_type_builder("schema")
        .with_fields(fields)
        .build()
        .map_err(|e| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("write_parquet: schema build error: {e}"),
                None,
            )
        })?;

    let props = WriterProperties::builder()
        .set_compression(compression)
        .build();

    let file = std::fs::File::create(path).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("write_parquet: cannot create '{path}': {e}"),
            None,
        )
    })?;

    let mut writer =
        SerializedFileWriter::new(file, Arc::new(schema), Arc::new(props)).map_err(|e| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("write_parquet: writer init error: {e}"),
                None,
            )
        })?;

    // Write in batches of 10000 rows per row group
    let batch_size = 10_000;
    let mut row_offset = 0;

    while row_offset < nrows {
        let batch_end = (row_offset + batch_size).min(nrows);
        let batch_rows = batch_end - row_offset;

        let mut rg_writer = writer.next_row_group().map_err(|e| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("write_parquet: row group error: {e}"),
                None,
            )
        })?;

        for col_idx in 0..ncols {
            // Collect column values and definition levels
            let mut def_levels = Vec::with_capacity(batch_rows);
            match col_types[col_idx] {
                ColType::Int => {
                    let mut values = Vec::with_capacity(batch_rows);
                    for row_idx in row_offset..batch_end {
                        let val = table
                            .rows
                            .get(row_idx)
                            .and_then(|r| r.get(col_idx));
                        match val {
                            Some(Value::Int(i)) => {
                                values.push(*i);
                                def_levels.push(1);
                            }
                            Some(Value::Float(f)) => {
                                values.push(*f as i64);
                                def_levels.push(1);
                            }
                            Some(Value::Nil) | None => {
                                def_levels.push(0);
                            }
                            Some(other) => {
                                if let Ok(i) = format!("{other}").parse::<i64>() {
                                    values.push(i);
                                    def_levels.push(1);
                                } else {
                                    values.push(0);
                                    def_levels.push(1);
                                }
                            }
                        }
                    }
                    if let Some(mut col_writer) = rg_writer.next_column().map_err(|e| {
                        BioLangError::runtime(
                            ErrorKind::IOError,
                            format!("write_parquet: column error: {e}"),
                            None,
                        )
                    })? {
                        col_writer
                            .typed::<parquet::data_type::Int64Type>()
                            .write_batch(&values, Some(&def_levels), None)
                            .map_err(|e| {
                                BioLangError::runtime(
                                    ErrorKind::IOError,
                                    format!("write_parquet: write error: {e}"),
                                    None,
                                )
                            })?;
                        col_writer.close().map_err(|e| {
                            BioLangError::runtime(
                                ErrorKind::IOError,
                                format!("write_parquet: column close error: {e}"),
                                None,
                            )
                        })?;
                    }
                }
                ColType::Float => {
                    let mut values = Vec::with_capacity(batch_rows);
                    for row_idx in row_offset..batch_end {
                        let val = table
                            .rows
                            .get(row_idx)
                            .and_then(|r| r.get(col_idx));
                        match val {
                            Some(Value::Float(f)) => {
                                values.push(*f);
                                def_levels.push(1);
                            }
                            Some(Value::Int(i)) => {
                                values.push(*i as f64);
                                def_levels.push(1);
                            }
                            Some(Value::Nil) | None => {
                                def_levels.push(0);
                            }
                            Some(other) => {
                                if let Ok(f) = format!("{other}").parse::<f64>() {
                                    values.push(f);
                                    def_levels.push(1);
                                } else {
                                    values.push(0.0);
                                    def_levels.push(1);
                                }
                            }
                        }
                    }
                    if let Some(mut col_writer) = rg_writer.next_column().map_err(|e| {
                        BioLangError::runtime(
                            ErrorKind::IOError,
                            format!("write_parquet: column error: {e}"),
                            None,
                        )
                    })? {
                        col_writer
                            .typed::<parquet::data_type::DoubleType>()
                            .write_batch(&values, Some(&def_levels), None)
                            .map_err(|e| {
                                BioLangError::runtime(
                                    ErrorKind::IOError,
                                    format!("write_parquet: write error: {e}"),
                                    None,
                                )
                            })?;
                        col_writer.close().map_err(|e| {
                            BioLangError::runtime(
                                ErrorKind::IOError,
                                format!("write_parquet: column close error: {e}"),
                                None,
                            )
                        })?;
                    }
                }
                ColType::Bool => {
                    let mut values = Vec::with_capacity(batch_rows);
                    for row_idx in row_offset..batch_end {
                        let val = table
                            .rows
                            .get(row_idx)
                            .and_then(|r| r.get(col_idx));
                        match val {
                            Some(Value::Bool(b)) => {
                                values.push(*b);
                                def_levels.push(1);
                            }
                            Some(Value::Nil) | None => {
                                def_levels.push(0);
                            }
                            Some(other) => {
                                values.push(other.is_truthy());
                                def_levels.push(1);
                            }
                        }
                    }
                    if let Some(mut col_writer) = rg_writer.next_column().map_err(|e| {
                        BioLangError::runtime(
                            ErrorKind::IOError,
                            format!("write_parquet: column error: {e}"),
                            None,
                        )
                    })? {
                        col_writer
                            .typed::<parquet::data_type::BoolType>()
                            .write_batch(&values, Some(&def_levels), None)
                            .map_err(|e| {
                                BioLangError::runtime(
                                    ErrorKind::IOError,
                                    format!("write_parquet: write error: {e}"),
                                    None,
                                )
                            })?;
                        col_writer.close().map_err(|e| {
                            BioLangError::runtime(
                                ErrorKind::IOError,
                                format!("write_parquet: column close error: {e}"),
                                None,
                            )
                        })?;
                    }
                }
                ColType::Str => {
                    let mut values: Vec<ByteArray> = Vec::with_capacity(batch_rows);
                    for row_idx in row_offset..batch_end {
                        let val = table
                            .rows
                            .get(row_idx)
                            .and_then(|r| r.get(col_idx));
                        match val {
                            Some(Value::Str(s)) => {
                                values.push(ByteArray::from(s.as_str()));
                                def_levels.push(1);
                            }
                            Some(Value::Nil) | None => {
                                def_levels.push(0);
                            }
                            Some(other) => {
                                let s = format!("{other}");
                                values.push(ByteArray::from(s.as_str()));
                                def_levels.push(1);
                            }
                        }
                    }
                    if let Some(mut col_writer) = rg_writer.next_column().map_err(|e| {
                        BioLangError::runtime(
                            ErrorKind::IOError,
                            format!("write_parquet: column error: {e}"),
                            None,
                        )
                    })? {
                        col_writer
                            .typed::<parquet::data_type::ByteArrayType>()
                            .write_batch(&values, Some(&def_levels), None)
                            .map_err(|e| {
                                BioLangError::runtime(
                                    ErrorKind::IOError,
                                    format!("write_parquet: write error: {e}"),
                                    None,
                                )
                            })?;
                        col_writer.close().map_err(|e| {
                            BioLangError::runtime(
                                ErrorKind::IOError,
                                format!("write_parquet: column close error: {e}"),
                                None,
                            )
                        })?;
                    }
                }
            }
        }

        rg_writer.close().map_err(|e| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("write_parquet: row group close error: {e}"),
                None,
            )
        })?;

        row_offset = batch_end;
    }

    writer.close().map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("write_parquet: file close error: {e}"),
            None,
        )
    })?;

    let mut result = HashMap::new();
    result.insert("rows".to_string(), Value::Int(nrows as i64));
    result.insert("cols".to_string(), Value::Int(ncols as i64));
    result.insert("output".to_string(), Value::Str(path.to_string()));
    Ok(Value::Record(result))
}

/// Convert a list of records to a table, then write as Parquet.
fn write_parquet_from_records(items: &[Value], remaining_args: &[Value]) -> Result<Value> {
    use parquet::basic::Compression;

    if items.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "write_parquet: empty list has no columns to write",
            None,
        ));
    }

    // Extract column names from first record
    let first = match &items[0] {
        Value::Record(m) | Value::Map(m) => m,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "write_parquet() list items must be records, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };

    let col_names: Vec<String> = first.keys().cloned().collect();
    let ncols = col_names.len();

    // Build rows
    let mut rows = Vec::with_capacity(items.len());
    for item in items {
        match item {
            Value::Record(m) | Value::Map(m) => {
                let mut row = Vec::with_capacity(ncols);
                for col in &col_names {
                    row.push(m.get(col).cloned().unwrap_or(Value::Nil));
                }
                rows.push(row);
            }
            _ => {
                // Skip non-record items
                let row = vec![Value::Nil; ncols];
                rows.push(row);
            }
        }
    }

    let table = Table::new(col_names, rows);

    let path = match remaining_args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => {
            return Err(BioLangError::type_error(
                "write_parquet() path must be Str",
                None,
            ))
        }
    };

    let compression = if remaining_args.len() > 1 {
        extract_compression(&remaining_args[1])?
    } else {
        Compression::SNAPPY
    };

    write_table_as_parquet(&table, &path, compression)
}
