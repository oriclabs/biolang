use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};

// ── Public API ─────────────────────────────────────────────────

pub fn markdown_builtin_list() -> Vec<(&'static str, Arity)> {
    let mut builtins = vec![
        ("to_markdown", Arity::Exact(1)),
        ("to_html", Arity::Exact(1)),
    ];
    #[cfg(feature = "native")]
    {
        builtins.push(("write_markdown", Arity::Exact(2)));
        builtins.push(("write_html", Arity::Exact(2)));
    }
    builtins
}

pub fn is_markdown_builtin(name: &str) -> bool {
    match name {
        "to_markdown" | "to_html" => true,
        #[cfg(feature = "native")]
        "write_markdown" | "write_html" => true,
        _ => false,
    }
}

pub fn call_markdown_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "to_markdown" => builtin_to_markdown(args),
        "to_html" => builtin_to_html(args),
        #[cfg(feature = "native")]
        "write_markdown" => builtin_write_markdown(args),
        #[cfg(feature = "native")]
        "write_html" => builtin_write_html(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown markdown builtin '{name}'"),
            None,
        )),
    }
}

// ── Markdown Conversion ────────────────────────────────────────

/// Convert any BioLang value to a Markdown string.
pub fn value_to_markdown(v: &Value) -> String {
    match v {
        Value::Nil => String::new(),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => format_float(*f),
        Value::Str(s) => s.clone(),

        Value::DNA(seq) => format!("`{}`", seq.data),
        Value::RNA(seq) => format!("`{}`", seq.data),
        Value::Protein(seq) => format!("`{}`", seq.data),

        Value::List(items) => list_to_markdown(items),
        Value::Map(map) | Value::Record(map) => record_to_markdown(map),
        Value::Table(t) => table_to_markdown(t),

        Value::Interval(iv) => {
            format!("`{}:{}-{}`", iv.chrom, iv.start, iv.end)
        }
        Value::Variant { chrom, pos, ref_allele, alt_allele, .. } => {
            format!("`{chrom}:{pos} {ref_allele}/{alt_allele}`")
        }
        Value::Gene { symbol, description, .. } => {
            let mut s = format!("**{symbol}**");
            if !description.is_empty() {
                s.push_str(&format!(" — {description}"));
            }
            s
        }
        Value::AlignedRead(r) => {
            format!("`{} {}:{} MAPQ={}`", r.qname, r.rname, r.pos, r.mapq)
        }
        Value::Matrix(m) => matrix_to_markdown(m),

        // Fallback for other types
        other => format!("`{other}`"),
    }
}

fn format_float(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{f:.1}")
    } else {
        let s = format!("{f:.6}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn list_to_markdown(items: &[Value]) -> String {
    // If all items are records with the same keys, render as a table
    if items.len() >= 2 {
        if let Some(table_str) = try_records_as_table(items) {
            return table_str;
        }
    }
    // Otherwise render as a bullet list
    let mut lines = Vec::new();
    for item in items {
        let md = value_to_markdown(item);
        // Indent multi-line items
        let first_line = md.lines().next().unwrap_or("");
        lines.push(format!("- {first_line}"));
        for extra_line in md.lines().skip(1) {
            lines.push(format!("  {extra_line}"));
        }
    }
    lines.join("\n")
}

fn try_records_as_table(items: &[Value]) -> Option<String> {
    // Check all items are Record with same keys
    let first_keys: Vec<String> = match &items[0] {
        Value::Record(m) | Value::Map(m) => {
            let mut keys: Vec<String> = m.keys().cloned().collect();
            keys.sort();
            keys
        }
        _ => return None,
    };
    if first_keys.is_empty() {
        return None;
    }
    for item in &items[1..] {
        match item {
            Value::Record(m) | Value::Map(m) => {
                let mut keys: Vec<String> = m.keys().cloned().collect();
                keys.sort();
                if keys != first_keys {
                    return None;
                }
            }
            _ => return None,
        }
    }

    // Render as markdown table
    let mut lines = Vec::new();
    // Header
    let header: Vec<String> = first_keys.iter().map(|k| format!(" {k} ")).collect();
    lines.push(format!("|{}|", header.join("|")));
    // Separator
    let sep: Vec<String> = first_keys.iter().map(|k| "-".repeat(k.len() + 2)).collect();
    lines.push(format!("|{}|", sep.join("|")));
    // Rows
    for item in items {
        if let Value::Record(m) | Value::Map(m) = item {
            let cells: Vec<String> = first_keys
                .iter()
                .map(|k| {
                    let val = m.get(k).unwrap_or(&Value::Nil);
                    format!(" {} ", cell_value(val))
                })
                .collect();
            lines.push(format!("|{}|", cells.join("|")));
        }
    }
    Some(lines.join("\n"))
}

fn cell_value(v: &Value) -> String {
    match v {
        Value::Nil => String::new(),
        Value::Str(s) => s.clone(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => format_float(*f),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::DNA(seq) => format!("`{}`", seq.data),
        Value::RNA(seq) => format!("`{}`", seq.data),
        Value::Protein(seq) => format!("`{}`", seq.data),
        other => format!("{other}"),
    }
}

fn record_to_markdown(map: &std::collections::HashMap<String, Value>) -> String {
    let mut lines = Vec::new();
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();
    for key in keys {
        let val = &map[key];
        let md = value_to_markdown(val);
        if md.contains('\n') {
            lines.push(format!("**{key}**:\n"));
            lines.push(md);
            lines.push(String::new());
        } else {
            lines.push(format!("**{key}**: {md}"));
        }
    }
    lines.join("\n")
}

fn table_to_markdown(t: &bl_core::value::Table) -> String {
    let cols = &t.columns;
    let mut lines = Vec::new();

    // Header
    let header: Vec<String> = cols.iter().map(|c| format!(" {c} ")).collect();
    lines.push(format!("|{}|", header.join("|")));

    // Separator with alignment
    let sep: Vec<String> = cols.iter().map(|c| "-".repeat(c.len() + 2)).collect();
    lines.push(format!("|{}|", sep.join("|")));

    // Rows
    for row in &t.rows {
        let cells: Vec<String> = row.iter().map(|v| format!(" {} ", cell_value(v))).collect();
        lines.push(format!("|{}|", cells.join("|")));
    }
    lines.join("\n")
}

fn matrix_to_markdown(m: &bl_core::matrix::Matrix) -> String {
    let nrows = m.nrow;
    let ncols = m.ncol;
    let mut lines = Vec::new();

    // Header with column indices
    let mut header = vec![" ".to_string()];
    for c in 0..ncols.min(20) {
        header.push(format!(" {c} "));
    }
    if ncols > 20 {
        header.push(" ... ".to_string());
    }
    lines.push(format!("|{}|", header.join("|")));

    let mut sep = vec!["---".to_string()];
    for _ in 0..ncols.min(20) {
        sep.push("---".to_string());
    }
    if ncols > 20 {
        sep.push("---".to_string());
    }
    lines.push(format!("|{}|", sep.join("|")));

    // Rows (limit to 50)
    for r in 0..nrows.min(50) {
        let mut cells = vec![format!(" **{r}** ")];
        for c in 0..ncols.min(20) {
            cells.push(format!(" {} ", format_float(m.data[r * ncols + c])));
        }
        if ncols > 20 {
            cells.push(" ... ".to_string());
        }
        lines.push(format!("|{}|", cells.join("|")));
    }
    if nrows > 50 {
        lines.push(format!("| ... | *({} more rows)* |", nrows - 50));
    }
    lines.join("\n")
}

// ── HTML Conversion ────────────────────────────────────────────

/// Convert any BioLang value to a self-contained HTML report.
pub fn value_to_html(v: &Value, title: Option<&str>) -> String {
    let title = title.unwrap_or("BioLang Report");
    let body = value_to_html_body(v);
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif; max-width: 960px; margin: 2rem auto; padding: 0 1rem; color: #1e293b; line-height: 1.6; }}
  h1 {{ color: #0f172a; border-bottom: 2px solid #e2e8f0; padding-bottom: 0.5rem; }}
  table {{ border-collapse: collapse; width: 100%; margin: 1rem 0; font-size: 0.875rem; }}
  th {{ background: #f1f5f9; text-align: left; padding: 0.5rem 0.75rem; border: 1px solid #e2e8f0; font-weight: 600; }}
  td {{ padding: 0.5rem 0.75rem; border: 1px solid #e2e8f0; }}
  tr:nth-child(even) {{ background: #f8fafc; }}
  tr:hover {{ background: #e0f2fe; }}
  code {{ background: #f1f5f9; padding: 0.125rem 0.375rem; border-radius: 0.25rem; font-size: 0.85em; }}
  .kv-table {{ max-width: 600px; }}
  .kv-table th {{ width: 30%; }}
  ul {{ padding-left: 1.5rem; }}
  li {{ margin: 0.25rem 0; }}
  .meta {{ color: #64748b; font-size: 0.8rem; margin-top: 2rem; border-top: 1px solid #e2e8f0; padding-top: 0.5rem; }}
</style>
</head>
<body>
<h1>{title}</h1>
{body}
<div class="meta">Generated by BioLang</div>
</body>
</html>"#
    )
}

fn value_to_html_body(v: &Value) -> String {
    match v {
        Value::Nil => String::new(),
        Value::Bool(b) => format!("<p>{b}</p>"),
        Value::Int(n) => format!("<p>{n}</p>"),
        Value::Float(f) => format!("<p>{}</p>", format_float(*f)),
        Value::Str(s) => {
            // If it looks like SVG, embed directly
            if s.trim_start().starts_with("<svg") {
                return s.clone();
            }
            format!("<p>{}</p>", html_escape(s))
        }
        Value::DNA(seq) => format!("<p><code>{}</code></p>", seq.data),
        Value::RNA(seq) => format!("<p><code>{}</code></p>", seq.data),
        Value::Protein(seq) => format!("<p><code>{}</code></p>", seq.data),
        Value::List(items) => list_to_html(items),
        Value::Map(map) | Value::Record(map) => record_to_html(map),
        Value::Table(t) => table_to_html(t),
        Value::Interval(iv) => {
            format!("<p><code>{}:{}-{}</code></p>", iv.chrom, iv.start, iv.end)
        }
        Value::Variant { chrom, pos, ref_allele, alt_allele, .. } => {
            format!(
                "<p><code>{}:{} {}/{}</code></p>",
                html_escape(chrom),
                pos,
                html_escape(ref_allele),
                html_escape(alt_allele)
            )
        }
        Value::Gene { symbol, description, .. } => {
            let mut html = format!("<p><strong>{}</strong>", html_escape(symbol));
            if !description.is_empty() {
                html.push_str(&format!(" &mdash; {}", html_escape(description)));
            }
            html.push_str("</p>");
            html
        }
        Value::Matrix(m) => matrix_to_html(m),
        other => format!("<p><code>{other}</code></p>"),
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn list_to_html(items: &[Value]) -> String {
    // If all items are records with the same keys, render as a table
    if items.len() >= 2 {
        if let Some(html) = try_records_as_html_table(items) {
            return html;
        }
    }
    let mut html = String::from("<ul>\n");
    for item in items {
        let body = value_to_html_body(item);
        html.push_str(&format!("  <li>{body}</li>\n"));
    }
    html.push_str("</ul>");
    html
}

fn try_records_as_html_table(items: &[Value]) -> Option<String> {
    let first_keys: Vec<String> = match &items[0] {
        Value::Record(m) | Value::Map(m) => {
            let mut keys: Vec<String> = m.keys().cloned().collect();
            keys.sort();
            keys
        }
        _ => return None,
    };
    if first_keys.is_empty() {
        return None;
    }
    for item in &items[1..] {
        match item {
            Value::Record(m) | Value::Map(m) => {
                let mut keys: Vec<String> = m.keys().cloned().collect();
                keys.sort();
                if keys != first_keys {
                    return None;
                }
            }
            _ => return None,
        }
    }

    let mut html = String::from("<table>\n<thead>\n<tr>");
    for k in &first_keys {
        html.push_str(&format!("<th>{}</th>", html_escape(k)));
    }
    html.push_str("</tr>\n</thead>\n<tbody>\n");
    for item in items {
        if let Value::Record(m) | Value::Map(m) = item {
            html.push_str("<tr>");
            for k in &first_keys {
                let val = m.get(k).unwrap_or(&Value::Nil);
                html.push_str(&format!("<td>{}</td>", html_cell(val)));
            }
            html.push_str("</tr>\n");
        }
    }
    html.push_str("</tbody>\n</table>");
    Some(html)
}

fn html_cell(v: &Value) -> String {
    match v {
        Value::Nil => String::new(),
        Value::Str(s) => html_escape(s),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => format_float(*f),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::DNA(seq) => format!("<code>{}</code>", seq.data),
        Value::RNA(seq) => format!("<code>{}</code>", seq.data),
        Value::Protein(seq) => format!("<code>{}</code>", seq.data),
        other => html_escape(&format!("{other}")),
    }
}

fn record_to_html(map: &std::collections::HashMap<String, Value>) -> String {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();
    let mut html = String::from("<table class=\"kv-table\">\n<thead><tr><th>Key</th><th>Value</th></tr></thead>\n<tbody>\n");
    for key in keys {
        let val = &map[key];
        html.push_str(&format!(
            "<tr><th>{}</th><td>{}</td></tr>\n",
            html_escape(key),
            html_cell(val)
        ));
    }
    html.push_str("</tbody>\n</table>");
    html
}

fn table_to_html(t: &bl_core::value::Table) -> String {
    let mut html = String::from("<table>\n<thead>\n<tr>");
    for col in &t.columns {
        html.push_str(&format!("<th>{}</th>", html_escape(col)));
    }
    html.push_str("</tr>\n</thead>\n<tbody>\n");
    for row in &t.rows {
        html.push_str("<tr>");
        for v in row {
            html.push_str(&format!("<td>{}</td>", html_cell(v)));
        }
        html.push_str("</tr>\n");
    }
    html.push_str("</tbody>\n</table>");
    html
}

fn matrix_to_html(m: &bl_core::matrix::Matrix) -> String {
    let nrows = m.nrow;
    let ncols = m.ncol;
    let max_rows = nrows.min(100);
    let max_cols = ncols.min(30);

    let mut html = String::from("<table>\n<thead>\n<tr><th></th>");
    for c in 0..max_cols {
        html.push_str(&format!("<th>{c}</th>"));
    }
    if ncols > max_cols {
        html.push_str("<th>...</th>");
    }
    html.push_str("</tr>\n</thead>\n<tbody>\n");
    for r in 0..max_rows {
        html.push_str(&format!("<tr><th>{r}</th>"));
        for c in 0..max_cols {
            html.push_str(&format!("<td>{}</td>", format_float(m.data[r * ncols + c])));
        }
        if ncols > max_cols {
            html.push_str("<td>...</td>");
        }
        html.push_str("</tr>\n");
    }
    if nrows > max_rows {
        html.push_str(&format!(
            "<tr><td colspan=\"{}\"><em>({} more rows)</em></td></tr>\n",
            max_cols + 1,
            nrows - max_rows
        ));
    }
    html.push_str("</tbody>\n</table>");
    html
}

// ── Builtin Implementations ────────────────────────────────────

fn builtin_to_markdown(args: Vec<Value>) -> Result<Value> {
    Ok(Value::Str(value_to_markdown(&args[0])))
}

fn builtin_to_html(args: Vec<Value>) -> Result<Value> {
    Ok(Value::Str(value_to_html(&args[0], None)))
}

#[cfg(feature = "native")]
fn builtin_write_markdown(args: Vec<Value>) -> Result<Value> {
    let md = value_to_markdown(&args[0]);
    let path = match &args[1] {
        Value::Str(s) => s.as_str(),
        other => {
            return Err(BioLangError::type_error(
                format!("write_markdown() path requires Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    std::fs::write(path, md).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("write_markdown() failed: {e}"),
            None,
        )
    })?;
    Ok(Value::Str(path.to_string()))
}

#[cfg(feature = "native")]
fn builtin_write_html(args: Vec<Value>) -> Result<Value> {
    let html = value_to_html(&args[0], None);
    let path = match &args[1] {
        Value::Str(s) => s.as_str(),
        other => {
            return Err(BioLangError::type_error(
                format!("write_html() path requires Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    std::fs::write(path, html).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("write_html() failed: {e}"),
            None,
        )
    })?;
    Ok(Value::Str(path.to_string()))
}

