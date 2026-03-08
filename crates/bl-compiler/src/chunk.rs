use crate::opcode::OpCode;
use bl_core::ast::Expr;
use bl_core::span::{Span, Spanned};
use bl_core::value::Value;

/// A compiled chunk of bytecode with its constant pool.
#[derive(Debug, Clone)]
pub struct Chunk {
    /// The bytecode instructions.
    pub code: Vec<OpCode>,
    /// Constant pool.
    pub constants: Vec<Constant>,
    /// Sparse mapping: instruction index → source span.
    pub spans: Vec<(usize, Span)>,
    /// Name of this chunk (function name or "<script>").
    pub name: String,
}

impl Chunk {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            spans: Vec::new(),
            name: name.into(),
        }
    }

    /// Emit an opcode, returning its index.
    pub fn emit(&mut self, op: OpCode) -> usize {
        let idx = self.code.len();
        self.code.push(op);
        idx
    }

    /// Emit an opcode with an associated source span.
    pub fn emit_span(&mut self, op: OpCode, span: Span) -> usize {
        let idx = self.emit(op);
        self.spans.push((idx, span));
        idx
    }

    /// Add a constant, returning its index.
    pub fn add_constant(&mut self, constant: Constant) -> u16 {
        let idx = self.constants.len();
        self.constants.push(constant);
        idx as u16
    }

    /// Add a Value constant, returning its index.
    pub fn add_value(&mut self, value: Value) -> u16 {
        self.add_constant(Constant::Value(value))
    }

    /// Add a name constant, returning its index.
    pub fn add_name(&mut self, name: String) -> u16 {
        // Deduplicate name constants
        for (i, c) in self.constants.iter().enumerate() {
            if let Constant::Name(ref existing) = c {
                if existing == &name {
                    return i as u16;
                }
            }
        }
        self.add_constant(Constant::Name(name))
    }

    /// Patch a jump instruction at `idx` with the correct offset.
    pub fn patch_jump(&mut self, idx: usize) {
        let offset = (self.code.len() as i32 - idx as i32 - 1) as i16;
        match &mut self.code[idx] {
            OpCode::Jump(ref mut o)
            | OpCode::JumpIfFalse(ref mut o)
            | OpCode::JumpIfTrue(ref mut o)
            | OpCode::IterNext(ref mut o)
            | OpCode::NullCoalesce(ref mut o) => *o = offset,
            OpCode::TryBegin(ref mut o) => *o = offset as u16,
            _ => panic!("patch_jump on non-jump opcode at {idx}"),
        }
    }

    /// Get the source span for a given instruction index.
    pub fn span_at(&self, idx: usize) -> Option<Span> {
        // Find the latest span at or before idx
        let mut result = None;
        for &(span_idx, span) in &self.spans {
            if span_idx <= idx {
                result = Some(span);
            } else {
                break;
            }
        }
        result
    }
}

/// Constant pool entry.
#[derive(Debug, Clone)]
pub enum Constant {
    /// An immediate runtime value.
    Value(Value),
    /// An identifier name (for globals, fields, etc.).
    Name(String),
    /// A compiled nested function.
    Function(CompiledFunction),
    /// An AST fragment stored for Formula values.
    AstFragment(Box<Spanned<Expr>>),
}

/// A compiled function (stored in the constant pool).
#[derive(Debug, Clone)]
pub struct CompiledFunction {
    pub name: Option<String>,
    pub arity: u8,
    pub has_rest_param: bool,
    pub params: Vec<ParamInfo>,
    pub chunk: Chunk,
    pub upvalue_count: u16,
    pub is_generator: bool,
    pub is_async: bool,
}

/// Parameter metadata for a compiled function.
#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub name: String,
    /// Index into the chunk's constant pool for the default value expression,
    /// or None if no default.
    pub default_constant: Option<u16>,
    pub is_rest: bool,
}
