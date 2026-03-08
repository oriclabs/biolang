/// Stack-based bytecode instructions.
#[derive(Debug, Clone, PartialEq)]
pub enum OpCode {
    // ── Constants & literals ──
    /// Push constant from pool onto stack.
    Constant(u16),
    Nil,
    True,
    False,

    // ── Stack manipulation ──
    Pop,
    Dup,

    // ── Local variables ──
    GetLocal(u16),
    SetLocal(u16),

    // ── Global variables ──
    DefineGlobal(u16),
    GetGlobal(u16),
    SetGlobal(u16),

    // ── Upvalues (closures) ──
    GetUpvalue(u16),
    SetUpvalue(u16),

    // ── Arithmetic & logic ──
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Negate,
    Not,

    // ── Comparison ──
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,

    // ── Control flow ──
    /// Unconditional relative jump (signed offset from *after* this instruction).
    Jump(i16),
    /// Jump if TOS is falsy (pops TOS).
    JumpIfFalse(i16),
    /// Jump if TOS is truthy (does NOT pop — for short-circuit `||`).
    JumpIfTrue(i16),
    /// Backward jump for loop bodies (unsigned offset).
    Loop(u16),

    // ── Functions ──
    /// Call TOS function with `arg_count` arguments below it.
    Call(u8),
    /// Fast path for native builtins: builtin_id + arg_count.
    CallNative(u16, u8),
    Return,
    /// Push a closure. u16 indexes a `Constant::Function`.
    /// Followed in the constant pool by upvalue descriptors.
    Closure(u16),
    /// Close the topmost open upvalue on the stack.
    CloseUpvalue,

    // ── Data construction ──
    MakeList(u16),
    MakeRecord(u16),
    MakeSet(u16),
    /// Build a Range value. u8: 1 = inclusive, 0 = exclusive.
    MakeRange(u8),

    // ── Field / Index access ──
    /// Get field by name constant index.
    GetField(u16),
    /// Set field by name constant index (value on TOS, object below).
    SetField(u16),
    /// Optional chaining: like GetField but pushes Nil instead of erroring.
    GetFieldOpt(u16),
    /// Get index: object[index] — index on TOS, object below.
    GetIndex,
    /// Set index: object[index] = value — value, index, object on stack.
    SetIndex,

    // ── Iteration ──
    /// Pop iterable from TOS, push internal iterator onto iterator stack.
    PushIter,
    /// Advance iterator; push next value or jump if exhausted.
    IterNext(i16),
    /// Pop from iterator stack.
    PopIter,

    // ── String interpolation ──
    /// Pop `n` parts from stack, concatenate.
    StringInterp(u16),

    // ── Bio literals ──
    MakeDna(u16),
    MakeRna(u16),
    MakeProtein(u16),

    // ── Exception handling ──
    /// Begin try block: u16 is offset to catch handler.
    TryBegin(u16),
    /// End try block (pop handler).
    TryEnd,
    /// Throw TOS as error.
    Throw,

    // ── Null coalescing ──
    /// Pop TOS; if Nil, keep going (evaluate right side); if non-Nil, jump past right side.
    NullCoalesce(i16),

    // ── Special ──
    /// Store an AST fragment constant as a Formula value.
    MakeFormula(u16),
    /// Import: path constant index + has_alias flag.
    Import(u16, u8),
    /// Assert: pop value, error if falsy.
    AssertCheck,

    // ── Debug ──
    /// Source span for the following instruction(s).
    DebugSpan(u32, u32),
}

impl OpCode {
    /// Human-readable name for disassembly.
    pub fn name(&self) -> &'static str {
        match self {
            OpCode::Constant(_) => "CONSTANT",
            OpCode::Nil => "NIL",
            OpCode::True => "TRUE",
            OpCode::False => "FALSE",
            OpCode::Pop => "POP",
            OpCode::Dup => "DUP",
            OpCode::GetLocal(_) => "GET_LOCAL",
            OpCode::SetLocal(_) => "SET_LOCAL",
            OpCode::DefineGlobal(_) => "DEFINE_GLOBAL",
            OpCode::GetGlobal(_) => "GET_GLOBAL",
            OpCode::SetGlobal(_) => "SET_GLOBAL",
            OpCode::GetUpvalue(_) => "GET_UPVALUE",
            OpCode::SetUpvalue(_) => "SET_UPVALUE",
            OpCode::Add => "ADD",
            OpCode::Sub => "SUB",
            OpCode::Mul => "MUL",
            OpCode::Div => "DIV",
            OpCode::Mod => "MOD",
            OpCode::Negate => "NEGATE",
            OpCode::Not => "NOT",
            OpCode::Equal => "EQUAL",
            OpCode::NotEqual => "NOT_EQUAL",
            OpCode::Less => "LESS",
            OpCode::Greater => "GREATER",
            OpCode::LessEqual => "LESS_EQUAL",
            OpCode::GreaterEqual => "GREATER_EQUAL",
            OpCode::Jump(_) => "JUMP",
            OpCode::JumpIfFalse(_) => "JUMP_IF_FALSE",
            OpCode::JumpIfTrue(_) => "JUMP_IF_TRUE",
            OpCode::Loop(_) => "LOOP",
            OpCode::Call(_) => "CALL",
            OpCode::CallNative(_, _) => "CALL_NATIVE",
            OpCode::Return => "RETURN",
            OpCode::Closure(_) => "CLOSURE",
            OpCode::CloseUpvalue => "CLOSE_UPVALUE",
            OpCode::MakeList(_) => "MAKE_LIST",
            OpCode::MakeRecord(_) => "MAKE_RECORD",
            OpCode::MakeSet(_) => "MAKE_SET",
            OpCode::MakeRange(_) => "MAKE_RANGE",
            OpCode::GetField(_) => "GET_FIELD",
            OpCode::SetField(_) => "SET_FIELD",
            OpCode::GetFieldOpt(_) => "GET_FIELD_OPT",
            OpCode::GetIndex => "GET_INDEX",
            OpCode::SetIndex => "SET_INDEX",
            OpCode::PushIter => "PUSH_ITER",
            OpCode::IterNext(_) => "ITER_NEXT",
            OpCode::PopIter => "POP_ITER",
            OpCode::StringInterp(_) => "STRING_INTERP",
            OpCode::MakeDna(_) => "MAKE_DNA",
            OpCode::MakeRna(_) => "MAKE_RNA",
            OpCode::MakeProtein(_) => "MAKE_PROTEIN",
            OpCode::TryBegin(_) => "TRY_BEGIN",
            OpCode::TryEnd => "TRY_END",
            OpCode::Throw => "THROW",
            OpCode::NullCoalesce(_) => "NULL_COALESCE",
            OpCode::MakeFormula(_) => "MAKE_FORMULA",
            OpCode::Import(_, _) => "IMPORT",
            OpCode::AssertCheck => "ASSERT_CHECK",
            OpCode::DebugSpan(_, _) => "DEBUG_SPAN",
        }
    }
}
