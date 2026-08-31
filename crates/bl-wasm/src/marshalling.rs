use bl_core::matrix::Matrix;
use bl_core::value::{BioSequence, GenomicInterval, Strand, Table, Value};
use js_sys::{Array, Float64Array, Object, Reflect, Uint8Array};
use std::collections::HashMap;
use wasm_bindgen::{JsCast, JsValue};

const TYPE_KEY: &str = "__biolangType";

fn error(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}

fn set(object: &Object, key: &str, value: &JsValue) -> Result<(), JsValue> {
    Reflect::set(object, &JsValue::from_str(key), value).map(|_| ())
}

fn tagged(kind: &str) -> Result<Object, JsValue> {
    let object = Object::new();
    set(&object, TYPE_KEY, &JsValue::from_str(kind))?;
    Ok(object)
}

fn null_object() -> Object {
    // JavaScript permits `null` as Object.create's prototype. js-sys models
    // the argument as Object, so this unchecked reference expresses that one
    // valid nullable case without evaluating source or invoking a constructor.
    Object::create(JsValue::NULL.unchecked_ref::<Object>())
}

fn consume(budget: &mut usize, amount: usize) -> Result<(), JsValue> {
    if amount > *budget {
        return Err(error(
            "BioLang value exceeds the inline JavaScript byte limit",
        ));
    }
    *budget -= amount;
    Ok(())
}

pub fn value_to_js(value: &Value, maximum_bytes: usize) -> Result<JsValue, JsValue> {
    let mut budget = maximum_bytes;
    value_to_js_inner(value, &mut budget, 0)
}

pub fn handle_descriptor(
    session: u32,
    id: u32,
    generation: u32,
    value: &Value,
) -> Result<JsValue, JsValue> {
    let object = tagged("handle")?;
    set(&object, "session", &JsValue::from_f64(session as f64))?;
    set(&object, "id", &JsValue::from_f64(id as f64))?;
    set(&object, "generation", &JsValue::from_f64(generation as f64))?;
    set(
        &object,
        "valueType",
        &JsValue::from_str(&value.type_of().to_string()),
    )?;
    match value {
        Value::Table(table) => {
            set(&object, "rows", &JsValue::from_f64(table.num_rows() as f64))?;
            set(
                &object,
                "columns",
                &JsValue::from_f64(table.num_cols() as f64),
            )?;
        }
        Value::Matrix(matrix) => {
            set(&object, "rows", &JsValue::from_f64(matrix.nrow as f64))?;
            set(&object, "columns", &JsValue::from_f64(matrix.ncol as f64))?;
        }
        Value::SparseMatrix(matrix) => {
            set(&object, "rows", &JsValue::from_f64(matrix.nrow as f64))?;
            set(&object, "columns", &JsValue::from_f64(matrix.ncol as f64))?;
            set(&object, "nonZero", &JsValue::from_f64(matrix.nnz() as f64))?;
        }
        Value::List(values) => {
            set(&object, "length", &JsValue::from_f64(values.len() as f64))?;
        }
        Value::Map(values) | Value::Record(values) => {
            set(&object, "length", &JsValue::from_f64(values.len() as f64))?;
        }
        Value::Set(values) | Value::Tuple(values) => {
            set(&object, "length", &JsValue::from_f64(values.len() as f64))?;
        }
        Value::DNA(sequence) | Value::RNA(sequence) | Value::Protein(sequence) => {
            set(
                &object,
                "length",
                &JsValue::from_f64(sequence.data.len() as f64),
            )?;
        }
        Value::Quality(values) => {
            set(&object, "length", &JsValue::from_f64(values.len() as f64))?;
        }
        _ => {}
    }
    Ok(object.into())
}

pub fn handle_page(value: &Value, offset: usize, limit: usize) -> Result<JsValue, JsValue> {
    let limit = limit.clamp(1, 1_000);
    match value {
        Value::Table(table) => {
            let end = offset.saturating_add(limit).min(table.rows.len());
            let page = Table::new(
                table.columns.clone(),
                table.rows.get(offset..end).unwrap_or(&[]).to_vec(),
            );
            value_to_js(&Value::Table(page), 64 * 1024 * 1024)
        }
        Value::Matrix(matrix) => {
            let end = offset.saturating_add(limit).min(matrix.nrow);
            let mut data = Vec::with_capacity((end.saturating_sub(offset)) * matrix.ncol);
            if offset < end {
                data.extend_from_slice(&matrix.data[offset * matrix.ncol..end * matrix.ncol]);
            }
            let mut page =
                Matrix::new(data, end.saturating_sub(offset), matrix.ncol).map_err(error)?;
            page.row_names = matrix
                .row_names
                .as_ref()
                .map(|names| names.get(offset..end).unwrap_or(&[]).to_vec());
            page.col_names = matrix.col_names.clone();
            value_to_js(&Value::Matrix(page.into()), 64 * 1024 * 1024)
        }
        Value::SparseMatrix(matrix) => {
            let end = offset.saturating_add(limit).min(matrix.nrow);
            let mut data = Vec::with_capacity((end.saturating_sub(offset)) * matrix.ncol);
            for row in offset..end {
                for column in 0..matrix.ncol {
                    data.push(matrix.get(row, column));
                }
            }
            let mut page =
                Matrix::new(data, end.saturating_sub(offset), matrix.ncol).map_err(error)?;
            page.row_names = matrix
                .row_names
                .as_ref()
                .map(|names| names.get(offset..end).unwrap_or(&[]).to_vec());
            page.col_names = matrix.col_names.clone();
            value_to_js(&Value::Matrix(page.into()), 64 * 1024 * 1024)
        }
        Value::List(values) => {
            let end = offset.saturating_add(limit).min(values.len());
            value_to_js(
                &Value::List(values.get(offset..end).unwrap_or(&[]).to_vec().into()),
                64 * 1024 * 1024,
            )
        }
        Value::Record(values) | Value::Map(values) => {
            let mut entries = values.iter().collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            let selected = entries
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect::<HashMap<_, _>>();
            let page = if matches!(value, Value::Record(_)) {
                Value::Record(selected.into())
            } else {
                Value::Map(selected.into())
            };
            value_to_js(&page, 64 * 1024 * 1024)
        }
        Value::Set(values) | Value::Tuple(values) => {
            let end = offset.saturating_add(limit).min(values.len());
            value_to_js(
                &Value::List(values.get(offset..end).unwrap_or(&[]).to_vec().into()),
                64 * 1024 * 1024,
            )
        }
        Value::DNA(sequence) | Value::RNA(sequence) | Value::Protein(sequence) => {
            let data = sequence
                .data
                .chars()
                .skip(offset)
                .take(limit)
                .collect::<String>();
            Ok(JsValue::from_str(&data))
        }
        Value::Quality(values) => {
            let end = offset.saturating_add(limit).min(values.len());
            Ok(Uint8Array::from(values.get(offset..end).unwrap_or(&[])).into())
        }
        _ => Err(error(format!(
            "{} does not support paged JavaScript access",
            value.type_of()
        ))),
    }
}

pub fn handle_float64(value: &Value) -> Result<Float64Array, JsValue> {
    match value {
        Value::Matrix(matrix) => Ok(Float64Array::from(matrix.data.as_slice())),
        Value::SparseMatrix(_) => Err(error(
            "SparseMatrix cannot be represented by one Float64Array without losing its indices",
        )),
        Value::List(values) => {
            let numbers = values
                .iter()
                .map(|value| match value {
                    Value::Int(value) => Ok(*value as f64),
                    Value::Float(value) => Ok(*value),
                    other => Err(error(format!(
                        "cannot convert {} to Float64Array",
                        other.type_of()
                    ))),
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Float64Array::from(numbers.as_slice()))
        }
        other => Err(error(format!(
            "{} cannot be converted to Float64Array",
            other.type_of()
        ))),
    }
}

fn value_to_js_inner(value: &Value, budget: &mut usize, depth: usize) -> Result<JsValue, JsValue> {
    if depth > 64 {
        return Err(error("BioLang value nesting exceeds 64 levels"));
    }
    consume(budget, 8)?;
    match value {
        Value::Nil => Ok(JsValue::NULL),
        Value::Bool(value) => Ok(JsValue::from_bool(*value)),
        Value::Int(value) if value.unsigned_abs() <= 9_007_199_254_740_991 => {
            Ok(JsValue::from_f64(*value as f64))
        }
        Value::Int(value) => {
            let object = tagged("int64")?;
            set(&object, "value", &JsValue::from_str(&value.to_string()))?;
            Ok(object.into())
        }
        Value::Float(value) => Ok(JsValue::from_f64(*value)),
        Value::Str(value) => {
            consume(budget, value.len())?;
            Ok(JsValue::from_str(value))
        }
        Value::List(values) => {
            let array = Array::new_with_length(values.len() as u32);
            for (index, value) in values.iter().enumerate() {
                array.set(index as u32, value_to_js_inner(value, budget, depth + 1)?);
            }
            Ok(array.into())
        }
        Value::Record(values) => {
            let object = tagged("record")?;
            let entries = null_object();
            for (key, value) in values.iter() {
                consume(budget, key.len())?;
                set(&entries, key, &value_to_js_inner(value, budget, depth + 1)?)?;
            }
            set(&object, "entries", &entries.into())?;
            Ok(object.into())
        }
        Value::Map(values) => {
            let object = tagged("map")?;
            let entries = null_object();
            for (key, value) in values.iter() {
                consume(budget, key.len())?;
                set(&entries, key, &value_to_js_inner(value, budget, depth + 1)?)?;
            }
            set(&object, "entries", &entries.into())?;
            Ok(object.into())
        }
        Value::Set(values) | Value::Tuple(values) => {
            let kind = if matches!(value, Value::Set(_)) {
                "set"
            } else {
                "tuple"
            };
            let object = tagged(kind)?;
            let array = Array::new_with_length(values.len() as u32);
            for (index, value) in values.iter().enumerate() {
                array.set(index as u32, value_to_js_inner(value, budget, depth + 1)?);
            }
            set(&object, "values", &array.into())?;
            Ok(object.into())
        }
        Value::Table(table) => table_to_js(table, budget, depth),
        Value::Matrix(matrix) => matrix_to_js(matrix, budget),
        Value::DNA(sequence) | Value::RNA(sequence) | Value::Protein(sequence) => {
            consume(budget, sequence.data.len())?;
            let object = tagged("sequence")?;
            let sequence_kind = match value {
                Value::DNA(_) => "dna",
                Value::RNA(_) => "rna",
                _ => "protein",
            };
            set(&object, "sequenceKind", &JsValue::from_str(sequence_kind))?;
            set(&object, "data", &JsValue::from_str(&sequence.data))?;
            Ok(object.into())
        }
        Value::Quality(scores) => {
            consume(budget, scores.len())?;
            let object = tagged("quality")?;
            set(&object, "data", &Uint8Array::from(scores.as_slice()).into())?;
            Ok(object.into())
        }
        Value::Range {
            start,
            end,
            inclusive,
        } => {
            let object = tagged("range")?;
            set(&object, "start", &integer_to_js(*start)?)?;
            set(&object, "end", &integer_to_js(*end)?)?;
            set(&object, "inclusive", &JsValue::from_bool(*inclusive))?;
            Ok(object.into())
        }
        Value::Interval(interval) => {
            consume(budget, interval.chrom.len())?;
            let object = tagged("interval")?;
            set(&object, "chrom", &JsValue::from_str(&interval.chrom))?;
            set(&object, "start", &integer_to_js(interval.start)?)?;
            set(&object, "end", &integer_to_js(interval.end)?)?;
            set(
                &object,
                "strand",
                &JsValue::from_str(&interval.strand.to_string()),
            )?;
            Ok(object.into())
        }
        Value::Regex { pattern, flags } => {
            consume(budget, pattern.len().saturating_add(flags.len()))?;
            let object = tagged("regex")?;
            set(&object, "pattern", &JsValue::from_str(pattern))?;
            set(&object, "flags", &JsValue::from_str(flags))?;
            Ok(object.into())
        }
        Value::EnumValue {
            enum_name,
            variant,
            fields,
        } => {
            consume(budget, enum_name.len().saturating_add(variant.len()))?;
            let object = tagged("enum")?;
            set(&object, "enumName", &JsValue::from_str(enum_name))?;
            set(&object, "variant", &JsValue::from_str(variant))?;
            let array = Array::new_with_length(fields.len() as u32);
            for (index, field) in fields.iter().enumerate() {
                array.set(index as u32, value_to_js_inner(field, budget, depth + 1)?);
            }
            set(&object, "fields", &array.into())?;
            Ok(object.into())
        }
        // Sparse matrices, streams, functions, futures, annotations, variants,
        // genomes and aligned reads remain session-owned handles. They either
        // cannot be copied faithfully or can be unexpectedly large.
        other => Err(error(format!(
            "{} must remain a session-bound BioLang handle",
            other.type_of()
        ))),
    }
}

fn integer_to_js(value: i64) -> Result<JsValue, JsValue> {
    if value.unsigned_abs() <= 9_007_199_254_740_991 {
        Ok(JsValue::from_f64(value as f64))
    } else {
        let object = tagged("int64")?;
        set(&object, "value", &JsValue::from_str(&value.to_string()))?;
        Ok(object.into())
    }
}

fn table_to_js(table: &Table, budget: &mut usize, depth: usize) -> Result<JsValue, JsValue> {
    let object = tagged("table")?;
    let columns = Array::new_with_length(table.columns.len() as u32);
    for (index, column) in table.columns.iter().enumerate() {
        consume(budget, column.len())?;
        columns.set(index as u32, JsValue::from_str(column));
    }
    let rows = Array::new_with_length(table.rows.len() as u32);
    for (row_index, row) in table.rows.iter().enumerate() {
        let cells = Array::new_with_length(row.len() as u32);
        for (column_index, value) in row.iter().enumerate() {
            cells.set(
                column_index as u32,
                value_to_js_inner(value, budget, depth + 1)?,
            );
        }
        rows.set(row_index as u32, cells.into());
    }
    set(&object, "columns", &columns.into())?;
    set(&object, "rows", &rows.into())?;
    Ok(object.into())
}

fn matrix_to_js(matrix: &Matrix, budget: &mut usize) -> Result<JsValue, JsValue> {
    consume(budget, matrix.data.len().saturating_mul(8))?;
    let object = tagged("matrix")?;
    set(&object, "nrow", &JsValue::from_f64(matrix.nrow as f64))?;
    set(&object, "ncol", &JsValue::from_f64(matrix.ncol as f64))?;
    set(
        &object,
        "data",
        &Float64Array::from(matrix.data.as_slice()).into(),
    )?;
    if let Some(names) = &matrix.row_names {
        consume(budget, names.iter().map(String::len).sum::<usize>())?;
        set(&object, "rowNames", &strings_to_array(names).into())?;
    }
    if let Some(names) = &matrix.col_names {
        consume(budget, names.iter().map(String::len).sum::<usize>())?;
        set(&object, "columnNames", &strings_to_array(names).into())?;
    }
    Ok(object.into())
}

fn strings_to_array(values: &[String]) -> Array {
    let array = Array::new_with_length(values.len() as u32);
    for (index, value) in values.iter().enumerate() {
        array.set(index as u32, JsValue::from_str(value));
    }
    array
}

pub fn js_to_value(
    value: &JsValue,
    handle_lookup: &dyn Fn(u32, u32, u32) -> Option<Value>,
) -> Result<Value, JsValue> {
    js_to_value_inner(value, handle_lookup, 0)
}

fn js_to_value_inner(
    value: &JsValue,
    handle_lookup: &dyn Fn(u32, u32, u32) -> Option<Value>,
    depth: usize,
) -> Result<Value, JsValue> {
    if depth > 64 {
        return Err(error("JavaScript value nesting exceeds 64 levels"));
    }
    if value.is_null() || value.is_undefined() {
        return Ok(Value::Nil);
    }
    if let Some(value) = value.as_bool() {
        return Ok(Value::Bool(value));
    }
    if let Some(value) = value.as_f64() {
        if value.is_finite() && value.fract() == 0.0 && value.abs() <= 9_007_199_254_740_991.0 {
            return Ok(Value::Int(value as i64));
        }
        if value.is_finite() && value.fract() == 0.0 {
            return Err(error(
                "unsafe JavaScript integer; pass a bigint to preserve it exactly",
            ));
        }
        return Ok(Value::Float(value));
    }
    if let Some(value) = value.as_string() {
        return Ok(Value::Str(value));
    }
    if Array::is_array(value) {
        let array = Array::from(value);
        let mut values = Vec::with_capacity(array.length() as usize);
        for index in 0..array.length() {
            values.push(js_to_value_inner(
                &array.get(index),
                handle_lookup,
                depth + 1,
            )?);
        }
        return Ok(Value::List(values.into()));
    }
    if value.is_instance_of::<Float64Array>() {
        return Err(error(
            "bare Float64Array has no persistent BioLang vector type; pass a JavaScript Array or a tagged matrix",
        ));
    }
    if value.is_instance_of::<Uint8Array>() {
        return Err(error(
            "bare Uint8Array is ambiguous; pass a JavaScript Array or a tagged Quality value",
        ));
    }
    if !value.is_object() {
        return Err(error("unsupported JavaScript value"));
    }

    let kind = property_string(value, TYPE_KEY)?;
    match kind.as_deref() {
        Some("int64") => property_string_required(value, "value")?
            .parse::<i64>()
            .map(Value::Int)
            .map_err(|_| error("invalid signed 64-bit integer")),
        Some("handle") => {
            let session = property_u32(value, "session")?;
            let id = property_u32(value, "id")?;
            let generation = property_u32(value, "generation")?;
            handle_lookup(session, id, generation).ok_or_else(|| {
                error("BioLang handle is stale, disposed, or belongs to another session")
            })
        }
        Some("sequence") => {
            let sequence = BioSequence {
                data: property_string_required(value, "data")?,
            };
            match property_string_required(value, "sequenceKind")?.as_str() {
                "dna" => Ok(Value::DNA(sequence)),
                "rna" => Ok(Value::RNA(sequence)),
                "protein" => Ok(Value::Protein(sequence)),
                _ => Err(error("unknown biological sequence kind")),
            }
        }
        Some("quality") => {
            let data = Reflect::get(value, &JsValue::from_str("data"))?;
            if !data.is_instance_of::<Uint8Array>() {
                return Err(error("quality data must be Uint8Array"));
            }
            Ok(Value::Quality(Uint8Array::new(&data).to_vec()))
        }
        Some("table") => js_table_to_value(value, handle_lookup, depth),
        Some("matrix") => js_matrix_to_value(value),
        Some("record") | Some("map") => {
            let entries = Reflect::get(value, &JsValue::from_str("entries"))?;
            let entries = object_to_map(&entries, handle_lookup, depth + 1)?.into();
            if kind.as_deref() == Some("record") {
                Ok(Value::Record(entries))
            } else {
                Ok(Value::Map(entries))
            }
        }
        Some("set") | Some("tuple") => {
            let values = Reflect::get(value, &JsValue::from_str("values"))?;
            let Value::List(values) = js_to_value_inner(&values, handle_lookup, depth + 1)? else {
                return Err(error("set or tuple values must be an array"));
            };
            if kind.as_deref() == Some("set") {
                Ok(Value::Set(values.as_ref().clone()))
            } else {
                Ok(Value::Tuple(values.as_ref().clone()))
            }
        }
        Some("range") => Ok(Value::Range {
            start: property_i64(value, "start")?,
            end: property_i64(value, "end")?,
            inclusive: Reflect::get(value, &JsValue::from_str("inclusive"))?
                .as_bool()
                .unwrap_or(false),
        }),
        Some("interval") => Ok(Value::Interval(GenomicInterval {
            chrom: property_string_required(value, "chrom")?,
            start: property_i64(value, "start")?,
            end: property_i64(value, "end")?,
            strand: Strand::from_str_lossy(&property_string_required(value, "strand")?),
        })),
        Some("regex") => Ok(Value::Regex {
            pattern: property_string_required(value, "pattern")?,
            flags: property_string_required(value, "flags")?,
        }),
        Some("enum") => {
            let fields = Reflect::get(value, &JsValue::from_str("fields"))?;
            let Value::List(fields) = js_to_value_inner(&fields, handle_lookup, depth + 1)? else {
                return Err(error("enum fields must be an array"));
            };
            Ok(Value::EnumValue {
                enum_name: property_string_required(value, "enumName")?,
                variant: property_string_required(value, "variant")?,
                fields: fields.as_ref().clone(),
            })
        }
        Some(other) => Err(error(format!(
            "unsupported JavaScript BioLang type '{other}'"
        ))),
        None => Ok(Value::Record(
            object_to_map(value, handle_lookup, depth + 1)?.into(),
        )),
    }
}

fn js_table_to_value(
    value: &JsValue,
    handle_lookup: &dyn Fn(u32, u32, u32) -> Option<Value>,
    depth: usize,
) -> Result<Value, JsValue> {
    let columns = strings_from_array(&Reflect::get(value, &JsValue::from_str("columns"))?)?;
    let rows_value = Reflect::get(value, &JsValue::from_str("rows"))?;
    if !Array::is_array(&rows_value) {
        return Err(error("table rows must be an array"));
    }
    let rows_array = Array::from(&rows_value);
    let mut rows = Vec::with_capacity(rows_array.length() as usize);
    for row_index in 0..rows_array.length() {
        let row = rows_array.get(row_index);
        if !Array::is_array(&row) {
            return Err(error("each table row must be an array"));
        }
        let cells = Array::from(&row);
        if cells.length() as usize != columns.len() {
            return Err(error("table row width does not match its columns"));
        }
        let mut converted = Vec::with_capacity(cells.length() as usize);
        for column_index in 0..cells.length() {
            converted.push(js_to_value_inner(
                &cells.get(column_index),
                handle_lookup,
                depth + 1,
            )?);
        }
        rows.push(converted);
    }
    Ok(Value::Table(Table::new(columns, rows)))
}

fn js_matrix_to_value(value: &JsValue) -> Result<Value, JsValue> {
    let nrow = property_u32(value, "nrow")? as usize;
    let ncol = property_u32(value, "ncol")? as usize;
    let data = Reflect::get(value, &JsValue::from_str("data"))?;
    if !data.is_instance_of::<Float64Array>() {
        return Err(error("matrix data must be Float64Array"));
    }
    let mut matrix = Matrix::new(Float64Array::new(&data).to_vec(), nrow, ncol).map_err(error)?;
    let row_names = Reflect::get(value, &JsValue::from_str("rowNames"))?;
    if !row_names.is_undefined() && !row_names.is_null() {
        matrix.row_names = Some(strings_from_array(&row_names)?);
    }
    let column_names = Reflect::get(value, &JsValue::from_str("columnNames"))?;
    if !column_names.is_undefined() && !column_names.is_null() {
        matrix.col_names = Some(strings_from_array(&column_names)?);
    }
    Ok(Value::Matrix(matrix.into()))
}

fn object_to_map(
    value: &JsValue,
    handle_lookup: &dyn Fn(u32, u32, u32) -> Option<Value>,
    depth: usize,
) -> Result<HashMap<String, Value>, JsValue> {
    let object: Object = value.clone().unchecked_into();
    let keys = Object::keys(&object);
    let mut result = HashMap::with_capacity(keys.length() as usize);
    for index in 0..keys.length() {
        let key = keys
            .get(index)
            .as_string()
            .ok_or_else(|| error("object key is not a string"))?;
        let item = Reflect::get(value, &JsValue::from_str(&key))?;
        result.insert(key, js_to_value_inner(&item, handle_lookup, depth + 1)?);
    }
    Ok(result)
}

fn strings_from_array(value: &JsValue) -> Result<Vec<String>, JsValue> {
    if !Array::is_array(value) {
        return Err(error("expected an array of strings"));
    }
    let array = Array::from(value);
    (0..array.length())
        .map(|index| {
            array
                .get(index)
                .as_string()
                .ok_or_else(|| error("expected a string"))
        })
        .collect()
}

fn property_string(value: &JsValue, key: &str) -> Result<Option<String>, JsValue> {
    let value = Reflect::get(value, &JsValue::from_str(key))?;
    if value.is_undefined() || value.is_null() {
        Ok(None)
    } else {
        value
            .as_string()
            .map(Some)
            .ok_or_else(|| error(format!("{key} must be a string")))
    }
}

fn property_string_required(value: &JsValue, key: &str) -> Result<String, JsValue> {
    property_string(value, key)?.ok_or_else(|| error(format!("missing {key}")))
}

fn property_u32(value: &JsValue, key: &str) -> Result<u32, JsValue> {
    Reflect::get(value, &JsValue::from_str(key))?
        .as_f64()
        .filter(|value| value.is_finite() && *value >= 0.0 && *value <= u32::MAX as f64)
        .map(|value| value as u32)
        .ok_or_else(|| error(format!("{key} must be an unsigned integer")))
}

fn property_i64(value: &JsValue, key: &str) -> Result<i64, JsValue> {
    js_to_value_inner(
        &Reflect::get(value, &JsValue::from_str(key))?,
        &|_, _, _| None,
        0,
    )
    .and_then(|value| match value {
        Value::Int(value) => Ok(value),
        _ => Err(error(format!("{key} must be an integer"))),
    })
}
