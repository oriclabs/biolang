pub mod chunk;
pub mod compiler;
pub mod debug;
pub mod loop_ctx;
pub mod opcode;
pub mod upvalue;

pub use chunk::{Chunk, CompiledFunction, Constant, ParamInfo};
pub use compiler::{CompileError, Compiler};
pub use debug::disassemble_function;
pub use opcode::OpCode;
pub use upvalue::UpvalueDescriptor;

use bl_core::ast::Program;

/// Convenience: compile a program to a top-level function.
pub fn compile_program(program: &Program) -> Result<CompiledFunction, CompileError> {
    Compiler::new().compile_program(program)
}
