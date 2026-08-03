mod notebook;
mod python;
mod r;

use bl_core::error::BioLangError;
use bl_lexer::Lexer;
use bl_parser::Parser;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub source_format: String,
    pub source_name: String,
    pub source_content: String,
    pub suggested_name: String,
    pub notebook: bool,
    pub content: String,
    pub validation: ValidationReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    pub valid: bool,
    pub units_checked: usize,
    pub diagnostics: Vec<ValidationDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationDiagnostic {
    pub unit: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub rendered: String,
}

/// Detect the source language/format from file extension.
pub fn detect_language(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("py") => Some("python"),
        Some("r") | Some("R") => Some("r"),
        Some("ipynb") => Some("ipynb"),
        Some("rmd") | Some("Rmd") | Some("RMD") => Some("rmd"),
        _ => None,
    }
}

/// Convert source code from `lang` to BioLang source (.bl).
pub fn convert(source: &str, lang: &str, filename: &str) -> String {
    match lang {
        "python" | "py" => python::convert(source, filename),
        "r" => r::convert(source, filename),
        _ => format!(
            "# Cannot convert: unknown language '{}'\n# Source: {}\n\n{}",
            lang, filename, source
        ),
    }
}

/// Convert source code for use as a notebook cell — strips the file-level header block
/// so the output is just the converted code without repeating metadata on every cell.
pub fn convert_cell(source: &str, lang: &str, cell_ref: &str) -> String {
    let raw = convert(source, lang, cell_ref);
    strip_file_header(&raw)
}

/// Strip the leading comment header block that `convert()` prepends to every file.
/// The header is the first contiguous block of comments separated from content by a blank line.
fn strip_file_header(s: &str) -> String {
    // Find the first blank line — everything before it is the auto-generated header
    let mut past_header = false;
    let mut out_lines: Vec<&str> = Vec::new();

    for line in s.lines() {
        if past_header {
            out_lines.push(line);
        } else if line.trim().is_empty() {
            // First blank line marks end of header block
            past_header = true;
        }
        // else: header comment line — skip it
    }

    // Trailing footer: drop "# Conversion complete…" summary line from cell output
    while out_lines
        .last()
        .map(|l| l.starts_with("# Conversion complete"))
        .unwrap_or(false)
    {
        out_lines.pop();
    }
    // Drop trailing blank lines
    while out_lines
        .last()
        .map(|l| l.trim().is_empty())
        .unwrap_or(false)
    {
        out_lines.pop();
    }

    out_lines.join("\n")
}

/// Convert a notebook format to a BioLang notebook (.bln).
pub fn convert_notebook(source: &str, format: &str, filename: &str) -> String {
    match format {
        "ipynb" => notebook::ipynb_to_bln(source, filename),
        "rmd" => notebook::rmd_to_bln(source, filename),
        _ => format!(
            "# Cannot convert: unknown notebook format '{}'\n# Source: {}\n\n{}",
            format, filename, source
        ),
    }
}

/// Whether the detected language is a notebook format.
pub fn is_notebook_format(lang: &str) -> bool {
    matches!(lang, "ipynb" | "rmd")
}

/// Convert one supported source document and validate the generated BioLang syntax.
pub fn import_source(source: &str, format: &str, filename: &str) -> Result<ImportResult, String> {
    let format =
        normalize_format(format).ok_or_else(|| format!("Unsupported import format '{format}'"))?;
    let notebook = is_notebook_format(format);
    let content = if notebook {
        convert_notebook(source, format, filename)
    } else {
        convert(source, format, filename)
    };
    if content.starts_with("# ERROR:") {
        return Err(content
            .lines()
            .next()
            .unwrap_or("Notebook conversion failed")
            .to_string());
    }
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("imported");
    let validation = validate_biolang(&content, notebook);
    Ok(ImportResult {
        source_format: format.to_string(),
        source_name: filename.to_string(),
        source_content: source.to_string(),
        suggested_name: format!("{stem}.{}", if notebook { "bln" } else { "bl" }),
        notebook,
        content,
        validation,
    })
}

pub fn normalize_format(format: &str) -> Option<&'static str> {
    match format.trim().to_ascii_lowercase().as_str() {
        "python" | "py" => Some("python"),
        "r" => Some("r"),
        "jupyter" | "ipynb" => Some("ipynb"),
        "rmarkdown" | "r-markdown" | "rmd" => Some("rmd"),
        _ => None,
    }
}

/// Validate generated `.bl` source or each `biolang` code fence in a `.bln` notebook.
pub fn validate_biolang(source: &str, notebook: bool) -> ValidationReport {
    let units = if notebook {
        notebook_units(source)
    } else {
        vec![("script".to_string(), source.to_string(), 1)]
    };
    let units_checked = units.len();
    let diagnostics = units
        .into_iter()
        .flat_map(|(unit, code, start_line)| validate_unit(&unit, &code, start_line))
        .collect::<Vec<_>>();
    ValidationReport {
        valid: diagnostics.is_empty(),
        units_checked,
        diagnostics,
    }
}

fn validate_unit(unit: &str, source: &str, start_line: usize) -> Vec<ValidationDiagnostic> {
    let tokens = match Lexer::new(source).tokenize() {
        Ok(tokens) => tokens,
        Err(error) => return vec![diagnostic(unit, source, start_line, &error)],
    };
    let parsed = match Parser::new(tokens).parse() {
        Ok(parsed) => parsed,
        Err(error) => return vec![diagnostic(unit, source, start_line, &error)],
    };
    parsed
        .errors
        .iter()
        .map(|error| diagnostic(unit, source, start_line, error))
        .collect()
}

fn diagnostic(
    unit: &str,
    source: &str,
    start_line: usize,
    error: &BioLangError,
) -> ValidationDiagnostic {
    let (local_line, column) = error
        .span
        .map(|span| line_column(source, span.start))
        .unwrap_or((1, 1));
    ValidationDiagnostic {
        unit: unit.to_string(),
        line: start_line + local_line - 1,
        column,
        message: error.message.clone(),
        rendered: error.format_with_source(source),
    }
}

fn line_column(source: &str, offset: usize) -> (usize, usize) {
    // Walk char boundaries so we never slice mid-codepoint regardless of what
    // offset the lexer/parser stored (byte vs char ambiguity, or clamping edge cases).
    let bounded = source
        .char_indices()
        .take_while(|(i, _)| *i <= offset)
        .last()
        .map(|(i, _)| i)
        .unwrap_or(0);
    let prefix = &source[..bounded];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit_once('\n')
        .map_or(prefix.chars().count() + 1, |(_, tail)| {
            tail.chars().count() + 1
        });
    (line, column)
}

fn notebook_units(source: &str) -> Vec<(String, String, usize)> {
    let mut units = Vec::new();
    let mut code = String::new();
    let mut in_biolang = false;
    let mut start_line = 1;
    for (index, line) in source.lines().enumerate() {
        if !in_biolang {
            let language = line
                .trim_start()
                .strip_prefix("```")
                .map(str::trim)
                .unwrap_or_default();
            if matches!(language, "biolang" | "bio") {
                in_biolang = true;
                start_line = index + 2;
                code.clear();
            }
        } else if line.trim() == "```" {
            let unit = format!("cell {}", units.len() + 1);
            units.push((unit, std::mem::take(&mut code), start_line));
            in_biolang = false;
        } else {
            code.push_str(line);
            code.push('\n');
        }
    }
    if in_biolang {
        units.push((format!("cell {}", units.len() + 1), code, start_line));
    }
    units
}

#[cfg(test)]
mod tests {
    use super::{import_source, validate_biolang};

    #[test]
    fn imports_and_validates_python_scripts() {
        let result = import_source("x = 1\nprint(x)\n", "python", "analysis.py").unwrap();
        assert_eq!(result.suggested_name, "analysis.bl");
        assert_eq!(result.source_content, "x = 1\nprint(x)\n");
        assert!(!result.notebook);
        assert!(result.validation.units_checked > 0);
        assert!(
            result.validation.valid,
            "{:?}",
            result.validation.diagnostics
        );
    }

    #[test]
    fn imports_r_scripts() {
        let result = import_source("x <- 2\nprint(x)\n", "r", "analysis.R").unwrap();
        assert_eq!(result.suggested_name, "analysis.bl");
        assert!(!result.notebook);
        assert_eq!(result.validation.units_checked, 1);
        assert!(
            result.validation.valid,
            "{:?}",
            result.validation.diagnostics
        );
    }

    #[test]
    fn imports_jupyter_notebooks() {
        let source = r##"{
          "cells": [
            {"cell_type":"markdown","metadata":{},"source":["# Analysis"]},
            {"cell_type":"code","execution_count":null,"metadata":{},"outputs":[],"source":["x = 3\n","print(x)\n"]}
          ],
          "metadata":{"kernelspec":{"language":"python","name":"python3"}},
          "nbformat":4,
          "nbformat_minor":5
        }"##;
        let result = import_source(source, "jupyter", "analysis.ipynb").unwrap();
        assert_eq!(result.suggested_name, "analysis.bln");
        assert!(result.notebook);
        assert_eq!(result.validation.units_checked, 1);
        assert!(
            result.validation.valid,
            "{:?}",
            result.validation.diagnostics
        );
    }

    #[test]
    fn imports_r_markdown_notebooks() {
        let source = "# Analysis\n\n```{r}\nx <- 4\nprint(x)\n```\n";
        let result = import_source(source, "rmarkdown", "analysis.Rmd").unwrap();
        assert_eq!(result.suggested_name, "analysis.bln");
        assert!(result.notebook);
        assert_eq!(result.validation.units_checked, 1);
        assert!(
            result.validation.valid,
            "{:?}",
            result.validation.diagnostics
        );
    }

    #[test]
    fn validates_each_notebook_cell() {
        let report = validate_biolang(
            "# Notebook\n\n```biolang\nlet x = 1\n```\n\n```biolang\nlet =\n```\n",
            true,
        );
        assert!(!report.valid);
        assert_eq!(report.units_checked, 2);
        assert_eq!(report.diagnostics[0].unit, "cell 2");
        assert!(report.diagnostics[0].line >= 7);
    }
}
