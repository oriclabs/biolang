//! Shared, filesystem-free language services used by the native LSP and WASM
//! notebook editors. Filesystem-aware definition and rename remain in bl-lsp.

use bl_core::value::Arity;
use bl_lexer::Lexer;
use bl_parser::Parser;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageDiagnostic {
    pub severity: &'static str,
    pub message: String,
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageCompletion {
    pub label: String,
    pub kind: &'static str,
    pub detail: String,
    pub insert_text: String,
}

pub fn diagnose(source: &str) -> Vec<LanguageDiagnostic> {
    let tokens = match Lexer::new(source).tokenize() {
        Ok(tokens) => tokens,
        Err(error) => {
            let span = error.span.unwrap_or_default();
            return vec![diagnostic(source, error.message, span.start, span.end)];
        }
    };
    match Parser::new(tokens).parse() {
        Ok(result) => result
            .errors
            .into_iter()
            .map(|error| {
                let span = error.span.unwrap_or_default();
                diagnostic(source, error.message, span.start, span.end)
            })
            .collect(),
        Err(error) => {
            let span = error.span.unwrap_or_default();
            vec![diagnostic(source, error.message, span.start, span.end)]
        }
    }
}

pub fn completions(prefix: &str) -> Vec<LanguageCompletion> {
    let wanted = prefix.to_ascii_lowercase();
    let mut values = bl_runtime::builtins::all_builtin_arities()
        .into_iter()
        .filter(|(name, _)| name.to_ascii_lowercase().starts_with(&wanted))
        .map(|(name, arity)| LanguageCompletion {
            detail: signature(&name, &arity),
            insert_text: name.clone(),
            label: name,
            kind: "function",
        })
        .collect::<Vec<_>>();
    for keyword in KEYWORDS {
        if keyword.starts_with(&wanted) {
            values.push(LanguageCompletion {
                label: (*keyword).into(),
                kind: "keyword",
                detail: "BioLang keyword".into(),
                insert_text: (*keyword).into(),
            });
        }
    }
    values.sort_by(|left, right| left.label.cmp(&right.label));
    values
}

pub fn builtin_signature(name: &str) -> Option<String> {
    bl_runtime::builtins::all_builtin_arities()
        .into_iter()
        .find(|(candidate, _)| candidate == name)
        .map(|(name, arity)| signature(&name, &arity))
}

fn signature(name: &str, arity: &Arity) -> String {
    let arguments = match arity {
        Arity::Exact(count) => numbered(1, *count, false),
        Arity::AtLeast(minimum) => {
            let mut values = numbered(1, *minimum, false);
            if values.is_empty() {
                values.push_str("...args");
            } else {
                values.push_str(", ...args");
            }
            values
        }
        Arity::Range(minimum, maximum) => {
            let required = numbered(1, *minimum, false);
            let optional = numbered(*minimum + 1, *maximum, true);
            [required, optional]
                .into_iter()
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join(", ")
        }
    };
    format!("{name}({arguments})")
}

fn numbered(start: usize, end: usize, optional: bool) -> String {
    if start > end {
        return String::new();
    }
    (start..=end)
        .map(|index| format!("arg{index}{}", if optional { "?" } else { "" }))
        .collect::<Vec<_>>()
        .join(", ")
}

fn diagnostic(source: &str, message: String, start: usize, end: usize) -> LanguageDiagnostic {
    let end = end.max(start.saturating_add(1));
    let (line, column) = line_column(source, start);
    let (end_line, end_column) = line_column(source, end);
    LanguageDiagnostic {
        severity: "error",
        message,
        start,
        end,
        line,
        column,
        end_line,
        end_column,
    }
}

fn line_column(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 0;
    let mut column = 0;
    for (index, character) in source.char_indices() {
        if index >= offset {
            break;
        }
        if character == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
    }
    (line, column)
}

const KEYWORDS: &[&str] = &[
    "and", "as", "assert", "break", "const", "continue", "defer", "else", "enum", "false", "fn",
    "for", "from", "guard", "if", "impl", "import", "in", "let", "match", "nil", "not", "or",
    "parallel", "return", "stage", "struct", "true", "try", "unless", "while", "with", "yield",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_parse_locations() {
        let result = diagnose("let x = [1, 2");
        assert!(!result.is_empty());
        assert_eq!(result[0].severity, "error");
    }

    #[test]
    fn completes_builtins_and_keywords() {
        assert!(completions("mea").iter().any(|item| item.label == "mean"));
        assert!(completions("wh").iter().any(|item| item.label == "while"));
        assert!(builtin_signature("mean").is_some());
    }
}
