use bl_core::ast::*;
use bl_core::span::Span;
use bl_core::types::Type;

/// A type-check warning (not a hard error).
#[derive(Debug, Clone)]
pub struct TypeWarning {
    pub message: String,
    pub span: Option<Span>,
}

/// Gradual type checker — only validates annotated bindings.
pub struct Checker {
    warnings: Vec<TypeWarning>,
}

impl Checker {
    pub fn new() -> Self {
        Self {
            warnings: Vec::new(),
        }
    }

    /// Check a parsed program and return warnings.
    pub fn check(&mut self, program: &Program) -> Vec<TypeWarning> {
        for stmt in &program.stmts {
            self.check_stmt(&stmt.node);
        }
        std::mem::take(&mut self.warnings)
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let {
                type_ann,
                value,
                name,
            } => {
                if let Some(ann) = type_ann {
                    let expected = self.resolve_type_ann(ann);
                    if let Some(inferred) = self.infer_expr_type(&value.node) {
                        if !self.types_compatible(&expected, &inferred) {
                            self.warnings.push(TypeWarning {
                                message: format!(
                                    "type mismatch: '{name}' annotated as {expected}, but expression has type {inferred}"
                                ),
                                span: Some(value.span),
                            });
                        }
                    }
                }
            }
            Stmt::Fn {
                params, body, ..
            } => {
                // Check param defaults against annotations
                for p in params {
                    if let (Some(ann), Some(default)) = (&p.type_ann, &p.default) {
                        let expected = self.resolve_type_ann(ann);
                        if let Some(inferred) = self.infer_expr_type(&default.node) {
                            if !self.types_compatible(&expected, &inferred) {
                                self.warnings.push(TypeWarning {
                                    message: format!(
                                        "default value for '{}' has type {inferred}, expected {expected}",
                                        p.name
                                    ),
                                    span: Some(default.span),
                                });
                            }
                        }
                    }
                }
                // Check body statements recursively
                for s in body {
                    self.check_stmt(&s.node);
                }
            }
            Stmt::For { body, .. } | Stmt::While { body, .. } => {
                for s in body {
                    self.check_stmt(&s.node);
                }
            }
            Stmt::Impl { methods, .. } => {
                for m in methods {
                    self.check_stmt(&m.node);
                }
            }
            Stmt::Expr(e) => {
                if let Expr::Block(stmts) = &e.node {
                    for s in stmts {
                        self.check_stmt(&s.node);
                    }
                }
            }
            _ => {}
        }
    }

    fn resolve_type_ann(&self, ann: &TypeAnnotation) -> Type {
        match ann.name.as_str() {
            "Any" => Type::Any,
            "Nil" => Type::Nil,
            "Bool" => Type::Bool,
            "Int" => Type::Int,
            "Float" => Type::Float,
            "Str" | "String" => Type::Str,
            "DNA" => Type::DNA,
            "RNA" => Type::RNA,
            "Protein" => Type::Protein,
            "Interval" => Type::Interval,
            "Table" => Type::Table,
            "List" => Type::List,
            "Map" => Type::Map,
            "File" => Type::File,
            "Record" => Type::Record,
            "Matrix" => Type::Matrix,
            "Range" => Type::Range,
            "Set" => Type::Set,
            "Regex" => Type::Regex,
            "Stream" => Type::Stream,
            "Function" | "Fn" => Type::Function,
            "Future" => Type::Future,
            "Kmer" => Type::Kmer,
            "SparseMatrix" => Type::SparseMatrix,
            _ => Type::Any, // Unknown annotation = treat as Any
        }
    }

    /// Infer the type of a literal expression. Returns None for complex expressions.
    fn infer_expr_type(&self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Nil => Some(Type::Nil),
            Expr::Bool(_) => Some(Type::Bool),
            Expr::Int(_) => Some(Type::Int),
            Expr::Float(_) => Some(Type::Float),
            Expr::Str(_) | Expr::StringInterp(_) => Some(Type::Str),
            Expr::DnaLit(_) => Some(Type::DNA),
            Expr::RnaLit(_) => Some(Type::RNA),
            Expr::ProteinLit(_) => Some(Type::Protein),
            Expr::List(_) | Expr::ListComp { .. } => Some(Type::List),
            Expr::Record(_) | Expr::MapComp { .. } => Some(Type::Record),
            Expr::SetLiteral(_) => Some(Type::Set),
            Expr::Range { .. } => Some(Type::Range),
            Expr::Lambda { .. } => Some(Type::Function),
            Expr::Formula(_) => Some(Type::Formula),
            Expr::Regex { .. } => Some(Type::Regex),
            _ => None, // Cannot infer — skip check
        }
    }

    fn types_compatible(&self, expected: &Type, actual: &Type) -> bool {
        if matches!(expected, Type::Any) || matches!(actual, Type::Any) {
            return true;
        }
        if let Type::Optional(inner) = expected {
            if matches!(actual, Type::Nil) || self.types_compatible(inner, actual) {
                return true;
            }
        }
        if let Type::Union(types) = expected {
            return types.iter().any(|t| self.types_compatible(t, actual));
        }
        // Numeric compatibility: Int matches where Float expected
        if matches!(expected, Type::Float) && matches!(actual, Type::Int) {
            return true;
        }
        expected == actual
    }
}
