use bl_core::ast::Stmt;
use bl_core::span::Spanned;
use bl_core::value::Value;
use bl_runtime::Interpreter;

/// Extracted symbol from a BioLang source file.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: u32,
    pub col: u32,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,
    Function,
    Parameter,
    Import,
    Enum,
}

/// Extract symbols (variables, functions, imports) from a parsed program.
pub fn extract_symbols(program: &[Spanned<Stmt>]) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    for stmt in program {
        extract_from_stmt(&stmt.node, stmt.span.start as u32, &mut symbols);
    }
    symbols
}

fn extract_from_stmt(stmt: &Stmt, offset: u32, symbols: &mut Vec<Symbol>) {
    match stmt {
        Stmt::Let { name, .. } => {
            symbols.push(Symbol {
                name: name.clone(),
                kind: SymbolKind::Variable,
                line: offset,
                col: 0,
                doc: None,
            });
        }
        Stmt::Fn {
            name, params, doc, ..
        } => {
            symbols.push(Symbol {
                name: name.clone(),
                kind: SymbolKind::Function,
                line: offset,
                col: 0,
                doc: doc.clone(),
            });
            for p in params {
                symbols.push(Symbol {
                    name: p.name.clone(),
                    kind: SymbolKind::Parameter,
                    line: offset,
                    col: 0,
                    doc: None,
                });
            }
        }
        Stmt::Import { alias, .. } => {
            if let Some(alias) = alias {
                symbols.push(Symbol {
                    name: alias.clone(),
                    kind: SymbolKind::Import,
                    line: offset,
                    col: 0,
                    doc: None,
                });
            }
        }
        Stmt::Enum { name, .. } => {
            symbols.push(Symbol {
                name: name.clone(),
                kind: SymbolKind::Enum,
                line: offset,
                col: 0,
                doc: None,
            });
        }
        _ => {}
    }
}

/// Collect all builtin function names by creating a fresh interpreter and
/// listing its NativeFunction entries.
pub fn builtin_names() -> Vec<String> {
    let interp = Interpreter::new();
    interp
        .env()
        .list_global_vars()
        .into_iter()
        .filter_map(|(name, val)| {
            if matches!(val, Value::NativeFunction { .. }) {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Check if a name is a builtin function.
pub fn is_builtin(name: &str) -> bool {
    let interp = Interpreter::new();
    interp
        .env()
        .list_global_vars()
        .into_iter()
        .any(|(n, val)| n == name && matches!(val, Value::NativeFunction { .. }))
}

/// BioLang keywords for completion.
pub fn keywords() -> Vec<&'static str> {
    vec![
        "let", "fn", "if", "else", "for", "in", "while", "break", "continue", "return",
        "match", "import", "true", "false", "nil", "and", "or", "not", "try", "catch",
        "pipeline", "assert", "yield", "enum",
    ]
}
