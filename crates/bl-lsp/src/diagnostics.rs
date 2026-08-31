use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

/// Parse source text and return LSP diagnostics for any errors.
pub fn diagnose(source: &str) -> Vec<Diagnostic> {
    bl_language::diagnose(source)
        .into_iter()
        .map(|item| Diagnostic {
            range: Range::new(
                Position::new(item.line, item.column),
                Position::new(item.end_line, item.end_column),
            ),
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("biolang".into()),
            message: item.message,
            ..Default::default()
        })
        .collect()
}
