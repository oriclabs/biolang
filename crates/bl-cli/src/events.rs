use bl_core::value::Value;
use serde_json::{json, Map, Value as JsonValue};

const MAX_ITEMS: usize = 500;
const MAX_COLUMNS: usize = 50;
const MAX_TEXT_BYTES: usize = 1024 * 1024;

fn bounded_text(value: &str) -> (String, bool) {
    if value.len() <= MAX_TEXT_BYTES {
        return (value.to_string(), false);
    }
    let mut end = MAX_TEXT_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    (value[..end].to_string(), true)
}

fn map_value(values: &std::collections::HashMap<String, Value>, depth: usize) -> JsonValue {
    let mut keys = values.keys().collect::<Vec<_>>();
    keys.sort();
    let mut object = Map::new();
    for key in keys.into_iter().take(MAX_COLUMNS) {
        object.insert(key.clone(), value_to_json_inner(&values[key], depth + 1));
    }
    JsonValue::Object(object)
}

fn value_to_json_inner(value: &Value, depth: usize) -> JsonValue {
    if depth > 8 {
        return json!({ "kind": "truncated", "display": format!("{value}") });
    }
    match value {
        Value::Nil => json!({ "kind": "nil", "value": null }),
        Value::Bool(value) => json!({ "kind": "boolean", "value": value }),
        Value::Int(value) => json!({ "kind": "integer", "value": value }),
        Value::Float(value) => json!({ "kind": "float", "value": value }),
        Value::Str(value) => {
            let (text, truncated) = bounded_text(value);
            if value.trim_start().starts_with("<svg") {
                json!({ "kind": "plot", "format": "svg", "data": text, "truncated": truncated })
            } else {
                json!({ "kind": "string", "value": text, "truncated": truncated })
            }
        }
        Value::List(values) => json!({
            "kind": "list",
            "items": values.iter().take(MAX_ITEMS).map(|value| value_to_json_inner(value, depth + 1)).collect::<Vec<_>>(),
            "totalItems": values.len(),
            "truncated": values.len() > MAX_ITEMS,
        }),
        Value::Map(values) => json!({ "kind": "map", "value": map_value(values, depth) }),
        Value::Record(values) => json!({ "kind": "record", "value": map_value(values, depth) }),
        Value::DNA(sequence) => json!({
            "kind": "sequence",
            "alphabet": "DNA",
            "sequence": sequence.data,
            "length": sequence.data.len(),
        }),
        Value::RNA(sequence) => json!({
            "kind": "sequence",
            "alphabet": "RNA",
            "sequence": sequence.data,
            "length": sequence.data.len(),
        }),
        Value::Protein(sequence) => json!({
            "kind": "sequence",
            "alphabet": "protein",
            "sequence": sequence.data,
            "length": sequence.data.len(),
        }),
        Value::Table(table) => {
            let columns = table
                .columns
                .iter()
                .take(MAX_COLUMNS)
                .cloned()
                .collect::<Vec<_>>();
            let rows = table
                .rows
                .iter()
                .take(MAX_ITEMS)
                .map(|row| {
                    row.iter()
                        .take(columns.len())
                        .map(|value| value_to_json_inner(value, depth + 1))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            json!({
                "kind": "table",
                "columns": columns,
                "rows": rows,
                "totalRows": table.rows.len(),
                "totalColumns": table.columns.len(),
                "truncated": table.rows.len() > MAX_ITEMS || table.columns.len() > MAX_COLUMNS,
            })
        }
        Value::Interval(interval) => json!({
            "kind": "interval",
            "chromosome": interval.chrom,
            "start": interval.start,
            "end": interval.end,
            "strand": interval.strand.to_string(),
        }),
        Value::Matrix(matrix) => {
            let rows = (0..matrix.nrow.min(MAX_ITEMS))
                .map(|row| {
                    (0..matrix.ncol.min(MAX_COLUMNS))
                        .map(|column| matrix.get(row, column))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            json!({
                "kind": "matrix",
                "rows": rows,
                "rowNames": matrix.row_names,
                "columnNames": matrix.col_names,
                "totalRows": matrix.nrow,
                "totalColumns": matrix.ncol,
                "truncated": matrix.nrow > MAX_ITEMS || matrix.ncol > MAX_COLUMNS,
            })
        }
        Value::SparseMatrix(matrix) => json!({
            "kind": "sparseMatrix",
            "rowPointers": matrix.indptr.iter().take(MAX_ITEMS + 1).collect::<Vec<_>>(),
            "columnIndices": matrix.indices.iter().take(MAX_ITEMS).collect::<Vec<_>>(),
            "values": matrix.data.iter().take(MAX_ITEMS).collect::<Vec<_>>(),
            "rowNames": matrix.row_names,
            "columnNames": matrix.col_names,
            "totalRows": matrix.nrow,
            "totalColumns": matrix.ncol,
            "nonZero": matrix.data.len(),
            "truncated": matrix.data.len() > MAX_ITEMS,
        }),
        Value::Kmer(kmer) => json!({
            "kind": "kmer",
            "sequence": kmer.to_string(),
            "length": kmer.k,
        }),
        Value::Stream(stream) => json!({
            "kind": "stream",
            "label": stream.label,
            "started": stream.is_started(),
            "exhausted": stream.is_exhausted(),
        }),
        other => json!({
            "kind": "opaque",
            "type": other.type_of().to_string(),
            "display": format!("{other}"),
        }),
    }
}

pub fn value_to_json(value: &Value) -> JsonValue {
    value_to_json_inner(value, 0)
}

pub fn emit(event: JsonValue) {
    println!("{event}");
}

#[cfg(test)]
mod tests {
    use super::value_to_json;
    use bl_core::value::{Table, Value};

    #[test]
    fn serializes_typed_table_results() {
        let table = Value::Table(Table::new(
            vec!["gene".into(), "count".into()],
            vec![vec![Value::Str("BRCA1".into()), Value::Int(12)]],
        ));
        let json = value_to_json(&table);
        assert_eq!(json["kind"], "table");
        assert_eq!(json["columns"][0], "gene");
        assert_eq!(json["rows"][0][1]["value"], 12);
    }

    #[test]
    fn recognizes_svg_plot_results() {
        let json = value_to_json(&Value::Str("<svg><rect /></svg>".into()));
        assert_eq!(json["kind"], "plot");
        assert_eq!(json["format"], "svg");
    }
}
