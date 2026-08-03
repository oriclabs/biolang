use bl_core::value::Value;
use serde_json::{json, Map, Value as JsonValue};
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

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

/// True when a value is worth promoting to a typed result on its own.
///
/// `print`/`println` see every displayed value, and most of them are progress
/// strings. Only shapes the Output pane can actually render — tables, matrices,
/// record lists, and SVG plots — earn a `result` event; everything else stays
/// text in the log.
pub fn is_structured_result(value: &Value) -> bool {
    match value {
        Value::Table(_) | Value::Matrix(_) | Value::SparseMatrix(_) => true,
        Value::Str(text) => text.trim_start().starts_with("<svg"),
        Value::List(items) => {
            !items.is_empty() && items.iter().all(|item| matches!(item, Value::Record(_)))
        }
        _ => false,
    }
}

fn result_data_ref(path: &Path) -> String {
    std::env::current_dir()
        .ok()
        .and_then(|root| path.strip_prefix(root).ok().map(Path::to_path_buf))
        .unwrap_or_else(|| path.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/")
}

fn write_full_result(value: &Value, directory: &Path) -> Option<(String, &'static str)> {
    fs::create_dir_all(directory).ok()?;
    match value {
        Value::Table(table) if table.rows.len() > MAX_ITEMS => {
            let path = directory.join("table-1.jsonl");
            let file = File::create(&path).ok()?;
            let mut writer = BufWriter::new(file);
            for row in &table.rows {
                let values = row
                    .iter()
                    .map(|value| value_to_json_inner(value, 1))
                    .collect::<Vec<_>>();
                serde_json::to_writer(&mut writer, &values).ok()?;
                writeln!(writer).ok()?;
            }
            writer.flush().ok()?;
            Some((result_data_ref(&path), "jsonl"))
        }
        Value::Matrix(matrix) if matrix.nrow > MAX_ITEMS => {
            let path = directory.join("matrix-1.jsonl");
            let file = File::create(&path).ok()?;
            let mut writer = BufWriter::new(file);
            for row in 0..matrix.nrow {
                let values = (0..matrix.ncol)
                    .map(|column| matrix.get(row, column))
                    .collect::<Vec<_>>();
                serde_json::to_writer(&mut writer, &values).ok()?;
                writeln!(writer).ok()?;
            }
            writer.flush().ok()?;
            Some((result_data_ref(&path), "jsonl"))
        }
        _ => None,
    }
}

pub fn value_to_json(value: &Value) -> JsonValue {
    let directory = std::env::var_os("BIOLANG_RESULT_DIR");
    value_to_json_with_result_directory(value, directory.as_deref().map(Path::new))
}

fn value_to_json_with_result_directory(value: &Value, directory: Option<&Path>) -> JsonValue {
    let mut result = value_to_json_inner(value, 0);
    if let Some((data_ref, encoding)) = directory.and_then(|path| write_full_result(value, path)) {
        if let Some(object) = result.as_object_mut() {
            object.insert("dataRef".to_string(), JsonValue::String(data_ref));
            object.insert(
                "encoding".to_string(),
                JsonValue::String(encoding.to_string()),
            );
        }
    }
    result
}

pub fn emit(event: JsonValue) {
    println!("{event}");
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

/// Start offsets of every line, for turning a statement span into a line number.
///
/// Built once per run rather than counting newlines per printed value, which
/// would be quadratic in a script that prints inside a loop.
pub struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut starts = vec![0usize];
        starts.extend(
            source
                .char_indices()
                .filter(|(_, character)| *character == '\n')
                .map(|(offset, _)| offset + 1),
        );
        Self { starts }
    }

    /// 1-based line containing `offset`.
    pub fn line_of(&self, offset: usize) -> usize {
        match self.starts.binary_search(&offset) {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }
}

/// A one-line rendering of a value for an inline annotation beside the source.
pub fn value_preview(value: &Value) -> String {
    const MAX_WIDTH: usize = 80;
    let text = match value {
        // A whole SVG document beside the line that drew it is noise; the plot
        // itself already renders in the Output pane.
        Value::Str(text) if text.trim_start().starts_with("<svg") => "<plot>".to_string(),
        other => other.to_string(),
    };
    let single_line = text.replace(['\n', '\r'], " ");
    if single_line.chars().count() <= MAX_WIDTH {
        return single_line;
    }
    let truncated: String = single_line.chars().take(MAX_WIDTH).collect();
    format!("{truncated}...")
}

#[cfg(test)]
mod tests {
    use super::{value_preview, value_to_json, value_to_json_with_result_directory, LineIndex};
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

    #[test]
    fn large_tables_stream_complete_rows_when_a_result_directory_is_configured() {
        let root = std::env::temp_dir().join(format!("biolang-result-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let table = Value::Table(Table::new(
            vec!["value".into()],
            (0..501).map(|value| vec![Value::Int(value)]).collect(),
        ));
        let json = value_to_json_with_result_directory(&table, Some(&root));
        assert_eq!(json["encoding"], "jsonl");
        assert!(json["dataRef"].as_str().unwrap().ends_with("table-1.jsonl"));
        assert_eq!(
            std::fs::read_to_string(root.join("table-1.jsonl"))
                .unwrap()
                .lines()
                .count(),
            501
        );
        let _ = std::fs::remove_dir_all(root);
    }
    #[test]
    fn line_index_maps_offsets_to_one_based_lines() {
        let source = "let a = 1
let b = 2

println(b)
";
        let index = LineIndex::new(source);
        assert_eq!(index.line_of(0), 1);
        assert_eq!(index.line_of(source.find("let b").unwrap()), 2);
        assert_eq!(index.line_of(source.find("println").unwrap()), 4);
    }

    #[test]
    fn value_preview_collapses_newlines_and_truncates() {
        let long = Value::Str("x".repeat(200));
        let preview = value_preview(&long);
        assert!(preview.ends_with("..."));
        assert_eq!(
            value_preview(&Value::Str(
                "a
b"
                .into()
            )),
            "a b"
        );
    }

    #[test]
    fn value_preview_summarises_plots_rather_than_dumping_them() {
        let svg = Value::Str("<svg><rect /></svg>".into());
        assert_eq!(value_preview(&svg), "<plot>");
    }
}
