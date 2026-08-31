//! BioLang AST to JavaScript SDK source.
//!
//! The generated program contains no embedded BioLang source. JavaScript
//! constructs the same AST through the public `biolang` builder API; the
//! resulting program is still evaluated by the Rust runtime.

use bl_core::ast::{Arg, BinaryOp, Expr, ForPattern, Param, Program, RecordEntry, Stmt, UnaryOp};
use bl_core::span::Spanned;
use bl_lexer::Lexer;
use bl_parser::Parser;

pub fn transpile(source: &str) -> Result<String, String> {
    let tokens = Lexer::new(source)
        .tokenize()
        .map_err(|error| error.message)?;
    let parsed = Parser::new(tokens).parse().map_err(|error| error.message)?;
    if parsed.has_errors() {
        return Err(parsed
            .errors
            .into_iter()
            .map(|error| error.message)
            .collect::<Vec<_>>()
            .join("; "));
    }
    emit_program(&parsed.program)
}

fn emit_program(program: &Program) -> Result<String, String> {
    let items = program
        .stmts
        .iter()
        .map(emit_stmt)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(format!(
        "// `bl` is the persistent BioLang session; `bio` is the JavaScript SDK.\nconst result = await bio.program(\n{}\n).run(bl);\nresult;",
        indent_join(&items)
    ))
}

fn emit_stmt(stmt: &Spanned<Stmt>) -> Result<String, String> {
    Ok(match &stmt.node {
        Stmt::Let { name, value, .. } => {
            format!("bio.let_({}, {})", quote(name), emit_expr(value)?)
        }
        Stmt::Const { name, value, .. } => {
            format!("bio.const_({}, {})", quote(name), emit_expr(value)?)
        }
        Stmt::Assign { name, value } => {
            format!("bio.assign({}, {})", quote(name), emit_expr(value)?)
        }
        Stmt::IndexAssign { name, index, value } => format!(
            "bio.indexAssign({}, {}, {})",
            quote(name),
            emit_expr(index)?,
            emit_expr(value)?
        ),
        Stmt::Expr(value) => format!("bio.expr_({})", emit_expr(value)?),
        Stmt::Return(value) => match value {
            Some(value) => format!("bio.return_({})", emit_expr(value)?),
            None => "bio.return_()".into(),
        },
        Stmt::Fn {
            name,
            params,
            body,
            is_async,
            is_generator,
            ..
        } => format!(
            "bio.function_({}, [{}], [{}], {{ async: {}, generator: {} }})",
            quote(name),
            emit_params(params)?,
            emit_stmts(body)?,
            is_async,
            is_generator
        ),
        Stmt::For {
            pattern,
            iter,
            when_guard,
            body,
            else_body,
        } => format!(
            "bio.for_({}, {}, [{}], {})",
            emit_for_pattern(pattern),
            emit_expr(iter)?,
            emit_stmts(body)?,
            emit_options(&[
                (
                    "when",
                    when_guard
                        .as_ref()
                        .map(|value| emit_expr(value))
                        .transpose()?
                ),
                (
                    "elseBody",
                    else_body
                        .as_ref()
                        .map(|values| emit_stmts(values).map(|v| format!("[{v}]")))
                        .transpose()?
                ),
            ])
        ),
        Stmt::While { condition, body } => format!(
            "bio.while_({}, [{}])",
            emit_expr(condition)?,
            emit_stmts(body)?
        ),
        Stmt::Break => "bio.break_()".into(),
        Stmt::Continue => "bio.continue_()".into(),
        Stmt::Assert { condition, message } => format!(
            "bio.assert_({}, {})",
            emit_expr(condition)?,
            option_expr(message.as_ref())?
        ),
        Stmt::Import { path, alias } => format!(
            "bio.import_({}, {})",
            quote(path),
            option_string(alias.as_ref())
        ),
        Stmt::FromImport { path, names } => format!(
            "bio.fromImport({}, [{}])",
            quote(path),
            names
                .iter()
                .map(|name| quote(name))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Stmt::NilAssign { name, value } => {
            format!("bio.nilAssign({}, {})", quote(name), emit_expr(value)?)
        }
        Stmt::Yield(value) => format!("bio.yield_({})", emit_expr(value)?),
        Stmt::Defer(value) => format!("bio.defer_({})", emit_expr(value)?),
        other => {
            return Err(format!(
                "JavaScript frontend does not yet support statement {other:?}"
            ))
        }
    })
}

fn emit_expr(expr: &Spanned<Expr>) -> Result<String, String> {
    Ok(match &expr.node {
        Expr::Nil => "null".into(),
        Expr::Bool(value) => value.to_string(),
        Expr::Int(value) => value.to_string(),
        Expr::Float(value) => {
            let text = value.to_string();
            if text.contains(['.', 'e', 'E']) {
                text
            } else {
                format!("{text}.0")
            }
        }
        Expr::Str(value) => quote(value),
        Expr::DnaLit(value) => format!("bio.dna({})", quote(value)),
        Expr::RnaLit(value) => format!("bio.rna({})", quote(value)),
        Expr::ProteinLit(value) => format!("bio.protein({})", quote(value)),
        Expr::QualLit(value) => format!("bio.quality({})", quote(value)),
        Expr::Ident(name) => format!("bio.ref({})", quote(name)),
        Expr::Unary { op, expr } => format!(
            "bio.unary({}, {})",
            quote(unary_name(*op)),
            emit_expr(expr)?
        ),
        Expr::Binary { op, left, right } => format!(
            "bio.binary({}, {}, {})",
            quote(binary_name(*op)),
            emit_expr(left)?,
            emit_expr(right)?
        ),
        Expr::Pipe { left, right } => {
            format!("bio.pipe({}, {})", emit_expr(left)?, emit_expr(right)?)
        }
        Expr::PipeInto { value, name } => {
            format!("bio.pipeInto({}, {})", emit_expr(value)?, quote(name))
        }
        Expr::Call { callee, args } => emit_call(callee, args)?,
        Expr::Field {
            object,
            field,
            optional,
        } => format!(
            "bio.field({}, {}, {})",
            emit_expr(object)?,
            quote(field),
            optional
        ),
        Expr::Index { object, index } => {
            format!("bio.index({}, {})", emit_expr(object)?, emit_expr(index)?)
        }
        Expr::Lambda { params, body } => format!(
            "bio.lambdaExpr([{}], {})",
            emit_params(params)?,
            emit_expr(body)?
        ),
        Expr::Block(body) => format!("bio.blockExpr([{}])", emit_stmts(body)?),
        Expr::If {
            condition,
            then_body,
            else_body,
        } => format!(
            "bio.ifExpr({}, [{}], {})",
            emit_expr(condition)?,
            emit_stmts(then_body)?,
            match else_body {
                Some(body) => format!("[{}]", emit_stmts(body)?),
                None => "null".into(),
            }
        ),
        Expr::List(values) => format!("[{}]", emit_exprs(values)?),
        Expr::TupleLit(values) => format!("bio.tuple([{}])", emit_exprs(values)?),
        Expr::SetLiteral(values) => format!("bio.set([{}])", emit_exprs(values)?),
        Expr::Record(entries) => emit_record(entries)?,
        Expr::Formula(value) => format!("bio.formula({})", emit_expr(value)?),
        Expr::NullCoalesce { left, right } => {
            format!("bio.coalesce({}, {})", emit_expr(left)?, emit_expr(right)?)
        }
        Expr::Range {
            start,
            end,
            inclusive,
        } => format!(
            "bio.range({}, {}, {{ inclusive: {} }})",
            emit_expr(start)?,
            emit_expr(end)?,
            inclusive
        ),
        Expr::Ternary {
            value,
            condition,
            else_value,
        } => format!(
            "bio.ternary({}, {}, {})",
            emit_expr(condition)?,
            emit_expr(value)?,
            emit_expr(else_value)?
        ),
        Expr::In {
            left,
            right,
            negated,
        } => format!(
            "bio.in_({}, {}, {{ negated: {} }})",
            emit_expr(left)?,
            emit_expr(right)?,
            negated
        ),
        Expr::TypeCast { expr, target } => {
            format!("bio.cast({}, {})", emit_expr(expr)?, quote(target))
        }
        Expr::Slice {
            object,
            start,
            end,
            step,
        } => format!(
            "bio.slice({}, {}, {}, {})",
            emit_expr(object)?,
            option_expr(start.as_deref())?,
            option_expr(end.as_deref())?,
            option_expr(step.as_deref())?
        ),
        Expr::TapPipe { left, right } => {
            format!("bio.tapPipe({}, {})", emit_expr(left)?, emit_expr(right)?)
        }
        other => {
            return Err(format!(
                "JavaScript frontend does not yet support expression {other:?}"
            ))
        }
    })
}

fn emit_call(callee: &Spanned<Expr>, args: &[Arg]) -> Result<String, String> {
    let args = args
        .iter()
        .map(|arg| {
            let value = emit_expr(&arg.value)?;
            Ok(if arg.spread {
                format!("bio.spread({value})")
            } else if let Some(name) = &arg.name {
                format!("bio.named({}, {value})", quote(name))
            } else {
                value
            })
        })
        .collect::<Result<Vec<_>, String>>()?
        .join(", ");
    match &callee.node {
        Expr::Ident(name) => Ok(format!("bio.callExpr({}, [{}])", quote(name), args)),
        _ => Ok(format!("bio.invoke({}, [{}])", emit_expr(callee)?, args)),
    }
}

fn emit_record(entries: &[RecordEntry]) -> Result<String, String> {
    let values = entries
        .iter()
        .map(|entry| {
            Ok(match entry {
                RecordEntry::Field(name, value) => {
                    format!("bio.fieldEntry({}, {})", quote(name), emit_expr(value)?)
                }
                RecordEntry::Spread(value) => format!("bio.spreadEntry({})", emit_expr(value)?),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(format!("bio.record([{}])", values.join(", ")))
}

fn emit_params(params: &[Param]) -> Result<String, String> {
    params
        .iter()
        .map(|param| {
            Ok(format!(
                "bio.param({}, {})",
                quote(&param.name),
                emit_options(&[
                    (
                        "default",
                        param.default.as_ref().map(emit_expr).transpose()?
                    ),
                    ("rest", param.rest.then(|| "true".into())),
                ])
            ))
        })
        .collect::<Result<Vec<_>, String>>()
        .map(|values| values.join(", "))
}

fn emit_stmts(values: &[Spanned<Stmt>]) -> Result<String, String> {
    values
        .iter()
        .map(emit_stmt)
        .collect::<Result<Vec<_>, _>>()
        .map(|values| values.join(", "))
}

fn emit_exprs(values: &[Spanned<Expr>]) -> Result<String, String> {
    values
        .iter()
        .map(emit_expr)
        .collect::<Result<Vec<_>, _>>()
        .map(|values| values.join(", "))
}

fn option_expr(value: Option<&Spanned<Expr>>) -> Result<String, String> {
    value
        .map(emit_expr)
        .transpose()
        .map(|value| value.unwrap_or_else(|| "null".into()))
}

fn option_string(value: Option<&String>) -> String {
    value
        .map(|value| quote(value))
        .unwrap_or_else(|| "null".into())
}

fn emit_options(values: &[(&str, Option<String>)]) -> String {
    let fields = values
        .iter()
        .filter_map(|(name, value)| value.as_ref().map(|value| format!("{name}: {value}")))
        .collect::<Vec<_>>();
    format!("{{{}}}", fields.join(", "))
}

fn emit_for_pattern(pattern: &ForPattern) -> String {
    match pattern {
        ForPattern::Single(name) => quote(name),
        ForPattern::ListDestr(names) => format!(
            "bio.listPattern([{}])",
            names
                .iter()
                .map(|name| quote(name))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ForPattern::RecordDestr(names) => format!(
            "bio.recordPattern([{}])",
            names
                .iter()
                .map(|name| quote(name))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ForPattern::TupleDestr(names) => format!(
            "bio.tuplePattern([{}])",
            names
                .iter()
                .map(|name| quote(name))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn unary_name(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::Not => "!",
    }
}
fn binary_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Pow => "**",
        BinaryOp::Eq => "==",
        BinaryOp::Neq => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Gt => ">",
        BinaryOp::Le => "<=",
        BinaryOp::Ge => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
        BinaryOp::BitAnd => "&",
        BinaryOp::BitXor => "^",
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
        BinaryOp::Concat => "++",
    }
}

fn quote(value: &str) -> String {
    serde_json::to_string(value).expect("strings always serialize")
}
fn indent_join(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("  {item},"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::transpile;

    #[test]
    fn emits_structural_javascript_without_embedding_source() {
        let js = transpile("let measurements = [12, 14, 15]\nsummary(measurements)").unwrap();
        assert!(js.contains("bio.let_(\"measurements\", [12, 14, 15])"));
        assert!(js.contains("bio.callExpr(\"summary\", [bio.ref(\"measurements\")])"));
        assert!(!js.contains("let measurements = [12, 14, 15]"));
    }

    #[test]
    fn emits_pipes_records_lambdas_and_named_arguments() {
        let js = transpile(
            "rows |> filter(|row| row.Age >= 18) |> histogram({bins: 20}, format: \"svg\")",
        )
        .unwrap();
        assert!(js.contains("bio.pipe("));
        assert!(js.contains("bio.lambdaExpr"));
        assert!(js.contains("bio.record"));
        assert!(js.contains("bio.named(\"format\", \"svg\")"));
    }
}
