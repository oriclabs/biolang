//! BioLang AST to JavaScript SDK source.
//!
//! Ordinary notebook code is emitted as readable JavaScript (`let`, operators,
//! functions, callbacks, and direct `bl.*` calls). Scientific builtins still
//! execute in the BioLang WASM runtime. Rare declarations that do not have a
//! natural JavaScript form use the public structural builder API instead; no
//! generated program embeds BioLang source for a second parse.

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
    if !program.stmts.iter().all(|stmt| is_direct_stmt(&stmt.node)) {
        return Ok(None);
    }

    let last_value = program.stmts.len().checked_sub(1);
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
                lines.push(if locals.contains(name) {
                    format!("{name} = {value_source};")
                } else {
                    format!("let {name} = {value_source};")
                });
                locals.insert(name.clone());
                if last {
                    lines.push("null;".into());
                }
            }
            Stmt::Const { name, value, .. } => {
                if !is_safe_javascript_binding(name) {
                    return Ok(None);
                }
                let value_source = emit_direct_value(value, &locals)?;
                lines.push(format!("const {name} = {value_source};"));
                locals.insert(name.clone());
                if last {
                    lines.push("null;".into());
                }
            }
            Stmt::Assign { name, value } => {
                if !is_safe_javascript_binding(name) {
                    return Ok(None);
                }
                lines.push(format!("{name} = {};", emit_direct_value(value, &locals)?));
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
            other => {
                lines.extend(emit_direct_statement(other, &mut locals, 0)?);
                if last {
                    lines.push("null;".into());
                }
            }
        }
    }
    for comment in &comments[comment_index..] {
        push_direct_comment(&mut lines, comment);
    }
    Ok(Some(lines.join("\n")))
}

fn is_direct_stmt(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Let { name, value, .. }
        | Stmt::Const { name, value, .. }
        | Stmt::Assign { name, value } => {
            is_safe_javascript_binding(name) && is_direct_expr(&value.node)
        }
        Stmt::IndexAssign { name, index, value } => {
            is_safe_javascript_binding(name)
                && is_direct_expr(&index.node)
                && is_direct_expr(&value.node)
        }
        Stmt::Expr(value) | Stmt::Yield(value) => is_direct_expr(&value.node),
        Stmt::Return(value) => value
            .as_ref()
            .is_none_or(|value| is_direct_expr(&value.node)),
        Stmt::Fn {
            name, params, body, ..
        } => {
            is_safe_javascript_binding(name)
                && params.iter().all(|param| {
                    is_safe_javascript_binding(&param.name)
                        && param
                            .default
                            .as_ref()
                            .is_none_or(|value| is_synchronous_javascript_expr(&value.node))
                })
                && body.iter().all(|stmt| is_direct_stmt(&stmt.node))
        }
        Stmt::For {
            pattern,
            iter,
            when_guard,
            body,
            else_body,
        } => {
            is_direct_for_pattern(pattern)
                && is_direct_expr(&iter.node)
                && when_guard
                    .as_ref()
                    .is_none_or(|value| is_direct_expr(&value.node))
                && body.iter().all(|stmt| is_direct_stmt(&stmt.node))
                && else_body
                    .as_ref()
                    .is_none_or(|body| body.iter().all(|stmt| is_direct_stmt(&stmt.node)))
        }
        Stmt::While { condition, body } => {
            is_direct_expr(&condition.node) && body.iter().all(|stmt| is_direct_stmt(&stmt.node))
        }
        Stmt::Break | Stmt::Continue => true,
        Stmt::Assert { condition, message } => {
            is_direct_expr(&condition.node)
                && message
                    .as_ref()
                    .is_none_or(|value| is_direct_expr(&value.node))
        }
        Stmt::Stage { name, expr } => {
            is_safe_javascript_binding(name) && is_direct_expr(&expr.node)
        }
        Stmt::Pipeline { name, params, body } => {
            is_safe_javascript_binding(name)
                && params.iter().all(|param| {
                    is_safe_javascript_binding(&param.name)
                        && param
                            .default
                            .as_ref()
                            .is_none_or(|value| is_synchronous_javascript_expr(&value.node))
                })
                && body.iter().all(|statement| is_direct_stmt(&statement.node))
        }
        _ => false,
    }
}

fn is_direct_for_pattern(pattern: &ForPattern) -> bool {
    match pattern {
        ForPattern::Single(name) => is_safe_javascript_binding(name),
        ForPattern::ListDestr(names)
        | ForPattern::TupleDestr(names)
        | ForPattern::RecordDestr(names) => {
            names.iter().all(|name| is_safe_javascript_binding(name))
        }
    }
}

fn emit_direct_statement(
    stmt: &Stmt,
    locals: &mut HashSet<String>,
    depth: usize,
) -> Result<Vec<String>, String> {
    let pad = "  ".repeat(depth);
    let mut lines = Vec::new();
    match stmt {
        Stmt::Let { name, value, .. } => {
            let declaration = if locals.contains(name) { "" } else { "let " };
            lines.push(format!(
                "{pad}{declaration}{name} = {};",
                emit_direct_value(value, locals)?
            ));
            locals.insert(name.clone());
        }
        Stmt::Const { name, value, .. } => {
            lines.push(format!(
                "{pad}const {name} = {};",
                emit_direct_value(value, locals)?
            ));
            locals.insert(name.clone());
        }
        Stmt::Assign { name, value } => lines.push(format!(
            "{pad}{name} = {};",
            emit_direct_value(value, locals)?
        )),
        Stmt::IndexAssign { name, index, value } => lines.push(format!(
            "{pad}{name}[{}] = {};",
            emit_direct_value(index, locals)?,
            emit_direct_value(value, locals)?
        )),
        Stmt::Expr(value) => {
            if let Expr::If {
                condition,
                then_body,
                else_body,
            } = &value.node
            {
                lines.push(format!(
                    "{pad}if ({}) {{",
                    emit_direct_value(condition, locals)?
                ));
                let mut then_locals = locals.clone();
                for statement in then_body {
                    lines.extend(emit_direct_statement(
                        &statement.node,
                        &mut then_locals,
                        depth + 1,
                    )?);
                }
                if let Some(else_body) = else_body {
                    lines.push(format!("{pad}}} else {{"));
                    let mut else_locals = locals.clone();
                    for statement in else_body {
                        lines.extend(emit_direct_statement(
                            &statement.node,
                            &mut else_locals,
                            depth + 1,
                        )?);
                    }
                }
                lines.push(format!("{pad}}}"));
            } else {
                lines.push(format!("{pad}{};", emit_direct_run(value, locals)?));
            }
        }
        Stmt::Return(value) => lines.push(match value {
            Some(value) => format!("{pad}return {};", emit_direct_value(value, locals)?),
            None => format!("{pad}return;"),
        }),
        Stmt::Yield(value) => {
            lines.push(format!("{pad}yield {};", emit_direct_value(value, locals)?))
        }
        Stmt::Fn {
            name,
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            let mut function_locals = locals.clone();
            function_locals.insert(name.clone());
            let parameters = emit_direct_parameters(params, &mut function_locals, locals)?;
            lines.push(format!(
                "{pad}{}function{} {name}({parameters}) {{",
                if *is_async { "async " } else { "" },
                if *is_generator { "*" } else { "" }
            ));
            lines.push(emit_direct_block_body(body, &function_locals, depth + 1)?);
            lines.push(format!("{pad}}}"));
            lines.push(format!(
                "{pad}{name}.__biolangParameters = [{}];",
                params
                    .iter()
                    .map(|param| quote(&param.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            locals.insert(name.clone());
        }
        Stmt::For {
            pattern,
            iter,
            when_guard,
            body,
            else_body,
        } => {
            let rendered_pattern = emit_direct_for_binding(pattern);
            let completed = format!("__looped_{depth}");
            if else_body.is_some() {
                lines.push(format!("{pad}let {completed} = false;"));
            }
            lines.push(format!(
                "{pad}for (const {rendered_pattern} of {}) {{",
                emit_direct_value(iter, locals)?
            ));
            let mut loop_locals = locals.clone();
            add_for_pattern_locals(pattern, &mut loop_locals);
            if let Some(guard) = when_guard {
                lines.push(format!(
                    "{}if (!({})) continue;",
                    "  ".repeat(depth + 1),
                    emit_direct_value(guard, &loop_locals)?
                ));
            }
            if else_body.is_some() {
                lines.push(format!("{}{} = true;", "  ".repeat(depth + 1), completed));
            }
            for statement in body {
                lines.extend(emit_direct_statement(
                    &statement.node,
                    &mut loop_locals,
                    depth + 1,
                )?);
            }
            lines.push(format!("{pad}}}"));
            if let Some(else_body) = else_body {
                lines.push(format!("{pad}if (!{completed}) {{"));
                let mut else_locals = locals.clone();
                for statement in else_body {
                    lines.extend(emit_direct_statement(
                        &statement.node,
                        &mut else_locals,
                        depth + 1,
                    )?);
                }
                lines.push(format!("{pad}}}"));
            }
        }
        Stmt::While { condition, body } => {
            lines.push(format!(
                "{pad}while ({}) {{",
                emit_direct_value(condition, locals)?
            ));
            let mut loop_locals = locals.clone();
            for statement in body {
                lines.extend(emit_direct_statement(
                    &statement.node,
                    &mut loop_locals,
                    depth + 1,
                )?);
            }
            lines.push(format!("{pad}}}"));
        }
        Stmt::Break => lines.push(format!("{pad}break;")),
        Stmt::Continue => lines.push(format!("{pad}continue;")),
        Stmt::Assert { condition, message } => {
            let message = message
                .as_ref()
                .map(|value| emit_direct_value(value, locals))
                .transpose()?
                .unwrap_or_else(|| quote("assertion failed"));
            lines.push(format!(
                "{pad}if (!({})) throw new Error({message});",
                emit_direct_value(condition, locals)?
            ));
        }
        Stmt::Stage { name, expr } => {
            let declaration = if locals.contains(name) { "" } else { "let " };
            lines.push(format!(
                "{pad}{declaration}{name} = {};",
                emit_direct_value(expr, locals)?
            ));
            locals.insert(name.clone());
        }
        Stmt::Pipeline { name, params, body } => {
            if params.is_empty() {
                lines.push(format!("{pad}let {name} = (() => {{"));
                lines.push(emit_direct_block_body(body, locals, depth + 1)?);
                lines.push(format!("{pad}}})();"));
            } else {
                let mut function_locals = locals.clone();
                function_locals.insert(name.clone());
                let parameters = emit_direct_parameters(params, &mut function_locals, locals)?;
                lines.push(format!("{pad}function {name}({parameters}) {{"));
                lines.push(emit_direct_block_body(body, &function_locals, depth + 1)?);
                lines.push(format!("{pad}}}"));
                lines.push(format!(
                    "{pad}{name}.__biolangParameters = [{}];",
                    params
                        .iter()
                        .map(|param| quote(&param.name))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            locals.insert(name.clone());
        }
        other => return Err(format!("cannot emit direct JavaScript statement {other:?}")),
    }
    Ok(lines)
}

fn emit_direct_parameters(
    params: &[Param],
    function_locals: &mut HashSet<String>,
    outer_locals: &HashSet<String>,
) -> Result<String, String> {
    params
        .iter()
        .map(|param| {
            function_locals.insert(param.name.clone());
            let prefix = if param.rest { "..." } else { "" };
            Ok(match &param.default {
                Some(value) => format!(
                    "{prefix}{} = {}",
                    param.name,
                    emit_direct_value(value, outer_locals)?
                ),
                None => format!("{prefix}{}", param.name),
            })
        })
        .collect::<Result<Vec<_>, String>>()
        .map(|values| values.join(", "))
}

fn emit_direct_for_binding(pattern: &ForPattern) -> String {
    match pattern {
        ForPattern::Single(name) => name.clone(),
        ForPattern::ListDestr(names) | ForPattern::TupleDestr(names) => {
            format!("[{}]", names.join(", "))
        }
        ForPattern::RecordDestr(names) => format!("{{ {} }}", names.join(", ")),
    }
}

fn add_for_pattern_locals(pattern: &ForPattern, locals: &mut HashSet<String>) {
    match pattern {
        ForPattern::Single(name) => {
            locals.insert(name.clone());
        }
        ForPattern::ListDestr(names)
        | ForPattern::TupleDestr(names)
        | ForPattern::RecordDestr(names) => {
            locals.extend(names.iter().cloned());
        }
    }
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
            RecordEntry::Spread(value) => is_direct_expr(&value.node),
        }),
        Expr::Call { callee, args } => {
            matches!(&callee.node, Expr::Ident(name) if is_safe_javascript_binding(name))
                && args.iter().all(|arg| is_direct_expr(&arg.value.node))
        }
        Expr::Unary { expr, .. } => is_direct_expr(&expr.node),
        Expr::Binary { left, right, .. }
        | Expr::NullCoalesce { left, right }
        | Expr::In { left, right, .. } => is_direct_expr(&left.node) && is_direct_expr(&right.node),
        Expr::Pipe { left, right } => {
            is_direct_expr(&left.node) && is_direct_pipe_stage(&right.node)
        }
        Expr::Lambda { params, body } => {
            params.iter().all(|param| {
                is_safe_javascript_binding(&param.name)
                    && param
                        .default
                        .as_ref()
                        .is_none_or(|value| is_synchronous_javascript_expr(&value.node))
            }) && is_synchronous_javascript_expr(&body.node)
        }
        Expr::Block(body) => body.iter().all(|stmt| is_direct_stmt(&stmt.node)),
        Expr::If {
            condition,
            then_body,
            else_body,
        } => {
            is_direct_expr(&condition.node)
                && then_body.iter().all(|stmt| is_direct_stmt(&stmt.node))
                && else_body
                    .as_ref()
                    .is_none_or(|body| body.iter().all(|stmt| is_direct_stmt(&stmt.node)))
        }
        Expr::TryCatch {
            body,
            error_var,
            catch_body,
        } => {
            error_var
                .as_ref()
                .is_none_or(|name| is_safe_javascript_binding(name))
                && body.iter().all(|stmt| is_direct_stmt(&stmt.node))
                && catch_body.iter().all(|stmt| is_direct_stmt(&stmt.node))
        }
        Expr::Field { object, field, .. } => {
            is_safe_javascript_binding(field) && is_direct_expr(&object.node)
        }
        Expr::Index { object, index } => {
            is_direct_expr(&object.node) && is_direct_expr(&index.node)
        }
        Expr::StringInterp(parts) => parts.iter().all(|part| match part {
            StringPart::Lit(_) => true,
            StringPart::Expr(value) | StringPart::Formatted(value, _) => {
                is_direct_expr(&value.node)
            }
        }),
        Expr::Match { expr, arms } => {
            is_direct_expr(&expr.node)
                && arms.iter().all(|arm| {
                    is_direct_match_pattern(&arm.pattern.node)
                        && is_synchronous_javascript_expr(&arm.body.node)
                        && arm
                            .guard
                            .as_ref()
                            .is_none_or(|guard| is_synchronous_javascript_expr(&guard.node))
                })
        }
        Expr::Ternary {
            value,
            condition,
            else_value,
        } => {
            is_direct_expr(&value.node)
                && is_direct_expr(&condition.node)
                && is_direct_expr(&else_value.node)
        }
        Expr::Slice {
            object,
            start,
            end,
            step,
        } => {
            is_direct_expr(&object.node)
                && start
                    .as_ref()
                    .is_none_or(|value| is_direct_expr(&value.node))
                && end.as_ref().is_none_or(|value| is_direct_expr(&value.node))
                && step.is_none()
        }
        Expr::Range { start, end, .. } => is_direct_expr(&start.node) && is_direct_expr(&end.node),
        Expr::Await(value) => is_direct_expr(&value.node),
        _ => false,
    }
}

fn is_direct_match_pattern(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Wildcard | Pattern::Ident(_) => true,
        Pattern::Literal(value) => is_synchronous_javascript_expr(&value.node),
        Pattern::Or(values) => values
            .iter()
            .all(|value| is_direct_match_pattern(&value.node)),
        _ => false,
    }
}

fn is_direct_pipe_stage(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(name) => is_safe_javascript_binding(name),
        Expr::Call { callee, args } => {
            matches!(&callee.node, Expr::Ident(name) if is_safe_javascript_binding(name))
                && args.iter().all(|arg| is_direct_expr(&arg.value.node))
        }
        Expr::Lambda { .. } => is_direct_expr(expr),
        _ => false,
    }
}

/// Direct callbacks must remain synchronous. Session callback dispatch permits
/// bounded nested `bl.*` calls, so callback expressions can still use BioLang
/// builtins without introducing `async` functions or Promises.
fn is_synchronous_javascript_expr(expr: &Expr) -> bool {
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
        Expr::Unary { expr, .. } => is_synchronous_javascript_expr(&expr.node),
        Expr::Binary { left, right, .. }
        | Expr::NullCoalesce { left, right }
        | Expr::In { left, right, .. } => {
            is_synchronous_javascript_expr(&left.node)
                && is_synchronous_javascript_expr(&right.node)
        }
        Expr::Field { object, field, .. } => {
            is_safe_javascript_binding(field) && is_synchronous_javascript_expr(&object.node)
        }
        Expr::Index { object, index } => {
            is_synchronous_javascript_expr(&object.node)
                && is_synchronous_javascript_expr(&index.node)
        }
        Expr::List(values) | Expr::TupleLit(values) => values
            .iter()
            .all(|value| is_synchronous_javascript_expr(&value.node)),
        Expr::Record(entries) => entries.iter().all(|entry| match entry {
            RecordEntry::Field(_, value) | RecordEntry::Spread(value) => {
                is_synchronous_javascript_expr(&value.node)
            }
        }),
        Expr::Ternary {
            value,
            condition,
            else_value,
        } => {
            is_synchronous_javascript_expr(&value.node)
                && is_synchronous_javascript_expr(&condition.node)
                && is_synchronous_javascript_expr(&else_value.node)
        }
        Expr::Call { callee, args } => {
            matches!(&callee.node, Expr::Ident(name) if is_safe_javascript_binding(name))
                && args
                    .iter()
                    .all(|arg| is_synchronous_javascript_expr(&arg.value.node))
        }
        Expr::Lambda { params, body } => {
            params.iter().all(|param| {
                is_safe_javascript_binding(&param.name)
                    && param
                        .default
                        .as_ref()
                        .is_none_or(|value| is_synchronous_javascript_expr(&value.node))
            }) && is_synchronous_javascript_expr(&body.node)
        }
        Expr::Pipe { left, right } => {
            is_synchronous_javascript_expr(&left.node)
                && match &right.node {
                    Expr::Ident(name) => is_safe_javascript_binding(name),
                    Expr::Call { callee, args } => {
                        matches!(&callee.node, Expr::Ident(name) if is_safe_javascript_binding(name))
                            && args
                                .iter()
                                .all(|arg| is_synchronous_javascript_expr(&arg.value.node))
                    }
                    _ => false,
                }
        }
        Expr::If {
            condition,
            then_body,
            else_body: Some(else_body),
        } => {
            is_synchronous_javascript_expr(&condition.node)
                && direct_block_value(then_body)
                    .is_some_and(|value| is_synchronous_javascript_expr(&value.node))
                && direct_block_value(else_body)
                    .is_some_and(|value| is_synchronous_javascript_expr(&value.node))
        }
        _ => false,
    }
}

fn direct_block_value(body: &[Spanned<Stmt>]) -> Option<&Spanned<Expr>> {
    match body {
        [statement] => match &statement.node {
            Stmt::Expr(value) | Stmt::Return(Some(value)) => Some(value),
            _ => None,
        },
        _ => None,
    }
}

fn emit_direct_run(expr: &Spanned<Expr>, locals: &HashSet<String>) -> Result<String, String> {
    if let Expr::Call { callee, args } = &expr.node {
        if matches!(&callee.node, Expr::Ident(_)) {
            return emit_direct_call_value(callee, args, locals);
        }
    }
    emit_direct_value(expr, locals)
}

fn emit_direct_call_value(
    callee: &Spanned<Expr>,
    args: &[Arg],
    locals: &HashSet<String>,
) -> Result<String, String> {
    let positional = args
        .iter()
        .filter(|arg| arg.name.is_none())
        .map(|arg| {
            let value = emit_direct_value(&arg.value, locals)?;
            Ok(if arg.spread {
                format!("...({value})")
            } else {
                value
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    match &callee.node {
        Expr::Ident(name) if args.iter().any(|arg| arg.name.is_some()) => {
            let named = args
                .iter()
                .filter_map(|arg| arg.name.as_ref().map(|name| (name, &arg.value)))
                .map(|(name, value)| {
                    Ok(format!(
                        "{}: {}",
                        if is_javascript_identifier(name) {
                            name.clone()
                        } else {
                            quote(name)
                        },
                        emit_direct_value(value, locals)?
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(if locals.contains(name) {
                format!(
                    "bl.callNamedFunction({name}, [{}], {{ {} }})",
                    positional.join(", "),
                    named.join(", ")
                )
            } else {
                format!(
                    "bl.callNamed({}, [{}], {{ {} }})",
                    quote(name),
                    positional.join(", "),
                    named.join(", ")
                )
            })
        }
        Expr::Ident(name) if locals.contains(name) => {
            Ok(format!("{name}({})", positional.join(", ")))
        }
        Expr::Ident(name) => Ok(format!(
            "bl.{}({})",
            javascript_api_name(name),
            positional.join(", ")
        )),
        _ => Err("direct JavaScript calls require a named BioLang function".into()),
    }
}

fn emit_direct_value(expr: &Spanned<Expr>, locals: &HashSet<String>) -> Result<String, String> {
    Ok(match &expr.node {
        Expr::Ident(name)
            if !locals.contains(name)
                && bl_runtime::builtins::all_builtin_names().contains(&name.as_str()) =>
        {
            format!("bl.{}", javascript_api_name(name))
        }
        Expr::Ident(name) => name.clone(),
        Expr::DnaLit(value) => format!("bl.dna({})", quote(value)),
        Expr::RnaLit(value) => format!("bl.rna({})", quote(value)),
        Expr::ProteinLit(value) => format!("bl.protein({})", quote(value)),
        Expr::QualLit(value) => {
            format!("bl.evalValue(bio.quality({}))", quote(value))
        }
        Expr::Call { callee, args } => emit_direct_call_value(callee, args, locals)?,
        Expr::Unary { op, expr } => format!(
            "({}{})",
            match op {
                UnaryOp::Neg => "-",
                UnaryOp::Not => "!",
            },
            emit_direct_value(expr, locals)?
        ),
        Expr::Binary { op, left, right }
            if matches!(op, BinaryOp::Add | BinaryOp::Concat)
                && matches!(&left.node, Expr::List(_)) =>
        {
            format!(
                "({}).concat({})",
                emit_direct_value(left, locals)?,
                emit_direct_value(right, locals)?
            )
        }
        Expr::Binary { op, left, right }
            if matches!(op, BinaryOp::Add | BinaryOp::Concat)
                && matches!(&right.node, Expr::List(_)) =>
        {
            format!(
                "({}).concat({})",
                emit_direct_value(left, locals)?,
                emit_direct_value(right, locals)?
            )
        }
        Expr::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } => format!(
            "bl.addValues({}, {})",
            emit_direct_value(left, locals)?,
            emit_direct_value(right, locals)?
        ),
        Expr::Binary {
            op: BinaryOp::Eq,
            left,
            right,
        } => format!(
            "bl.equalValues({}, {})",
            emit_direct_value(left, locals)?,
            emit_direct_value(right, locals)?
        ),
        Expr::Binary {
            op: BinaryOp::Neq,
            left,
            right,
        } => format!(
            "!bl.equalValues({}, {})",
            emit_direct_value(left, locals)?,
            emit_direct_value(right, locals)?
        ),
        Expr::Binary { op, left, right } => format!(
            "({} {} {})",
            emit_direct_value(left, locals)?,
            match op {
                BinaryOp::Eq => "===",
                BinaryOp::Neq => "!==",
                BinaryOp::Concat => "+",
                _ => binary_name(*op),
            },
            emit_direct_value(right, locals)?
        ),
        Expr::NullCoalesce { left, right } => format!(
            "({} ?? {})",
            emit_direct_value(left, locals)?,
            emit_direct_value(right, locals)?
        ),
        Expr::In {
            left,
            right,
            negated,
        } => format!(
            "({}{}({}).includes({}))",
            if *negated { "!" } else { "" },
            "",
            emit_direct_value(right, locals)?,
            emit_direct_value(left, locals)?
        ),
        Expr::Pipe { left, right } => emit_direct_pipe(left, right, locals)?,
        Expr::Lambda { params, body } => {
            let mut lambda_locals = locals.clone();
            let parameters = params
                .iter()
                .map(|param| {
                    lambda_locals.insert(param.name.clone());
                    let prefix = if param.rest { "..." } else { "" };
                    match &param.default {
                        Some(value) => Ok(format!(
                            "{prefix}{} = {}",
                            param.name,
                            emit_direct_value(value, locals)?
                        )),
                        None => Ok(format!("{prefix}{}", param.name)),
                    }
                })
                .collect::<Result<Vec<_>, String>>()?;
            format!(
                "({}) => ({})",
                parameters.join(", "),
                emit_direct_value(body, &lambda_locals)?
            )
        }
        Expr::Block(body) => emit_direct_block_expression(body, locals)?,
        Expr::If {
            condition,
            then_body,
            else_body,
        } => {
            if let (Some(then_value), Some(else_value)) = (
                direct_block_value(then_body),
                else_body.as_deref().and_then(direct_block_value),
            ) {
                return Ok(format!(
                    "({} ? {} : {})",
                    emit_direct_value(condition, locals)?,
                    emit_direct_value(then_value, locals)?,
                    emit_direct_value(else_value, locals)?
                ));
            }
            let then_value = emit_direct_block_body(then_body, locals, 2)?;
            let else_value = else_body
                .as_ref()
                .map(|body| emit_direct_block_body(body, locals, 2))
                .transpose()?
                .unwrap_or_else(|| "    return null;".into());
            format!(
                "(() => {{\n  if ({}) {{\n{}\n  }} else {{\n{}\n  }}\n}})()",
                emit_direct_value(condition, locals)?,
                then_value,
                else_value
            )
        }
        Expr::TryCatch {
            body,
            error_var,
            catch_body,
        } => {
            let error_name = error_var.as_deref().unwrap_or("error");
            let caught_name = unique_javascript_name("__caught", locals);
            let mut catch_locals = locals.clone();
            catch_locals.insert(error_name.into());
            format!(
                "(() => {{\n  try {{\n{}\n  }} catch ({caught_name}) {{\n    let {error_name} = {caught_name} instanceof Error ? {caught_name}.message : String({caught_name});\n{}\n  }}\n}})()",
                emit_direct_block_body(body, locals, 2)?,
                emit_direct_block_body(catch_body, &catch_locals, 2)?
            )
        }
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
            "bl.indexValue({}, {})",
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
            if entries.iter().all(|entry| match entry {
                RecordEntry::Field(_, value) | RecordEntry::Spread(value) => {
                    is_direct_expr(&value.node)
                }
            }) =>
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
                    RecordEntry::Spread(value) => {
                        Ok(format!("...({})", emit_direct_value(value, locals)?))
                    }
                })
                .collect::<Result<Vec<_>, String>>()?;
            format!("{{ {} }}", fields.join(", "))
        }
        Expr::StringInterp(parts) => {
            let pieces = parts
                .iter()
                .map(|part| match part {
                    StringPart::Lit(value) => Ok(value
                        .replace('\\', "\\\\")
                        .replace('`', "\\`")
                        .replace("${", "\\${")),
                    StringPart::Expr(value) => {
                        Ok(format!("${{{}}}", emit_direct_value(value, locals)?))
                    }
                    StringPart::Formatted(value, spec) => Ok(format!(
                        "${{bl.formatValue({}, {})}}",
                        emit_direct_value(value, locals)?,
                        quote(&format_spec_source(spec))
                    )),
                })
                .collect::<Result<Vec<_>, String>>()?;
            format!("`{}`", pieces.join(""))
        }
        Expr::Match { expr, arms } => emit_direct_match(expr, arms, locals)?,
        Expr::Ternary {
            value,
            condition,
            else_value,
        } => format!(
            "({} ? {} : {})",
            emit_direct_value(condition, locals)?,
            emit_direct_value(value, locals)?,
            emit_direct_value(else_value, locals)?
        ),
        Expr::Slice {
            object,
            start,
            end,
            step: None,
        } => format!(
            "({}).slice({}, {})",
            emit_direct_value(object, locals)?,
            start
                .as_deref()
                .map(|value| emit_direct_value(value, locals))
                .transpose()?
                .unwrap_or_else(|| "0".into()),
            end.as_deref()
                .map(|value| emit_direct_value(value, locals))
                .transpose()?
                .unwrap_or_else(|| "undefined".into())
        ),
        Expr::Range {
            start,
            end,
            inclusive,
        } => format!(
            "bl.range({}, {})",
            emit_direct_value(start, locals)?,
            if *inclusive {
                format!("({}) + 1", emit_direct_value(end, locals)?)
            } else {
                emit_direct_value(end, locals)?
            }
        ),
        Expr::Await(value) => format!("await ({})", emit_direct_value(value, locals)?),
        _ => emit_expr(expr)?,
    })
}

fn emit_direct_match(
    expr: &Spanned<Expr>,
    arms: &[MatchArm],
    locals: &HashSet<String>,
) -> Result<String, String> {
    let match_name = unique_javascript_name("__match", locals);
    let mut lines = vec![format!(
        "(() => {{ const {match_name} = {};",
        emit_direct_value(expr, locals)?
    )];
    for arm in arms {
        let mut arm_locals = locals.clone();
        let (condition, binding) =
            emit_direct_match_condition(&arm.pattern.node, &match_name, &mut arm_locals)?;
        let guard = arm
            .guard
            .as_ref()
            .map(|guard| emit_direct_value(guard, &arm_locals))
            .transpose()?;
        let body = emit_direct_value(&arm.body, &arm_locals)?;
        if let Some(binding) = binding {
            lines.push(match guard {
                Some(guard) => format!(" {{ {binding}if ({guard}) return {body}; }}"),
                None => format!(" {{ {binding}return {body}; }}"),
            });
            continue;
        }
        let condition = match (condition, guard) {
            (Some(pattern), Some(guard)) => Some(format!("({pattern}) && ({guard})")),
            (Some(pattern), None) => Some(pattern),
            (None, Some(guard)) => Some(guard),
            (None, None) => None,
        };
        let prefix = condition
            .map(|condition| format!("if ({condition}) "))
            .unwrap_or_default();
        lines.push(format!(" {prefix}{{ return {body}; }}"));
    }
    lines.push(" return null; })()".into());
    Ok(lines.join(""))
}

fn emit_direct_match_condition(
    pattern: &Pattern,
    value: &str,
    locals: &mut HashSet<String>,
) -> Result<(Option<String>, Option<String>), String> {
    Ok(match pattern {
        Pattern::Wildcard => (None, None),
        Pattern::Ident(name) => {
            locals.insert(name.clone());
            (None, Some(format!("const {name} = {value}; ")))
        }
        Pattern::Literal(literal) => (
            Some(format!(
                "{value} === {}",
                emit_direct_value(literal, locals)?
            )),
            None,
        ),
        Pattern::Or(values) => {
            let conditions = values
                .iter()
                .map(|pattern| {
                    let (condition, binding) =
                        emit_direct_match_condition(&pattern.node, value, locals)?;
                    if binding.is_some() {
                        return Err("direct JavaScript or-patterns cannot bind names".into());
                    }
                    Ok(condition.unwrap_or_else(|| "true".into()))
                })
                .collect::<Result<Vec<_>, String>>()?;
            (Some(conditions.join(" || ")), None)
        }
        _ => return Err("unsupported direct JavaScript match pattern".into()),
    })
}

fn emit_direct_block_expression(
    body: &[Spanned<Stmt>],
    locals: &HashSet<String>,
) -> Result<String, String> {
    Ok(format!(
        "await (async () => {{\n{}\n}})()",
        emit_direct_block_body(body, locals, 1)?
    ))
}

fn emit_direct_block_body(
    body: &[Spanned<Stmt>],
    locals: &HashSet<String>,
    depth: usize,
) -> Result<String, String> {
    if body.is_empty() {
        return Ok(format!("{}return null;", "  ".repeat(depth)));
    }
    let mut block_locals = locals.clone();
    let mut lines = Vec::new();
    for (index, statement) in body.iter().enumerate() {
        let last = index + 1 == body.len();
        if last {
            match &statement.node {
                Stmt::Expr(value) => {
                    lines.push(format!(
                        "{}return {};",
                        "  ".repeat(depth),
                        emit_direct_value(value, &block_locals)?
                    ));
                    continue;
                }
                Stmt::Stage { name, expr } => {
                    let declaration = if block_locals.contains(name) {
                        ""
                    } else {
                        "let "
                    };
                    lines.push(format!(
                        "{}{declaration}{name} = {};",
                        "  ".repeat(depth),
                        emit_direct_value(expr, &block_locals)?
                    ));
                    lines.push(format!("{}return {name};", "  ".repeat(depth)));
                    continue;
                }
                _ => {}
            }
        }
        lines.extend(emit_direct_statement(
            &statement.node,
            &mut block_locals,
            depth,
        )?);
    }
    if !matches!(
        body.last().map(|statement| &statement.node),
        Some(Stmt::Expr(_) | Stmt::Return(_) | Stmt::Stage { .. })
    ) {
        lines.push(format!("{}return null;", "  ".repeat(depth)));
    }
    Ok(lines.join("\n"))
}

fn emit_direct_pipe(
    left: &Spanned<Expr>,
    right: &Spanned<Expr>,
    locals: &HashSet<String>,
) -> Result<String, String> {
    let left = emit_direct_value(left, locals)?;
    match &right.node {
        Expr::Ident(name) if locals.contains(name) => Ok(format!("{name}({left})")),
        Expr::Ident(name) => Ok(format!("bl.{}({left})", javascript_api_name(name))),
        Expr::Call { callee, args } => {
            let Expr::Ident(name) = &callee.node else {
                return Err("direct JavaScript pipelines require a named stage".into());
            };
            let mut values = vec![left];
            values.extend(
                args.iter()
                    .filter(|arg| arg.name.is_none())
                    .map(|arg| {
                        let value = emit_direct_value(&arg.value, locals)?;
                        Ok(if arg.spread {
                            format!("...({value})")
                        } else {
                            value
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?,
            );
            if args.iter().any(|arg| arg.name.is_some()) {
                let named = args
                    .iter()
                    .filter_map(|arg| arg.name.as_ref().map(|name| (name, &arg.value)))
                    .map(|(name, value)| {
                        Ok(format!(
                            "{}: {}",
                            if is_javascript_identifier(name) {
                                name.clone()
                            } else {
                                quote(name)
                            },
                            emit_direct_value(value, locals)?
                        ))
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                return Ok(if locals.contains(name) {
                    format!(
                        "bl.callNamedFunction({name}, [{}], {{ {} }})",
                        values.join(", "),
                        named.join(", ")
                    )
                } else {
                    format!(
                        "bl.callNamed({}, [{}], {{ {} }})",
                        quote(name),
                        values.join(", "),
                        named.join(", ")
                    )
                });
            }
            Ok(if locals.contains(name) {
                format!("{name}({})", values.join(", "))
            } else {
                format!("bl.{}({})", javascript_api_name(name), values.join(", "))
            })
        }
        Expr::Lambda { .. } => Ok(format!("({})({left})", emit_direct_value(right, locals)?)),
        _ => Err("unsupported direct JavaScript pipeline stage".into()),
    }
}

fn javascript_api_name(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut uppercase_next = false;
    for character in value.chars() {
        if character == '_' {
            uppercase_next = true;
        } else if uppercase_next {
            result.push(character.to_ascii_uppercase());
            uppercase_next = false;
        } else {
            result.push(character);
        }
    }
    result
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
    fn emits_direct_javascript_without_embedding_source() {
        let js = transpile("let measurements = [12, 14, 15]\nsummary(measurements)").unwrap();
        assert!(js.contains("let measurements = [12, 14, 15]"));
        assert!(js.contains("bl.summary(measurements)"));
        assert!(!js.contains("bl.define"));
        assert!(!js.contains("bl.run"));
    }

    #[test]
    fn emits_pipes_records_lambdas_and_named_arguments() {
        let js = transpile(
            "rows |> filter(|row| row.Age >= 18) |> histogram({bins: 20}, format: \"svg\")",
        )
        .unwrap();
        assert!(js.starts_with("// Direct JavaScript API;"));
        assert!(js.contains("bl.filter(rows, (row) =>"));
        assert!(js.contains("bl.callNamed(\"histogram\""));
        assert!(js.contains("format: \"svg\""));
        assert!(!js.contains("bio.let_("));
    }

    #[test]
    fn legacy_package_syntax_transpiles_to_canonical_builders() {
        let js = transpile("rows |> sort_by(fn(row) -> row.score, descending = true)").unwrap();
        assert!(js.contains("bl.callNamed(\"sort_by\""));
        assert!(js.contains("(row) => ((row).score)"));
        assert!(js.contains("descending: true"));
    }

    #[test]
    fn emits_interpolated_strings_format_specs_and_try_catch_directly() {
        let js = transpile(
            "let mu = 12.3456\ntry { f\"mean={mu:.2f}\" } catch err { f\"failed: {err}\" }",
        )
        .unwrap();
        assert!(js.starts_with("// Direct JavaScript API;"));
        assert!(js.contains("try {"));
        assert!(js.contains("bl.formatValue(mu, \".2f\")"));
        assert!(js.contains("`failed: ${err}`"));
        assert!(!js.contains("bio.let_("));
    }

    #[test]
    fn emits_match_patterns_guards_and_bodies_directly() {
        let js = transpile(
            "match base { \"A\" => \"adenine\", value if value == \"T\" => \"thymine\", _ => \"other\" }",
        )
        .unwrap();
        assert!(js.starts_with("// Direct JavaScript API;"));
        assert!(js.contains("const __match = base"));
        assert!(js.contains("__match === \"A\""));
        assert!(js.contains("const value = __match"));
        assert!(js.contains("return \"other\""));
        assert!(!js.contains("bio.matchExpr"));
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
        assert!(js.contains("let report = bl.summary([1, 2, 3])"));
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
        assert!(js.contains("bl.dna(\"ATCG\")"));
        assert!(js.contains("// the input"));
        assert!(js.contains("// R is a purine"));
        assert!(js.contains("bl.println(bl.iupacMatch"));
    }

    #[test]
    fn keeps_comments_beside_direct_pipeline_statements() {
        let js =
            transpile("# Keep only adults\nrows |> filter(|row| row.age >= 18) # used by the plot")
                .unwrap();
        let note = js.find("// Keep only adults").unwrap();
        let pipe = js.find("bl.filter(").unwrap();
        let inline = js.find("// used by the plot").unwrap();
        assert!(note < pipe && pipe < inline);
        assert!(!js.contains("bio.program("));
    }

    #[test]
    fn emits_functions_conditionals_and_loops_as_ordinary_javascript() {
        let js = transpile(
            "fn adult(age) { if age >= 18 { true } else { false } }\nlet found = false\nfor age in [12, 21] { if adult(age) { found = true } }\nfound",
        )
        .unwrap();
        assert!(js.starts_with("// Direct JavaScript API;"));
        assert!(js.contains("function adult(age)"));
        assert!(js.contains("for (const age of [12, 21])"));
        assert!(js.contains("(age >= 18) ? true : false"));
        assert!(!js.contains("bio.let_("));
    }

    #[test]
    fn direct_functions_keep_named_arguments_and_list_addition_semantics() {
        let js = transpile(
            "fn greet(name, greeting = \"Hello\") { greeting + name }\nlet body = [\"b\"]\n[greet(\"BioLang\", greeting: \"Welcome \"), \"a\"] + body",
        )
        .unwrap();
        assert!(js.contains("greet.__biolangParameters = [\"name\", \"greeting\"]"));
        assert!(
            js.contains("bl.callNamedFunction(greet, [\"BioLang\"], { greeting: \"Welcome \" })")
        );
        assert!(js.contains(".concat(body)"));
    }

    #[test]
    fn emits_nested_callback_pipelines_as_direct_javascript() {
        let js = transpile(
            "let ids = range(0, 5) |> map(|unused| random_int(0, 4))\nrange(0, 5) |> map(|id| ids |> filter(|draw| draw == id) |> len())",
        )
        .unwrap();
        assert!(js.starts_with("// Direct JavaScript API;"));
        assert!(js.contains("bl.map(bl.range(0, 5), (unused) => (bl.randomInt(0, 4)))"));
        assert!(js.contains("(draw) => (bl.equalValues(draw, id))"));
        assert!(!js.contains("bio.program("));
    }

    #[test]
    fn emits_early_return_helpers_as_direct_javascript() {
        let js = transpile(
            "fn survival_at(curve, at) { if len(curve) == 0 { return 1.0 } last(curve) }\nsurvival_at([0.9], 1)",
        )
        .unwrap();
        assert!(js.starts_with("// Direct JavaScript API;"));
        assert!(js.contains("function survival_at(curve, at)"));
        assert!(js.contains("return 1.0"));
        assert!(!js.contains("bio.program("));
    }

    #[test]
    fn repeated_let_in_one_block_becomes_an_assignment() {
        let js = transpile(
            "for item in [1] { let line = \" \" let line = str_replace(line, 0, \"*\") }",
        )
        .unwrap();
        assert!(js.contains("let line = \" \";"));
        assert!(js.contains("line = bl.strReplace(line, 0, \"*\");"));
    }

    #[test]
    fn a_trailing_let_stays_direct_and_returns_biolang_nil() {
        let js = transpile("let bases = dna\"ATCG\"").unwrap();
        assert!(js.contains("let bases = bl.dna(\"ATCG\");"));
        assert!(js.ends_with("null;"));
        assert!(!js.contains("bio.program("));
    }

    #[test]
    fn pipeline_statements_are_readable_direct_javascript() {
        let js = transpile("pipeline qc(sample) { return len(sample) }").unwrap();
        assert!(js.contains("function qc(sample)"));
        assert!(js.contains("return bl.len(sample);"));
        assert!(!js.contains("bio.pipeline_("));
    }

    #[test]
    fn immediate_pipeline_stages_return_the_final_stage() {
        let js =
            transpile("pipeline qc { stage raw -> [1, 2, 3] stage count -> len(raw) }").unwrap();
        assert!(js.contains("let qc = (() =>"));
        assert!(js.contains("let raw = [1, 2, 3];"));
        assert!(js.contains("let count = bl.len(raw);"));
        assert!(js.contains("return count;"));
    }
}
