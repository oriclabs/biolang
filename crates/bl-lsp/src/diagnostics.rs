use bl_lexer::Lexer;
use bl_parser::Parser;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

/// Parse source text and return LSP diagnostics for any errors.
pub fn diagnose(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let tokens = match Lexer::new(source).tokenize() {
        Ok(t) => t,
        Err(e) => {
            let (line, col) = offset_to_line_col(source, e.span.map(|s| s.start).unwrap_or(0));
            diagnostics.push(Diagnostic {
                range: Range::new(
                    Position::new(line, col),
                    Position::new(line, col + 1),
                ),
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("biolang".into()),
                message: e.message.clone(),
                ..Default::default()
            });
            return diagnostics;
        }
    };

    match Parser::new(tokens).parse() {
        Ok(result) => {
            for e in &result.errors {
                let (line, col) =
                    offset_to_line_col(source, e.span.map(|s| s.start).unwrap_or(0));
                diagnostics.push(Diagnostic {
                    range: Range::new(
                        Position::new(line, col),
                        Position::new(line, col + 1),
                    ),
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("biolang".into()),
                    message: e.message.clone(),
                    ..Default::default()
                });
            }
        }
        Err(e) => {
            let (line, col) =
                offset_to_line_col(source, e.span.map(|s| s.start).unwrap_or(0));
            diagnostics.push(Diagnostic {
                range: Range::new(
                    Position::new(line, col),
                    Position::new(line, col + 1),
                ),
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("biolang".into()),
                message: e.message.clone(),
                ..Default::default()
            });
        }
    }

    diagnostics
}

/// Convert a byte offset to (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}
