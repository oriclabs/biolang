//! Bytecode compilation and VM execution bridge.
//!
//! This module connects `bl-compiler` and `bl-jit` to the tree-walking interpreter,
//! enabling `@compile`-decorated functions and whole-program bytecode execution.

use std::sync::Arc;

use bl_compiler::{compile_program, Compiler};
use bl_core::ast::Program;
use bl_core::error::Result;
use bl_core::value::{Arity, Value};
use bl_jit::{BuiltinCallback, BuiltinRegistry, ObjClosure, Vm};

use crate::builtins::{call_builtin, register_builtins};
use crate::env::Environment;

/// Adapter that bridges bl-runtime's builtins to bl-jit's BuiltinCallback trait.
struct RuntimeBuiltinCallback;

impl BuiltinCallback for RuntimeBuiltinCallback {
    fn call_builtin(&self, name: &str, args: Vec<Value>) -> Result<Value> {
        call_builtin(name, args)
    }

    fn builtin_list(&self) -> Vec<(String, Arity)> {
        // Collect all builtins by probing the environment
        let mut env = Environment::new();
        register_builtins(&mut env);
        let globals = env.list_global_vars();
        globals
            .into_iter()
            .filter_map(|(name, value)| {
                if let Value::NativeFunction { arity, .. } = value {
                    Some((name.to_string(), arity.clone()))
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Compile and run a whole program through the bytecode VM.
/// Returns the last expression value.
pub fn compile_and_run(program: &Program) -> Result<Value> {
    let callback = RuntimeBuiltinCallback;
    let registry = BuiltinRegistry::new(Box::new(callback));

    // Compile
    let func = compile_program(program).map_err(|e| {
        bl_core::error::BioLangError::runtime(
            bl_core::error::ErrorKind::TypeError,
            format!("compilation error: {}", e.message),
            e.span,
        )
    })?;

    // Execute
    let mut vm = Vm::new(registry);

    // Register builtin globals (NativeFunction values)
    let mut env = Environment::new();
    register_builtins(&mut env);
    for (name, value) in env.list_global_vars() {
        vm.define_global(name.to_string(), value.clone());
    }

    vm.execute(func)
}

/// Compile a single function to a CompiledClosure value.
/// Used by the `@compile` decorator.
pub fn compile_function_to_closure(
    name: &str,
    params: &[bl_core::ast::Param],
    body: &[bl_core::span::Spanned<bl_core::ast::Stmt>],
    is_generator: bool,
    is_async: bool,
) -> Result<Value> {
    let mut compiler = Compiler::new();

    // Register builtin IDs for CallNative optimization
    let callback = RuntimeBuiltinCallback;
    let registry = BuiltinRegistry::new(Box::new(callback));
    compiler.register_builtins(&registry.name_id_pairs());

    let func = compiler
        .compile_function_def(name, params, body, is_generator, is_async)
        .map_err(|e| {
            bl_core::error::BioLangError::runtime(
                bl_core::error::ErrorKind::TypeError,
                format!("compilation error: {}", e.message),
                e.span,
            )
        })?;

    let closure = ObjClosure {
        function: Arc::new(func),
        upvalues: Vec::new(),
    };

    Ok(Value::CompiledClosure(Arc::new(closure)))
}

/// Execute a CompiledClosure value with given arguments.
/// Used by the interpreter's `call_value` dispatch.
pub fn call_compiled_closure(closure_any: &Arc<dyn std::any::Any + Send + Sync>, args: Vec<Value>) -> Result<Value> {
    let closure = closure_any
        .downcast_ref::<ObjClosure>()
        .ok_or_else(|| {
            bl_core::error::BioLangError::type_error("invalid compiled closure", None)
        })?;

    let callback = RuntimeBuiltinCallback;
    let registry = BuiltinRegistry::new(Box::new(callback));
    let mut vm = Vm::new(registry);

    // Register builtin globals
    let mut env = Environment::new();
    register_builtins(&mut env);
    for (name, value) in env.list_global_vars() {
        vm.define_global(name.to_string(), value.clone());
    }

    vm.execute_closure(closure.clone(), args)
}
