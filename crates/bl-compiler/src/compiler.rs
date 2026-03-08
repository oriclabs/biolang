use std::collections::HashMap;

use bl_core::ast::*;
use bl_core::span::{Span, Spanned};
use bl_core::value::Value;

use crate::chunk::{Chunk, CompiledFunction, Constant, ParamInfo};
use crate::loop_ctx::LoopContext;
use crate::opcode::OpCode;
use crate::upvalue::UpvalueDescriptor;

/// A local variable in the current compilation scope.
#[derive(Debug, Clone)]
struct Local {
    name: String,
    depth: u32,
    is_captured: bool,
}

/// Compilation context — one per function being compiled.
struct CompilerContext {
    chunk: Chunk,
    locals: Vec<Local>,
    upvalues: Vec<UpvalueDescriptor>,
    scope_depth: u32,
    loop_stack: Vec<LoopContext>,
    /// Number of local slots allocated (may differ from locals.len() due to pops).
    slot_count: u16,
    /// Function metadata
    fn_name: Option<String>,
    fn_arity: u8,
    has_rest_param: bool,
    params: Vec<ParamInfo>,
    is_generator: bool,
    is_async: bool,
}

impl CompilerContext {
    fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            chunk: Chunk::new(name.clone()),
            locals: Vec::new(),
            upvalues: Vec::new(),
            scope_depth: 0,
            loop_stack: Vec::new(),
            slot_count: 0,
            fn_name: if name == "<script>" { None } else { Some(name) },
            fn_arity: 0,
            has_rest_param: false,
            params: Vec::new(),
            is_generator: false,
            is_async: false,
        }
    }
}

/// Compiles BioLang AST to bytecode.
pub struct Compiler {
    contexts: Vec<CompilerContext>,
    /// Map from builtin name to a numeric ID for CallNative.
    builtin_ids: HashMap<String, u16>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            contexts: vec![CompilerContext::new("<script>")],
            builtin_ids: HashMap::new(),
        }
    }

    /// Register known builtin names for CallNative optimization.
    pub fn register_builtins(&mut self, names: &[(String, u16)]) {
        for (name, id) in names {
            self.builtin_ids.insert(name.clone(), *id);
        }
    }

    // ── Context helpers ──

    fn ctx(&self) -> &CompilerContext {
        self.contexts.last().expect("no compiler context")
    }

    fn ctx_mut(&mut self) -> &mut CompilerContext {
        self.contexts.last_mut().expect("no compiler context")
    }

    fn chunk(&self) -> &Chunk {
        &self.ctx().chunk
    }

    fn emit(&mut self, op: OpCode) -> usize {
        self.ctx_mut().chunk.emit(op)
    }

    fn emit_span(&mut self, op: OpCode, span: Span) -> usize {
        self.ctx_mut().chunk.emit_span(op, span)
    }

    fn add_constant(&mut self, c: Constant) -> u16 {
        self.ctx_mut().chunk.add_constant(c)
    }

    fn add_name(&mut self, name: String) -> u16 {
        self.ctx_mut().chunk.add_name(name)
    }

    fn current_offset(&self) -> usize {
        self.chunk().code.len()
    }

    fn patch_jump(&mut self, idx: usize) {
        self.ctx_mut().chunk.patch_jump(idx);
    }

    // ── Scope management ──

    fn begin_scope(&mut self) {
        self.ctx_mut().scope_depth += 1;
    }

    fn end_scope(&mut self) {
        let depth = self.ctx().scope_depth;
        self.ctx_mut().scope_depth = depth - 1;

        // Pop locals in this scope, emitting CloseUpvalue for captured ones.
        while let Some(local) = self.ctx().locals.last() {
            if local.depth <= depth - 1 {
                break;
            }
            if local.is_captured {
                self.emit(OpCode::CloseUpvalue);
            } else {
                self.emit(OpCode::Pop);
            }
            self.ctx_mut().locals.pop();
        }
    }

    fn add_local(&mut self, name: String) -> u16 {
        let depth = self.ctx().scope_depth;
        let slot = self.ctx().locals.len() as u16;
        self.ctx_mut().locals.push(Local {
            name,
            depth,
            is_captured: false,
        });
        if slot >= self.ctx().slot_count {
            self.ctx_mut().slot_count = slot + 1;
        }
        slot
    }

    fn resolve_local(&self, name: &str) -> Option<u16> {
        for (i, local) in self.ctx().locals.iter().enumerate().rev() {
            if local.name == name {
                return Some(i as u16);
            }
        }
        None
    }

    fn resolve_upvalue(&mut self, name: &str) -> Option<u16> {
        if self.contexts.len() < 2 {
            return None;
        }

        let enclosing_idx = self.contexts.len() - 2;

        // Check enclosing locals
        for (i, local) in self.contexts[enclosing_idx].locals.iter().enumerate().rev() {
            if local.name == name {
                self.contexts[enclosing_idx].locals[i].is_captured = true;
                return Some(self.add_upvalue(UpvalueDescriptor {
                    is_local: true,
                    index: i as u16,
                }));
            }
        }

        // Check enclosing upvalues (recursive capture)
        if enclosing_idx > 0 {
            // Temporarily swap out to recurse
            let current = self.contexts.pop().unwrap();
            let result = self.resolve_upvalue(name);
            self.contexts.push(current);

            if let Some(upvalue_idx) = result {
                return Some(self.add_upvalue(UpvalueDescriptor {
                    is_local: false,
                    index: upvalue_idx,
                }));
            }
        }

        None
    }

    fn add_upvalue(&mut self, desc: UpvalueDescriptor) -> u16 {
        // Deduplicate
        for (i, existing) in self.ctx().upvalues.iter().enumerate() {
            if *existing == desc {
                return i as u16;
            }
        }
        let idx = self.ctx().upvalues.len() as u16;
        self.ctx_mut().upvalues.push(desc);
        idx
    }

    // ── Public API ──

    /// Compile a full program to a top-level chunk.
    pub fn compile_program(mut self, program: &Program) -> Result<CompiledFunction, CompileError> {
        for stmt in &program.stmts {
            self.compile_stmt(stmt)?;
        }
        self.emit(OpCode::Nil);
        self.emit(OpCode::Return);
        self.finish_function()
    }

    /// Compile a single function definition.
    pub fn compile_function_def(
        &mut self,
        name: &str,
        params: &[Param],
        body: &[Spanned<Stmt>],
        is_generator: bool,
        is_async: bool,
    ) -> Result<CompiledFunction, CompileError> {
        self.contexts.push(CompilerContext::new(name));
        self.begin_scope();

        let ctx = self.ctx_mut();
        ctx.fn_arity = params.iter().filter(|p| !p.rest).count() as u8;
        ctx.is_generator = is_generator;
        ctx.is_async = is_async;
        ctx.has_rest_param = params.iter().any(|p| p.rest);

        // Compile parameters
        for param in params {
            let slot = self.add_local(param.name.clone());
            let default_constant = if let Some(ref default_expr) = param.default {
                // Compile default as a constant value (simple literals only for now)
                let const_idx = self.compile_default_value(default_expr);
                Some(const_idx)
            } else {
                None
            };
            self.ctx_mut().params.push(ParamInfo {
                name: param.name.clone(),
                default_constant,
                is_rest: param.rest,
            });
            let _ = slot; // slot is used implicitly
        }

        // Compile body
        for stmt in body {
            self.compile_stmt(stmt)?;
        }

        // Implicit nil return
        self.emit(OpCode::Nil);
        self.emit(OpCode::Return);

        self.finish_function()
    }

    fn compile_default_value(&mut self, expr: &Spanned<Expr>) -> u16 {
        // For default params, store the AST fragment as a constant
        // The VM will evaluate it when needed
        match &expr.node {
            Expr::Nil => self.add_constant(Constant::Value(Value::Nil)),
            Expr::Bool(b) => self.add_constant(Constant::Value(Value::Bool(*b))),
            Expr::Int(n) => self.add_constant(Constant::Value(Value::Int(*n))),
            Expr::Float(f) => self.add_constant(Constant::Value(Value::Float(*f))),
            Expr::Str(s) => self.add_constant(Constant::Value(Value::Str(s.clone()))),
            _ => {
                // Complex default: store as AST fragment
                self.add_constant(Constant::AstFragment(Box::new(expr.clone())))
            }
        }
    }

    fn finish_function(&mut self) -> Result<CompiledFunction, CompileError> {
        let ctx = self.contexts.pop().expect("no context to finish");
        Ok(CompiledFunction {
            name: ctx.fn_name,
            arity: ctx.fn_arity,
            has_rest_param: ctx.has_rest_param,
            params: ctx.params,
            chunk: ctx.chunk,
            upvalue_count: ctx.upvalues.len() as u16,
            is_generator: ctx.is_generator,
            is_async: ctx.is_async,
        })
    }

    // ── Statement compilation ──

    fn compile_stmt(&mut self, stmt: &Spanned<Stmt>) -> Result<(), CompileError> {
        match &stmt.node {
            Stmt::Expr(expr) => {
                self.compile_expr(expr)?;
                self.emit(OpCode::Pop);
            }
            Stmt::Let { name, value, .. } => {
                self.compile_expr(value)?;
                if self.ctx().scope_depth > 0 {
                    self.add_local(name.clone());
                } else {
                    let idx = self.add_name(name.clone());
                    self.emit(OpCode::DefineGlobal(idx));
                }
            }
            Stmt::Assign { name, value } => {
                self.compile_expr(value)?;
                if let Some(slot) = self.resolve_local(name) {
                    self.emit(OpCode::SetLocal(slot));
                } else if let Some(slot) = self.resolve_upvalue(name) {
                    self.emit(OpCode::SetUpvalue(slot));
                } else {
                    let idx = self.add_name(name.clone());
                    self.emit(OpCode::SetGlobal(idx));
                }
            }
            Stmt::Fn {
                name,
                params,
                body,
                is_generator,
                is_async,
                decorators: _,
                ..
            } => {
                let func = self.compile_function_def(name, params, body, *is_generator, *is_async)?;
                let upvalue_descs: Vec<UpvalueDescriptor> = if !self.contexts.is_empty() {
                    // The upvalues were stored in the context that just got popped
                    // We need to get them from the compiled function's metadata
                    // Actually, the upvalues are stored in the context we just finished
                    // Let's grab them before finish
                    Vec::new() // upvalues handled inside compile_function_def
                } else {
                    Vec::new()
                };

                let const_idx = self.add_constant(Constant::Function(func));
                self.emit_span(OpCode::Closure(const_idx), stmt.span);

                // Emit upvalue descriptors (handled by VM during Closure creation)
                for desc in &upvalue_descs {
                    let _ = desc; // upvalues encoded in the CompiledFunction
                }

                if self.ctx().scope_depth > 0 {
                    self.add_local(name.clone());
                } else {
                    let name_idx = self.add_name(name.clone());
                    self.emit(OpCode::DefineGlobal(name_idx));
                }
            }
            Stmt::Return(value) => {
                if let Some(expr) = value {
                    self.compile_expr(expr)?;
                } else {
                    self.emit(OpCode::Nil);
                }
                self.emit_span(OpCode::Return, stmt.span);
            }
            Stmt::For { pattern, iter, body, .. } => {
                self.compile_expr(iter)?;
                self.emit(OpCode::PushIter);

                let loop_start = self.current_offset();
                let exit_jump = self.emit(OpCode::IterNext(0)); // placeholder; pushes next value

                // Define loop variable as local (the IterNext pushed value IS the local)
                self.begin_scope();
                match pattern {
                    ForPattern::Single(var) => { self.add_local(var.clone()); }
                    ForPattern::ListDestr(names) => {
                        // For bytecode, bind first name as the whole item (simplified)
                        self.add_local(names[0].clone());
                    }
                    ForPattern::RecordDestr(names) => {
                        self.add_local(names[0].clone());
                    }
                    ForPattern::TupleDestr(names) => {
                        self.add_local(names[0].clone());
                    }
                }

                // Push loop context
                let scope_depth = self.ctx().scope_depth;
                self.ctx_mut()
                    .loop_stack
                    .push(LoopContext::new(loop_start, scope_depth));

                // Compile body
                for s in body {
                    self.compile_stmt(s)?;
                }

                // Pop the loop variable before looping back
                // (end_scope won't do it because we manually handle it)
                self.ctx_mut().locals.pop(); // remove local tracking
                self.ctx_mut().scope_depth -= 1;
                self.emit(OpCode::Pop); // pop the loop variable value

                // Loop back to IterNext
                let back = (self.current_offset() - loop_start) as u16;
                self.emit(OpCode::Loop(back));

                // Patch exit jump — when IterNext exhausts, jump here
                // No value was pushed, so no pop needed
                self.patch_jump(exit_jump);

                // Patch break jumps
                let loop_ctx = self.ctx_mut().loop_stack.pop().unwrap();
                for break_jump in loop_ctx.break_jumps {
                    self.patch_jump(break_jump);
                }

                self.emit(OpCode::PopIter);
            }
            Stmt::While { condition, body } => {
                let loop_start = self.current_offset();

                self.compile_expr(condition)?;
                let exit_jump = self.emit(OpCode::JumpIfFalse(0));

                self.begin_scope();
                let scope_depth = self.ctx().scope_depth;
                self.ctx_mut()
                    .loop_stack
                    .push(LoopContext::new(loop_start, scope_depth));

                for s in body {
                    self.compile_stmt(s)?;
                }

                let back = (self.current_offset() - loop_start) as u16;
                self.emit(OpCode::Loop(back));

                self.patch_jump(exit_jump);

                let loop_ctx = self.ctx_mut().loop_stack.pop().unwrap();
                for break_jump in loop_ctx.break_jumps {
                    self.patch_jump(break_jump);
                }
                self.end_scope();
            }
            Stmt::Break => {
                let jump = self.emit_span(OpCode::Jump(0), stmt.span);
                if let Some(loop_ctx) = self.ctx_mut().loop_stack.last_mut() {
                    loop_ctx.break_jumps.push(jump);
                }
            }
            Stmt::Continue => {
                if let Some(loop_ctx) = self.ctx().loop_stack.last() {
                    let loop_start = loop_ctx.loop_start;
                    let back = (self.current_offset() - loop_start) as u16;
                    self.emit_span(OpCode::Loop(back), stmt.span);
                }
            }
            Stmt::DestructLet { pattern, value } => {
                self.compile_expr(value)?;
                match pattern {
                    DestructPattern::List(names) => {
                        for (i, name) in names.iter().enumerate() {
                            self.emit(OpCode::Dup);
                            let idx = self.add_constant(Constant::Value(Value::Int(i as i64)));
                            self.emit(OpCode::Constant(idx));
                            self.emit(OpCode::GetIndex);
                            if self.ctx().scope_depth > 0 {
                                self.add_local(name.clone());
                            } else {
                                let name_idx = self.add_name(name.clone());
                                self.emit(OpCode::DefineGlobal(name_idx));
                            }
                        }
                        self.emit(OpCode::Pop); // pop the original value
                    }
                    DestructPattern::Record(names) => {
                        for name in names {
                            self.emit(OpCode::Dup);
                            let field_idx = self.add_name(name.clone());
                            self.emit(OpCode::GetField(field_idx));
                            if self.ctx().scope_depth > 0 {
                                self.add_local(name.clone());
                            } else {
                                let name_idx = self.add_name(name.clone());
                                self.emit(OpCode::DefineGlobal(name_idx));
                            }
                        }
                        self.emit(OpCode::Pop);
                    }
                    DestructPattern::ListWithRest { elements, rest_name } => {
                        for (i, name) in elements.iter().enumerate() {
                            self.emit(OpCode::Dup);
                            let idx = self.add_constant(Constant::Value(Value::Int(i as i64)));
                            self.emit(OpCode::Constant(idx));
                            self.emit(OpCode::GetIndex);
                            if self.ctx().scope_depth > 0 {
                                self.add_local(name.clone());
                            } else {
                                let name_idx = self.add_name(name.clone());
                                self.emit(OpCode::DefineGlobal(name_idx));
                            }
                        }
                        // rest not yet fully supported in compiler — bind as Null
                        let null_idx = self.add_constant(Constant::Value(Value::Nil));
                        self.emit(OpCode::Constant(null_idx));
                        if self.ctx().scope_depth > 0 {
                            self.add_local(rest_name.clone());
                        } else {
                            let name_idx = self.add_name(rest_name.clone());
                            self.emit(OpCode::DefineGlobal(name_idx));
                        }
                        self.emit(OpCode::Pop);
                    }
                    DestructPattern::RecordWithRest { fields, rest_name } => {
                        for name in fields {
                            self.emit(OpCode::Dup);
                            let field_idx = self.add_name(name.clone());
                            self.emit(OpCode::GetField(field_idx));
                            if self.ctx().scope_depth > 0 {
                                self.add_local(name.clone());
                            } else {
                                let name_idx = self.add_name(name.clone());
                                self.emit(OpCode::DefineGlobal(name_idx));
                            }
                        }
                        // rest not yet fully supported in compiler — bind as Null
                        let null_idx = self.add_constant(Constant::Value(Value::Nil));
                        self.emit(OpCode::Constant(null_idx));
                        if self.ctx().scope_depth > 0 {
                            self.add_local(rest_name.clone());
                        } else {
                            let name_idx = self.add_name(rest_name.clone());
                            self.emit(OpCode::DefineGlobal(name_idx));
                        }
                        self.emit(OpCode::Pop);
                    }
                }
            }
            Stmt::Assert { condition, message } => {
                self.compile_expr(condition)?;
                if let Some(msg_expr) = message {
                    self.compile_expr(msg_expr)?;
                } else {
                    self.emit(OpCode::Nil);
                }
                self.emit_span(OpCode::AssertCheck, stmt.span);
            }
            Stmt::Pipeline { body, .. } => {
                self.begin_scope();
                for s in body {
                    self.compile_stmt(s)?;
                }
                self.end_scope();
            }
            Stmt::Import { path, alias } => {
                let path_idx = self.add_name(path.clone());
                let has_alias = if alias.is_some() { 1 } else { 0 };
                if let Some(alias_name) = alias {
                    let _ = self.add_name(alias_name.clone());
                }
                self.emit_span(OpCode::Import(path_idx, has_alias), stmt.span);
            }
            Stmt::Yield(expr) => {
                // Yield compiles similarly to return in generator context
                self.compile_expr(expr)?;
                // The VM handles yield collection
                self.emit_span(OpCode::Return, stmt.span);
            }
            Stmt::Enum { name, variants } => {
                // Emit as a series of global definitions
                for variant in variants {
                    if variant.fields.is_empty() {
                        // Unit variant: define as a constant
                        let val = Value::EnumValue {
                            enum_name: name.clone(),
                            variant: variant.name.clone(),
                            fields: Vec::new(),
                        };
                        let const_idx = self.add_constant(Constant::Value(val));
                        self.emit(OpCode::Constant(const_idx));
                        let name_idx = self.add_name(variant.name.clone());
                        self.emit(OpCode::DefineGlobal(name_idx));
                    }
                    // Tuple variants need constructor functions - handled at runtime level
                }
            }
            Stmt::Const { name, value, .. } => {
                // Compile like let — immutability enforced at runtime
                self.compile_expr(value)?;
                if self.ctx().scope_depth > 0 {
                    self.add_local(name.clone());
                } else {
                    let name_idx = self.add_name(name.clone());
                    self.emit(OpCode::DefineGlobal(name_idx));
                }
            }
            Stmt::With { .. } => {
                // With blocks not yet supported in bytecode compiler
            }
            Stmt::Struct { .. } | Stmt::Trait { .. } | Stmt::Impl { .. } => {
                // These are metadata-only at compile time.
                // Runtime handles struct/trait/impl registration.
            }
            // New statement types — not yet supported in bytecode compiler
            Stmt::Unless { .. } | Stmt::Guard { .. } | Stmt::Defer(_)
            | Stmt::ParallelFor { .. } | Stmt::Stage { .. }
            | Stmt::FromImport { .. } | Stmt::NilAssign { .. }
            | Stmt::TypeAlias { .. } => {}
        }
        Ok(())
    }

    // ── Expression compilation ──

    fn compile_expr(&mut self, expr: &Spanned<Expr>) -> Result<(), CompileError> {
        match &expr.node {
            Expr::Nil => {
                self.emit(OpCode::Nil);
            }
            Expr::Bool(true) => {
                self.emit(OpCode::True);
            }
            Expr::Bool(false) => {
                self.emit(OpCode::False);
            }
            Expr::Int(n) => {
                let idx = self.add_constant(Constant::Value(Value::Int(*n)));
                self.emit(OpCode::Constant(idx));
            }
            Expr::Float(f) => {
                let idx = self.add_constant(Constant::Value(Value::Float(*f)));
                self.emit(OpCode::Constant(idx));
            }
            Expr::Str(s) => {
                let idx = self.add_constant(Constant::Value(Value::Str(s.clone())));
                self.emit(OpCode::Constant(idx));
            }
            Expr::DnaLit(s) => {
                let idx = self.add_name(s.clone());
                self.emit(OpCode::MakeDna(idx));
            }
            Expr::RnaLit(s) => {
                let idx = self.add_name(s.clone());
                self.emit(OpCode::MakeRna(idx));
            }
            Expr::ProteinLit(s) => {
                let idx = self.add_name(s.clone());
                self.emit(OpCode::MakeProtein(idx));
            }
            Expr::Ident(name) => {
                if let Some(slot) = self.resolve_local(name) {
                    self.emit_span(OpCode::GetLocal(slot), expr.span);
                } else if let Some(slot) = self.resolve_upvalue(name) {
                    self.emit_span(OpCode::GetUpvalue(slot), expr.span);
                } else {
                    let idx = self.add_name(name.clone());
                    self.emit_span(OpCode::GetGlobal(idx), expr.span);
                }
            }
            Expr::Unary { op, expr: inner } => {
                self.compile_expr(inner)?;
                match op {
                    UnaryOp::Neg => self.emit(OpCode::Negate),
                    UnaryOp::Not => self.emit(OpCode::Not),
                };
            }
            Expr::Binary { op, left, right } => {
                match op {
                    BinaryOp::And => {
                        self.compile_expr(left)?;
                        let jump = self.emit(OpCode::JumpIfFalse(0));
                        self.emit(OpCode::Pop);
                        self.compile_expr(right)?;
                        self.patch_jump(jump);
                        return Ok(());
                    }
                    BinaryOp::Or => {
                        self.compile_expr(left)?;
                        let jump = self.emit(OpCode::JumpIfTrue(0));
                        self.emit(OpCode::Pop);
                        self.compile_expr(right)?;
                        self.patch_jump(jump);
                        return Ok(());
                    }
                    _ => {}
                }
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                match op {
                    BinaryOp::Add => self.emit(OpCode::Add),
                    BinaryOp::Sub => self.emit(OpCode::Sub),
                    BinaryOp::Mul => self.emit(OpCode::Mul),
                    BinaryOp::Div => self.emit(OpCode::Div),
                    BinaryOp::Mod => self.emit(OpCode::Mod),
                    BinaryOp::Eq => self.emit(OpCode::Equal),
                    BinaryOp::Neq => self.emit(OpCode::NotEqual),
                    BinaryOp::Lt => self.emit(OpCode::Less),
                    BinaryOp::Gt => self.emit(OpCode::Greater),
                    BinaryOp::Le => self.emit(OpCode::LessEqual),
                    BinaryOp::Ge => self.emit(OpCode::GreaterEqual),
                    BinaryOp::Pow | BinaryOp::BitAnd | BinaryOp::BitXor
                    | BinaryOp::Shl | BinaryOp::Shr => {
                        // Stub: emit Nil for now (not yet supported in bytecode)
                        self.emit(OpCode::Nil)
                    }
                    BinaryOp::And | BinaryOp::Or => unreachable!(),
                };
            }
            Expr::Pipe { left, right } => {
                // `a |> f(b, c)` desugars to `f(a, b, c)`
                match &right.node {
                    Expr::Call { callee, args } => {
                        self.compile_expr(callee)?;
                        self.compile_expr(left)?; // first argument
                        for arg in args {
                            self.compile_expr(&arg.value)?;
                        }
                        let arg_count = (args.len() + 1) as u8;
                        self.emit_span(OpCode::Call(arg_count), expr.span);
                    }
                    Expr::Ident(_) => {
                        // `a |> f` → `f(a)`
                        self.compile_expr(right)?;
                        self.compile_expr(left)?;
                        self.emit_span(OpCode::Call(1), expr.span);
                    }
                    _ => {
                        // fallback: compile right as callee, left as arg
                        self.compile_expr(right)?;
                        self.compile_expr(left)?;
                        self.emit_span(OpCode::Call(1), expr.span);
                    }
                }
            }
            Expr::Call { callee, args } => {
                // Check for builtin call optimization
                if let Expr::Ident(name) = &callee.node {
                    if let Some(&builtin_id) = self.builtin_ids.get(name.as_str()) {
                        // Compile args
                        for arg in args {
                            self.compile_expr(&arg.value)?;
                        }
                        self.emit_span(
                            OpCode::CallNative(builtin_id, args.len() as u8),
                            expr.span,
                        );
                        return Ok(());
                    }
                }

                self.compile_expr(callee)?;
                for arg in args {
                    self.compile_expr(&arg.value)?;
                }
                self.emit_span(OpCode::Call(args.len() as u8), expr.span);
            }
            Expr::Field {
                object,
                field,
                optional,
            } => {
                self.compile_expr(object)?;
                let idx = self.add_name(field.clone());
                if *optional {
                    self.emit_span(OpCode::GetFieldOpt(idx), expr.span);
                } else {
                    self.emit_span(OpCode::GetField(idx), expr.span);
                }
            }
            Expr::Index { object, index } => {
                self.compile_expr(object)?;
                self.compile_expr(index)?;
                self.emit_span(OpCode::GetIndex, expr.span);
            }
            Expr::Lambda { params, body } => {
                // Compile lambda as an anonymous function
                let name = "<lambda>";
                self.contexts.push(CompilerContext::new(name));
                self.begin_scope();

                let ctx = self.ctx_mut();
                ctx.fn_arity = params.iter().filter(|p| !p.rest).count() as u8;
                ctx.has_rest_param = params.iter().any(|p| p.rest);

                for param in params {
                    let slot = self.add_local(param.name.clone());
                    self.ctx_mut().params.push(ParamInfo {
                        name: param.name.clone(),
                        default_constant: None,
                        is_rest: param.rest,
                    });
                    let _ = slot;
                }

                // Lambda body is a single expression
                self.compile_expr(body)?;
                self.emit(OpCode::Return);

                let func = self.finish_function()?;
                let const_idx = self.add_constant(Constant::Function(func));
                self.emit_span(OpCode::Closure(const_idx), expr.span);
            }
            Expr::Block(stmts) => {
                self.begin_scope();
                let mut last_was_expr = false;
                for (i, s) in stmts.iter().enumerate() {
                    if i == stmts.len() - 1 {
                        if let Stmt::Expr(e) = &s.node {
                            self.compile_expr(e)?;
                            last_was_expr = true;
                            continue;
                        }
                    }
                    self.compile_stmt(s)?;
                }
                if !last_was_expr {
                    self.emit(OpCode::Nil);
                }
                self.end_scope();
            }
            Expr::If {
                condition,
                then_body,
                else_body,
            } => {
                self.compile_expr(condition)?;
                let else_jump = self.emit(OpCode::JumpIfFalse(0));

                // Then branch
                self.begin_scope();
                let mut then_result = false;
                for (i, s) in then_body.iter().enumerate() {
                    if i == then_body.len() - 1 {
                        if let Stmt::Expr(e) = &s.node {
                            self.compile_expr(e)?;
                            then_result = true;
                            continue;
                        }
                    }
                    self.compile_stmt(s)?;
                }
                if !then_result {
                    self.emit(OpCode::Nil);
                }
                self.end_scope();

                let end_jump = self.emit(OpCode::Jump(0));
                self.patch_jump(else_jump);

                // Else branch
                if let Some(else_stmts) = else_body {
                    self.begin_scope();
                    let mut else_result = false;
                    for (i, s) in else_stmts.iter().enumerate() {
                        if i == else_stmts.len() - 1 {
                            if let Stmt::Expr(e) = &s.node {
                                self.compile_expr(e)?;
                                else_result = true;
                                continue;
                            }
                        }
                        self.compile_stmt(s)?;
                    }
                    if !else_result {
                        self.emit(OpCode::Nil);
                    }
                    self.end_scope();
                } else {
                    self.emit(OpCode::Nil);
                }

                self.patch_jump(end_jump);
            }
            Expr::List(elements) => {
                for elem in elements {
                    self.compile_expr(elem)?;
                }
                self.emit(OpCode::MakeList(elements.len() as u16));
            }
            Expr::Record(fields) => {
                for (key, value) in fields {
                    let idx = self.add_name(key.clone());
                    self.emit(OpCode::Constant(idx));
                    self.compile_expr(value)?;
                }
                self.emit(OpCode::MakeRecord(fields.len() as u16));
            }
            Expr::Formula(inner) => {
                let const_idx =
                    self.add_constant(Constant::AstFragment(Box::new(inner.as_ref().clone())));
                self.emit(OpCode::MakeFormula(const_idx));
            }
            Expr::Match { expr: match_expr, arms } => {
                self.compile_expr(match_expr)?;

                let mut end_jumps = Vec::new();

                for arm in arms {
                    // Duplicate the match value for each arm test
                    self.emit(OpCode::Dup);

                    match &arm.pattern.node {
                        Pattern::Wildcard => {
                            // Always matches — pop the dup'd value
                            self.emit(OpCode::Pop);
                        }
                        Pattern::Literal(lit_expr) => {
                            self.compile_expr(lit_expr)?;
                            self.emit(OpCode::Equal);
                            let skip = self.emit(OpCode::JumpIfFalse(0));
                            self.emit(OpCode::Pop); // pop match value

                            // Compile arm body
                            self.compile_expr(&arm.body)?;
                            end_jumps.push(self.emit(OpCode::Jump(0)));

                            self.patch_jump(skip);
                            continue;
                        }
                        Pattern::Ident(name) => {
                            // Bind the value to a local
                            self.begin_scope();
                            self.add_local(name.clone());
                            // The dup'd value is now the local
                        }
                        Pattern::EnumVariant { .. } => {
                            // Simplified: pop and continue
                            self.emit(OpCode::Pop);
                        }
                        Pattern::TypePattern { binding, .. } => {
                            // Type pattern: bind value if named
                            if let Some(name) = binding {
                                self.begin_scope();
                                self.add_local(name.clone());
                            } else {
                                self.emit(OpCode::Pop);
                            }
                        }
                        Pattern::Or(_alternatives) => {
                            // Or-pattern: simplified — pop and treat as wildcard for bytecode
                            self.emit(OpCode::Pop);
                        }
                    }

                    // Check guard
                    if let Some(guard) = &arm.guard {
                        self.compile_expr(guard)?;
                        let skip = self.emit(OpCode::JumpIfFalse(0));
                        self.compile_expr(&arm.body)?;
                        end_jumps.push(self.emit(OpCode::Jump(0)));
                        self.patch_jump(skip);
                        continue;
                    }

                    self.compile_expr(&arm.body)?;
                    end_jumps.push(self.emit(OpCode::Jump(0)));
                }

                // Default: push nil
                self.emit(OpCode::Pop); // pop match value
                self.emit(OpCode::Nil);

                for j in end_jumps {
                    self.patch_jump(j);
                }
            }
            Expr::TryCatch {
                body,
                error_var,
                catch_body,
            } => {
                let try_begin = self.emit(OpCode::TryBegin(0)); // placeholder jump to catch

                self.begin_scope();
                for s in body {
                    self.compile_stmt(s)?;
                }
                self.end_scope();
                self.emit(OpCode::TryEnd);
                self.emit(OpCode::Nil); // try completed successfully, push nil as result
                let end_jump = self.emit(OpCode::Jump(0)); // skip catch

                // Catch handler
                self.patch_jump(try_begin);

                self.begin_scope();
                if let Some(var_name) = error_var {
                    // Error value is on TOS (pushed by VM)
                    self.add_local(var_name.clone());
                }
                for s in catch_body {
                    self.compile_stmt(s)?;
                }
                self.end_scope();

                self.patch_jump(end_jump);
            }
            Expr::NullCoalesce { left, right } => {
                self.compile_expr(left)?;
                let jump = self.emit(OpCode::NullCoalesce(0));
                self.emit(OpCode::Pop); // pop nil
                self.compile_expr(right)?;
                self.patch_jump(jump);
            }
            Expr::StringInterp(parts) => {
                for part in parts {
                    match part {
                        StringPart::Lit(s) => {
                            let idx =
                                self.add_constant(Constant::Value(Value::Str(s.clone())));
                            self.emit(OpCode::Constant(idx));
                        }
                        StringPart::Expr(e) => {
                            self.compile_expr(e)?;
                        }
                    }
                }
                self.emit(OpCode::StringInterp(parts.len() as u16));
            }
            Expr::Range {
                start,
                end,
                inclusive,
            } => {
                self.compile_expr(start)?;
                self.compile_expr(end)?;
                self.emit(OpCode::MakeRange(if *inclusive { 1 } else { 0 }));
            }
            Expr::ListComp {
                expr: elem_expr,
                var,
                iter,
                condition,
            } => {
                // Compile as: make empty list, iterate, push elements
                self.emit(OpCode::MakeList(0)); // accumulator
                self.compile_expr(iter)?;
                self.emit(OpCode::PushIter);
                let loop_start = self.current_offset();
                let exit_jump = self.emit(OpCode::IterNext(0));

                self.begin_scope();
                self.add_local(var.clone());

                // Optional condition
                if let Some(cond) = condition {
                    self.compile_expr(cond)?;
                    let skip = self.emit(OpCode::JumpIfFalse(0));
                    self.emit(OpCode::Pop); // pop condition result

                    // element expression
                    // Get accumulator, compute element, push to list
                    self.compile_expr(elem_expr)?;

                    self.patch_jump(skip);
                    // If condition was false, don't add element
                } else {
                    self.compile_expr(elem_expr)?;
                }

                self.end_scope();

                let back = (self.current_offset() - loop_start) as u16;
                self.emit(OpCode::Loop(back));
                self.patch_jump(exit_jump);
                self.emit(OpCode::Pop); // pop iter variable
                self.emit(OpCode::PopIter);
                // List comp result is built via VM-level list accumulation
            }
            Expr::MapComp { .. } => {
                // Simplified: emit nil for now, full implementation in VM
                self.emit(OpCode::Nil);
            }
            Expr::Ternary {
                value,
                condition,
                else_value,
            } => {
                // `value if condition else else_value`
                self.compile_expr(condition)?;
                let else_jump = self.emit(OpCode::JumpIfFalse(0));
                self.compile_expr(value)?;
                let end_jump = self.emit(OpCode::Jump(0));
                self.patch_jump(else_jump);
                self.compile_expr(else_value)?;
                self.patch_jump(end_jump);
            }
            Expr::ChainedCmp { operands, ops } => {
                // `a < b < c` → `a < b && b < c`
                if operands.len() < 2 {
                    self.emit(OpCode::True);
                    return Ok(());
                }

                self.compile_expr(&operands[0])?;

                let mut end_jumps = Vec::new();

                for i in 0..ops.len() {
                    self.compile_expr(&operands[i + 1])?;
                    // Duplicate right operand for next comparison (except last)
                    if i < ops.len() - 1 {
                        self.emit(OpCode::Dup);
                    }
                    match ops[i] {
                        BinaryOp::Lt => self.emit(OpCode::Less),
                        BinaryOp::Gt => self.emit(OpCode::Greater),
                        BinaryOp::Le => self.emit(OpCode::LessEqual),
                        BinaryOp::Ge => self.emit(OpCode::GreaterEqual),
                        BinaryOp::Eq => self.emit(OpCode::Equal),
                        BinaryOp::Neq => self.emit(OpCode::NotEqual),
                        _ => self.emit(OpCode::Equal),
                    };
                    if i < ops.len() - 1 {
                        end_jumps.push(self.emit(OpCode::JumpIfFalse(0)));
                        self.emit(OpCode::Pop);
                    }
                }

                let end = self.current_offset();
                for j in end_jumps {
                    self.patch_jump(j);
                }
                let _ = end;
            }
            Expr::SetLiteral(elements) => {
                for elem in elements {
                    self.compile_expr(elem)?;
                }
                self.emit(OpCode::MakeSet(elements.len() as u16));
            }
            Expr::Regex { pattern, flags } => {
                let p_idx = self.add_constant(Constant::Value(Value::Str(pattern.clone())));
                let f_idx = self.add_constant(Constant::Value(Value::Str(flags.clone())));
                self.emit(OpCode::Constant(p_idx));
                self.emit(OpCode::Constant(f_idx));
                // VM constructs Regex from two strings on stack
                self.emit(OpCode::MakeList(2)); // temporary: VM decodes
            }
            Expr::Await(inner) => {
                self.compile_expr(inner)?;
                // Await is handled at runtime level
                // For bytecode, we just pass through
            }
            Expr::StructLit { name, fields } => {
                // Compile as Record with __type field
                let type_key_idx = self.add_name("__type".to_string());
                self.emit(OpCode::Constant(type_key_idx));
                let type_val_idx = self.add_name(name.clone());
                self.emit(OpCode::Constant(type_val_idx));
                for (key, val) in fields {
                    let idx = self.add_name(key.clone());
                    self.emit(OpCode::Constant(idx));
                    self.compile_expr(val)?;
                }
                self.emit(OpCode::MakeRecord((fields.len() + 1) as u16));
            }
            Expr::TupleLit(items) => {
                // Compile as List (tuples are runtime-only distinction)
                for item in items {
                    self.compile_expr(item)?;
                }
                self.emit(OpCode::MakeList(items.len() as u16));
            }
            Expr::QualLit(ascii) => {
                let scores: Vec<u8> = ascii.bytes().map(|b| b.wrapping_sub(33)).collect();
                let val = Value::Quality(scores);
                let idx = self.add_constant(Constant::Value(val));
                self.emit(OpCode::Constant(idx));
            }
            // New expression types — fall back to Nil in bytecode compiler (experimental)
            Expr::In { .. } | Expr::DoBlock { .. } | Expr::TypeCast { .. } | Expr::ThenPipe { .. } | Expr::Slice { .. }
            | Expr::TapPipe { .. } | Expr::Given { .. } | Expr::Retry { .. } | Expr::RecordSpread { .. } => {
                self.emit(OpCode::Nil);
            }
        }
        Ok(())
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Compilation error.
#[derive(Debug, Clone)]
pub struct CompileError {
    pub message: String,
    pub span: Option<Span>,
}

impl CompileError {
    pub fn new(message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CompileError: {}", self.message)
    }
}

impl std::error::Error for CompileError {}
