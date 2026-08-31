//! BioLang AST to JavaScript SDK source.
//!
//! The generated program contains no embedded BioLang source. JavaScript
//! constructs the same AST through the public `biolang` builder API; the
//! resulting program is still evaluated by the Rust runtime.

use bl_core::ast::{
    Arg, BinaryOp, Expr, ForPattern, FormatSpec, MatchArm, Param, Pattern, Program, RecordEntry,
    Stmt, StringPart, UnaryOp,
};
use bl_core::span::Spanned;
use bl_lexer::Lexer;
use bl_parser::{Parser, SourceComment};
use std::collections::HashSet;

pub fn transpile(source: &str) -> Result<String, String> {
    let tokens = Lexer::new(source)
        .tokenize()
        .map_err(|error| error.message)?;
    let parsed = Parser::new(tokens).parse().map_err(|error| error.message)?;
    if parsed.has_errors() {
        return Err(parsed
            .errors
            .iter()
            .map(|error| error.message.clone())
            .collect::<Vec<_>>()
            .join("; "));
    }
    emit_program(&parsed.program, &parsed.comments)
}

fn emit_program(program: &Program, comments: &[SourceComment]) -> Result<String, String> {
    if let Some(source) = emit_direct_program(program, comments)? {
        return Ok(source);
    }
    Ok(format!(
        "// `bl` is the persistent BioLang session; `bio` is the JavaScript SDK.\nconst result = await bio.program(\n{}\n).run(bl);\nresult;",
        indent_join_with_comments(&program.stmts, comments)?
    ))
}

fn emit_direct_program(
    program: &Program,
    comments: &[SourceComment],
) -> Result<Option<String>, String> {
    if program.stmts.is_empty() {
        let mut lines =
            vec!["// Direct JavaScript API; computation still runs in BioLang WASM.".into()];
        for comment in comments {
            lines.push(js_line_comment(comment));
        }
        lines.push("null;".into());
        return Ok(Some(lines.join("\n")));
    }
    if !program.stmts.iter().all(|stmt| match &stmt.node {
        Stmt::Let { value, .. } | Stmt::Expr(value) => is_direct_expr(&value.node),
        _ => false,
    }) {
        return Ok(None);
    }

    let last_value = program.stmts.len().checked_sub(1);
    if !matches!(
        last_value.map(|index| &program.stmts[index].node),
        Some(Stmt::Let { .. } | Stmt::Expr(_))
    ) {
        return Ok(None);
    }

    let mut locals = HashSet::new();
    let mut lines: Vec<String> =
        vec!["// Direct JavaScript API; computation still runs in BioLang WASM.".into()];
    let mut comment_index = 0;
    for (index, stmt) in program.stmts.iter().enumerate() {
        while comments
            .get(comment_index)
            .is_some_and(|comment| comment.span.start < stmt.span.start)
        {
            push_direct_comment(&mut lines, &comments[comment_index]);
            comment_index += 1;
        }
        let last = Some(index) == last_value;
        match &stmt.node {
            Stmt::Let { name, value, .. } => {
                if !is_safe_javascript_binding(name) {
                    return Ok(None);
                }
                let value_source = emit_direct_value(value, &locals)?;
                lines.push(format!("let {name} = {value_source};"));
                locals.insert(name.clone());
                if last {
                    lines.push("null;".into());
                }
            }
            Stmt::Expr(value) => {
                let expression = emit_direct_run(value, &locals)?;
                if last {
                    let result_name = unique_javascript_name("result", &locals);
                    lines.push(format!("let {result_name} = {expression};"));
                    lines.push(format!("{result_name};"));
                } else {
                    lines.push(format!("{expression};"));
                }
            }
            _ => return Ok(None),
        }
    }
    for comment in &comments[comment_index..] {
        push_direct_comment(&mut lines, comment);
    }
    Ok(Some(lines.join("\n")))
}

fn js_line_comment(comment: &SourceComment) -> String {
    format!(
        "//{}{}",
        if comment.text.is_empty() { "" } else { " " },
        comment.text
    )
}

fn push_direct_comment(lines: &mut Vec<String>, comment: &SourceComment) {
    let rendered = js_line_comment(comment);
    if comment.inline && lines.len() > 1 {
        if let Some(previous) = lines.last_mut() {
            previous.push(' ');
            previous.push_str(&rendered);
        }
    } else {
        lines.push(rendered);
    }
}

fn is_direct_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Nil
        | Expr::Bool(_)
        | Expr::Int(_)
        | Expr::Float(_)
        | Expr::Str(_)
        | Expr::DnaLit(_)
        | Expr::RnaLit(_)
        | Expr::ProteinLit(_)
        | Expr::QualLit(_) => true,
        Expr::Ident(name) => is_safe_javascript_binding(name),
        Expr::List(values) | Expr::TupleLit(values) => {
            values.iter().all(|value| is_direct_expr(&value.node))
        }
        Expr::Record(entries) => entries.iter().all(|entry| match entry {
            RecordEntry::Field(_, value) => is_direct_expr(&value.node),
            RecordEntry::Spread(_) => false,
        }),
        Expr::Call { callee, args } => {
            matches!(&callee.node, Expr::Ident(name) if is_safe_javascript_binding(name))
                && args
                    .iter()
                    .all(|arg| !arg.spread && arg.name.is_none() && is_direct_expr(&arg.value.node))
        }
        Expr::Field { object, field, .. } => {
            is_safe_javascript_binding(field) && is_direct_expr(&object.node)
        }
        Expr::Index { object, index } => {
            is_direct_expr(&object.node) && is_direct_expr(&index.node)
        }
        _ => false,
    }
}

fn emit_direct_run(expr: &Spanned<Expr>, locals: &HashSet<String>) -> Result<String, String> {
    if let Expr::Call { callee, args } = &expr.node {
        if let Expr::Ident(name) = &callee.node {
            let args = args
                .iter()
                .map(|arg| {
                    let value = emit_direct_value(&arg.value, locals)?;
                    Ok(if arg.spread {
                        format!("bio.spread({value})")
                    } else if let Some(name) = &arg.name {
                        format!("bio.named({}, {value})", quote(name))
                    } else {
                        value
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            return Ok(format!("await bl.{name}({})", args.join(", ")));
        }
    }
    match &expr.node {
        Expr::Ident(name) if !locals.contains(name) => {
            Ok(format!("await bl.getValue({})", quote(name)))
        }
        _ => emit_direct_value(expr, locals),
    }
}

fn emit_direct_call_value(
    callee: &Spanned<Expr>,
    args: &[Arg],
    locals: &HashSet<String>,
) -> Result<String, String> {
    let args = args
        .iter()
        .map(|arg| {
            let value = emit_direct_value(&arg.value, locals)?;
            Ok(if arg.spread {
                format!("bio.spread({value})")
            } else if let Some(name) = &arg.name {
                format!("bio.named({}, {value})", quote(name))
            } else {
                value
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    match &callee.node {
        Expr::Ident(name) => Ok(format!("await bl.{name}({})", args.join(", "))),
        _ => Err("direct JavaScript calls require a named BioLang function".into()),
    }
}

fn emit_direct_value(expr: &Spanned<Expr>, locals: &HashSet<String>) -> Result<String, String> {
    Ok(match &expr.node {
        Expr::Ident(name) if locals.contains(name) && is_safe_javascript_binding(name) => {
            name.clone()
        }
        Expr::Ident(name) => format!("await bl.getValue({})", quote(name)),
        Expr::DnaLit(value) => format!("await bl.dna({})", quote(value)),
        Expr::RnaLit(value) => format!("await bl.rna({})", quote(value)),
        Expr::ProteinLit(value) => format!("await bl.protein({})", quote(value)),
        Expr::QualLit(value) => {
            format!("await bl.evalValue(bio.quality({}))", quote(value))
        }
        Expr::Call { callee, args } => emit_direct_call_value(callee, args, locals)?,
        Expr::Field {
            object,
            field,
            optional,
        } if is_safe_javascript_binding(field) => format!(
            "({}){}{}",
            emit_direct_value(object, locals)?,
            if *optional { "?." } else { "." },
            field
        ),
        Expr::Index { object, index } => format!(
            "({})[{}]",
            emit_direct_value(object, locals)?,
            emit_direct_value(index, locals)?
        ),
        Expr::List(values) => format!(
            "[{}]",
            values
                .iter()
                .map(|value| emit_direct_value(value, locals))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        ),
        Expr::Record(entries)
            if entries
                .iter()
                .all(|entry| matches!(entry, RecordEntry::Field(_, _))) =>
        {
            let fields = entries
                .iter()
                .map(|entry| match entry {
                    RecordEntry::Field(name, value) => Ok(format!(
                        "{}: {}",
                        if is_javascript_identifier(name) {
                            name.clone()
                        } else {
                            quote(name)
                        },
                        emit_direct_value(value, locals)?
                    )),
                    RecordEntry::Spread(_) => unreachable!(),
                })
                .collect::<Result<Vec<_>, String>>()?;
            format!("{{ {} }}", fields.join(", "))
        }
        _ => emit_expr(expr)?,
    })
}

fn is_javascript_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first == '_' || first == '$' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch == '$' || ch.is_ascii_alphanumeric())
}

fn is_safe_javascript_binding(value: &str) -> bool {
    is_javascript_identifier(value)
        && !matches!(
            value,
            "await"
                | "break"
                | "case"
                | "catch"
                | "class"
                | "const"
                | "continue"
                | "debugger"
                | "default"
                | "delete"
                | "do"
                | "else"
                | "enum"
                | "export"
                | "extends"
                | "false"
                | "finally"
                | "for"
                | "function"
                | "if"
                | "implements"
                | "import"
                | "in"
                | "instanceof"
                | "interface"
                | "let"
                | "new"
                | "null"
                | "package"
                | "private"
                | "protected"
                | "public"
                | "return"
                | "static"
                | "super"
                | "switch"
                | "this"
                | "throw"
                | "true"
                | "try"
                | "typeof"
                | "var"
                | "void"
                | "while"
                | "with"
                | "yield"
                | "bio"
                | "bl"
        )
}

fn unique_javascript_name(preferred: &str, locals: &HashSet<String>) -> String {
    if !locals.contains(preferred) {
        return preferred.into();
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{preferred}{suffix}");
        if !locals.contains(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
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
        Stmt::Pipeline { name, params, body } => format!(
            "bio.pipeline_({}, [{}], [{}])",
            quote(name),
            emit_params(params)?,
            emit_stmts(body)?
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
        Expr::TryCatch {
            body,
            error_var,
            catch_body,
        } => format!(
            "bio.tryCatch([{}], {}, [{}])",
            emit_stmts(body)?,
            option_string(error_var.as_ref()),
            emit_stmts(catch_body)?
        ),
        Expr::Match { expr, arms } => format!(
            "bio.matchExpr({}, [{}])",
            emit_expr(expr)?,
            arms.iter()
                .map(emit_match_arm)
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        ),
        Expr::StringInterp(parts) => emit_string_interp(parts)?,
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

fn emit_string_interp(parts: &[StringPart]) -> Result<String, String> {
    let values = parts
        .iter()
        .map(|part| {
            Ok(match part {
                StringPart::Lit(value) => format!("bio.stringText({})", quote(value)),
                StringPart::Expr(value) => format!("bio.stringValue({})", emit_expr(value)?),
                StringPart::Formatted(value, spec) => format!(
                    "bio.stringFormatted({}, {})",
                    emit_expr(value)?,
                    quote(&format_spec_source(spec))
                ),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(format!("bio.stringInterp([{}])", values.join(", ")))
}

fn emit_match_arm(arm: &MatchArm) -> Result<String, String> {
    Ok(format!(
        "bio.matchArm({}, {}, {})",
        emit_pattern(&arm.pattern)?,
        emit_expr(&arm.body)?,
        option_expr(arm.guard.as_deref())?
    ))
}

fn emit_pattern(pattern: &Spanned<Pattern>) -> Result<String, String> {
    Ok(match &pattern.node {
        Pattern::Wildcard => "bio.wildcardPattern()".into(),
        Pattern::Literal(value) => format!("bio.literalPattern({})", emit_expr(value)?),
        Pattern::Ident(name) => format!("bio.identPattern({})", quote(name)),
        Pattern::EnumVariant { variant, bindings } => format!(
            "bio.enumPattern({}, [{}])",
            quote(variant),
            bindings
                .iter()
                .map(|binding| quote(binding))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Pattern::TypePattern { type_name, binding } => format!(
            "bio.typePattern({}, {})",
            quote(type_name),
            option_string(binding.as_ref())
        ),
        Pattern::Or(values) => format!(
            "bio.orPattern([{}])",
            values
                .iter()
                .map(emit_pattern)
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        ),
    })
}

fn format_spec_source(spec: &FormatSpec) -> String {
    let mut value = String::new();
    if let Some(align) = spec.align {
        value.push(align);
    }
    if let Some(width) = spec.width {
        value.push_str(&width.to_string());
    }
    if let Some(precision) = spec.precision {
        value.push('.');
        value.push_str(&precision.to_string());
    }
    if let Some(kind) = spec.kind {
        value.push(kind);
    }
    value
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
    // JSON strings are valid JavaScript strings. Escaping every HTML end-tag
    // opener as `<\/` additionally keeps generated source safe if a consumer
    // later embeds it in a script element instead of loading it as a module.
    serde_json::to_string(value)
        .expect("strings always serialize")
        .replace("</", "<\\/")
}
fn indent_join_with_comments(
    statements: &[Spanned<Stmt>],
    comments: &[SourceComment],
) -> Result<String, String> {
    let mut lines = Vec::new();
    let mut comment_index = 0;

    for (statement_index, statement) in statements.iter().enumerate() {
        while comments
            .get(comment_index)
            .is_some_and(|comment| comment.span.start < statement.span.start)
        {
            lines.push(format!("  {}", js_line_comment(&comments[comment_index])));
            comment_index += 1;
        }

        let next_start = statements
            .get(statement_index + 1)
            .map_or(usize::MAX, |next| next.span.start);
        let mut rendered = format!("  {},", emit_stmt(statement)?);
        while comments
            .get(comment_index)
            .is_some_and(|comment| comment.span.start < next_start)
        {
            let comment = &comments[comment_index];
            if comment.inline {
                rendered.push(' ');
                rendered.push_str(&js_line_comment(comment));
            } else {
                lines.push(rendered);
                rendered = format!("  {}", js_line_comment(comment));
            }
            comment_index += 1;
        }
        lines.push(rendered);
    }

    for comment in &comments[comment_index..] {
        lines.push(format!("  {}", js_line_comment(comment)));
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::transpile;

    #[test]
    fn emits_structural_javascript_without_embedding_source() {
        let js = transpile("let measurements = [12, 14, 15]\nsummary(measurements)").unwrap();
        assert!(js.contains("let measurements = [12, 14, 15]"));
        assert!(js.contains("await bl.summary(measurements)"));
        assert!(!js.contains("bl.define"));
        assert!(!js.contains("bl.run"));
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

    #[test]
    fn legacy_package_syntax_transpiles_to_canonical_builders() {
        let js = transpile("rows |> sort_by(fn(row) -> row.score, descending = true)").unwrap();
        assert!(js.contains("bio.lambdaExpr"));
        assert!(js.contains("bio.named(\"descending\", true)"));
    }

    #[test]
    fn emits_interpolated_strings_format_specs_and_try_catch_structurally() {
        let js = transpile(
            "let mu = 12.3456\ntry { f\"mean={mu:.2f}\" } catch err { f\"failed: {err}\" }",
        )
        .unwrap();
        assert!(js.contains("bio.tryCatch"));
        assert!(js.contains("bio.stringText(\"mean=\")"));
        assert!(js.contains("bio.stringFormatted(bio.ref(\"mu\"), \".2f\")"));
        assert!(js.contains("bio.stringValue(bio.ref(\"err\"))"));
    }

    #[test]
    fn emits_match_patterns_guards_and_bodies_structurally() {
        let js = transpile(
            "match base { \"A\" => \"adenine\", value if value == \"T\" => \"thymine\", _ => \"other\" }",
        )
        .unwrap();
        assert!(js.contains("bio.matchExpr"));
        assert!(js.contains("bio.literalPattern(\"A\")"));
        assert!(js.contains("bio.identPattern(\"value\")"));
        assert!(js.contains("bio.wildcardPattern()"));
    }

    #[test]
    fn generated_javascript_strings_are_safe_for_script_embedding() {
        let js = transpile("print(\"</script><SCRIPT>\")").unwrap();
        assert!(js.contains("<\\/script><SCRIPT>"));
        assert!(!js.contains("</script>"));
    }

    #[test]
    fn emits_direct_calls_variables_and_dot_access_when_safe() {
        let js = transpile("let report = summary([1, 2, 3])\nreport.mean").unwrap();
        assert!(js.contains("let report = await bl.summary([1, 2, 3])"));
        assert!(js.contains("(report).mean"));
        assert!(!js.contains("bl.define"));
        assert!(!js.contains("bl.run"));
    }

    #[test]
    fn emits_an_editable_direct_cell_for_empty_or_comment_only_source() {
        for source in ["", "# BioLang code"] {
            let js = transpile(source).unwrap();
            assert!(js.starts_with("// Direct JavaScript API;"));
            assert!(js.ends_with("null;"));
            assert!(!js.contains("bio.program("));
        }
    }

    #[test]
    fn preserves_standalone_inline_and_nested_expression_comments() {
        let js = transpile(
            "# Count all 4-mers\nlet seq = dna\"ATCG\" # the input\nprintln(iupac_match(seq, \"RAATTC\")) # R is a purine",
        )
        .unwrap();
        assert!(js.contains("// Count all 4-mers"));
        assert!(js.contains("await bl.dna(\"ATCG\")"));
        assert!(js.contains("// the input"));
        assert!(js.contains("// R is a purine"));
        assert!(js.contains("await bl.println(await bl.iupac_match"));
    }

    #[test]
    fn keeps_comments_beside_structural_fallback_statements() {
        let js =
            transpile("# Keep only adults\nrows |> filter(|row| row.age >= 18) # used by the plot")
                .unwrap();
        let note = js.find("// Keep only adults").unwrap();
        let pipe = js.find("bio.pipe(").unwrap();
        let inline = js.find("// used by the plot").unwrap();
        assert!(note < pipe && pipe < inline);
        assert!(js.contains("bio.program("));
    }

    #[test]
    fn a_trailing_let_stays_direct_and_returns_biolang_nil() {
        let js = transpile("let bases = dna\"ATCG\"").unwrap();
        assert!(js.contains("let bases = await bl.dna(\"ATCG\");"));
        assert!(js.ends_with("null;"));
        assert!(!js.contains("bio.program("));
    }

    #[test]
    fn pipeline_statements_have_a_structural_javascript_builder() {
        let js = transpile("pipeline qc(sample) { return len(sample) }").unwrap();
        assert!(js.contains("bio.pipeline_(\"qc\""));
        assert!(!js.contains("does not yet support statement"));
    }
}
