use bl_core::ast::*;
use bl_core::error::{BioLangError, ErrorKind, Result, StackFrame};
use bl_core::span::Spanned;
use bl_core::value::{Arity, BioSequence, Value};

use crate::builtins::{call_builtin, register_builtins};
use crate::env::Environment;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Iterator adapter that receives values from a generator thread via mpsc channel.
struct GeneratorIterator {
    rx: std::sync::mpsc::Receiver<Value>,
}

impl Iterator for GeneratorIterator {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        self.rx.recv().ok()
    }
}

// SAFETY: Receiver is Send (values are Send)
unsafe impl Send for GeneratorIterator {}

pub struct Interpreter {
    env: Environment,
    /// Cache of loaded modules: canonical path → exported (name, value) pairs
    loaded_modules: HashMap<PathBuf, HashMap<String, Value>>,
    /// Set of modules currently being loaded (for circular import detection)
    loading_modules: HashSet<PathBuf>,
    /// Path of the currently executing file (for relative import resolution)
    current_file: Option<PathBuf>,
    /// Output capture buffer for TUI/Web IDE (if set, print/println write here instead of stdout)
    pub output_buffer: Option<std::sync::Arc<std::sync::Mutex<String>>>,
    /// Call stack for stack traces (F3)
    call_stack: Vec<StackFrame>,
    /// Yield collector for eager generator execution (legacy)
    yield_collector: Option<Vec<Value>>,
    /// Yield sender for lazy generator execution (channel-based)
    yield_sender: Option<std::sync::mpsc::SyncSender<Value>>,
    /// Profiling data: function name → (call_count, total_ns) (F14)
    pub profiling: Option<HashMap<String, (u64, u128)>>,
    /// Enable gradual type checking (opt-in)
    pub type_check: bool,
    /// Verbose mode: print each step to stderr as it executes
    pub verbose: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut env = Environment::new();
        register_builtins(&mut env);
        Self {
            env,
            loaded_modules: HashMap::new(),
            loading_modules: HashSet::new(),
            current_file: None,
            output_buffer: None,
            call_stack: Vec::new(),
            yield_collector: None,
            yield_sender: None,
            profiling: None,
            type_check: false,
            verbose: false,
        }
    }

    pub fn with_env(env: Environment) -> Self {
        Self {
            env,
            loaded_modules: HashMap::new(),
            loading_modules: HashSet::new(),
            current_file: None,
            output_buffer: None,
            call_stack: Vec::new(),
            yield_collector: None,
            yield_sender: None,
            profiling: None,
            type_check: false,
            verbose: false,
        }
    }

    pub fn env(&self) -> &Environment {
        &self.env
    }

    pub fn env_mut(&mut self) -> &mut Environment {
        &mut self.env
    }

    /// Reset the interpreter to a fresh state (re-register builtins, clear user vars).
    pub fn reset(&mut self) {
        self.env = Environment::new();
        register_builtins(&mut self.env);
        self.loaded_modules.clear();
        self.loading_modules.clear();
        self.current_file = None;
        self.call_stack.clear();
        self.yield_collector = None;
        self.yield_sender = None;
        self.profiling = None;
        // output_buffer intentionally preserved across reset
    }

    /// Run the gradual type checker on a parsed program and return warnings.
    pub fn check_program(&self, program: &Program) -> Vec<crate::checker::TypeWarning> {
        let mut checker = crate::checker::Checker::new();
        checker.check(program)
    }

    /// Convert a value to an iterable list of items (F1 helper).
    fn value_to_iter(&self, val: &Value, span: bl_core::span::Span) -> Result<Vec<Value>> {
        match val {
            Value::List(items) => Ok(items.clone()),
            Value::Str(s) => Ok(s.chars().map(|c| Value::Str(c.to_string())).collect()),
            Value::Table(t) => Ok((0..t.num_rows())
                .map(|i| Value::Record(t.row_to_record(i)))
                .collect()),
            Value::Map(m) | Value::Record(m) => Ok(m
                .iter()
                .map(|(k, v)| {
                    let mut rec = HashMap::new();
                    rec.insert("key".to_string(), Value::Str(k.clone()));
                    rec.insert("value".to_string(), v.clone());
                    Value::Record(rec)
                })
                .collect()),
            Value::Range { start, end, inclusive } => {
                let end_val = if *inclusive { *end + 1 } else { *end };
                let count = (end_val - *start).max(0) as u64;
                if count > 10_000_000 {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!(
                            "range too large to materialize ({count} elements). \
                             Use a stream or reduce the range. Max: 10,000,000 elements."
                        ),
                        Some(span),
                    ));
                }
                Ok((*start..end_val).map(Value::Int).collect())
            }
            Value::Stream(s) => {
                if s.is_exhausted() {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!(
                            "stream already consumed: Stream({}) has been fully iterated. \
                             Streams can only be consumed once — use collect() on first use \
                             to store results if you need them again.",
                            s.label
                        ),
                        Some(span),
                    ));
                }
                Ok(s.collect_all())
            }
            Value::Set(items) => Ok(items.clone()),
            other => Err(BioLangError::type_error(
                format!("cannot iterate over {}", other.type_of()),
                Some(span),
            )),
        }
    }

    pub fn set_current_file(&mut self, path: Option<PathBuf>) {
        self.current_file = path;
    }

    pub fn current_file(&self) -> Option<&PathBuf> {
        self.current_file.as_ref()
    }

    pub fn run(&mut self, program: &Program) -> Result<Value> {
        let mut last = Value::Nil;
        for (i, stmt) in program.stmts.iter().enumerate() {
            if self.verbose {
                let label = verbose_stmt_label(&stmt.node);
                eprintln!("\x1b[2m  [{}/{}] {label}\x1b[0m", i + 1, program.stmts.len());
            }
            last = self.exec_stmt(stmt)?;
        }
        Ok(last)
    }

    #[inline(never)]
    pub fn exec_stmt(&mut self, stmt: &Spanned<Stmt>) -> Result<Value> {
        match &stmt.node {
            Stmt::Let { name, value, .. } => {
                let val = self.eval_expr(value)?;
                self.env.define(name.clone(), val);
                Ok(Value::Nil)
            }
            Stmt::Const { name, value, .. } => {
                let val = self.eval_expr(value)?;
                self.env.define(name.clone(), val);
                self.env.define(format!("__const_{name}"), Value::Bool(true));
                Ok(Value::Nil)
            }
            Stmt::Fn {
                name, params, body, doc, is_generator, decorators, is_async, named_returns, where_clause, ..
            } => {
                let closure_env = self.env.current_scope_id();
                let mut func = Value::Function {
                    name: Some(name.clone()),
                    params: params.clone(),
                    body: body.clone(),
                    closure_env: Some(closure_env),
                    doc: doc.clone(),
                    is_generator: *is_generator,
                };
                self.env.define(name.clone(), func.clone());
                // Apply decorators in reverse order
                for dec_name in decorators.iter().rev() {
                    if dec_name == "compile" {
                        #[cfg(feature = "bytecode")]
                        {
                            match crate::compiled::compile_function_to_closure(
                                name, params, body, *is_generator, *is_async,
                            ) {
                                Ok(compiled) => {
                                    func = compiled;
                                    self.env.set(name, func.clone(), Some(stmt.span))?;
                                }
                                Err(e) => {
                                    eprintln!("warning: @compile failed: {e}, falling back to interpreter");
                                }
                            }
                        }
                        #[cfg(not(feature = "bytecode"))]
                        {
                            eprintln!("warning: @compile requires 'bytecode' feature — falling back to interpreter");
                        }
                        continue;
                    }
                    if dec_name == "resources" {
                        eprintln!("warning: @resources decorator is informational only — constraints not enforced");
                        continue;
                    }
                    if dec_name == "validate" {
                        // Mark this function for parameter type validation at call time
                        self.env.define(format!("__validate_{name}"), Value::Bool(true));
                        continue;
                    }
                    if dec_name == "memoize" || dec_name == "memo" || dec_name == "cache" {
                        self.env.define(format!("__memoize_{name}"), Value::Record(std::collections::HashMap::new()));
                        continue;
                    }
                    if let Ok(decorator) = self.env.get(dec_name, None).cloned() {
                        func = self.call_value(&decorator, vec![func], stmt.span)?;
                        self.env.set(name, func.clone(), Some(stmt.span))?;
                    }
                }
                // Mark async functions
                if *is_async {
                    self.env.define(
                        format!("__async_{name}"),
                        Value::Bool(true),
                    );
                }
                // Store named return field names for auto-wrapping
                if !named_returns.is_empty() {
                    let names: Vec<Value> = named_returns
                        .iter()
                        .map(|(n, _)| Value::Str(n.clone()))
                        .collect();
                    self.env.define(format!("__named_returns_{name}"), Value::List(names));
                }
                // If there's a where clause, prepend it as a guard to the function body
                if let Some(where_expr) = where_clause {
                    use bl_core::ast::Stmt as S;
                    let assert_stmt = Spanned::new(
                        S::Assert {
                            condition: where_expr.clone(),
                            message: Some(Spanned::new(
                                Expr::Str(format!("where clause failed: precondition not met")),
                                where_expr.span,
                            )),
                        },
                        where_expr.span,
                    );
                    let mut new_body = vec![assert_stmt];
                    new_body.extend(body.iter().cloned());
                    let new_func = Value::Function {
                        name: Some(name.clone()),
                        params: params.clone(),
                        body: new_body,
                        closure_env: Some(closure_env),
                        doc: doc.clone(),
                        is_generator: *is_generator,
                    };
                    self.env.set(name, new_func, Some(stmt.span))?;
                }
                Ok(Value::Nil)
            }
            Stmt::Assign { name, value } => {
                if self.env.get(&format!("__const_{name}"), None).is_ok() {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!("cannot reassign const binding '{name}'"),
                        Some(stmt.span),
                    ));
                }
                let val = self.eval_expr(value)?;
                self.env.set(name, val, Some(stmt.span))?;
                Ok(Value::Nil)
            }
            Stmt::Expr(expr) => self.eval_expr(expr),
            Stmt::Return(value) => {
                let val = match value {
                    Some(expr) => self.eval_expr(expr)?,
                    None => Value::Nil,
                };
                Err(BioLangError::return_val(val, Some(stmt.span)))
            }
            Stmt::For { pattern, iter, when_guard, body, else_body } => {
                let iterable = self.eval_expr(iter)?;

                // Streams are consumed lazily — one item at a time, no materialization
                if let Value::Stream(ref s) = iterable {
                    if s.is_exhausted() {
                        return Err(BioLangError::runtime(
                            ErrorKind::TypeError,
                            format!(
                                "stream already consumed: Stream({}) has been fully iterated.",
                                s.label
                            ),
                            Some(iter.span),
                        ));
                    }
                    let mut last = Value::Nil;
                    let mut did_break = false;
                    while let Some(item) = s.next() {
                        let prev = self.env.push_scope();
                        self.bind_for_pattern(pattern, item);
                        if let Some(guard) = when_guard {
                            let cond = self.eval_expr(guard)?;
                            if !cond.is_truthy() {
                                self.env.pop_scope(prev);
                                continue;
                            }
                        }
                        match self.exec_block(body) {
                            Ok(val) => last = val,
                            Err(e) if e.kind == ErrorKind::Break => {
                                self.env.pop_scope(prev);
                                did_break = true;
                                break;
                            }
                            Err(e) if e.kind == ErrorKind::Continue => {
                                self.env.pop_scope(prev);
                                continue;
                            }
                            Err(e) => {
                                self.env.pop_scope(prev);
                                return Err(e);
                            }
                        }
                        self.env.pop_scope(prev);
                    }
                    if !did_break {
                        if let Some(eb) = else_body {
                            last = self.exec_block(eb)?;
                        }
                    }
                    return Ok(last);
                }

                let items = self.value_to_iter(&iterable, iter.span)?;

                let mut last = Value::Nil;
                let mut did_break = false;
                for item in items {
                    let prev = self.env.push_scope();
                    self.bind_for_pattern(pattern, item);
                    // Evaluate `when` guard — skip iteration if false
                    if let Some(guard) = when_guard {
                        let cond = self.eval_expr(guard)?;
                        if !cond.is_truthy() {
                            self.env.pop_scope(prev);
                            continue;
                        }
                    }
                    match self.exec_block(body) {
                        Ok(val) => last = val,
                        Err(e) if e.kind == ErrorKind::Break => {
                            self.env.pop_scope(prev);
                            did_break = true;
                            break;
                        }
                        Err(e) if e.kind == ErrorKind::Continue => {
                            self.env.pop_scope(prev);
                            continue;
                        }
                        Err(e) if e.kind == ErrorKind::Return => {
                            self.env.pop_scope(prev);
                            return Err(e);
                        }
                        Err(e) => {
                            self.env.pop_scope(prev);
                            return Err(e);
                        }
                    }
                    self.env.pop_scope(prev);
                }
                // `else` body runs if loop completed without break
                if !did_break {
                    if let Some(eb) = else_body {
                        last = self.exec_block(eb)?;
                    }
                }
                Ok(last)
            }
            Stmt::While { condition, body } => {
                let mut last = Value::Nil;
                loop {
                    let cond = self.eval_expr(condition)?;
                    if !cond.is_truthy() {
                        break;
                    }
                    let prev = self.env.push_scope();
                    match self.exec_block(body) {
                        Ok(val) => last = val,
                        Err(e) if e.kind == ErrorKind::Break => {
                            self.env.pop_scope(prev);
                            break;
                        }
                        Err(e) if e.kind == ErrorKind::Continue => {
                            self.env.pop_scope(prev);
                            continue;
                        }
                        Err(e) if e.kind == ErrorKind::Return => {
                            self.env.pop_scope(prev);
                            return Err(e);
                        }
                        Err(e) => {
                            self.env.pop_scope(prev);
                            return Err(e);
                        }
                    }
                    self.env.pop_scope(prev);
                }
                Ok(last)
            }
            Stmt::Break => Err(BioLangError::new(ErrorKind::Break, "break", Some(stmt.span))),
            Stmt::Continue => Err(BioLangError::new(ErrorKind::Continue, "continue", Some(stmt.span))),
            Stmt::DestructLet { pattern, value } => {
                let val = self.eval_expr(value)?;
                match pattern {
                    DestructPattern::List(names) => {
                        let items = match &val {
                            Value::List(items) => items.clone(),
                            other => {
                                return Err(BioLangError::type_error(
                                    format!("cannot destructure {} as list", other.type_of()),
                                    Some(value.span),
                                ))
                            }
                        };
                        for (i, name) in names.iter().enumerate() {
                            let item = items.get(i).cloned().unwrap_or(Value::Nil);
                            self.env.define(name.clone(), item);
                        }
                    }
                    DestructPattern::ListWithRest { elements, rest_name } => {
                        let items = match &val {
                            Value::List(items) => items.clone(),
                            other => {
                                return Err(BioLangError::type_error(
                                    format!("cannot destructure {} as list", other.type_of()),
                                    Some(value.span),
                                ))
                            }
                        };
                        for (i, name) in elements.iter().enumerate() {
                            let item = items.get(i).cloned().unwrap_or(Value::Nil);
                            self.env.define(name.clone(), item);
                        }
                        let rest: Vec<Value> = items.into_iter().skip(elements.len()).collect();
                        self.env.define(rest_name.clone(), Value::List(rest));
                    }
                    DestructPattern::Record(names) => {
                        let map = match &val {
                            Value::Record(m) | Value::Map(m) => m.clone(),
                            other => {
                                return Err(BioLangError::type_error(
                                    format!("cannot destructure {} as record", other.type_of()),
                                    Some(value.span),
                                ))
                            }
                        };
                        for name in names {
                            let item = map.get(name).cloned().unwrap_or(Value::Nil);
                            self.env.define(name.clone(), item);
                        }
                    }
                    DestructPattern::RecordWithRest { fields, rest_name } => {
                        let map = match &val {
                            Value::Record(m) | Value::Map(m) => m.clone(),
                            other => {
                                return Err(BioLangError::type_error(
                                    format!("cannot destructure {} as record", other.type_of()),
                                    Some(value.span),
                                ))
                            }
                        };
                        let field_set: HashSet<&String> = fields.iter().collect();
                        for name in fields {
                            let item = map.get(name).cloned().unwrap_or(Value::Nil);
                            self.env.define(name.clone(), item);
                        }
                        let rest: HashMap<String, Value> = map
                            .into_iter()
                            .filter(|(k, _)| !field_set.contains(k))
                            .collect();
                        self.env.define(rest_name.clone(), Value::Record(rest));
                    }
                }
                Ok(Value::Nil)
            }
            Stmt::Assert { condition, message } => {
                let val = self.eval_expr(condition)?;
                if !val.is_truthy() {
                    let msg = match message {
                        Some(expr) => format!("{}", self.eval_expr(expr)?),
                        None => format!("assertion failed: expression evaluated to {val}"),
                    };
                    Err(BioLangError::runtime(
                        ErrorKind::AssertionFailed,
                        msg,
                        Some(condition.span),
                    ))
                } else {
                    Ok(Value::Nil)
                }
            }
            Stmt::Pipeline { name, params, body } => {
                if params.is_empty() {
                    // No parameters: execute the pipeline immediately
                    let prev = self.env.push_scope();
                    let result = self.exec_block(body);
                    self.env.pop_scope(prev);
                    match result {
                        Ok(val) => {
                            // Also bind the pipeline name to its result
                            self.env.define(name.clone(), val.clone());
                            Ok(val)
                        }
                        Err(e) if e.kind == ErrorKind::Return => Ok(Value::Nil),
                        Err(e) => Err(BioLangError::runtime(
                            e.kind,
                            format!("in pipeline '{name}': {}", e.message),
                            e.span,
                        )),
                    }
                } else {
                    // Parameterized pipeline: define as a callable function (template)
                    let closure_env = self.env.current_scope_id();
                    let func = Value::Function {
                        name: Some(name.clone()),
                        params: params.clone(),
                        body: body.clone(),
                        closure_env: Some(closure_env),
                        doc: Some(format!("pipeline {name}")),
                        is_generator: false,
                    };
                    self.env.define(name.clone(), func);
                    Ok(Value::Nil)
                }
            }
            Stmt::Import { path, alias } => {
                #[cfg(feature = "native")]
                {
                    match self.resolve_module_path(path, Some(stmt.span)) {
                        Ok(resolved) => {
                            let exports = self.load_module(&resolved, Some(stmt.span))?;
                            if let Some(alias_name) = alias {
                                self.env.define(alias_name.clone(), Value::Record(exports));
                            } else {
                                for (name, value) in exports {
                                    self.env.define(name, value);
                                }
                            }
                        }
                        Err(_) => {
                            // Fall back to plugin resolution
                            let exports = crate::plugins::load_plugin(path)?;
                            if exports.is_empty() {
                                return Err(BioLangError::import_error(
                                    format!("module or plugin '{path}' not found"),
                                    Some(stmt.span),
                                ));
                            }
                            if let Some(alias_name) = alias {
                                self.env.define(alias_name.clone(), Value::Record(exports));
                            } else {
                                for (name, value) in exports {
                                    self.env.define(name, value);
                                }
                            }
                        }
                    }
                }
                #[cfg(not(feature = "native"))]
                {
                    let _ = (path, alias);
                    return Err(BioLangError::import_error(
                        "import is not available in browser mode",
                        Some(stmt.span),
                    ));
                }
                #[allow(unreachable_code)]
                Ok(Value::Nil)
            }
            Stmt::Yield(expr) => {
                let val = self.eval_expr(expr)?;
                // Lazy generator: send via channel (blocks until consumer pulls)
                if let Some(ref sender) = self.yield_sender {
                    let _ = sender.send(val); // ignore error if consumer dropped
                    Ok(Value::Nil)
                } else if let Some(ref mut collector) = self.yield_collector {
                    // Legacy eager collector path
                    collector.push(val);
                    Ok(Value::Nil)
                } else {
                    Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "yield can only be used inside a generator function (fn*)",
                        Some(stmt.span),
                    ))
                }
            }
            Stmt::Enum { name, variants } => {
                for variant in variants {
                    if variant.fields.is_empty() {
                        // Unit variant — register as a value
                        self.env.define(
                            variant.name.clone(),
                            Value::EnumValue {
                                enum_name: name.clone(),
                                variant: variant.name.clone(),
                                fields: Vec::new(),
                            },
                        );
                    } else {
                        // Tuple variant — register as a constructor function
                        let enum_name = name.clone();
                        let variant_name = variant.name.clone();
                        let field_count = variant.fields.len();
                        let params: Vec<Param> = variant
                            .fields
                            .iter()
                            .map(|f| Param {
                                name: f.clone(),
                                type_ann: None,
                                default: None,
                                rest: false,
                            })
                            .collect();
                        // Build body that returns an EnumValue record
                        let body_stmts: Vec<Spanned<Stmt>> = Vec::new();
                        let closure_env = self.env.current_scope_id();
                        // Store as a native-like tagged function
                        // We'll use a special naming convention for enum constructors
                        self.env.define(
                            variant_name.clone(),
                            Value::Function {
                                name: Some(format!("{}::{}", enum_name, variant_name)),
                                params,
                                body: body_stmts,
                                closure_env: Some(closure_env),
                                doc: None,
                                is_generator: false,
                            },
                        );
                        // Store metadata for the call handler
                        self.env.define(
                            format!("__enum_ctor_{}_{}", enum_name, variant_name),
                            Value::Record({
                                let mut m = HashMap::new();
                                m.insert("enum_name".to_string(), Value::Str(enum_name.clone()));
                                m.insert("variant".to_string(), Value::Str(variant_name.clone()));
                                m.insert("field_count".to_string(), Value::Int(field_count as i64));
                                m
                            }),
                        );
                    }
                }
                Ok(Value::Nil)
            }
            Stmt::Struct { name, fields } => {
                // Store struct metadata
                let mut field_meta = Vec::new();
                for f in fields {
                    let mut meta = HashMap::new();
                    meta.insert("name".to_string(), Value::Str(f.name.clone()));
                    meta.insert("has_default".to_string(), Value::Bool(f.default.is_some()));
                    if let Some(ref default_expr) = f.default {
                        let default_val = self.eval_expr(default_expr)?;
                        meta.insert("default".to_string(), default_val);
                    }
                    field_meta.push(Value::Record(meta));
                }
                let mut struct_meta = HashMap::new();
                struct_meta.insert("__type".to_string(), Value::Str("struct_def".to_string()));
                struct_meta.insert("name".to_string(), Value::Str(name.clone()));
                struct_meta.insert("fields".to_string(), Value::List(field_meta));
                self.env.define(format!("__struct_{name}"), Value::Record(struct_meta));

                // Register constructor function
                let _struct_name = name.clone();
                let struct_fields = fields.clone();
                let params: Vec<Param> = struct_fields.iter().map(|f| Param {
                    name: f.name.clone(),
                    type_ann: f.type_ann.clone(),
                    default: f.default.clone(),
                    rest: false,
                }).collect();
                let closure_env = self.env.current_scope_id();
                self.env.define(
                    name.clone(),
                    Value::Function {
                        name: Some(name.clone()),
                        params,
                        body: Vec::new(), // sentinel: empty body means struct constructor
                        closure_env: Some(closure_env),
                        doc: None,
                        is_generator: false,
                    },
                );
                // Mark as struct constructor
                self.env.define(
                    format!("__struct_ctor_{name}"),
                    Value::Bool(true),
                );
                Ok(Value::Nil)
            }
            Stmt::Trait { name, methods } => {
                // Store required method names
                let method_names: Vec<Value> = methods
                    .iter()
                    .map(|m| Value::Str(m.name.clone()))
                    .collect();
                let mut meta = HashMap::new();
                meta.insert("__type".to_string(), Value::Str("trait_def".to_string()));
                meta.insert("name".to_string(), Value::Str(name.clone()));
                meta.insert("methods".to_string(), Value::List(method_names));
                self.env.define(format!("__trait_{name}"), Value::Record(meta));
                Ok(Value::Nil)
            }
            Stmt::Impl { type_name, trait_name, methods } => {
                for method_stmt in methods {
                    if let Stmt::Fn { name: method_name, params, body, doc, is_generator, .. } = &method_stmt.node {
                        let closure_env = self.env.current_scope_id();
                        let func = Value::Function {
                            name: Some(method_name.clone()),
                            params: params.clone(),
                            body: body.clone(),
                            closure_env: Some(closure_env),
                            doc: doc.clone(),
                            is_generator: *is_generator,
                        };
                        self.env.define(
                            format!("__impl_{type_name}_{method_name}"),
                            func,
                        );
                    }
                }
                // Validate trait implementation if specified
                if let Some(ref trait_n) = trait_name {
                    if let Ok(trait_meta) = self.env.get(&format!("__trait_{trait_n}"), None).cloned() {
                        if let Value::Record(meta) = trait_meta {
                            if let Some(Value::List(required)) = meta.get("methods") {
                                for req in required {
                                    if let Value::Str(method_name) = req {
                                        let key = format!("__impl_{type_name}_{method_name}");
                                        if self.env.get(&key, None).is_err() {
                                            return Err(BioLangError::runtime(
                                                ErrorKind::TypeError,
                                                format!("impl {trait_n} for {type_name}: missing required method '{method_name}'"),
                                                Some(stmt.span),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Value::Nil)
            }
            Stmt::With { expr, body } => {
                let ctx = self.eval_expr(expr)?;
                let prev = self.env.push_scope();
                if let Value::Record(ref map) | Value::Map(ref map) = ctx {
                    for (k, v) in map {
                        self.env.define(k.clone(), v.clone());
                    }
                }
                let result = self.exec_block(body);
                self.env.pop_scope(prev);
                result.map(|_| Value::Nil)
            }
            Stmt::Unless { condition, body } => {
                let cond = self.eval_expr(condition)?;
                if !cond.is_truthy() {
                    let prev = self.env.push_scope();
                    let result = self.exec_block(body);
                    self.env.pop_scope(prev);
                    result
                } else {
                    Ok(Value::Nil)
                }
            }
            Stmt::Guard { condition, else_body } => {
                let cond = self.eval_expr(condition)?;
                if !cond.is_truthy() {
                    let prev = self.env.push_scope();
                    let result = self.exec_block(else_body);
                    self.env.pop_scope(prev);
                    result
                } else {
                    Ok(Value::Nil)
                }
            }
            Stmt::Defer(expr) => {
                // Defer is best-effort: store the expression for later.
                // In this tree-walking interpreter, we evaluate it immediately
                // since we don't have a true scope-exit mechanism. This gives
                // the user a place to put cleanup code that documents intent.
                let _ = self.eval_expr(expr)?;
                Ok(Value::Nil)
            }
            Stmt::ParallelFor { pattern, iter, body } => {
                // In the tree-walking interpreter, parallel for runs sequentially.
                // A future bytecode/JIT backend can parallelize this.
                let iterable = self.eval_expr(iter)?;
                let items = self.value_to_iter(&iterable, iter.span)?;

                let mut last = Value::Nil;
                for item in items {
                    let prev = self.env.push_scope();
                    self.bind_for_pattern(pattern, item);
                    match self.exec_block(body) {
                        Ok(val) => last = val,
                        Err(e) if e.kind == ErrorKind::Break => {
                            self.env.pop_scope(prev);
                            break;
                        }
                        Err(e) if e.kind == ErrorKind::Continue => {
                            self.env.pop_scope(prev);
                            continue;
                        }
                        Err(e) => {
                            self.env.pop_scope(prev);
                            return Err(e);
                        }
                    }
                    self.env.pop_scope(prev);
                }
                Ok(last)
            }
            Stmt::Stage { name, expr } => {
                // Inside a pipeline block, stages just evaluate their expression
                let val = self.eval_expr(expr)?;
                // Store the stage result with its name for provenance
                self.env.define(format!("__stage_{name}"), val.clone());
                Ok(val)
            }
            Stmt::NilAssign { name, value } => {
                // `name ?= expr` — only assign if name is nil or undefined
                let current = self.env.get(name, None).cloned().unwrap_or(Value::Nil);
                if current == Value::Nil {
                    let val = self.eval_expr(value)?;
                    if self.env.get(name, None).is_ok() {
                        self.env.set(name, val, Some(stmt.span))?;
                    } else {
                        self.env.define(name.clone(), val);
                    }
                }
                Ok(Value::Nil)
            }
            Stmt::TypeAlias { name, target } => {
                // Store type alias as a string mapping for documentation/reflection
                self.env.define(format!("__type_alias_{name}"), Value::Str(target.name.clone()));
                Ok(Value::Nil)
            }
            Stmt::FromImport { path, names } => {
                // `from "module" import name1, name2`
                // First load the module normally, then selectively import names
                let import_stmt = Spanned::new(
                    Stmt::Import { path: path.clone(), alias: None },
                    stmt.span,
                );
                self.exec_stmt(&import_stmt)?;
                // The module's exported names should now be in the module cache.
                // Re-load from cache and bind only the requested names.
                #[cfg(feature = "native")]
                {
                    let canonical = self.resolve_module_path(path, Some(stmt.span))?;
                    if let Some(exports) = self.loaded_modules.get(&canonical).cloned() {
                        for name in names {
                            if let Some(val) = exports.get(name) {
                                self.env.define(name.clone(), val.clone());
                            } else {
                                return Err(BioLangError::import_error(
                                    format!("name '{name}' not found in module '{path}'"),
                                    Some(stmt.span),
                                ));
                            }
                        }
                    }
                }
                #[cfg(not(feature = "native"))]
                {
                    return Err(BioLangError::runtime(
                        ErrorKind::ImportError,
                        "from-import not available without native feature",
                        Some(stmt.span),
                    ));
                }
                Ok(Value::Nil)
            }
        }
    }

    /// Bind a for-loop pattern to a value in the current scope.
    fn bind_for_pattern(&mut self, pattern: &ForPattern, item: Value) {
        match pattern {
            ForPattern::Single(var) => {
                self.env.define(var.clone(), item);
            }
            ForPattern::ListDestr(names) => {
                if let Value::List(ref elems) = item {
                    for (i, name) in names.iter().enumerate() {
                        self.env.define(name.clone(), elems.get(i).cloned().unwrap_or(Value::Nil));
                    }
                } else if let Value::Tuple(ref elems) = item {
                    for (i, name) in names.iter().enumerate() {
                        self.env.define(name.clone(), elems.get(i).cloned().unwrap_or(Value::Nil));
                    }
                } else {
                    self.env.define(names[0].clone(), item);
                }
            }
            ForPattern::RecordDestr(names) => {
                if let Value::Record(ref map) | Value::Map(ref map) = item {
                    for name in names {
                        self.env.define(name.clone(), map.get(name).cloned().unwrap_or(Value::Nil));
                    }
                } else {
                    self.env.define(names[0].clone(), item);
                }
            }
            ForPattern::TupleDestr(names) => {
                match &item {
                    Value::Tuple(elems) | Value::List(elems) => {
                        for (i, name) in names.iter().enumerate() {
                            self.env.define(name.clone(), elems.get(i).cloned().unwrap_or(Value::Nil));
                        }
                    }
                    _ => {
                        self.env.define(names[0].clone(), item);
                    }
                }
            }
        }
    }

    fn exec_block(&mut self, stmts: &[Spanned<Stmt>]) -> Result<Value> {
        let mut last = Value::Nil;
        for stmt in stmts {
            last = self.exec_stmt(stmt)?;
        }
        Ok(last)
    }

    // ── Module loading (native only) ─────────────────────────────

    /// Resolve an import path to a canonical filesystem path.
    #[cfg(feature = "native")]
    fn resolve_module_path(
        &self,
        import_path: &str,
        span: Option<bl_core::span::Span>,
    ) -> Result<PathBuf> {
        let home_dir = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .ok();
        let biolang_dir = home_dir.as_ref().map(|h| h.join(".biolang"));

        // Handle std/ and pkg/ prefixes
        if let Some(rest) = import_path.strip_prefix("std/") {
            // Standard library: ~/.biolang/stdlib/<rest>.bl
            if let Some(ref bl_dir) = biolang_dir {
                let std_file = bl_dir.join("stdlib").join(format!("{rest}.bl"));
                if std_file.is_file() {
                    return std_file.canonicalize().map_err(|e| {
                        BioLangError::import_error(format!("cannot resolve '{import_path}': {e}"), span)
                    });
                }
                let std_dir = bl_dir.join("stdlib").join(rest).join("main.bl");
                if std_dir.is_file() {
                    return std_dir.canonicalize().map_err(|e| {
                        BioLangError::import_error(format!("cannot resolve '{import_path}': {e}"), span)
                    });
                }
            }
            return Err(BioLangError::import_error(
                format!("standard library module '{import_path}' not found"),
                span,
            ));
        }
        if let Some(rest) = import_path.strip_prefix("pkg/") {
            // Package: ~/.biolang/packages/<rest>/lib.bl or <rest>.bl
            if let Some(ref bl_dir) = biolang_dir {
                let pkg_lib = bl_dir.join("packages").join(rest).join("lib.bl");
                if pkg_lib.is_file() {
                    return pkg_lib.canonicalize().map_err(|e| {
                        BioLangError::import_error(format!("cannot resolve '{import_path}': {e}"), span)
                    });
                }
                let pkg_file = bl_dir.join("packages").join(format!("{rest}.bl"));
                if pkg_file.is_file() {
                    return pkg_file.canonicalize().map_err(|e| {
                        BioLangError::import_error(format!("cannot resolve '{import_path}': {e}"), span)
                    });
                }
            }
            return Err(BioLangError::import_error(
                format!("package '{import_path}' not found"),
                span,
            ));
        }

        let mut search_dirs: Vec<PathBuf> = Vec::new();

        // 1. Relative to importing file's directory (or CWD for REPL)
        if let Some(ref file) = self.current_file {
            if let Some(parent) = file.parent() {
                search_dirs.push(parent.to_path_buf());
            }
        }
        if let Ok(cwd) = std::env::current_dir() {
            search_dirs.push(cwd);
        }

        // 2. BIOLANG_PATH env var
        if let Ok(bp) = std::env::var("BIOLANG_PATH") {
            let sep = if cfg!(windows) { ';' } else { ':' };
            for dir in bp.split(sep) {
                let p = PathBuf::from(dir);
                if p.is_dir() {
                    search_dirs.push(p);
                }
            }
        }

        // 3. ~/.biolang/stdlib/ (fallback for unqualified imports)
        if let Some(ref bl_dir) = biolang_dir {
            let stdlib_dir = bl_dir.join("stdlib");
            if stdlib_dir.is_dir() {
                search_dirs.push(stdlib_dir);
            }
        }

        // 4. ~/.biolang/packages/
        if let Some(ref bl_dir) = biolang_dir {
            let pkg_dir = bl_dir.join("packages");
            if pkg_dir.is_dir() {
                search_dirs.push(pkg_dir);
            }
        }

        for dir in &search_dirs {
            // Try <path>.br
            let as_file = dir.join(format!("{import_path}.bl"));
            if as_file.is_file() {
                return as_file.canonicalize().map_err(|e| {
                    BioLangError::import_error(format!("cannot resolve '{import_path}': {e}"), span)
                });
            }
            // Try <path>/main.br
            let as_dir = dir.join(import_path).join("main.bl");
            if as_dir.is_file() {
                return as_dir.canonicalize().map_err(|e| {
                    BioLangError::import_error(format!("cannot resolve '{import_path}': {e}"), span)
                });
            }
        }

        Err(BioLangError::import_error(
            format!("module '{import_path}' not found"),
            span,
        ))
    }

    /// Load a module from a resolved path, returning its exports.
    #[cfg(feature = "native")]
    fn load_module(
        &mut self,
        resolved: &PathBuf,
        span: Option<bl_core::span::Span>,
    ) -> Result<HashMap<String, Value>> {
        // Check cache
        if let Some(exports) = self.loaded_modules.get(resolved) {
            return Ok(exports.clone());
        }

        // Circular import detection
        if self.loading_modules.contains(resolved) {
            return Err(BioLangError::import_error(
                format!(
                    "circular import detected: '{}'",
                    resolved.display()
                ),
                span,
            ));
        }

        self.loading_modules.insert(resolved.clone());

        // Read source
        let source = std::fs::read_to_string(resolved).map_err(|e| {
            BioLangError::import_error(
                format!("cannot read '{}': {e}", resolved.display()),
                span,
            )
        })?;

        // Lex
        let tokens = bl_lexer::Lexer::new(&source).tokenize().map_err(|e| {
            BioLangError::import_error(
                format!("in module '{}': {}", resolved.display(), e.message),
                span,
            )
        })?;

        // Parse
        let parse_result = bl_parser::Parser::new(tokens).parse().map_err(|e| {
            BioLangError::import_error(
                format!("in module '{}': {}", resolved.display(), e.message),
                span,
            )
        })?;
        if let Some(first_err) = parse_result.errors.first() {
            return Err(BioLangError::import_error(
                format!("in module '{}': {}", resolved.display(), first_err.message),
                span,
            ));
        }
        let program = parse_result.program;

        // Execute in an isolated scope, saving/restoring current_file
        let prev_file = self.current_file.take();
        self.current_file = Some(resolved.clone());

        let prev_scope = self.env.push_scope();
        let run_result = self.run(&program);
        let exports_vec = self.env.list_current_scope_vars();
        self.env.pop_scope(prev_scope);

        self.current_file = prev_file;
        self.loading_modules.remove(resolved);

        // Propagate runtime errors from the module
        run_result?;

        // Collect exports (filter out NativeFunctions from builtins that leaked in)
        let exports: HashMap<String, Value> = exports_vec
            .into_iter()
            .filter(|(_, v)| !matches!(v, Value::NativeFunction { .. }))
            .collect();

        // Cache
        self.loaded_modules.insert(resolved.clone(), exports.clone());

        Ok(exports)
    }

    #[inline(never)]
    pub fn eval_expr(&mut self, expr: &Spanned<Expr>) -> Result<Value> {
        match &expr.node {
            Expr::Nil => Ok(Value::Nil),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Int(n) => Ok(Value::Int(*n)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::DnaLit(s) => Ok(Value::DNA(bl_core::value::BioSequence {
                data: s.to_uppercase(),
            })),
            Expr::RnaLit(s) => Ok(Value::RNA(bl_core::value::BioSequence {
                data: s.to_uppercase(),
            })),
            Expr::ProteinLit(s) => Ok(Value::Protein(bl_core::value::BioSequence {
                data: s.to_uppercase(),
            })),
            Expr::QualLit(s) => {
                let scores: Vec<u8> = s.bytes().map(|b| b.saturating_sub(33)).collect();
                Ok(Value::Quality(scores))
            }
            Expr::Ident(name) => self.env.get(name, Some(expr.span)).cloned(),
            Expr::Unary { op, expr: inner } => {
                let val = self.eval_expr(inner)?;
                match op {
                    UnaryOp::Neg => match val {
                        Value::Int(n) => Ok(Value::Int(-n)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        other => Err(BioLangError::type_error(
                            format!("cannot negate {}", other.type_of()),
                            Some(expr.span),
                        )),
                    },
                    UnaryOp::Not => Ok(Value::Bool(!val.is_truthy())),
                }
            }
            Expr::Binary {
                op,
                left,
                right,
            } => {
                let lhs = self.eval_expr(left)?;
                // Short-circuit for && and ||
                match op {
                    BinaryOp::And => {
                        if !lhs.is_truthy() {
                            return Ok(Value::Bool(false));
                        }
                        let rhs = self.eval_expr(right)?;
                        return Ok(Value::Bool(rhs.is_truthy()));
                    }
                    BinaryOp::Or => {
                        if lhs.is_truthy() {
                            return Ok(Value::Bool(true));
                        }
                        let rhs = self.eval_expr(right)?;
                        return Ok(Value::Bool(rhs.is_truthy()));
                    }
                    _ => {}
                }
                let rhs = self.eval_expr(right)?;
                self.eval_binary(*op, &lhs, &rhs, expr.span)
            }
            Expr::Pipe { left, right } => self.eval_pipe(left, right),
            Expr::PipeInto { value, name } => {
                let val = self.eval_expr(value)?;
                self.env.define(name.clone(), val.clone());
                Ok(val)
            }
            Expr::Call { callee, args } => self.eval_call(callee, args, expr.span),
            Expr::Field { object, field, optional } => {
                let obj = self.eval_expr(object)?;
                // Optional chaining: nil?.field → nil
                if *optional && matches!(obj, Value::Nil) {
                    return Ok(Value::Nil);
                }
                match obj {
                    Value::Record(ref map) | Value::Map(ref map) => match map.get(field) {
                        Some(val) => Ok(val.clone()),
                        None => {
                            if *optional {
                                return Ok(Value::Nil);
                            }
                            // UFCS: try looking up field as a function
                            if let Ok(func) = self.env.get(field, None).cloned() {
                                if matches!(func, Value::Function { .. } | Value::NativeFunction { .. }) {
                                    return Err(BioLangError::name_error(
                                        format!("no field '{field}' on record"),
                                        Some(expr.span),
                                    ));
                                }
                            }
                            Err(BioLangError::name_error(
                                format!("no field '{field}' on record"),
                                Some(expr.span),
                            ))
                        }
                    },
                    Value::Table(t) => match field.as_str() {
                        "columns" => Ok(Value::List(
                            t.columns.iter().map(|c| Value::Str(c.clone())).collect(),
                        )),
                        "num_rows" => Ok(Value::Int(t.num_rows() as i64)),
                        "num_cols" => Ok(Value::Int(t.num_cols() as i64)),
                        col_name => {
                            // Access column by name → returns column as List
                            match t.col_index(col_name) {
                                Some(ci) => Ok(Value::List(
                                    t.rows.iter().map(|row| row[ci].clone()).collect(),
                                )),
                                None => Err(BioLangError::name_error(
                                    format!("no column '{col_name}' on Table"),
                                    Some(expr.span),
                                )),
                            }
                        }
                    },
                    Value::Interval(iv) => match field.as_str() {
                        "chrom" => Ok(Value::Str(iv.chrom.clone())),
                        "start" => Ok(Value::Int(iv.start)),
                        "end" => Ok(Value::Int(iv.end)),
                        "strand" => Ok(Value::Str(iv.strand.to_string())),
                        other => Err(BioLangError::name_error(
                            format!("no field '{other}' on Interval"),
                            Some(expr.span),
                        )),
                    },
                    Value::Gene { symbol, gene_id, chrom, start, end, strand, biotype, description } => match field.as_str() {
                        "symbol" => Ok(Value::Str(symbol.clone())),
                        "gene_id" => Ok(Value::Str(gene_id.clone())),
                        "chrom" => Ok(Value::Str(chrom.clone())),
                        "start" => Ok(Value::Int(start)),
                        "end" => Ok(Value::Int(end)),
                        "strand" => Ok(Value::Str(strand.clone())),
                        "biotype" => Ok(Value::Str(biotype.clone())),
                        "description" => Ok(Value::Str(description.clone())),
                        other => Err(BioLangError::name_error(
                            format!("no field '{other}' on Gene"),
                            Some(expr.span),
                        )),
                    },
                    Value::Variant { ref chrom, pos, ref id, ref ref_allele, ref alt_allele, quality, ref filter, ref info } => match field.as_str() {
                        "chrom" => Ok(Value::Str(chrom.clone())),
                        "pos" => Ok(Value::Int(pos)),
                        "id" => Ok(Value::Str(id.clone())),
                        "ref_allele" | "ref" => Ok(Value::Str(ref_allele.clone())),
                        "alt_allele" | "alt" => Ok(Value::Str(alt_allele.clone())),
                        "quality" | "qual" => Ok(Value::Float(quality)),
                        "filter" => Ok(Value::Str(filter.clone())),
                        "info" => {
                            // Lazy INFO parsing: if _raw key present, parse it now
                            if info.len() == 1 {
                                if let Some(Value::Str(raw)) = info.get("_raw") {
                                    if raw == "." || raw.is_empty() {
                                        return Ok(Value::Record(HashMap::new()));
                                    }
                                    let mut map = HashMap::new();
                                    for part in raw.split(';') {
                                        if part.is_empty() { continue; }
                                        if let Some((key, val)) = part.split_once('=') {
                                            if val.contains(',') {
                                                let items: Vec<Value> = val.split(',').map(|v| {
                                                    if v == "." { Value::Nil }
                                                    else if let Ok(n) = v.parse::<i64>() { Value::Int(n) }
                                                    else if let Ok(f) = v.parse::<f64>() { Value::Float(f) }
                                                    else { Value::Str(v.to_string()) }
                                                }).collect();
                                                map.insert(key.to_string(), Value::List(items));
                                            } else if val == "." {
                                                map.insert(key.to_string(), Value::Nil);
                                            } else if let Ok(n) = val.parse::<i64>() {
                                                map.insert(key.to_string(), Value::Int(n));
                                            } else if let Ok(f) = val.parse::<f64>() {
                                                map.insert(key.to_string(), Value::Float(f));
                                            } else {
                                                map.insert(key.to_string(), Value::Str(val.to_string()));
                                            }
                                        } else {
                                            map.insert(part.to_string(), Value::Bool(true));
                                        }
                                    }
                                    return Ok(Value::Record(map));
                                }
                            }
                            Ok(Value::Record(info.clone()))
                        }
                        // Computed variant classification properties
                        "is_snp" => {
                            let first_alt = alt_allele.split(',').next().unwrap_or("");
                            Ok(Value::Bool(bl_core::bio_core::vcf_ops::classify_variant(ref_allele, first_alt) == bl_core::bio_core::VariantType::Snp))
                        }
                        "is_indel" => {
                            let first_alt = alt_allele.split(',').next().unwrap_or("");
                            Ok(Value::Bool(bl_core::bio_core::vcf_ops::classify_variant(ref_allele, first_alt) == bl_core::bio_core::VariantType::Indel))
                        }
                        "is_transition" => {
                            let first_alt = alt_allele.split(',').next().unwrap_or("");
                            let is_snp = bl_core::bio_core::vcf_ops::classify_variant(ref_allele, first_alt) == bl_core::bio_core::VariantType::Snp;
                            Ok(Value::Bool(is_snp && bl_core::bio_core::vcf_ops::is_transition(ref_allele, first_alt)))
                        }
                        "is_transversion" => {
                            let first_alt = alt_allele.split(',').next().unwrap_or("");
                            let is_snp = bl_core::bio_core::vcf_ops::classify_variant(ref_allele, first_alt) == bl_core::bio_core::VariantType::Snp;
                            Ok(Value::Bool(is_snp && !bl_core::bio_core::vcf_ops::is_transition(ref_allele, first_alt)))
                        }
                        "variant_type" => {
                            let first_alt = alt_allele.split(',').next().unwrap_or("");
                            let vt = bl_core::bio_core::vcf_ops::classify_variant(ref_allele, first_alt);
                            let name = match vt {
                                bl_core::bio_core::VariantType::Snp => "Snp",
                                bl_core::bio_core::VariantType::Indel => "Indel",
                                bl_core::bio_core::VariantType::Mnp => "Mnp",
                                bl_core::bio_core::VariantType::Other => "Other",
                            };
                            Ok(Value::Str(name.to_string()))
                        }
                        "alt_alleles" => {
                            Ok(Value::List(alt_allele.split(',').map(|s| Value::Str(s.to_string())).collect()))
                        }
                        "is_multiallelic" => Ok(Value::Bool(alt_allele.contains(','))),
                        "is_het" | "is_hom_ref" | "is_hom_alt" => {
                            let gt_str = info.get("GT").or_else(|| info.get("gt"));
                            let result = match gt_str {
                                Some(Value::Str(gt)) => {
                                    let sep = if gt.contains('|') { '|' } else { '/' };
                                    let alleles: Vec<Option<u8>> = gt.split(sep)
                                        .map(|a| if a == "." { None } else { a.parse().ok() })
                                        .collect();
                                    match field.as_str() {
                                        "is_het" => {
                                            let vals: Vec<u8> = alleles.iter().filter_map(|a| *a).collect();
                                            vals.len() >= 2 && vals.windows(2).any(|w| w[0] != w[1])
                                        }
                                        "is_hom_ref" => alleles.iter().all(|a| *a == Some(0)),
                                        "is_hom_alt" => {
                                            let vals: Vec<u8> = alleles.iter().filter_map(|a| *a).collect();
                                            vals.len() >= 2 && vals[0] > 0 && vals.iter().all(|&a| a == vals[0])
                                        }
                                        _ => false,
                                    }
                                }
                                _ => false,
                            };
                            Ok(Value::Bool(result))
                        }
                        other => Err(BioLangError::name_error(
                            format!("no field '{other}' on Variant"),
                            Some(expr.span),
                        )),
                    },
                    Value::Genome { name, species, assembly, chromosomes } => match field.as_str() {
                        "name" => Ok(Value::Str(name.clone())),
                        "species" => Ok(Value::Str(species.clone())),
                        "assembly" => Ok(Value::Str(assembly.clone())),
                        "chromosomes" => {
                            let chroms = chromosomes.iter().map(|(n, l)| {
                                let mut rec = HashMap::new();
                                rec.insert("name".to_string(), Value::Str(n.clone()));
                                rec.insert("length".to_string(), Value::Int(*l));
                                Value::Record(rec)
                            }).collect();
                            Ok(Value::List(chroms))
                        }
                        other => Err(BioLangError::name_error(
                            format!("no field '{other}' on Genome"),
                            Some(expr.span),
                        )),
                    },
                    Value::Quality(ref scores) => match field.as_str() {
                        "scores" => Ok(Value::List(scores.iter().map(|s| Value::Int(*s as i64)).collect())),
                        "length" => Ok(Value::Int(scores.len() as i64)),
                        "mean" => {
                            if scores.is_empty() {
                                Ok(Value::Float(0.0))
                            } else {
                                let sum: f64 = scores.iter().map(|s| *s as f64).sum();
                                Ok(Value::Float(sum / scores.len() as f64))
                            }
                        }
                        "min" => Ok(Value::Int(scores.iter().copied().min().unwrap_or(0) as i64)),
                        other => Err(BioLangError::name_error(
                            format!("no field '{other}' on Quality"),
                            Some(expr.span),
                        )),
                    },
                    Value::AlignedRead(ref read) => match field.as_str() {
                        "qname" => Ok(Value::Str(read.qname.clone())),
                        "flag" => Ok(Value::Int(read.flag as i64)),
                        "rname" => Ok(Value::Str(read.rname.clone())),
                        "pos" => Ok(Value::Int(read.pos)),
                        "mapq" => Ok(Value::Int(read.mapq as i64)),
                        "cigar" => Ok(Value::Str(read.cigar.clone())),
                        "rnext" => Ok(Value::Str(read.rnext.clone())),
                        "pnext" => Ok(Value::Int(read.pnext)),
                        "tlen" => Ok(Value::Int(read.tlen)),
                        "seq" => Ok(Value::DNA(BioSequence { data: read.seq.clone() })),
                        "qual" => Ok(Value::Quality(bl_core::bio_core::QualityOps::from_ascii(&read.qual))),
                        // Computed properties
                        "is_paired" => Ok(Value::Bool(read.is_paired())),
                        "is_proper_pair" => Ok(Value::Bool(read.is_proper_pair())),
                        "is_unmapped" => Ok(Value::Bool(read.is_unmapped())),
                        "is_mapped" => Ok(Value::Bool(read.is_mapped())),
                        "is_reverse" => Ok(Value::Bool(read.is_reverse())),
                        "is_read1" => Ok(Value::Bool(read.is_read1())),
                        "is_read2" => Ok(Value::Bool(read.is_read2())),
                        "is_duplicate" => Ok(Value::Bool(read.is_duplicate())),
                        "is_secondary" => Ok(Value::Bool(read.is_secondary())),
                        "is_supplementary" => Ok(Value::Bool(read.is_supplementary())),
                        "is_primary" => Ok(Value::Bool(read.is_primary())),
                        "aligned_length" => Ok(Value::Int(read.aligned_length())),
                        "query_length" => Ok(Value::Int(read.query_length())),
                        "end_pos" => Ok(Value::Int(read.end_pos())),
                        "interval" => Ok(Value::Interval(read.to_interval())),
                        other => Err(BioLangError::name_error(
                            format!("no field '{other}' on AlignedRead"),
                            Some(expr.span),
                        )),
                    },
                    Value::Matrix(ref m) => match field.as_str() {
                        "nrow" => Ok(Value::Int(m.nrow as i64)),
                        "ncol" => Ok(Value::Int(m.ncol as i64)),
                        "shape" => Ok(Value::List(vec![Value::Int(m.nrow as i64), Value::Int(m.ncol as i64)])),
                        "row_names" => Ok(m.row_names.as_ref()
                            .map(|names| Value::List(names.iter().map(|s| Value::Str(s.clone())).collect()))
                            .unwrap_or(Value::Nil)),
                        "col_names" => Ok(m.col_names.as_ref()
                            .map(|names| Value::List(names.iter().map(|s| Value::Str(s.clone())).collect()))
                            .unwrap_or(Value::Nil)),
                        "data" => Ok(Value::List(m.data.iter().map(|v| Value::Float(*v)).collect())),
                        other => Err(BioLangError::name_error(
                            format!("no field '{other}' on Matrix (available: nrow, ncol, shape, row_names, col_names, data)"),
                            Some(expr.span),
                        )),
                    },
                    other => Err(BioLangError::type_error(
                        format!("cannot access field on {}", other.type_of()),
                        Some(expr.span),
                    )),
                }
            }
            Expr::Index { object, index } => {
                let obj = self.eval_expr(object)?;
                let idx = self.eval_expr(index)?;
                match (&obj, &idx) {
                    (Value::List(list), Value::Int(i)) => {
                        let i = if *i < 0 {
                            (list.len() as i64 + i) as usize
                        } else {
                            *i as usize
                        };
                        list.get(i).cloned().ok_or_else(|| {
                            BioLangError::runtime(
                                ErrorKind::IndexOutOfBounds,
                                format!("index {i} out of bounds (len {})", list.len()),
                                Some(expr.span),
                            )
                        })
                    }
                    (Value::Map(map) | Value::Record(map), Value::Str(key)) => {
                        map.get(key).cloned().ok_or_else(|| {
                            BioLangError::name_error(
                                format!("key '{key}' not found"),
                                Some(expr.span),
                            )
                        })
                    }
                    (Value::Table(t), Value::Int(i)) => {
                        let i = if *i < 0 {
                            (t.num_rows() as i64 + i) as usize
                        } else {
                            *i as usize
                        };
                        if i < t.num_rows() {
                            Ok(Value::Record(t.row_to_record(i)))
                        } else {
                            Err(BioLangError::runtime(
                                ErrorKind::IndexOutOfBounds,
                                format!("index {i} out of bounds (table has {} rows)", t.num_rows()),
                                Some(expr.span),
                            ))
                        }
                    }
                    (Value::Table(t), Value::Str(col_name)) => {
                        match t.col_index(col_name) {
                            Some(ci) => Ok(Value::List(
                                t.rows.iter().map(|row| row[ci].clone()).collect(),
                            )),
                            None => Err(BioLangError::name_error(
                                format!("no column '{col_name}' in table"),
                                Some(expr.span),
                            )),
                        }
                    }
                    (Value::Str(s), Value::Int(i)) => {
                        let i = if *i < 0 {
                            (s.len() as i64 + i) as usize
                        } else {
                            *i as usize
                        };
                        s.chars().nth(i).map(|c| Value::Str(c.to_string())).ok_or_else(|| {
                            BioLangError::runtime(
                                ErrorKind::IndexOutOfBounds,
                                format!("index {i} out of bounds"),
                                Some(expr.span),
                            )
                        })
                    }
                    // Tuple indexing
                    (Value::Tuple(items), Value::Int(i)) => {
                        let i = if *i < 0 {
                            (items.len() as i64 + i) as usize
                        } else {
                            *i as usize
                        };
                        items.get(i).cloned().ok_or_else(|| {
                            BioLangError::runtime(
                                ErrorKind::IndexOutOfBounds,
                                format!("tuple index {i} out of bounds (len {})", items.len()),
                                Some(expr.span),
                            )
                        })
                    }
                    // Slicing: list[start..end] or str[start..end]
                    (Value::List(list), Value::Range { start, end, inclusive }) => {
                        let end = if *inclusive { *end as usize + 1 } else { *end as usize };
                        let start = *start as usize;
                        Ok(Value::List(list.get(start..end.min(list.len())).unwrap_or(&[]).to_vec()))
                    }
                    (Value::Str(s), Value::Range { start, end, inclusive }) => {
                        let end = if *inclusive { *end as usize + 1 } else { *end as usize };
                        let start = *start as usize;
                        let chars: Vec<char> = s.chars().collect();
                        let slice: String = chars.get(start..end.min(chars.len())).unwrap_or(&[]).iter().collect();
                        Ok(Value::Str(slice))
                    }
                    (Value::DNA(seq), Value::Range { start, end, inclusive }) => {
                        let end = if *inclusive { *end as usize + 1 } else { *end as usize };
                        let start = *start as usize;
                        let chars: Vec<char> = seq.data.chars().collect();
                        let slice: String = chars.get(start..end.min(chars.len())).unwrap_or(&[]).iter().collect();
                        Ok(Value::DNA(BioSequence { data: slice }))
                    }
                    (Value::RNA(seq), Value::Range { start, end, inclusive }) => {
                        let end = if *inclusive { *end as usize + 1 } else { *end as usize };
                        let start = *start as usize;
                        let chars: Vec<char> = seq.data.chars().collect();
                        let slice: String = chars.get(start..end.min(chars.len())).unwrap_or(&[]).iter().collect();
                        Ok(Value::RNA(BioSequence { data: slice }))
                    }
                    (Value::Protein(seq), Value::Range { start, end, inclusive }) => {
                        let end = if *inclusive { *end as usize + 1 } else { *end as usize };
                        let start = *start as usize;
                        let chars: Vec<char> = seq.data.chars().collect();
                        let slice: String = chars.get(start..end.min(chars.len())).unwrap_or(&[]).iter().collect();
                        Ok(Value::Protein(BioSequence { data: slice }))
                    }
                    _ => Err(BioLangError::type_error(
                        format!(
                            "cannot index {} with {}",
                            obj.type_of(),
                            idx.type_of()
                        ),
                        Some(expr.span),
                    )),
                }
            }
            Expr::Lambda { params, body } => {
                let closure_env = self.env.current_scope_id();
                Ok(Value::Function {
                    name: None,
                    params: params.clone(),
                    body: vec![Spanned::new(
                        Stmt::Return(Some(*body.clone())),
                        body.span,
                    )],
                    closure_env: Some(closure_env),
                    doc: None,
                    is_generator: false,
                })
            }
            Expr::Block(stmts) => {
                let prev = self.env.push_scope();
                let result = self.exec_block(stmts);
                self.env.pop_scope(prev);
                match result {
                    Ok(val) => Ok(val),
                    Err(e) if e.kind == ErrorKind::Return => {
                        // Unwrap return value from error message (hacky but works for now)
                        Err(e)
                    }
                    Err(e) => Err(e),
                }
            }
            Expr::If {
                condition,
                then_body,
                else_body,
            } => {
                let cond = self.eval_expr(condition)?;
                if cond.is_truthy() {
                    let prev = self.env.push_scope();
                    let result = self.exec_block(then_body);
                    self.env.pop_scope(prev);
                    result
                } else if let Some(else_body) = else_body {
                    let prev = self.env.push_scope();
                    let result = self.exec_block(else_body);
                    self.env.pop_scope(prev);
                    result
                } else {
                    Ok(Value::Nil)
                }
            }
            Expr::List(items) => {
                let mut values = Vec::new();
                for item in items {
                    values.push(self.eval_expr(item)?);
                }
                Ok(Value::List(values))
            }
            Expr::Record(fields) => {
                let mut map = std::collections::HashMap::new();
                for (key, value) in fields {
                    map.insert(key.clone(), self.eval_expr(value)?);
                }
                Ok(Value::Record(map))
            }
            Expr::Formula(inner) => Ok(Value::Formula(Box::new(inner.as_ref().clone()))),
            Expr::Match {
                expr: match_expr,
                arms,
            } => {
                let val = self.eval_expr(match_expr)?;
                for arm in arms {
                    if self.pattern_matches(&arm.pattern.node, &val)? {
                        let prev = self.env.push_scope();
                        // Bind pattern variables before evaluating guard
                        match &arm.pattern.node {
                            Pattern::Ident(name) => {
                                self.env.define(name.clone(), val.clone());
                            }
                            Pattern::EnumVariant { bindings, .. } => {
                                if let Value::EnumValue { fields, .. } = &val {
                                    for (name, field_val) in bindings.iter().zip(fields.iter()) {
                                        self.env.define(name.clone(), field_val.clone());
                                    }
                                }
                            }
                            Pattern::TypePattern { binding, .. } => {
                                if let Some(name) = binding {
                                    if name != "_" {
                                        self.env.define(name.clone(), val.clone());
                                    }
                                }
                            }
                            _ => {}
                        }
                        // Evaluate guard if present
                        if let Some(ref guard) = arm.guard {
                            let guard_val = self.eval_expr(guard)?;
                            if !guard_val.is_truthy() {
                                self.env.pop_scope(prev);
                                continue; // guard failed, try next arm
                            }
                        }
                        let result = self.eval_expr(&arm.body);
                        self.env.pop_scope(prev);
                        return result;
                    }
                }
                Ok(Value::Nil) // no arm matched
            }
            Expr::TryCatch {
                body,
                error_var,
                catch_body,
            } => {
                let prev = self.env.push_scope();
                let result = self.exec_block(body);
                self.env.pop_scope(prev);
                match result {
                    Ok(val) => Ok(val),
                    Err(e) if e.kind == ErrorKind::Return => Err(e), // propagate return
                    Err(e) if e.kind == ErrorKind::Break => Err(e),
                    Err(e) if e.kind == ErrorKind::Continue => Err(e),
                    Err(e) => {
                        let prev = self.env.push_scope();
                        let var_name = error_var.clone().unwrap_or_else(|| "error".to_string());
                        self.env.define(var_name, Value::Str(e.message.clone()));
                        let result = self.exec_block(catch_body);
                        self.env.pop_scope(prev);
                        result
                    }
                }
            }
            Expr::NullCoalesce { left, right } => {
                let lval = self.eval_expr(left)?;
                if matches!(lval, Value::Nil) {
                    self.eval_expr(right)
                } else {
                    Ok(lval)
                }
            }
            Expr::StringInterp(parts) => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        StringPart::Lit(s) => result.push_str(s),
                        StringPart::Expr(e) => {
                            let val = self.eval_expr(e)?;
                            result.push_str(&format!("{val}"));
                        }
                    }
                }
                Ok(Value::Str(result))
            }
            Expr::Range { start, end, inclusive } => {
                let s = self.eval_expr(start)?;
                let e = self.eval_expr(end)?;
                match (&s, &e) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Range {
                        start: *a,
                        end: *b,
                        inclusive: *inclusive,
                    }),
                    _ => Err(BioLangError::type_error(
                        format!("range bounds must be Int, got {} and {}", s.type_of(), e.type_of()),
                        Some(expr.span),
                    )),
                }
            }
            Expr::ListComp { expr: body_expr, var, iter, condition } => {
                let iterable = self.eval_expr(iter)?;
                let items = self.value_to_iter(&iterable, iter.span)?;
                let mut result = Vec::new();
                for item in items {
                    let prev = self.env.push_scope();
                    self.env.define(var.clone(), item);
                    let include = if let Some(cond) = condition {
                        self.eval_expr(cond)?.is_truthy()
                    } else {
                        true
                    };
                    if include {
                        result.push(self.eval_expr(body_expr)?);
                    }
                    self.env.pop_scope(prev);
                }
                Ok(Value::List(result))
            }
            Expr::Ternary { value, condition, else_value } => {
                let cond = self.eval_expr(condition)?;
                if cond.is_truthy() {
                    self.eval_expr(value)
                } else {
                    self.eval_expr(else_value)
                }
            }
            Expr::ChainedCmp { operands, ops } => {
                // Evaluate pairwise with short-circuit
                let mut left_val = self.eval_expr(&operands[0])?;
                for (i, op) in ops.iter().enumerate() {
                    let right_val = self.eval_expr(&operands[i + 1])?;
                    let cmp_result = self.eval_binary(*op, &left_val, &right_val, expr.span)?;
                    if !cmp_result.is_truthy() {
                        return Ok(Value::Bool(false));
                    }
                    left_val = right_val;
                }
                Ok(Value::Bool(true))
            }
            Expr::MapComp { key, value, var, iter, condition } => {
                let iterable = self.eval_expr(iter)?;
                let items = self.value_to_iter(&iterable, iter.span)?;
                let mut result = HashMap::new();
                for item in items {
                    let prev = self.env.push_scope();
                    self.env.define(var.clone(), item);
                    let include = if let Some(cond) = condition {
                        self.eval_expr(cond)?.is_truthy()
                    } else {
                        true
                    };
                    if include {
                        let k = self.eval_expr(key)?;
                        let v = self.eval_expr(value)?;
                        let k_str = match k {
                            Value::Str(s) => s,
                            other => format!("{other}"),
                        };
                        result.insert(k_str, v);
                    }
                    self.env.pop_scope(prev);
                }
                Ok(Value::Map(result))
            }
            Expr::SetLiteral(items) => {
                let mut result = Vec::new();
                for item in items {
                    let val = self.eval_expr(item)?;
                    if !result.contains(&val) {
                        result.push(val);
                    }
                }
                Ok(Value::Set(result))
            }
            Expr::Regex { pattern, flags } => {
                Ok(Value::Regex { pattern: pattern.clone(), flags: flags.clone() })
            }
            Expr::StructLit { name, fields } => {
                // Struct literal: evaluate fields and produce a Record
                // with a __struct field for dispatch
                let mut map = std::collections::HashMap::new();
                map.insert("__struct".to_string(), Value::Str(name.clone()));
                for (key, value) in fields {
                    map.insert(key.clone(), self.eval_expr(value)?);
                }
                Ok(Value::Record(map))
            }
            Expr::TupleLit(items) => {
                let vals: Vec<Value> = items.iter().map(|e| self.eval_expr(e)).collect::<Result<_>>()?;
                Ok(Value::Tuple(vals))
            }
            Expr::Await(inner) => {
                let val = self.eval_expr(inner)?;
                match val {
                    Value::Future(state) => {
                        let guard = state.lock().unwrap();
                        match &*guard {
                            bl_core::value::FutureState::Resolved(v) => Ok(v.clone()),
                            bl_core::value::FutureState::Pending { params, body, closure_env, args } => {
                                let params = params.clone();
                                let body = body.clone();
                                let closure_env = *closure_env;
                                let args = args.clone();
                                drop(guard);
                                // Execute the function synchronously
                                let result = self.call_function(
                                    &params, &body, &closure_env, args, Vec::new(), expr.span,
                                )?;
                                let mut guard = state.lock().unwrap();
                                *guard = bl_core::value::FutureState::Resolved(result.clone());
                                Ok(result)
                            }
                        }
                    }
                    // Await on non-future is identity
                    other => Ok(other),
                }
            }
            Expr::In { left, right, negated } => {
                let lhs = self.eval_expr(left)?;
                let rhs = self.eval_expr(right)?;
                let found = match &rhs {
                    Value::List(items) => items.contains(&lhs),
                    Value::Set(items) => items.contains(&lhs),
                    Value::Str(s) => {
                        if let Value::Str(needle) = &lhs {
                            s.contains(needle.as_str())
                        } else {
                            false
                        }
                    }
                    Value::Record(map) | Value::Map(map) => {
                        if let Value::Str(key) = &lhs {
                            map.contains_key(key)
                        } else {
                            false
                        }
                    }
                    Value::Range { start, end, inclusive } => {
                        if let Value::Int(n) = &lhs {
                            if *inclusive { *n >= *start && *n <= *end } else { *n >= *start && *n < *end }
                        } else {
                            false
                        }
                    }
                    _ => {
                        return Err(BioLangError::type_error(
                            format!("'in' not supported for {}", rhs.type_of()),
                            Some(expr.span),
                        ));
                    }
                };
                Ok(Value::Bool(if *negated { !found } else { found }))
            }
            Expr::DoBlock { params, body } => {
                let closure_env = self.env.current_scope_id();
                Ok(Value::Function {
                    name: None,
                    params: params.clone(),
                    body: body.clone(),
                    closure_env: Some(closure_env),
                    doc: None,
                    is_generator: false,
                })
            }
            Expr::TypeCast { expr: inner, target } => {
                let val = self.eval_expr(inner)?;
                self.type_cast(val, target, expr.span)
            }
            Expr::ThenPipe { left, var, right } => {
                let lhs = self.eval_expr(left)?;
                let prev = self.env.push_scope();
                self.env.define(var.clone(), lhs);
                let result = self.eval_expr(right);
                self.env.pop_scope(prev);
                result
            }
            Expr::Slice { object, start, end, step } => {
                let obj = self.eval_expr(object)?;
                let start_val = match start {
                    Some(s) => {
                        if let Value::Int(n) = self.eval_expr(s)? { Some(n) }
                        else { return Err(BioLangError::type_error("slice index must be Int", Some(expr.span))); }
                    }
                    None => None,
                };
                let end_val = match end {
                    Some(e) => {
                        if let Value::Int(n) = self.eval_expr(e)? { Some(n) }
                        else { return Err(BioLangError::type_error("slice index must be Int", Some(expr.span))); }
                    }
                    None => None,
                };
                let step_val = match step {
                    Some(s) => {
                        if let Value::Int(n) = self.eval_expr(s)? { Some(n) }
                        else { return Err(BioLangError::type_error("slice step must be Int", Some(expr.span))); }
                    }
                    None => None,
                };
                self.eval_slice(obj, start_val, end_val, step_val, expr.span)
            }
            Expr::TapPipe { left, right } => {
                let lhs = self.eval_expr(left)?;
                // Evaluate right side as a function call with lhs as argument,
                // or just evaluate it for side effects
                let prev = self.env.push_scope();
                self.env.define("_".to_string(), lhs.clone());
                match &right.node {
                    Expr::Call { .. } | Expr::Ident(_) => {
                        // If it's a function, call it with lhs as argument
                        let func = self.eval_expr(right)?;
                        let _ = self.call_value(&func, vec![lhs.clone()], expr.span);
                    }
                    _ => {
                        let _ = self.eval_expr(right);
                    }
                }
                self.env.pop_scope(prev);
                Ok(lhs) // Pass through the original value
            }
            Expr::Given { arms, otherwise } => {
                for (condition, body) in arms {
                    let cond = self.eval_expr(condition)?;
                    if cond.is_truthy() {
                        return self.eval_expr(body);
                    }
                }
                if let Some(default) = otherwise {
                    self.eval_expr(default)
                } else {
                    Ok(Value::Nil)
                }
            }
            Expr::Retry { count, delay, body } => {
                let max = match self.eval_expr(count)? {
                    Value::Int(n) => n,
                    _ => return Err(BioLangError::type_error("retry count must be Int", Some(expr.span))),
                };
                let delay_ms = if let Some(d) = delay {
                    match self.eval_expr(d)? {
                        Value::Int(n) => n as u64,
                        Value::Float(f) => f as u64,
                        _ => 0,
                    }
                } else {
                    0
                };
                let mut last_err = None;
                for attempt in 0..max {
                    match self.exec_block(body) {
                        Ok(val) => return Ok(val),
                        Err(e) if e.kind == ErrorKind::Return => return Err(e),
                        Err(e) => {
                            last_err = Some(e);
                            if attempt < max - 1 && delay_ms > 0 {
                                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                            }
                        }
                    }
                }
                Err(last_err.unwrap_or_else(|| BioLangError::runtime(
                    ErrorKind::TypeError,
                    "retry exhausted all attempts",
                    Some(expr.span),
                )))
            }
            Expr::RecordSpread { spreads, fields } => {
                let mut result: std::collections::HashMap<String, Value> = std::collections::HashMap::new();
                // First, apply spreads in order
                for spread_expr in spreads {
                    let val = self.eval_expr(spread_expr)?;
                    match val {
                        Value::Record(map) | Value::Map(map) => {
                            result.extend(map);
                        }
                        other => {
                            return Err(BioLangError::type_error(
                                format!("spread requires Record, got {}", other.type_of()),
                                Some(spread_expr.span),
                            ));
                        }
                    }
                }
                // Then, apply explicit fields (override spreads)
                for (key, value_expr) in fields {
                    let val = self.eval_expr(value_expr)?;
                    result.insert(key.clone(), val);
                }
                Ok(Value::Record(result))
            }
        }
    }

    /// Evaluate a slice operation on a list, string, or DNA sequence.
    fn eval_slice(
        &self,
        obj: Value,
        start: Option<i64>,
        end: Option<i64>,
        step: Option<i64>,
        span: bl_core::span::Span,
    ) -> Result<Value> {
        let step = step.unwrap_or(1);
        if step == 0 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "slice step cannot be zero",
                Some(span),
            ));
        }

        match obj {
            Value::List(ref items) => {
                let len = items.len() as i64;
                let s = resolve_slice_index(start.unwrap_or(0), len);
                let e = resolve_slice_index(end.unwrap_or(len), len);
                let result: Vec<Value> = if step > 0 {
                    (s..e).step_by(step as usize).filter_map(|i| items.get(i as usize).cloned()).collect()
                } else {
                    let mut indices = Vec::new();
                    let mut i = if start.is_none() { len - 1 } else { s };
                    let stop = if end.is_none() { -1 } else { e };
                    while i > stop {
                        indices.push(i);
                        i += step;
                    }
                    indices.into_iter().filter_map(|i| items.get(i as usize).cloned()).collect()
                };
                Ok(Value::List(result))
            }
            Value::Str(ref s) => {
                let chars: Vec<char> = s.chars().collect();
                let len = chars.len() as i64;
                let st = resolve_slice_index(start.unwrap_or(0), len);
                let en = resolve_slice_index(end.unwrap_or(len), len);
                let result: String = if step > 0 {
                    (st..en).step_by(step as usize).filter_map(|i| chars.get(i as usize)).collect()
                } else {
                    let mut indices = Vec::new();
                    let mut i = if start.is_none() { len - 1 } else { st };
                    let stop = if end.is_none() { -1 } else { en };
                    while i > stop {
                        indices.push(i);
                        i += step;
                    }
                    indices.into_iter().filter_map(|i| chars.get(i as usize)).collect()
                };
                Ok(Value::Str(result))
            }
            Value::DNA(ref bio) => {
                let chars: Vec<char> = bio.data.chars().collect();
                let len = chars.len() as i64;
                let st = resolve_slice_index(start.unwrap_or(0), len);
                let en = resolve_slice_index(end.unwrap_or(len), len);
                let result: String = if step > 0 {
                    (st..en).step_by(step as usize).filter_map(|i| chars.get(i as usize)).collect()
                } else {
                    let mut indices = Vec::new();
                    let mut i = if start.is_none() { len - 1 } else { st };
                    let stop = if end.is_none() { -1 } else { en };
                    while i > stop {
                        indices.push(i);
                        i += step;
                    }
                    indices.into_iter().filter_map(|i| chars.get(i as usize)).collect()
                };
                Ok(Value::DNA(BioSequence { data: result }))
            }
            _ => Err(BioLangError::type_error(
                format!("cannot slice {}", obj.type_of()),
                Some(span),
            )),
        }
    }

    /// Validate function arguments against their type annotations (@validate decorator).
    fn validate_args(
        &self,
        fn_name: &str,
        params: &[Param],
        args: &[Value],
        span: bl_core::span::Span,
    ) -> Result<()> {
        for (i, param) in params.iter().enumerate() {
            if let Some(ref type_ann) = param.type_ann {
                if let Some(arg) = args.get(i) {
                    let actual_type = format!("{}", arg.type_of());
                    if actual_type != type_ann.name
                        && !(type_ann.name == "Num" && matches!(arg, Value::Int(_) | Value::Float(_)))
                    {
                        return Err(BioLangError::runtime(
                            ErrorKind::TypeError,
                            format!(
                                "@validate: {fn_name}() parameter '{}' expected {}, got {}",
                                param.name, type_ann.name, actual_type
                            ),
                            Some(span),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Cast/narrow a value to a target type.
    fn type_cast(&self, val: Value, target: &str, span: bl_core::span::Span) -> Result<Value> {
        match target {
            "Int" => match &val {
                Value::Int(_) => Ok(val),
                Value::Float(f) => Ok(Value::Int(*f as i64)),
                Value::Str(s) => s.parse::<i64>().map(Value::Int).map_err(|_| {
                    BioLangError::type_error(format!("cannot cast '{s}' to Int"), Some(span))
                }),
                Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
                _ => Err(BioLangError::type_error(
                    format!("cannot cast {} to Int", val.type_of()), Some(span),
                )),
            },
            "Float" => match &val {
                Value::Float(_) => Ok(val),
                Value::Int(n) => Ok(Value::Float(*n as f64)),
                Value::Str(s) => s.parse::<f64>().map(Value::Float).map_err(|_| {
                    BioLangError::type_error(format!("cannot cast '{s}' to Float"), Some(span))
                }),
                _ => Err(BioLangError::type_error(
                    format!("cannot cast {} to Float", val.type_of()), Some(span),
                )),
            },
            "Str" => Ok(Value::Str(format!("{val}"))),
            "Bool" => Ok(Value::Bool(val.is_truthy())),
            "DNA" => match &val {
                Value::DNA(_) => Ok(val),
                Value::Str(s) => Ok(Value::DNA(BioSequence { data: s.to_uppercase() })),
                Value::RNA(seq) => Ok(Value::DNA(BioSequence {
                    data: seq.data.replace('U', "T"),
                })),
                _ => Err(BioLangError::type_error(
                    format!("cannot cast {} to DNA", val.type_of()), Some(span),
                )),
            },
            "RNA" => match &val {
                Value::RNA(_) => Ok(val),
                Value::Str(s) => Ok(Value::RNA(BioSequence { data: s.to_uppercase() })),
                Value::DNA(seq) => Ok(Value::RNA(BioSequence {
                    data: seq.data.replace('T', "U"),
                })),
                _ => Err(BioLangError::type_error(
                    format!("cannot cast {} to RNA", val.type_of()), Some(span),
                )),
            },
            "List" => match val {
                Value::List(_) => Ok(val),
                Value::Set(items) => Ok(Value::List(items)),
                Value::Tuple(items) => Ok(Value::List(items)),
                _ => Ok(Value::List(vec![val])),
            },
            _ => {
                // Identity cast if type name matches, otherwise error
                let actual = format!("{}", val.type_of());
                if actual == target {
                    Ok(val)
                } else {
                    Err(BioLangError::type_error(
                        format!("cannot cast {actual} to {target}"), Some(span),
                    ))
                }
            }
        }
    }

    fn eval_pipe(
        &mut self,
        left: &Spanned<Expr>,
        right: &Spanned<Expr>,
    ) -> Result<Value> {
        let lhs = self.eval_expr(left)?;

        // Pipe desugaring: `a |> f(b)` → `f(a, b)`
        // Also supports: `a |> |x| expr` (lambda application)
        match &right.node {
            Expr::Call { callee, args } => {
                // Evaluate callee
                let func = self.eval_expr(callee)?;
                // Evaluate existing args
                let mut evaluated_args = vec![lhs];
                for arg in args {
                    evaluated_args.push(self.eval_expr(&arg.value)?);
                }
                self.call_value(&func, evaluated_args, right.span)
            }
            Expr::Ident(_) => {
                // `a |> f` → `f(a)`
                let func = self.eval_expr(right)?;
                self.call_value(&func, vec![lhs], right.span)
            }
            Expr::Lambda { .. } => {
                // `a |> |x| expr` → evaluate lambda with a
                let func = self.eval_expr(right)?;
                self.call_value(&func, vec![lhs], right.span)
            }
            _ => {
                // For other expressions, evaluate RHS as a function and call it
                let func = self.eval_expr(right)?;
                self.call_value(&func, vec![lhs], right.span)
            }
        }
    }

    #[inline(never)]
    fn eval_call(
        &mut self,
        callee: &Spanned<Expr>,
        args: &[Arg],
        span: bl_core::span::Span,
    ) -> Result<Value> {
        // UFCS: obj.method(args) → method(obj, args)
        if let Expr::Field { object, field, .. } = &callee.node {
            let obj = self.eval_expr(object)?;
            // Try genuine field access (callable field)
            let field_val = match &obj {
                Value::Record(map) | Value::Map(map) => map.get(field).cloned(),
                _ => None,
            };
            if let Some(fv) = field_val {
                if matches!(fv, Value::Function { .. } | Value::NativeFunction { .. }) {
                    // Call the field value as a function
                    let mut positional = Vec::new();
                    let mut named = Vec::new();
                    for arg in args {
                        let val = self.eval_expr(&arg.value)?;
                        match &arg.name {
                            Some(name) => named.push((name.clone(), val)),
                            None => {
                                if arg.spread {
                                    if let Value::List(items) = val {
                                        positional.extend(items);
                                    } else {
                                        positional.push(val);
                                    }
                                } else {
                                    positional.push(val);
                                }
                            }
                        }
                    }
                    return match &fv {
                        Value::NativeFunction { name, arity } => {
                            check_arity(name, arity, positional.len(), span)?;
                            call_builtin(name, positional)
                        }
                        Value::Function { params, body, closure_env, .. } => {
                            self.call_function(params, body, closure_env, positional, named, span)
                        }
                        _ => unreachable!(),
                    };
                }
            }
            // Impl dispatch: check __impl_{Type}_{method} BEFORE UFCS global lookup
            let type_name = self.runtime_type_name(&obj);
            let impl_key = format!("__impl_{type_name}_{field}");
            if let Ok(func) = self.env.get(&impl_key, None).cloned() {
                if let Value::Function { params, body, closure_env, .. } = &func {
                    let mut positional = vec![obj];
                    let mut named = Vec::new();
                    for arg in args {
                        let val = self.eval_expr(&arg.value)?;
                        match &arg.name {
                            Some(name) => named.push((name.clone(), val)),
                            None => positional.push(val),
                        }
                    }
                    return self.call_function(params, body, closure_env, positional, named, span);
                }
            }
            // UFCS fallback: look up field as a function, prepend obj as first arg
            if let Ok(func) = self.env.get(field, None).cloned() {
                if matches!(func, Value::Function { .. } | Value::NativeFunction { .. }) {
                    let mut positional = vec![obj];
                    let mut named = Vec::new();
                    for arg in args {
                        let val = self.eval_expr(&arg.value)?;
                        match &arg.name {
                            Some(name) => named.push((name.clone(), val)),
                            None => {
                                if arg.spread {
                                    if let Value::List(items) = val {
                                        positional.extend(items);
                                    } else {
                                        positional.push(val);
                                    }
                                } else {
                                    positional.push(val);
                                }
                            }
                        }
                    }
                    return match &func {
                        Value::NativeFunction { name, arity } => {
                            match name.as_str() {
                                "map" | "filter" | "reduce" | "sort" | "mutate" | "summarize"
                                | "flat_map" | "scan"
                                | "mat_map" | "any" | "all" | "none" | "find" | "find_index" | "try_call" | "take_while" | "ode_solve"
                                | "par_map" | "par_filter" | "await_all"
                                | "stream_batch" | "scatter_by" | "bench"
                                | "where" | "case_when" | "each" | "tap" | "inspect" | "group_apply"
                                | "partition" | "sort_by" | "count_if" => {
                                    return self.call_hof_with_values(name, positional, span);
                                }
                                _ => {}
                            }
                            check_arity(name, arity, positional.len(), span)?;
                            call_builtin(name, positional)
                        }
                        Value::Function { params, body, closure_env, .. } => {
                            self.call_function(params, body, closure_env, positional, named, span)
                        }
                        _ => unreachable!(),
                    };
                }
            }
            return Err(BioLangError::type_error(
                format!("cannot call field '{}' on {}", field, obj.type_of()),
                Some(span),
            ));
        }

        let func = self.eval_expr(callee)?;

        let mut positional = Vec::new();
        let mut named = Vec::new();
        for arg in args {
            let val = self.eval_expr(&arg.value)?;
            match &arg.name {
                Some(name) => named.push((name.clone(), val)),
                None => {
                    if arg.spread {
                        // Spread: ...list expands into positional args
                        if let Value::List(items) = val {
                            positional.extend(items);
                        } else {
                            positional.push(val);
                        }
                    } else {
                        positional.push(val);
                    }
                }
            }
        }

        match &func {
            Value::NativeFunction { name, arity } => {
                // For builtins that take closures (map, filter, reduce, sort, mutate, summarize),
                // we need special handling
                match name.as_str() {
                    "map" | "filter" | "reduce" | "sort" | "mutate" | "summarize"
                    | "flat_map" | "scan"
                    | "mat_map" | "any" | "all" | "none" | "find" | "find_index" | "try_call" | "take_while" | "ode_solve"
                    | "par_map" | "par_filter" | "prop_test" | "await_all"
                    | "stream_batch" | "scatter_by" | "bench" | "each" | "tap" | "inspect" | "group_apply"
                    | "partition" | "sort_by" | "count_if" => {
                        return self.call_hof(name, args, span);
                    }
                    _ => {}
                }

                check_arity(name, arity, positional.len(), span)?;
                // Merge named args by position (builtins don't support named args yet)
                call_builtin(name, positional)
            }
            Value::Function {
                params,
                body,
                closure_env,
                is_generator,
                ..
            } => {
                if *is_generator {
                    let params = params.clone();
                    let body = body.clone();
                    let closure_env = *closure_env;
                    let env_snapshot = self.env.clone();
                    let (tx, rx) = std::sync::mpsc::sync_channel::<Value>(1);
                    std::thread::spawn(move || {
                        let mut interp = Interpreter::with_env(env_snapshot);
                        interp.yield_sender = Some(tx);
                        let _ = interp.call_function(&params, &body, &closure_env, positional, named, span);
                        // drop sender on return → receiver gets None
                    });
                    let iter = GeneratorIterator { rx };
                    Ok(Value::Stream(bl_core::value::StreamValue::new("generator", Box::new(iter))))
                } else {
                    self.call_function(params, body, closure_env, positional, named, span)
                }
            }
            #[cfg(feature = "native")]
            Value::PluginFunction {
                plugin_name,
                operation,
                plugin_dir,
                kind,
                entrypoint,
            } => crate::plugins::call_plugin(
                plugin_name,
                operation,
                plugin_dir,
                kind,
                entrypoint,
                positional,
            ),
            other => Err(BioLangError::type_error(
                format!("{} is not callable", other.type_of()),
                Some(span),
            )),
        }
    }

    #[inline(never)]
    fn call_value(
        &mut self,
        func: &Value,
        args: Vec<Value>,
        span: bl_core::span::Span,
    ) -> Result<Value> {
        match func {
            Value::NativeFunction { name, arity } => {
                // HOF builtins need interpreter access to call closures
                match name.as_str() {
                    "map" | "filter" | "reduce" | "sort" | "mutate" | "summarize"
                    | "flat_map" | "scan"
                    | "mat_map" | "any" | "all" | "none" | "find" | "find_index" | "try_call" | "take_while" | "ode_solve"
                    | "par_map" | "par_filter" | "prop_test" | "await_all"
                    | "stream_batch" | "scatter_by" | "bench"
                    | "where" | "case_when" | "each" | "tap" | "inspect" | "group_apply"
                    | "partition" | "sort_by" | "count_if" => {
                        return self.call_hof_with_values(name, args, span);
                    }
                    _ => {}
                }
                check_arity(name, arity, args.len(), span)?;
                call_builtin(name, args)
            }
            Value::Function {
                name: fn_name,
                params,
                body,
                closure_env,
                is_generator,
                ..
            } => {
                // Check for struct constructor (empty body + __struct_ctor_ marker)
                if let Some(ref name) = fn_name {
                    if self.env.get(&format!("__struct_ctor_{name}"), None).is_ok() {
                        return self.call_struct_constructor(name, params, &args, span);
                    }
                    // Check for enum constructor
                    if let Some(ref name_str) = fn_name {
                        if name_str.contains("::") {
                            // Enum constructor: build EnumValue from positional args
                            let parts: Vec<&str> = name_str.splitn(2, "::").collect();
                            if parts.len() == 2 {
                                return Ok(Value::EnumValue {
                                    enum_name: parts[0].to_string(),
                                    variant: parts[1].to_string(),
                                    fields: args,
                                });
                            }
                        }
                    }
                    // Check for async function
                    if self.env.get(&format!("__async_{name}"), None).is_ok() {
                        let future_state = bl_core::value::FutureState::Pending {
                            params: params.clone(),
                            body: body.clone(),
                            closure_env: *closure_env,
                            args,
                        };
                        return Ok(Value::Future(std::sync::Arc::new(std::sync::Mutex::new(future_state))));
                    }
                }
                if *is_generator {
                    let params = params.clone();
                    let body = body.clone();
                    let closure_env = *closure_env;
                    let env_snapshot = self.env.clone();
                    let (tx, rx) = std::sync::mpsc::sync_channel::<Value>(1);
                    std::thread::spawn(move || {
                        let mut interp = Interpreter::with_env(env_snapshot);
                        interp.yield_sender = Some(tx);
                        let _ = interp.call_function(&params, &body, &closure_env, args, vec![], span);
                    });
                    let iter = GeneratorIterator { rx };
                    Ok(Value::Stream(bl_core::value::StreamValue::new("generator", Box::new(iter))))
                } else {
                    // @validate decorator: check argument types against annotations
                    if let Some(ref name) = fn_name {
                        if self.env.get(&format!("__validate_{name}"), None).is_ok() {
                            self.validate_args(name, params, &args, span)?;
                        }
                    }
                    // @memoize decorator: check cache before calling
                    if let Some(ref name) = fn_name {
                        let memo_key = format!("__memoize_{name}");
                        if let Ok(Value::Record(cache)) = self.env.get(&memo_key, None).cloned() {
                            let args_key = format!("{:?}", args);
                            if let Some(cached) = cache.get(&args_key) {
                                return Ok(cached.clone());
                            }
                            let result = self.call_function(params, body, closure_env, args.clone(), vec![], span)?;
                            // Store in cache
                            let mut new_cache = cache;
                            new_cache.insert(args_key, result.clone());
                            let _ = self.env.set(&memo_key, Value::Record(new_cache), Some(span));
                            return Ok(result);
                        }
                    }
                    let result = self.call_function(params, body, closure_env, args, vec![], span)?;
                    // Named tuple returns: wrap Tuple/List into Record
                    if let Some(ref name) = fn_name {
                        if let Ok(Value::List(names)) = self.env.get(&format!("__named_returns_{name}"), None).cloned() {
                            let items = match &result {
                                Value::Tuple(t) => Some(t.clone()),
                                Value::List(l) => Some(l.clone()),
                                _ => None,
                            };
                            if let Some(items) = items {
                                let mut record = HashMap::new();
                                for (i, name_val) in names.iter().enumerate() {
                                    if let Value::Str(field_name) = name_val {
                                        record.insert(
                                            field_name.clone(),
                                            items.get(i).cloned().unwrap_or(Value::Nil),
                                        );
                                    }
                                }
                                return Ok(Value::Record(record));
                            }
                        }
                    }
                    Ok(result)
                }
            }
            #[cfg(feature = "native")]
            Value::PluginFunction {
                plugin_name,
                operation,
                plugin_dir,
                kind,
                entrypoint,
            } => crate::plugins::call_plugin(
                plugin_name,
                operation,
                plugin_dir,
                kind,
                entrypoint,
                args,
            ),
            #[cfg(feature = "bytecode")]
            Value::CompiledClosure(ref closure_any) => {
                crate::compiled::call_compiled_closure(closure_any, args)
            }
            other => Err(BioLangError::type_error(
                format!("{} is not callable", other.type_of()),
                Some(span),
            )),
        }
    }

    /// Handle HOF builtins with already-evaluated arguments.
    #[inline(never)]
    fn call_hof_with_values(
        &mut self,
        name: &str,
        mut args: Vec<Value>,
        span: bl_core::span::Span,
    ) -> Result<Value> {
        match name {
            "map" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "map() takes exactly 2 arguments (collection, fn)",
                        Some(span),
                    ));
                }
                let func = args.pop().unwrap();
                let collection = args.pop().unwrap();
                match collection {
                    Value::Table(t) => {
                        let mut result = Vec::with_capacity(t.num_rows());
                        for i in 0..t.num_rows() {
                            let row_rec = Value::Record(t.row_to_record(i));
                            result.push(self.call_value(&func, vec![row_rec], span)?);
                        }
                        Ok(Value::List(result))
                    }
                    Value::List(items) => {
                        let mut result = Vec::with_capacity(items.len());
                        for item in items {
                            result.push(self.call_value(&func, vec![item], span)?);
                        }
                        Ok(Value::List(result))
                    }
                    Value::Stream(s) => {
                        if s.is_exhausted() {
                            return Err(BioLangError::runtime(
                                ErrorKind::TypeError,
                                format!(
                                    "stream already consumed: Stream({}) has been fully iterated. \
                                     Streams can only be consumed once — re-read the file or \
                                     collect() into a variable on first use.",
                                    s.label
                                ),
                                Some(span),
                            ));
                        }
                        let source = s.clone();
                        let env_snapshot = self.env.clone();
                        let (tx, rx) = std::sync::mpsc::sync_channel::<Value>(1);
                        std::thread::spawn(move || {
                            let mut interp = Interpreter::with_env(env_snapshot);
                            while let Some(item) = source.next() {
                                match interp.call_value(&func, vec![item], span) {
                                    Ok(val) => { if tx.send(val).is_err() { break; } }
                                    Err(_) => break,
                                }
                            }
                        });
                        Ok(Value::Stream(bl_core::value::StreamValue::new("map", Box::new(GeneratorIterator { rx }))))
                    }
                    other => {
                        Err(BioLangError::type_error(
                            format!("map() requires List, Stream, or Table, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                }
            }
            "each" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "each() takes exactly 2 arguments (collection, fn)",
                        Some(span),
                    ));
                }
                let func = args.pop().unwrap();
                let collection = args.pop().unwrap();
                match collection {
                    Value::List(items) => {
                        let mut results = Vec::with_capacity(items.len());
                        for item in items {
                            results.push(self.call_value(&func, vec![item], span)?);
                        }
                        Ok(Value::List(results))
                    }
                    Value::Stream(s) => {
                        if s.is_exhausted() {
                            return Err(BioLangError::runtime(
                                ErrorKind::TypeError,
                                format!(
                                    "stream already consumed: Stream({}) has been fully iterated. \
                                     Streams can only be consumed once — re-read the file or \
                                     collect() into a variable on first use.",
                                    s.label
                                ),
                                Some(span),
                            ));
                        }
                        while let Some(item) = s.next() {
                            self.call_value(&func, vec![item], span)?;
                        }
                        Ok(Value::Nil)
                    }
                    Value::Table(t) => {
                        let mut results = Vec::with_capacity(t.num_rows());
                        for i in 0..t.num_rows() {
                            let row_rec = Value::Record(t.row_to_record(i));
                            results.push(self.call_value(&func, vec![row_rec], span)?);
                        }
                        Ok(Value::List(results))
                    }
                    other => {
                        Err(BioLangError::type_error(
                            format!("each() requires List, Stream, or Table, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                }
            }
            "filter" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "filter() takes exactly 2 arguments (collection, fn)",
                        Some(span),
                    ));
                }
                let func = args.pop().unwrap();
                let collection = args.pop().unwrap();
                match collection {
                    Value::Table(t) => {
                        let columns = t.columns.clone();
                        let mut kept_rows = Vec::new();
                        for i in 0..t.num_rows() {
                            let row_rec = Value::Record(t.row_to_record(i));
                            let keep = self.call_value(&func, vec![row_rec], span)?;
                            if keep.is_truthy() {
                                kept_rows.push(t.rows[i].clone());
                            }
                        }
                        Ok(Value::Table(bl_core::value::Table::new(columns, kept_rows)))
                    }
                    Value::List(items) => {
                        let mut result = Vec::new();
                        for item in items {
                            let keep = self.call_value(&func, vec![item.clone()], span)?;
                            if keep.is_truthy() {
                                result.push(item);
                            }
                        }
                        Ok(Value::List(result))
                    }
                    Value::Stream(s) => {
                        if s.is_exhausted() {
                            return Err(BioLangError::runtime(
                                ErrorKind::TypeError,
                                format!(
                                    "stream already consumed: Stream({}) has been fully iterated. \
                                     Streams can only be consumed once — re-read the file or \
                                     collect() into a variable on first use.",
                                    s.label
                                ),
                                Some(span),
                            ));
                        }
                        let source = s.clone();
                        let env_snapshot = self.env.clone();
                        let (tx, rx) = std::sync::mpsc::sync_channel::<Value>(1);
                        std::thread::spawn(move || {
                            let mut interp = Interpreter::with_env(env_snapshot);
                            while let Some(item) = source.next() {
                                match interp.call_value(&func, vec![item.clone()], span) {
                                    Ok(keep) => {
                                        if keep.is_truthy() {
                                            if tx.send(item).is_err() { break; }
                                        }
                                    }
                                    Err(_) => break,
                                }
                            }
                        });
                        Ok(Value::Stream(bl_core::value::StreamValue::new("filter", Box::new(GeneratorIterator { rx }))))
                    }
                    other => {
                        Err(BioLangError::type_error(
                            format!("filter() requires List, Stream, or Table, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                }
            }
            "flat_map" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "flat_map() takes exactly 2 arguments (collection, fn)",
                        Some(span),
                    ));
                }
                let func = args[1].clone();
                match &args[0] {
                    Value::List(l) => {
                        let mut result = Vec::new();
                        for item in l.clone() {
                            let mapped = self.call_value(&func, vec![item], span)?;
                            match mapped {
                                Value::List(inner) => result.extend(inner),
                                other => result.push(other),
                            }
                        }
                        Ok(Value::List(result))
                    }
                    Value::Table(t) => {
                        let mut result = Vec::new();
                        for i in 0..t.num_rows() {
                            let row_rec = Value::Record(t.row_to_record(i));
                            let mapped = self.call_value(&func, vec![row_rec], span)?;
                            match mapped {
                                Value::List(inner) => result.extend(inner),
                                other => result.push(other),
                            }
                        }
                        Ok(Value::List(result))
                    }
                    Value::Stream(s) => {
                        if s.is_exhausted() {
                            return Err(BioLangError::runtime(
                                ErrorKind::TypeError,
                                format!(
                                    "stream already consumed: Stream({}) has been fully iterated.",
                                    s.label
                                ),
                                Some(span),
                            ));
                        }
                        let source = s.clone();
                        let env_snapshot = self.env.clone();
                        let (tx, rx) = std::sync::mpsc::sync_channel::<Value>(1);
                        std::thread::spawn(move || {
                            let mut interp = Interpreter::with_env(env_snapshot);
                            while let Some(item) = source.next() {
                                match interp.call_value(&func, vec![item], span) {
                                    Ok(Value::List(inner)) => {
                                        for v in inner {
                                            if tx.send(v).is_err() { return; }
                                        }
                                    }
                                    Ok(val) => { if tx.send(val).is_err() { return; } }
                                    Err(_) => return,
                                }
                            }
                        });
                        Ok(Value::Stream(bl_core::value::StreamValue::new("flat_map", Box::new(GeneratorIterator { rx }))))
                    }
                    other => Err(BioLangError::type_error(
                        format!("flat_map() requires List, Stream, or Table, got {}", other.type_of()),
                        Some(span),
                    )),
                }
            }
            "scan" => {
                if args.len() != 3 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "scan() takes exactly 3 arguments (collection, initial, fn)",
                        Some(span),
                    ));
                }
                let (initial, func) = if args[2].is_callable() {
                    (args[1].clone(), args[2].clone())
                } else {
                    (args[2].clone(), args[1].clone())
                };
                match &args[0] {
                    Value::List(l) => {
                        let mut acc = initial;
                        let mut result = Vec::with_capacity(l.len());
                        for item in l.clone() {
                            acc = self.call_value(&func, vec![acc, item], span)?;
                            result.push(acc.clone());
                        }
                        Ok(Value::List(result))
                    }
                    Value::Stream(s) => {
                        if s.is_exhausted() {
                            return Err(BioLangError::runtime(
                                ErrorKind::TypeError,
                                format!(
                                    "stream already consumed: Stream({}) has been fully iterated.",
                                    s.label
                                ),
                                Some(span),
                            ));
                        }
                        let source = s.clone();
                        let env_snapshot = self.env.clone();
                        let (tx, rx) = std::sync::mpsc::sync_channel::<Value>(1);
                        std::thread::spawn(move || {
                            let mut interp = Interpreter::with_env(env_snapshot);
                            let mut acc = initial;
                            while let Some(item) = source.next() {
                                match interp.call_value(&func, vec![acc.clone(), item], span) {
                                    Ok(val) => {
                                        acc = val.clone();
                                        if tx.send(val).is_err() { return; }
                                    }
                                    Err(_) => return,
                                }
                            }
                        });
                        Ok(Value::Stream(bl_core::value::StreamValue::new("scan", Box::new(GeneratorIterator { rx }))))
                    }
                    other => Err(BioLangError::type_error(
                        format!("scan() requires List or Stream, got {}", other.type_of()),
                        Some(span),
                    )),
                }
            }
            "reduce" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "reduce() takes 2-3 arguments (collection, fn[, initial])",
                        Some(span),
                    ));
                }
                // Auto-detect argument order when 3 args:
                //   reduce(collection, fn, initial) OR reduce(collection, initial, fn)
                let func = if args.len() == 3 {
                    if args[1].is_callable() {
                        args[1].clone()
                    } else {
                        args[2].clone()
                    }
                } else {
                    args[1].clone()
                };
                match &args[0] {
                    Value::List(l) => {
                        let items = l.clone();
                        let (mut acc, start) = if args.len() == 3 {
                            let initial = if args[1].is_callable() { args[2].clone() } else { args[1].clone() };
                            (initial, 0)
                        } else {
                            if items.is_empty() {
                                return Err(BioLangError::runtime(
                                    ErrorKind::TypeError,
                                    "reduce() on empty list requires initial value",
                                    Some(span),
                                ));
                            }
                            (items[0].clone(), 1)
                        };
                        for item in &items[start..] {
                            acc = self.call_value(&func, vec![acc, item.clone()], span)?;
                        }
                        Ok(acc)
                    }
                    Value::Stream(s) => {
                        if s.is_exhausted() {
                            return Err(BioLangError::runtime(
                                ErrorKind::TypeError,
                                format!(
                                    "stream already consumed: Stream({}) has been fully iterated. \
                                     Streams can only be consumed once — re-read the file or \
                                     collect() into a variable on first use.",
                                    s.label
                                ),
                                Some(span),
                            ));
                        }
                        let mut acc = if args.len() == 3 {
                            let initial = if args[1].is_callable() { args[2].clone() } else { args[1].clone() };
                            initial
                        } else {
                            match s.next() {
                                Some(v) => v,
                                None => return Err(BioLangError::runtime(
                                    ErrorKind::TypeError,
                                    "reduce() on empty stream requires initial value",
                                    Some(span),
                                )),
                            }
                        };
                        while let Some(item) = s.next() {
                            acc = self.call_value(&func, vec![acc, item], span)?;
                        }
                        Ok(acc)
                    }
                    other => {
                        return Err(BioLangError::type_error(
                            format!("reduce() requires List or Stream, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                }
            }
            "sort" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "sort() takes 1-2 arguments (list[, compare_fn])",
                        Some(span),
                    ));
                }
                let mut items = match &args[0] {
                    Value::List(l) => l.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("sort() requires List, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                if args.len() == 2 {
                    let func = args[1].clone();
                    let mut err = None;
                    items.sort_by(|a, b| {
                        if err.is_some() {
                            return std::cmp::Ordering::Equal;
                        }
                        match self.call_value(&func, vec![a.clone(), b.clone()], span) {
                            Ok(Value::Int(n)) if n < 0 => std::cmp::Ordering::Less,
                            Ok(Value::Int(n)) if n > 0 => std::cmp::Ordering::Greater,
                            Ok(Value::Int(_)) => std::cmp::Ordering::Equal,
                            Ok(_) => {
                                err = Some(BioLangError::type_error(
                                    "sort compare function must return Int",
                                    Some(span),
                                ));
                                std::cmp::Ordering::Equal
                            }
                            Err(e) => {
                                err = Some(e);
                                std::cmp::Ordering::Equal
                            }
                        }
                    });
                    if let Some(e) = err {
                        return Err(e);
                    }
                } else {
                    let mut err = None;
                    items.sort_by(|a, b| {
                        if err.is_some() {
                            return std::cmp::Ordering::Equal;
                        }
                        match (a, b) {
                            (Value::Int(a), Value::Int(b)) => a.cmp(b),
                            (Value::Float(a), Value::Float(b)) => {
                                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                            }
                            (Value::Str(a), Value::Str(b)) => a.cmp(b),
                            _ => {
                                err = Some(BioLangError::type_error(
                                    "cannot compare values for sorting",
                                    Some(span),
                                ));
                                std::cmp::Ordering::Equal
                            }
                        }
                    });
                    if let Some(e) = err {
                        return Err(e);
                    }
                }
                Ok(Value::List(items))
            }
            "mutate" => {
                // mutate(table, "col_name", |row| expr)
                if args.len() != 3 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "mutate() takes 3 arguments (table, col_name, fn)",
                        Some(span),
                    ));
                }
                let table = match &args[0] {
                    Value::Table(t) => t,
                    other => {
                        return Err(BioLangError::type_error(
                            format!("mutate() requires Table, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let col_name = match &args[1] {
                    Value::Str(s) => s.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("mutate() column name must be Str, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let func = args[2].clone();

                // Compute new column values
                let mut new_col_vals = Vec::new();
                for i in 0..table.num_rows() {
                    let row_rec = Value::Record(table.row_to_record(i));
                    new_col_vals.push(self.call_value(&func, vec![row_rec], span)?);
                }

                // Build new table: add or replace column
                let mut columns = table.columns.clone();
                let mut rows = table.rows.clone();

                if let Some(ci) = table.col_index(&col_name) {
                    // Replace existing column
                    for (ri, row) in rows.iter_mut().enumerate() {
                        row[ci] = new_col_vals[ri].clone();
                    }
                } else {
                    // Add new column
                    columns.push(col_name);
                    for (ri, row) in rows.iter_mut().enumerate() {
                        row.push(new_col_vals[ri].clone());
                    }
                }

                Ok(Value::Table(bl_core::value::Table::new(columns, rows)))
            }
            "summarize" => {
                // summarize(grouped_map, |key, subtable| record)
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "summarize() takes 2 arguments (grouped_map, fn)",
                        Some(span),
                    ));
                }
                let groups = match &args[0] {
                    Value::Map(m) => m.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("summarize() requires Map (from group_by), got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let func = args[1].clone();

                let mut records = Vec::new();
                for (key, subtable) in &groups {
                    let result = self.call_value(
                        &func,
                        vec![Value::Str(key.clone()), subtable.clone()],
                        span,
                    )?;
                    match result {
                        Value::Record(rec) => records.push(rec),
                        other => {
                            return Err(BioLangError::type_error(
                                format!(
                                    "summarize() callback must return Record, got {}",
                                    other.type_of()
                                ),
                                Some(span),
                            ))
                        }
                    }
                }

                Ok(Value::Table(bl_core::value::Table::from_records(&records)))
            }
            "mat_map" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "mat_map() takes exactly 2 arguments (matrix, fn)",
                        Some(span),
                    ));
                }
                let m = match &args[0] {
                    Value::Matrix(m) => m.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("mat_map() requires Matrix, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let func = args[1].clone();
                let mut new_data = Vec::with_capacity(m.data.len());
                for &val in &m.data {
                    let result = self.call_value(&func, vec![Value::Float(val)], span)?;
                    match result {
                        Value::Float(f) => new_data.push(f),
                        Value::Int(n) => new_data.push(n as f64),
                        _ => {
                            return Err(BioLangError::type_error(
                                "mat_map() closure must return a number",
                                Some(span),
                            ))
                        }
                    }
                }
                let result = bl_core::matrix::Matrix::new(new_data, m.nrow, m.ncol)
                    .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, Some(span)))?;
                Ok(Value::Matrix(result))
            }
            "ode_solve" => {
                // ode_solve(f, y0, t_span) — RK4 integrator
                // f: |t, y| -> dy/dt (takes Float t + List y, returns List dy)
                // y0: List of initial values
                // t_span: List [t_start, t_end] or List [t_start, t_end, dt]
                if args.len() != 3 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "ode_solve() takes 3 arguments (f, y0, t_span)",
                        Some(span),
                    ));
                }
                let func = args[0].clone();
                let y0: Vec<f64> = match &args[1] {
                    Value::List(items) => items
                        .iter()
                        .map(|v| match v {
                            Value::Int(n) => Ok(*n as f64),
                            Value::Float(f) => Ok(*f),
                            _ => Err(BioLangError::type_error(
                                "ode_solve() y0 must be numeric list",
                                Some(span),
                            )),
                        })
                        .collect::<Result<Vec<_>>>()?,
                    Value::Int(n) => vec![*n as f64],
                    Value::Float(f) => vec![*f],
                    other => {
                        return Err(BioLangError::type_error(
                            format!("ode_solve() y0 must be List, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let t_span: Vec<f64> = match &args[2] {
                    Value::List(items) => items
                        .iter()
                        .map(|v| match v {
                            Value::Int(n) => Ok(*n as f64),
                            Value::Float(f) => Ok(*f),
                            _ => Err(BioLangError::type_error(
                                "ode_solve() t_span must be numeric list",
                                Some(span),
                            )),
                        })
                        .collect::<Result<Vec<_>>>()?,
                    _ => {
                        return Err(BioLangError::type_error(
                            "ode_solve() t_span must be List [t_start, t_end] or [t_start, t_end, dt]",
                            Some(span),
                        ))
                    }
                };
                if t_span.len() < 2 || t_span.len() > 3 {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "ode_solve() t_span must be [t_start, t_end] or [t_start, t_end, dt]",
                        Some(span),
                    ));
                }
                let t_start = t_span[0];
                let t_end = t_span[1];
                let dt = if t_span.len() == 3 { t_span[2] } else { (t_end - t_start) / 100.0 };
                if dt <= 0.0 {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "ode_solve() dt must be positive",
                        Some(span),
                    ));
                }

                let n = y0.len();
                let mut t = t_start;
                let mut y = y0;
                let mut t_out = vec![Value::Float(t)];
                let mut y_out: Vec<Vec<Value>> = (0..n).map(|i| vec![Value::Float(y[i])]).collect();

                let call_f = |interp: &mut Self, t_val: f64, y_val: &[f64]| -> Result<Vec<f64>> {
                    let y_list = Value::List(y_val.iter().map(|&v| Value::Float(v)).collect());
                    let result = interp.call_value(&func, vec![Value::Float(t_val), y_list], span)?;
                    match result {
                        Value::List(items) => items
                            .iter()
                            .map(|v| match v {
                                Value::Int(n) => Ok(*n as f64),
                                Value::Float(f) => Ok(*f),
                                _ => Err(BioLangError::type_error(
                                    "ode_solve() f must return numeric list",
                                    Some(span),
                                )),
                            })
                            .collect(),
                        Value::Float(f) => Ok(vec![f]),
                        Value::Int(iv) => Ok(vec![iv as f64]),
                        _ => Err(BioLangError::type_error(
                            "ode_solve() f must return numeric list or number",
                            Some(span),
                        )),
                    }
                };

                let max_steps = ((t_end - t_start) / dt).ceil() as usize;
                for _ in 0..max_steps {
                    if t >= t_end - 1e-14 {
                        break;
                    }
                    let h = dt.min(t_end - t);

                    // RK4 steps
                    let k1 = call_f(self, t, &y)?;
                    let y_tmp: Vec<f64> = (0..n).map(|i| y[i] + 0.5 * h * k1[i]).collect();
                    let k2 = call_f(self, t + 0.5 * h, &y_tmp)?;
                    let y_tmp: Vec<f64> = (0..n).map(|i| y[i] + 0.5 * h * k2[i]).collect();
                    let k3 = call_f(self, t + 0.5 * h, &y_tmp)?;
                    let y_tmp: Vec<f64> = (0..n).map(|i| y[i] + h * k3[i]).collect();
                    let k4 = call_f(self, t + h, &y_tmp)?;

                    for i in 0..n {
                        y[i] += h / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
                    }
                    t += h;

                    t_out.push(Value::Float(t));
                    for i in 0..n {
                        y_out[i].push(Value::Float(y[i]));
                    }
                }

                let mut result = std::collections::HashMap::new();
                result.insert("t".to_string(), Value::List(t_out));
                if n == 1 {
                    result.insert("y".to_string(), Value::List(y_out.into_iter().next().unwrap()));
                } else {
                    result.insert(
                        "y".to_string(),
                        Value::List(y_out.into_iter().map(Value::List).collect()),
                    );
                }
                Ok(Value::Record(result))
            }
            "any" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "any() takes exactly 2 arguments (list, fn)",
                        Some(span),
                    ));
                }
                let items = match &args[0] {
                    Value::List(l) => l.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("any() requires List, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let func = args[1].clone();
                for item in items {
                    let result = self.call_value(&func, vec![item], span)?;
                    if result.is_truthy() {
                        return Ok(Value::Bool(true));
                    }
                }
                Ok(Value::Bool(false))
            }
            "all" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "all() takes exactly 2 arguments (list, fn)",
                        Some(span),
                    ));
                }
                let items = match &args[0] {
                    Value::List(l) => l.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("all() requires List, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let func = args[1].clone();
                for item in items {
                    let result = self.call_value(&func, vec![item], span)?;
                    if !result.is_truthy() {
                        return Ok(Value::Bool(false));
                    }
                }
                Ok(Value::Bool(true))
            }
            "none" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "none() takes exactly 2 arguments (list, fn)",
                        Some(span),
                    ));
                }
                let items = match &args[0] {
                    Value::List(l) => l.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("none() requires List, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let func = args[1].clone();
                for item in items {
                    let result = self.call_value(&func, vec![item], span)?;
                    if result.is_truthy() {
                        return Ok(Value::Bool(false));
                    }
                }
                Ok(Value::Bool(true))
            }
            "take_while" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "take_while() takes exactly 2 arguments (list, fn)",
                        Some(span),
                    ));
                }
                let items = match &args[0] {
                    Value::List(l) => l.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("take_while() requires List, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let func = args[1].clone();
                let mut result = Vec::new();
                for item in items {
                    let test = self.call_value(&func, vec![item.clone()], span)?;
                    if !test.is_truthy() {
                        break;
                    }
                    result.push(item);
                }
                Ok(Value::List(result))
            }
            "find" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "find() takes exactly 2 arguments (list, fn)",
                        Some(span),
                    ));
                }
                let items = match &args[0] {
                    Value::List(l) => l.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("find() requires List, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let func = args[1].clone();
                for item in items {
                    let result = self.call_value(&func, vec![item.clone()], span)?;
                    if result.is_truthy() {
                        return Ok(item);
                    }
                }
                Ok(Value::Nil)
            }
            "find_index" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "find_index() takes exactly 2 arguments (list, fn)",
                        Some(span),
                    ));
                }
                let items = match &args[0] {
                    Value::List(l) => l.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("find_index() requires List, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let func = args[1].clone();
                for (i, item) in items.into_iter().enumerate() {
                    let result = self.call_value(&func, vec![item], span)?;
                    if result.is_truthy() {
                        return Ok(Value::Int(i as i64));
                    }
                }
                Ok(Value::Int(-1))
            }
            "try_call" => {
                if args.len() != 1 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "try_call() takes exactly 1 argument (fn)",
                        Some(span),
                    ));
                }
                let func = args[0].clone();
                match self.call_value(&func, vec![], span) {
                    Ok(val) => {
                        let mut rec = std::collections::HashMap::new();
                        rec.insert("ok".to_string(), Value::Bool(true));
                        rec.insert("value".to_string(), val);
                        rec.insert("error".to_string(), Value::Nil);
                        Ok(Value::Record(rec))
                    }
                    Err(e) => {
                        let mut rec = std::collections::HashMap::new();
                        rec.insert("ok".to_string(), Value::Bool(false));
                        rec.insert("value".to_string(), Value::Nil);
                        rec.insert("error".to_string(), Value::Str(format!("{e}")));
                        Ok(Value::Record(rec))
                    }
                }
            }
            "par_map" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "par_map() takes exactly 2 arguments (list, fn)",
                        Some(span),
                    ));
                }
                let items = match &args[0] {
                    Value::List(l) => l.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("par_map() requires List, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let func = args[1].clone();
                let num_threads = std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(4);
                let chunk_size = (items.len() / num_threads).max(1);
                let chunks: Vec<Vec<Value>> = items.chunks(chunk_size).map(|c| c.to_vec()).collect();
                let env_snapshot = self.env.clone();

                let chunk_results: Vec<std::result::Result<Vec<Value>, BioLangError>> =
                    std::thread::scope(|s| {
                        let handles: Vec<_> = chunks
                            .into_iter()
                            .map(|chunk| {
                                let func = func.clone();
                                let env = env_snapshot.clone();
                                s.spawn(move || {
                                    let mut interp = Interpreter::with_env(env);
                                    let mut results = Vec::with_capacity(chunk.len());
                                    for item in chunk {
                                        results.push(interp.call_value(&func, vec![item], span)?);
                                    }
                                    Ok(results)
                                })
                            })
                            .collect();
                        handles.into_iter().map(|h| h.join().unwrap()).collect()
                    });

                let mut results = Vec::with_capacity(items.len());
                for chunk_result in chunk_results {
                    results.extend(chunk_result?);
                }
                Ok(Value::List(results))
            }
            "par_filter" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "par_filter() takes exactly 2 arguments (list, fn)",
                        Some(span),
                    ));
                }
                let items = match &args[0] {
                    Value::List(l) => l.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("par_filter() requires List, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let func = args[1].clone();
                let num_threads = std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(4);
                let chunk_size = (items.len() / num_threads).max(1);
                let chunks: Vec<Vec<Value>> = items.chunks(chunk_size).map(|c| c.to_vec()).collect();
                let env_snapshot = self.env.clone();

                let chunk_results: Vec<std::result::Result<Vec<Value>, BioLangError>> =
                    std::thread::scope(|s| {
                        let handles: Vec<_> = chunks
                            .into_iter()
                            .map(|chunk| {
                                let func = func.clone();
                                let env = env_snapshot.clone();
                                s.spawn(move || {
                                    let mut interp = Interpreter::with_env(env);
                                    let mut kept = Vec::new();
                                    for item in chunk {
                                        let keep = interp.call_value(&func, vec![item.clone()], span)?;
                                        if keep.is_truthy() {
                                            kept.push(item);
                                        }
                                    }
                                    Ok(kept)
                                })
                            })
                            .collect();
                        handles.into_iter().map(|h| h.join().unwrap()).collect()
                    });

                let mut results = Vec::new();
                for chunk_result in chunk_results {
                    results.extend(chunk_result?);
                }
                Ok(Value::List(results))
            }
            // await_all: resolve a list of futures concurrently using threads
            "await_all" => {
                if args.len() != 1 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "await_all() takes exactly 1 argument (list of futures)",
                        Some(span),
                    ));
                }
                let futures = match &args[0] {
                    Value::List(l) => l.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("await_all() requires List, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let env_snapshot = self.env.clone();

                let thread_results: Vec<std::result::Result<Value, BioLangError>> =
                    std::thread::scope(|s| {
                        let handles: Vec<_> = futures
                            .into_iter()
                            .map(|val| {
                                let env = env_snapshot.clone();
                                s.spawn(move || {
                                    match val {
                                        Value::Future(state) => {
                                            let guard = state.lock().unwrap();
                                            match &*guard {
                                                bl_core::value::FutureState::Resolved(v) => Ok(v.clone()),
                                                bl_core::value::FutureState::Pending { params, body, closure_env, args } => {
                                                    let params = params.clone();
                                                    let body = body.clone();
                                                    let closure_env = *closure_env;
                                                    let args = args.clone();
                                                    drop(guard);
                                                    let mut interp = Interpreter::with_env(env);
                                                    let result = interp.call_function(
                                                        &params, &body, &closure_env, args, Vec::new(), span,
                                                    )?;
                                                    let mut guard = state.lock().unwrap();
                                                    *guard = bl_core::value::FutureState::Resolved(result.clone());
                                                    Ok(result)
                                                }
                                            }
                                        }
                                        other => Ok(other),
                                    }
                                })
                            })
                            .collect();
                        handles.into_iter().map(|h| h.join().unwrap()).collect()
                    });

                let mut results = Vec::with_capacity(thread_results.len());
                for r in thread_results {
                    results.push(r?);
                }
                Ok(Value::List(results))
            }
            // GAP 3: stream_batch — process stream in batches with a function
            "stream_batch" => {
                if args.len() != 3 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "stream_batch() takes exactly 3 arguments (stream, n, fn)",
                        Some(span),
                    ));
                }
                let stream = match &args[0] {
                    Value::Stream(s) => s.clone(),
                    Value::List(items) => bl_core::value::StreamValue::from_list("list", items.clone()),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("stream_batch() requires Stream or List, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let n = match &args[1] {
                    Value::Int(n) => *n as usize,
                    other => {
                        return Err(BioLangError::type_error(
                            format!("stream_batch() n must be Int, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let func = args[2].clone();
                let mut results = Vec::new();
                loop {
                    let mut batch = Vec::with_capacity(n);
                    for _ in 0..n {
                        match stream.next() {
                            Some(v) => batch.push(v),
                            None => break,
                        }
                    }
                    if batch.is_empty() {
                        break;
                    }
                    let batch_val = Value::List(batch);
                    let result = self.call_value(&func, vec![batch_val], span)?;
                    results.push(result);
                }
                Ok(Value::List(results))
            }
            // GAP 4: scatter_by — group items by key function
            "scatter_by" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "scatter_by() takes exactly 2 arguments (list, key_fn)",
                        Some(span),
                    ));
                }
                let items = match &args[0] {
                    Value::List(l) => l.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("scatter_by() requires List, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let key_fn = args[1].clone();
                let mut groups: std::collections::HashMap<String, Vec<Value>> = std::collections::HashMap::new();
                for item in items {
                    let key = self.call_value(&key_fn, vec![item.clone()], span)?;
                    let key_str = format!("{key}");
                    groups.entry(key_str).or_default().push(item);
                }
                let map: std::collections::HashMap<String, Value> = groups
                    .into_iter()
                    .map(|(k, v)| (k, Value::List(v)))
                    .collect();
                Ok(Value::Map(map))
            }
            // GAP 4: bench — benchmark a function
            "bench" => {
                if args.len() != 3 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "bench() takes exactly 3 arguments (fn, args, iterations)",
                        Some(span),
                    ));
                }
                let func = args[0].clone();
                let call_args = match &args[1] {
                    Value::List(l) => l.clone(),
                    _ => vec![args[1].clone()],
                };
                let iterations = match &args[2] {
                    Value::Int(n) => *n as usize,
                    other => {
                        return Err(BioLangError::type_error(
                            format!("bench() iterations must be Int, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };

                let mut times = Vec::with_capacity(iterations);
                for _ in 0..iterations {
                    let start = std::time::Instant::now();
                    let _ = self.call_value(&func, call_args.clone(), span)?;
                    let elapsed = start.elapsed().as_nanos() as i64;
                    times.push(elapsed);
                }

                let mean = times.iter().sum::<i64>() / times.len().max(1) as i64;
                let min_ns = *times.iter().min().unwrap_or(&0);
                let max_ns = *times.iter().max().unwrap_or(&0);

                let mut rec = std::collections::HashMap::new();
                rec.insert("mean_ns".to_string(), Value::Int(mean));
                rec.insert("min_ns".to_string(), Value::Int(min_ns));
                rec.insert("max_ns".to_string(), Value::Int(max_ns));
                rec.insert("iterations".to_string(), Value::Int(iterations as i64));
                Ok(Value::Record(rec))
            }
            "tap" | "inspect" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        format!("{name}() takes exactly 2 arguments (value, fn)"),
                        Some(span),
                    ));
                }
                let value = args[0].clone();
                let func = args[1].clone();
                self.call_value(&func, vec![value.clone()], span)?;
                return Ok(value);
            }
            "group_apply" => {
                if args.len() != 3 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "group_apply() takes exactly 3 arguments (table, key_col, fn)",
                        Some(span),
                    ));
                }
                let table = match &args[0] {
                    Value::Table(t) => t.clone(),
                    Value::List(items) => {
                        // Group a list of records by a key
                        let key_col = match &args[1] {
                            Value::Str(s) => s.clone(),
                            _ => return Err(BioLangError::type_error(
                                "group_apply() key must be a string", Some(span),
                            )),
                        };
                        let func = args[2].clone();
                        let mut groups: HashMap<String, Vec<Value>> = HashMap::new();
                        for item in items {
                            let key = match item {
                                Value::Record(ref m) | Value::Map(ref m) => {
                                    match m.get(&key_col) {
                                        Some(v) => format!("{v}"),
                                        None => "nil".to_string(),
                                    }
                                }
                                _ => format!("{item}"),
                            };
                            groups.entry(key).or_default().push(item.clone());
                        }
                        let mut results = Vec::new();
                        for (key, group) in &groups {
                            let result = self.call_value(&func, vec![
                                Value::Str(key.clone()),
                                Value::List(group.clone()),
                            ], span)?;
                            results.push(result);
                        }
                        return Ok(Value::List(results));
                    }
                    _ => return Err(BioLangError::type_error(
                        format!("group_apply() requires Table or List, got {}", args[0].type_of()),
                        Some(span),
                    )),
                };
                let key_col = match &args[1] {
                    Value::Str(s) => s.clone(),
                    _ => return Err(BioLangError::type_error(
                        "group_apply() key must be a string", Some(span),
                    )),
                };
                let func = args[2].clone();
                let col_idx = table.col_index(&key_col).ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!("column '{key_col}' not found in table"),
                        Some(span),
                    )
                })?;
                // Group rows by key column value
                let mut groups: HashMap<String, Vec<HashMap<String, Value>>> = HashMap::new();
                for i in 0..table.num_rows() {
                    let key = format!("{}", table.rows[i][col_idx]);
                    groups.entry(key).or_default().push(table.row_to_record(i));
                }
                let mut results = Vec::new();
                for (key, group_rows) in &groups {
                    let group_list: Vec<Value> = group_rows.iter().map(|r| Value::Record(r.clone())).collect();
                    let result = self.call_value(&func, vec![
                        Value::Str(key.clone()),
                        Value::List(group_list),
                    ], span)?;
                    results.push(result);
                }
                return Ok(Value::List(results));
            }
            "prop_test" => {
                if args.len() != 3 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "prop_test() takes exactly 3 arguments (property_fn, generator_fn, iterations)",
                        Some(span),
                    ));
                }
                let prop_fn = args[0].clone();
                let gen_fn = args[1].clone();
                let iterations = match &args[2] {
                    Value::Int(n) => *n,
                    _ => {
                        return Err(BioLangError::type_error(
                            "prop_test() iterations must be Int",
                            Some(span),
                        ))
                    }
                };
                for i in 0..iterations {
                    let input = self.call_value(&gen_fn, vec![Value::Int(i)], span)?;
                    let result = self.call_value(&prop_fn, vec![input.clone()], span)?;
                    if !result.is_truthy() {
                        let mut rec = HashMap::new();
                        rec.insert("passed".to_string(), Value::Bool(false));
                        rec.insert("iteration".to_string(), Value::Int(i));
                        rec.insert("failing_input".to_string(), input);
                        return Ok(Value::Record(rec));
                    }
                }
                let mut rec = HashMap::new();
                rec.insert("passed".to_string(), Value::Bool(true));
                rec.insert("iterations".to_string(), Value::Int(iterations));
                Ok(Value::Record(rec))
            }
            "partition" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "partition() takes exactly 2 arguments (list, fn)",
                        Some(span),
                    ));
                }
                let items = match &args[0] {
                    Value::List(l) => l.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("partition() requires List, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                let func = args[1].clone();
                let mut matching = Vec::new();
                let mut rest = Vec::new();
                for item in items {
                    let result = self.call_value(&func, vec![item.clone()], span)?;
                    if result.is_truthy() {
                        matching.push(item);
                    } else {
                        rest.push(item);
                    }
                }
                Ok(Value::List(vec![Value::List(matching), Value::List(rest)]))
            }
            "sort_by" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "sort_by() takes exactly 2 arguments (collection, fn)",
                        Some(span),
                    ));
                }
                // If first arg is a Stream, collect it into a List with a warning
                if let Value::Stream(ref stream) = args[0] {
                    eprintln!("\x1b[33mWarning:\x1b[0m sort_by() must collect the entire stream into memory to sort.");
                    eprintln!("  Tip: kmer_count() already returns results sorted by count (descending).");
                    eprintln!("  Use head(n) to get the top N entries without sorting: |> head(20)");
                    let mut items = Vec::new();
                    loop {
                        match stream.next() {
                            Some(v) => {
                                if items.len() % 100_000 == 0 && items.len() > 0 {
                                    eprint!("\r\x1b[2K  Collected {} items...", items.len());
                                }
                                items.push(v);
                            }
                            None => break,
                        }
                    }
                    if items.len() >= 100_000 {
                        eprintln!("\r\x1b[2K  Collected {} items — sorting...", items.len());
                    }
                    args[0] = Value::List(items);
                }
                let func = args[1].clone();
                // Detect if it's a 2-arg comparator or 1-arg key function
                let is_comparator = match &func {
                    Value::Function { params, .. } => params.len() >= 2,
                    _ => false,
                };

                if is_comparator {
                    // 2-arg comparator mode: sort_by(coll, |a, b| a.x - b.x)
                    let coll_to_items = |coll: &Value| -> Result<(Vec<Value>, bool, Vec<String>)> {
                        match coll {
                            Value::List(l) => Ok((l.clone(), false, vec![])),
                            Value::Table(tbl) => {
                                let cols = tbl.columns.clone();
                                let items: Vec<Value> = tbl.rows.iter().map(|row| {
                                    let mut rec = std::collections::HashMap::new();
                                    for (j, col) in cols.iter().enumerate() {
                                        if j < row.len() {
                                            rec.insert(col.clone(), row[j].clone());
                                        }
                                    }
                                    Value::Record(rec)
                                }).collect();
                                Ok((items, true, cols))
                            }
                            other => Err(BioLangError::type_error(
                                format!("sort_by() requires List or Table, got {}", other.type_of()),
                                Some(span),
                            )),
                        }
                    };
                    let (mut items, is_table, cols) = coll_to_items(&args[0])?;
                    let mut err = None;
                    items.sort_by(|a, b| {
                        if err.is_some() {
                            return std::cmp::Ordering::Equal;
                        }
                        match self.call_value(&func, vec![a.clone(), b.clone()], span) {
                            Ok(Value::Int(n)) if n < 0 => std::cmp::Ordering::Less,
                            Ok(Value::Int(n)) if n > 0 => std::cmp::Ordering::Greater,
                            Ok(Value::Int(_)) => std::cmp::Ordering::Equal,
                            Ok(_) => {
                                err = Some(BioLangError::type_error(
                                    "sort_by compare function must return Int",
                                    Some(span),
                                ));
                                std::cmp::Ordering::Equal
                            }
                            Err(e) => { err = Some(e); std::cmp::Ordering::Equal }
                        }
                    });
                    if let Some(e) = err { return Err(e); }
                    if is_table {
                        // Convert records back to table rows
                        let mut rows = Vec::with_capacity(items.len());
                        for item in &items {
                            if let Value::Record(rec) = item {
                                let row: Vec<Value> = cols.iter().map(|c| rec.get(c).cloned().unwrap_or(Value::Nil)).collect();
                                rows.push(row);
                            }
                        }
                        Ok(Value::Table(bl_core::value::Table::new(cols, rows)))
                    } else {
                        Ok(Value::List(items))
                    }
                } else {
                    // 1-arg key function mode: sort_by(coll, |r| r.score)
                    let cmp_keys = |ka: &Value, kb: &Value| -> std::cmp::Ordering {
                        match (ka, kb) {
                            (Value::Int(a), Value::Int(b)) => a.cmp(b),
                            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
                            (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
                            (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)).unwrap_or(std::cmp::Ordering::Equal),
                            (Value::Str(a), Value::Str(b)) => a.cmp(b),
                            _ => std::cmp::Ordering::Equal,
                        }
                    };
                    match &args[0] {
                        Value::List(l) => {
                            let items = l.clone();
                            let mut keyed: Vec<(Value, Value)> = Vec::with_capacity(items.len());
                            for item in items {
                                let key = self.call_value(&func, vec![item.clone()], span)?;
                                keyed.push((key, item));
                            }
                            keyed.sort_by(|(ka, _), (kb, _)| cmp_keys(ka, kb));
                            Ok(Value::List(keyed.into_iter().map(|(_, v)| v).collect()))
                        }
                        Value::Table(tbl) => {
                            let cols = &tbl.columns;
                            let mut keyed: Vec<(Value, usize)> = Vec::with_capacity(tbl.rows.len());
                            for (i, row) in tbl.rows.iter().enumerate() {
                                let mut rec = std::collections::HashMap::new();
                                for (j, col) in cols.iter().enumerate() {
                                    if j < row.len() {
                                        rec.insert(col.clone(), row[j].clone());
                                    }
                                }
                                let key = self.call_value(&func, vec![Value::Record(rec)], span)?;
                                keyed.push((key, i));
                            }
                            keyed.sort_by(|(ka, _), (kb, _)| cmp_keys(ka, kb));
                            let sorted_rows: Vec<Vec<Value>> = keyed.iter().map(|(_, i)| tbl.rows[*i].clone()).collect();
                            Ok(Value::Table(bl_core::value::Table::new(cols.clone(), sorted_rows)))
                        }
                        other => {
                            Err(BioLangError::type_error(
                                format!("sort_by() requires List or Table, got {}", other.type_of()),
                                Some(span),
                            ))
                        }
                    }
                }
            }
            "count_if" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "count_if() takes exactly 2 arguments (list, fn)",
                        Some(span),
                    ));
                }
                let func = args[1].clone();
                let mut count = 0i64;
                // Stream: count in place without materializing
                if let Value::Stream(s) = &args[0] {
                    while let Some(item) = s.next() {
                        let result = self.call_value(&func, vec![item], span)?;
                        if result.is_truthy() {
                            count += 1;
                        }
                    }
                    return Ok(Value::Int(count));
                }
                let items = match &args[0] {
                    Value::List(l) => l.clone(),
                    other => {
                        return Err(BioLangError::type_error(
                            format!("count_if() requires List or Stream, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                };
                for item in items {
                    let result = self.call_value(&func, vec![item], span)?;
                    if result.is_truthy() {
                        count += 1;
                    }
                }
                Ok(Value::Int(count))
            }
            _ => unreachable!(),
        }
    }

    #[inline(never)]
    fn call_function(
        &mut self,
        params: &[Param],
        body: &[Spanned<Stmt>],
        closure_env: &Option<usize>,
        positional: Vec<Value>,
        named: Vec<(String, Value)>,
        span: bl_core::span::Span,
    ) -> Result<Value> {
        // Check if this is an enum constructor call
        // Enum constructors have names like "EnumName::VariantName" and empty body
        let func_name_for_stack = "anonymous".to_string();

        // Stack trace: push frame
        let file = self.current_file.as_ref().map(|p| p.display().to_string());
        self.call_stack.push(StackFrame {
            function_name: func_name_for_stack.clone(),
            span: Some(span),
            file,
        });

        // Profiling: record start time
        let profile_start = if self.profiling.is_some() {
            Some(std::time::Instant::now())
        } else {
            None
        };

        // Create a new scope under the closure environment
        let prev = match closure_env {
            Some(env_id) => self.env.push_scope_under(*env_id),
            None => self.env.push_scope(),
        };

        // Bind parameters (with rest param support)
        let mut pos_idx = 0;
        for param in params {
            if param.rest {
                // Rest param: collect all remaining positional args into a List
                let rest_vals: Vec<Value> = positional[pos_idx..].to_vec();
                self.env.define(param.name.clone(), Value::List(rest_vals));
                pos_idx = positional.len();
            } else {
                let val = if let Some((_, v)) = named.iter().find(|(n, _)| n == &param.name) {
                    v.clone()
                } else if pos_idx < positional.len() {
                    let v = positional[pos_idx].clone();
                    pos_idx += 1;
                    v
                } else if let Some(default) = &param.default {
                    self.eval_expr(default)?
                } else {
                    self.env.pop_scope(prev);
                    self.call_stack.pop();
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        format!("missing argument '{}'", param.name),
                        Some(span),
                    ));
                };
                self.env.define(param.name.clone(), val);
            }
        }

        // Execute body
        let result = self.exec_block(body);
        self.env.pop_scope(prev);
        self.call_stack.pop();

        // Profiling: record elapsed
        if let (Some(start), Some(ref mut prof)) = (profile_start, &mut self.profiling) {
            let elapsed = start.elapsed().as_nanos();
            let entry = prof.entry(func_name_for_stack).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += elapsed;
        }

        match result {
            Ok(val) => Ok(val),
            Err(e) if e.kind == ErrorKind::Return => {
                Ok(e.return_value.map(|v| *v).unwrap_or(Value::Nil))
            }
            Err(mut e) => {
                // Attach stack trace to error
                if e.call_stack.is_empty() && !self.call_stack.is_empty() {
                    e.call_stack = self.call_stack.clone();
                }
                Err(e)
            }
        }
    }

    /// Handle higher-order function builtins (map, filter, reduce, sort).
    fn call_hof(
        &mut self,
        name: &str,
        args: &[Arg],
        span: bl_core::span::Span,
    ) -> Result<Value> {
        match name {
            "map" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "map() takes exactly 2 arguments (collection, fn)",
                        Some(span),
                    ));
                }
                let list = self.eval_expr(&args[0].value)?;
                let func = self.eval_expr(&args[1].value)?;
                if let Value::Table(t) = &list {
                    let mut result = Vec::new();
                    for i in 0..t.num_rows() {
                        let row_rec = Value::Record(t.row_to_record(i));
                        result.push(self.call_value(&func, vec![row_rec], span)?);
                    }
                    return Ok(Value::List(result));
                }
                let is_stream = matches!(&list, Value::Stream(_));
                let mut result = Vec::new();
                match list {
                    Value::List(l) => {
                        for item in l {
                            result.push(self.call_value(&func, vec![item], span)?);
                        }
                    }
                    Value::Stream(s) => {
                        while let Some(item) = s.next() {
                            result.push(self.call_value(&func, vec![item], span)?);
                        }
                    }
                    other => {
                        return Err(BioLangError::type_error(
                            format!("map() first argument must be List, Stream, or Table, got {}", other.type_of()),
                            Some(span),
                        ))
                    }
                }
                if is_stream {
                    Ok(Value::Stream(bl_core::value::StreamValue::from_list("map", result)))
                } else {
                    Ok(Value::List(result))
                }
            }
            "filter" => {
                if args.len() != 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "filter() takes exactly 2 arguments (collection, fn)",
                        Some(span),
                    ));
                }
                let list = self.eval_expr(&args[0].value)?;
                let func = self.eval_expr(&args[1].value)?;
                if let Value::Table(t) = &list {
                    let columns = t.columns.clone();
                    let mut kept_rows = Vec::new();
                    for i in 0..t.num_rows() {
                        let row_rec = Value::Record(t.row_to_record(i));
                        let keep = self.call_value(&func, vec![row_rec], span)?;
                        if keep.is_truthy() {
                            kept_rows.push(t.rows[i].clone());
                        }
                    }
                    return Ok(Value::Table(bl_core::value::Table::new(columns, kept_rows)));
                }
                let is_stream = matches!(&list, Value::Stream(_));
                let mut result = Vec::new();
                match list {
                    Value::List(l) => {
                        for item in l {
                            let keep = self.call_value(&func, vec![item.clone()], span)?;
                            if keep.is_truthy() {
                                result.push(item);
                            }
                        }
                    }
                    Value::Stream(s) => {
                        while let Some(item) = s.next() {
                            let keep = self.call_value(&func, vec![item.clone()], span)?;
                            if keep.is_truthy() {
                                result.push(item);
                            }
                        }
                    }
                    other => {
                        return Err(BioLangError::type_error(
                            format!(
                                "filter() first argument must be List, Stream, or Table, got {}",
                                other.type_of()
                            ),
                            Some(span),
                        ))
                    }
                }
                if is_stream {
                    Ok(Value::Stream(bl_core::value::StreamValue::from_list("filter", result)))
                } else {
                    Ok(Value::List(result))
                }
            }
            "reduce" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "reduce() takes 2-3 arguments (collection, fn[, initial])",
                        Some(span),
                    ));
                }
                let list = self.eval_expr(&args[0].value)?;
                let func = self.eval_expr(&args[1].value)?;
                match list {
                    Value::List(items) => {
                        let (mut acc, start) = if args.len() == 3 {
                            (self.eval_expr(&args[2].value)?, 0)
                        } else {
                            if items.is_empty() {
                                return Err(BioLangError::runtime(
                                    ErrorKind::TypeError,
                                    "reduce() on empty list requires initial value",
                                    Some(span),
                                ));
                            }
                            (items[0].clone(), 1)
                        };
                        for item in &items[start..] {
                            acc = self.call_value(&func, vec![acc, item.clone()], span)?;
                        }
                        Ok(acc)
                    }
                    Value::Stream(s) => {
                        let mut acc = if args.len() == 3 {
                            self.eval_expr(&args[2].value)?
                        } else {
                            match s.next() {
                                Some(v) => v,
                                None => return Err(BioLangError::runtime(
                                    ErrorKind::TypeError,
                                    "reduce() on empty stream requires initial value",
                                    Some(span),
                                )),
                            }
                        };
                        while let Some(item) = s.next() {
                            acc = self.call_value(&func, vec![acc, item], span)?;
                        }
                        Ok(acc)
                    }
                    other => {
                        Err(BioLangError::type_error(
                            format!(
                                "reduce() first argument must be List or Stream, got {}",
                                other.type_of()
                            ),
                            Some(span),
                        ))
                    }
                }
            }
            "sort" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(BioLangError::runtime(
                        ErrorKind::ArityError,
                        "sort() takes 1-2 arguments (list[, compare_fn])",
                        Some(span),
                    ));
                }
                let list = self.eval_expr(&args[0].value)?;
                let mut items = match list {
                    Value::List(l) => l,
                    other => {
                        return Err(BioLangError::type_error(
                            format!(
                                "sort() first argument must be List, got {}",
                                other.type_of()
                            ),
                            Some(span),
                        ))
                    }
                };

                if args.len() == 2 {
                    let func = self.eval_expr(&args[1].value)?;
                    let mut err = None;
                    items.sort_by(|a, b| {
                        if err.is_some() {
                            return std::cmp::Ordering::Equal;
                        }
                        match self.call_value(&func, vec![a.clone(), b.clone()], span) {
                            Ok(Value::Int(n)) => {
                                if n < 0 {
                                    std::cmp::Ordering::Less
                                } else if n > 0 {
                                    std::cmp::Ordering::Greater
                                } else {
                                    std::cmp::Ordering::Equal
                                }
                            }
                            Ok(_) => {
                                err = Some(BioLangError::type_error(
                                    "sort compare function must return Int",
                                    Some(span),
                                ));
                                std::cmp::Ordering::Equal
                            }
                            Err(e) => {
                                err = Some(e);
                                std::cmp::Ordering::Equal
                            }
                        }
                    });
                    if let Some(e) = err {
                        return Err(e);
                    }
                } else {
                    // Default sort
                    let mut err = None;
                    items.sort_by(|a, b| {
                        if err.is_some() {
                            return std::cmp::Ordering::Equal;
                        }
                        match (a, b) {
                            (Value::Int(a), Value::Int(b)) => a.cmp(b),
                            (Value::Float(a), Value::Float(b)) => {
                                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                            }
                            (Value::Str(a), Value::Str(b)) => a.cmp(b),
                            _ => {
                                err = Some(BioLangError::type_error(
                                    "cannot compare values for sorting",
                                    Some(span),
                                ));
                                std::cmp::Ordering::Equal
                            }
                        }
                    });
                    if let Some(e) = err {
                        return Err(e);
                    }
                }
                Ok(Value::List(items))
            }
            "mutate" | "summarize" | "par_map" | "par_filter" | "prop_test" | "await_all"
            | "flat_map" | "scan"
            | "stream_batch" | "scatter_by" | "bench"
            | "none" | "take_while" | "each" | "tap" | "inspect" | "group_apply"
            | "partition" | "sort_by" | "count_if" => {
                // Evaluate args and delegate to call_hof_with_values
                let mut vals = Vec::new();
                for arg in args {
                    vals.push(self.eval_expr(&arg.value)?);
                }
                self.call_hof_with_values(name, vals, span)
            }
            _ => unreachable!(),
        }
    }

    fn eval_binary(
        &mut self,
        op: BinaryOp,
        lhs: &Value,
        rhs: &Value,
        span: bl_core::span::Span,
    ) -> Result<Value> {
        // Try built-in operations first
        let result = match op {
            BinaryOp::Add => match (lhs, rhs) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
                (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{a}{b}"))),
                (Value::List(a), Value::List(b)) => {
                    let mut result = a.clone();
                    result.extend(b.iter().cloned());
                    Ok(Value::List(result))
                }
                _ => {
                    let mut err = BioLangError::type_error(
                        format!("cannot add {} and {}", lhs.type_of(), rhs.type_of()),
                        Some(span),
                    );
                    // Suggest type conversion for common Str + number mismatches
                    match (lhs, rhs) {
                        (Value::Str(_), Value::Int(_) | Value::Float(_))
                        | (Value::Int(_) | Value::Float(_), Value::Str(_)) => {
                            err = err.with_suggestion(
                                "use int() or float() to convert strings to numbers, or str() to convert numbers to strings",
                            );
                        }
                        _ => {}
                    }
                    Err(err)
                }
            },
            BinaryOp::Sub => numeric_op(lhs, rhs, |a, b| a - b, |a, b| a - b, "-", span),
            BinaryOp::Mul => match (lhs, rhs) {
                // String repeat: "ATG" * 3
                (Value::Str(s), Value::Int(n)) | (Value::Int(n), Value::Str(s)) if *n >= 0 => {
                    let total = s.len().saturating_mul(*n as usize);
                    if total > 100_000_000 {
                        return Err(BioLangError::runtime(
                            ErrorKind::TypeError,
                            format!(
                                "string repeat would produce {} bytes (limit: 100 MB). \
                                 Reduce the repeat count.",
                                total
                            ),
                            Some(span),
                        ));
                    }
                    Ok(Value::Str(s.repeat(*n as usize)))
                }
                _ => numeric_op(lhs, rhs, |a, b| a * b, |a, b| a * b, "*", span),
            },
            BinaryOp::Pow => match (lhs, rhs) {
                (Value::Int(a), Value::Int(b)) if *b >= 0 => Ok(Value::Int(a.pow(*b as u32))),
                (Value::Int(a), Value::Int(b)) => Ok(Value::Float((*a as f64).powf(*b as f64))),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(*b))),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).powf(*b))),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.powf(*b as f64))),
                _ => Err(BioLangError::type_error(
                    format!("cannot exponentiate {} and {}", lhs.type_of(), rhs.type_of()),
                    Some(span),
                )),
            },
            BinaryOp::Div => {
                // Check for division by zero
                match rhs {
                    Value::Int(0) => {
                        return Err(BioLangError::runtime(
                            ErrorKind::DivisionByZero,
                            "division by zero",
                            Some(span),
                        ))
                    }
                    Value::Float(f) if *f == 0.0 => {
                        return Err(BioLangError::runtime(
                            ErrorKind::DivisionByZero,
                            "division by zero",
                            Some(span),
                        ))
                    }
                    _ => {}
                }
                numeric_op(lhs, rhs, |a, b| a / b, |a, b| a / b, "/", span)
            }
            BinaryOp::Mod => {
                if let Value::Int(0) = rhs {
                    return Err(BioLangError::runtime(
                        ErrorKind::DivisionByZero,
                        "modulo by zero",
                        Some(span),
                    ));
                }
                numeric_op(lhs, rhs, |a, b| a % b, |a, b| a % b, "%", span)
            }
            BinaryOp::Eq => Ok(Value::Bool(lhs == rhs)),
            BinaryOp::Neq => Ok(Value::Bool(lhs != rhs)),
            BinaryOp::Lt => compare_op(lhs, rhs, |o| o.is_lt(), span),
            BinaryOp::Gt => compare_op(lhs, rhs, |o| o.is_gt(), span),
            BinaryOp::Le => compare_op(lhs, rhs, |o| o.is_le(), span),
            BinaryOp::Ge => compare_op(lhs, rhs, |o| o.is_ge(), span),
            BinaryOp::BitAnd => match (lhs, rhs) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a & b)),
                _ => Err(BioLangError::type_error(
                    format!("cannot bitwise AND {} and {}", lhs.type_of(), rhs.type_of()),
                    Some(span),
                )),
            },
            BinaryOp::BitXor => match (lhs, rhs) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a ^ b)),
                _ => Err(BioLangError::type_error(
                    format!("cannot bitwise XOR {} and {}", lhs.type_of(), rhs.type_of()),
                    Some(span),
                )),
            },
            BinaryOp::Shl => match (lhs, rhs) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a << b)),
                _ => Err(BioLangError::type_error(
                    format!("cannot shift {} and {}", lhs.type_of(), rhs.type_of()),
                    Some(span),
                )),
            },
            BinaryOp::Shr => match (lhs, rhs) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a >> b)),
                _ => Err(BioLangError::type_error(
                    format!("cannot shift {} and {}", lhs.type_of(), rhs.type_of()),
                    Some(span),
                )),
            },
            BinaryOp::And | BinaryOp::Or => {
                unreachable!("short-circuit handled in eval_expr")
            }
        };

        // If built-in failed, try operator overloading via impl methods
        if result.is_err() {
            let op_name = match op {
                BinaryOp::Add => "add",
                BinaryOp::Sub => "sub",
                BinaryOp::Mul => "mul",
                BinaryOp::Div => "div",
                BinaryOp::Mod => "mod_",
                BinaryOp::Pow => "pow",
                BinaryOp::Eq => "eq",
                BinaryOp::Lt => "lt",
                BinaryOp::Gt => "gt",
                BinaryOp::BitAnd => "bit_and",
                BinaryOp::BitXor => "bit_xor",
                BinaryOp::Shl => "shl",
                BinaryOp::Shr => "shr",
                _ => return result,
            };
            let type_name = self.runtime_type_name(lhs);
            let impl_key = format!("__impl_{type_name}_{op_name}");
            if let Ok(func) = self.env.get(&impl_key, None).cloned() {
                if let Value::Function { params, body, closure_env, .. } = &func {
                    return self.call_function(params, body, closure_env, vec![lhs.clone(), rhs.clone()], vec![], span);
                }
            }
        }

        result
    }

    /// Get the runtime type name for UFCS trait dispatch.
    fn runtime_type_name(&self, val: &Value) -> String {
        match val {
            Value::Record(map) => {
                // Check for struct identity
                if let Some(Value::Str(name)) = map.get("__struct") {
                    return name.clone();
                }
                "Record".to_string()
            }
            other => format!("{}", other.type_of()),
        }
    }

    fn call_struct_constructor(
        &mut self,
        struct_name: &str,
        params: &[Param],
        args: &[Value],
        span: bl_core::span::Span,
    ) -> Result<Value> {
        let mut record = HashMap::new();
        record.insert("__struct".to_string(), Value::Str(struct_name.to_string()));
        let mut pos_idx = 0;
        for param in params {
            let val = if pos_idx < args.len() {
                let v = args[pos_idx].clone();
                pos_idx += 1;
                v
            } else if let Some(ref default) = param.default {
                self.eval_expr(default)?
            } else {
                return Err(BioLangError::runtime(
                    ErrorKind::ArityError,
                    format!("struct {struct_name}: missing field '{}'", param.name),
                    Some(span),
                ));
            };
            record.insert(param.name.clone(), val);
        }
        Ok(Value::Record(record))
    }

    fn pattern_matches(&mut self, pattern: &Pattern, value: &Value) -> Result<bool> {
        match pattern {
            Pattern::Wildcard => Ok(true),
            Pattern::Ident(_) => Ok(true), // always matches, binds the value
            Pattern::Literal(lit) => {
                let lit_val = self.eval_expr(lit)?;
                Ok(lit_val == *value)
            }
            Pattern::EnumVariant { variant, bindings } => {
                if let Value::EnumValue { variant: vname, fields, .. } = value {
                    Ok(vname == variant && fields.len() == bindings.len())
                } else {
                    Ok(false)
                }
            }
            Pattern::TypePattern { type_name, .. } => {
                let val_type = format!("{}", value.type_of());
                Ok(val_type == *type_name)
            }
            Pattern::Or(alternatives) => {
                for alt in alternatives {
                    if self.pattern_matches(&alt.node, value)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

fn numeric_op(
    lhs: &Value,
    rhs: &Value,
    int_op: fn(i64, i64) -> i64,
    float_op: fn(f64, f64) -> f64,
    op_name: &str,
    span: bl_core::span::Span,
) -> Result<Value> {
    match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(*a, *b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(float_op(*a, *b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(float_op(*a as f64, *b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(float_op(*a, *b as f64))),
        _ => Err(BioLangError::type_error(
            format!(
                "cannot apply '{op_name}' to {} and {}",
                lhs.type_of(),
                rhs.type_of()
            ),
            Some(span),
        )),
    }
}

fn compare_op(
    lhs: &Value,
    rhs: &Value,
    pred: fn(std::cmp::Ordering) -> bool,
    span: bl_core::span::Span,
) -> Result<Value> {
    // Nil compared to anything → false (useful for missing VCF fields, etc.)
    if matches!(lhs, Value::Nil) || matches!(rhs, Value::Nil) {
        return Ok(Value::Bool(false));
    }
    // Auto-unwrap single-element lists; for multi-element, compare first element.
    // This supports VCF INFO fields like AF=0.35,0.10 where v.info.AF is a List.
    let lhs = match lhs {
        Value::List(items) if !items.is_empty() => &items[0],
        other => other,
    };
    let rhs = match rhs {
        Value::List(items) if !items.is_empty() => &items[0],
        other => other,
    };
    let ordering = match (lhs, rhs) {
        (Value::Int(a), Value::Int(b)) => a.cmp(b),
        (Value::Float(a), Value::Float(b)) => a
            .partial_cmp(b)
            .unwrap_or(std::cmp::Ordering::Equal),
        (Value::Int(a), Value::Float(b)) => (*a as f64)
            .partial_cmp(b)
            .unwrap_or(std::cmp::Ordering::Equal),
        (Value::Float(a), Value::Int(b)) => a
            .partial_cmp(&(*b as f64))
            .unwrap_or(std::cmp::Ordering::Equal),
        (Value::Str(a), Value::Str(b)) => a.cmp(b),
        _ => {
            return Err(BioLangError::type_error(
                format!(
                    "cannot compare {} and {}",
                    lhs.type_of(),
                    rhs.type_of()
                ),
                Some(span),
            ))
        }
    };
    Ok(Value::Bool(pred(ordering)))
}

fn check_arity(
    name: &str,
    arity: &Arity,
    count: usize,
    span: bl_core::span::Span,
) -> Result<()> {
    let ok = match arity {
        Arity::Exact(n) => count == *n,
        Arity::AtLeast(n) => count >= *n,
        Arity::Range(min, max) => count >= *min && count <= *max,
    };
    if !ok {
        let expected = match arity {
            Arity::Exact(n) => format!("{n}"),
            Arity::AtLeast(n) => format!("at least {n}"),
            Arity::Range(min, max) => format!("{min}-{max}"),
        };
        Err(BioLangError::runtime(
            ErrorKind::ArityError,
            format!("{name}() expected {expected} arguments, got {count}"),
            Some(span),
        ))
    } else {
        Ok(())
    }
}

/// Produce a short human-readable label for a statement (used in verbose mode).
fn verbose_stmt_label(stmt: &Stmt) -> String {
    match stmt {
        Stmt::Let { name, .. } => format!("let {name}"),
        Stmt::Const { name, .. } => format!("const {name}"),
        Stmt::Assign { name, .. } => format!("{name} = ..."),
        Stmt::Expr(expr) => verbose_expr_label(&expr.node),
        Stmt::Fn { name, .. } => format!("fn {name}()"),
        Stmt::Return { .. } => "return".into(),
        Stmt::While { .. } => "while ...".into(),
        Stmt::For { pattern, .. } => format!("for {pattern:?} in ..."),
        Stmt::Import { path, .. } => format!("import \"{path}\""),
        Stmt::Break => "break".into(),
        Stmt::Continue => "continue".into(),
        _ => "...".into(),
    }
}

/// Resolve a slice index, handling negative indices (Python-style).
fn resolve_slice_index(idx: i64, len: i64) -> i64 {
    if idx < 0 {
        (len + idx).max(0)
    } else {
        idx.min(len)
    }
}

fn verbose_expr_label(expr: &Expr) -> String {
    match expr {
        Expr::Call { callee, .. } => {
            let fname = verbose_expr_label(&callee.node);
            format!("{fname}()")
        }
        Expr::Pipe { left, right } => {
            let l = verbose_expr_label(&left.node);
            let r = verbose_expr_label(&right.node);
            format!("{l} |> {r}")
        }
        Expr::PipeInto { value, name } => {
            let v = verbose_expr_label(&value.node);
            format!("{v} |> into {name}")
        }
        Expr::Ident(name) => name.clone(),
        Expr::Field { object, field, .. } => {
            format!("{}.{field}", verbose_expr_label(&object.node))
        }
        _ => "expr".into(),
    }
}

