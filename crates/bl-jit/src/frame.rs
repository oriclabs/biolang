use std::sync::Arc;

use bl_compiler::CompiledFunction;
use bl_core::value::Value;

/// A closure object: compiled function + captured upvalues.
#[derive(Debug, Clone)]
pub struct ObjClosure {
    pub function: Arc<CompiledFunction>,
    pub upvalues: Vec<ObjUpvalue>,
}

/// An upvalue — either an open reference to a stack slot or a closed-over value.
#[derive(Debug, Clone)]
pub enum ObjUpvalue {
    /// Points to a stack slot that is still alive.
    Open { stack_index: usize },
    /// The value has been hoisted off the stack.
    Closed(Value),
}

/// A call frame representing a function invocation in the VM.
#[derive(Debug)]
pub struct CallFrame {
    pub closure: ObjClosure,
    /// Instruction pointer into the chunk's code array.
    pub ip: usize,
    /// Base index into the VM's value stack for this frame's locals.
    pub base: usize,
}

impl CallFrame {
    pub fn new(closure: ObjClosure, base: usize) -> Self {
        Self {
            closure,
            ip: 0,
            base,
        }
    }
}

/// Try/catch handler state.
#[derive(Debug, Clone)]
pub struct TryHandler {
    /// Frame index where the try block was entered.
    pub frame_index: usize,
    /// Instruction pointer to jump to on error (catch handler).
    pub catch_ip: usize,
    /// Stack depth at try entry (for unwinding).
    pub stack_depth: usize,
}
