use crate::span::Spanned;
use std::fmt;

/// A complete BioLang program.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub stmts: Vec<Spanned<Stmt>>,
}

/// Statements — things that don't produce a value (or whose value is discarded).
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `let name = expr`
    Let {
        name: String,
        type_ann: Option<TypeAnnotation>,
        value: Spanned<Expr>,
    },
    /// `fn name(params) -> RetType { body }` or `fn* name(params) { body }` (generator)
    Fn {
        name: String,
        params: Vec<Param>,
        return_type: Option<TypeAnnotation>,
        body: Vec<Spanned<Stmt>>,
        doc: Option<String>,
        is_generator: bool,
        decorators: Vec<String>,
        is_async: bool,
        /// Named tuple return: `-> (score: Int, cigar: Str)`
        named_returns: Vec<(String, TypeAnnotation)>,
        /// `where` clause: `fn f(x) where x > 0 { ... }`
        where_clause: Option<Spanned<Expr>>,
    },
    /// `name = expr` (reassignment)
    Assign {
        name: String,
        value: Spanned<Expr>,
    },
    /// Expression used as a statement
    Expr(Spanned<Expr>),
    /// `return expr`
    Return(Option<Spanned<Expr>>),
    /// `for var in iter { body }` or `for var in iter when cond { body }` with optional `else { ... }`
    For {
        pattern: ForPattern,
        iter: Spanned<Expr>,
        when_guard: Option<Spanned<Expr>>,
        body: Vec<Spanned<Stmt>>,
        else_body: Option<Vec<Spanned<Stmt>>>,
    },
    /// `while condition { body }`
    While {
        condition: Spanned<Expr>,
        body: Vec<Spanned<Stmt>>,
    },
    /// `break` (inside for/while)
    Break,
    /// `continue` (inside for/while)
    Continue,
    /// Destructuring let: `let [a, b] = expr` or `let {x, y} = expr`
    DestructLet {
        pattern: DestructPattern,
        value: Spanned<Expr>,
    },
    /// `assert condition` or `assert condition, "message"`
    Assert {
        condition: Spanned<Expr>,
        message: Option<Spanned<Expr>>,
    },
    /// `pipeline name { body }` or `pipeline name(params) { body }`
    Pipeline {
        name: String,
        params: Vec<Param>,
        body: Vec<Spanned<Stmt>>,
    },
    /// `import "path"` or `import "path" as alias`
    Import {
        path: String,
        alias: Option<String>,
    },
    /// `yield expr` (inside generator functions)
    Yield(Spanned<Expr>),
    /// `enum Name { A, B(x), C(x, y) }`
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
    },
    /// `struct Name { field: Type, field2: Type = default }`
    Struct {
        name: String,
        fields: Vec<StructField>,
    },
    /// `trait Name { fn method(self) fn method2(self, x) }`
    Trait {
        name: String,
        methods: Vec<TraitMethod>,
    },
    /// `impl Type { fn method(self) { body } }` or `impl Trait for Type { ... }`
    Impl {
        type_name: String,
        trait_name: Option<String>,
        methods: Vec<Spanned<Stmt>>,
    },
    /// `const NAME = expr` — immutable binding
    Const {
        name: String,
        type_ann: Option<TypeAnnotation>,
        value: Spanned<Expr>,
    },
    /// `with expr { body }` — scoped context block
    With {
        expr: Spanned<Expr>,
        body: Vec<Spanned<Stmt>>,
    },
    /// `unless condition { body }` or `unless condition then expr`
    Unless {
        condition: Spanned<Expr>,
        body: Vec<Spanned<Stmt>>,
    },
    /// `guard condition else { fallback }` or `guard condition else return expr`
    Guard {
        condition: Spanned<Expr>,
        else_body: Vec<Spanned<Stmt>>,
    },
    /// `defer expr` — runs when enclosing function exits
    Defer(Spanned<Expr>),
    /// `parallel for var in iter { body }`
    ParallelFor {
        pattern: ForPattern,
        iter: Spanned<Expr>,
        body: Vec<Spanned<Stmt>>,
    },
    /// `stage "name" -> expr` inside a pipeline block
    Stage {
        name: String,
        expr: Spanned<Expr>,
    },
    /// `from "module" import name1, name2`
    FromImport {
        path: String,
        names: Vec<String>,
    },
    /// `name ?= expr` — assign only if name is nil
    NilAssign {
        name: String,
        value: Spanned<Expr>,
    },
    /// `type Name = TypeExpr` — type alias
    TypeAlias {
        name: String,
        target: TypeAnnotation,
    },
}

/// Expressions — things that produce a value.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // Literals
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),

    // Bio literals
    DnaLit(String),
    RnaLit(String),
    ProteinLit(String),
    /// Quality score literal: `qual"FFFFFFFF"`
    QualLit(String),

    /// Variable reference
    Ident(String),

    /// Unary operation: `-x`, `!x`
    Unary {
        op: UnaryOp,
        expr: Box<Spanned<Expr>>,
    },

    /// Binary operation: `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `&&`, `||`
    Binary {
        op: BinaryOp,
        left: Box<Spanned<Expr>>,
        right: Box<Spanned<Expr>>,
    },

    /// Pipe: `expr |> expr`
    Pipe {
        left: Box<Spanned<Expr>>,
        right: Box<Spanned<Expr>>,
    },

    /// Pipe-into: `expr |> into name` — binds value to name, returns value
    PipeInto {
        value: Box<Spanned<Expr>>,
        name: String,
    },

    /// Function call: `f(a, b, key: value)`
    Call {
        callee: Box<Spanned<Expr>>,
        args: Vec<Arg>,
    },

    /// Field access: `obj.field` or optional `obj?.field`
    Field {
        object: Box<Spanned<Expr>>,
        field: String,
        optional: bool,
    },

    /// Index access: `obj[index]`
    Index {
        object: Box<Spanned<Expr>>,
        index: Box<Spanned<Expr>>,
    },

    /// Lambda: `|params| body`
    Lambda {
        params: Vec<Param>,
        body: Box<Spanned<Expr>>,
    },

    /// Block expression: `{ stmts; expr }`
    Block(Vec<Spanned<Stmt>>),

    /// If expression: `if cond { ... } else { ... }`
    If {
        condition: Box<Spanned<Expr>>,
        then_body: Vec<Spanned<Stmt>>,
        else_body: Option<Vec<Spanned<Stmt>>>,
    },

    /// List literal: `[a, b, c]`
    List(Vec<Spanned<Expr>>),

    /// Record literal: `{key: value, ...}` or `{...base, key: value}`
    Record(Vec<RecordEntry>),

    /// Formula: `~expr`
    Formula(Box<Spanned<Expr>>),

    /// Match expression
    Match {
        expr: Box<Spanned<Expr>>,
        arms: Vec<MatchArm>,
    },

    /// Try/catch: `try { body } catch err { handler }`
    TryCatch {
        body: Vec<Spanned<Stmt>>,
        error_var: Option<String>,
        catch_body: Vec<Spanned<Stmt>>,
    },

    /// Null coalescing: `expr ?? default`
    NullCoalesce {
        left: Box<Spanned<Expr>>,
        right: Box<Spanned<Expr>>,
    },

    /// String interpolation: `f"hello {name}, you are {age} years old"`
    StringInterp(Vec<StringPart>),

    /// Range literal: `1..10` or `1..=10`
    Range {
        start: Box<Spanned<Expr>>,
        end: Box<Spanned<Expr>>,
        inclusive: bool,
    },

    /// List comprehension: `[expr for x in iter if cond]`
    ListComp {
        expr: Box<Spanned<Expr>>,
        var: String,
        iter: Box<Spanned<Expr>>,
        condition: Option<Box<Spanned<Expr>>>,
    },

    /// Ternary: `value if condition else other`
    Ternary {
        value: Box<Spanned<Expr>>,
        condition: Box<Spanned<Expr>>,
        else_value: Box<Spanned<Expr>>,
    },

    /// Chained comparison: `a < b < c` → `a < b && b < c`
    ChainedCmp {
        operands: Vec<Spanned<Expr>>,
        ops: Vec<BinaryOp>,
    },

    /// Map comprehension: `{k: v for x in iter if cond}`
    MapComp {
        key: Box<Spanned<Expr>>,
        value: Box<Spanned<Expr>>,
        var: String,
        iter: Box<Spanned<Expr>>,
        condition: Option<Box<Spanned<Expr>>>,
    },

    /// Set literal: `#{1, 2, 3}`
    SetLiteral(Vec<Spanned<Expr>>),

    /// Tuple literal: `(a, b)` or `(a,)` for single-element
    TupleLit(Vec<Spanned<Expr>>),

    /// Regex literal: `/pattern/flags`
    Regex {
        pattern: String,
        flags: String,
    },

    /// Await expression: `await expr`
    Await(Box<Spanned<Expr>>),

    /// Struct literal: `Point { x: 1, y: 2 }`
    StructLit {
        name: String,
        fields: Vec<(String, Spanned<Expr>)>,
    },

    /// `in` membership test: `expr in collection`
    In {
        left: Box<Spanned<Expr>>,
        right: Box<Spanned<Expr>>,
        negated: bool, // `not in`
    },

    /// `do |params| ... end` multi-statement lambda
    DoBlock {
        params: Vec<Param>,
        body: Vec<Spanned<Stmt>>,
    },

    /// `expr as Type` — type narrowing/conversion
    TypeCast {
        expr: Box<Spanned<Expr>>,
        target: String,
    },

    /// `|> then var -> expr` — destructuring pipe
    ThenPipe {
        left: Box<Spanned<Expr>>,
        var: String,
        right: Box<Spanned<Expr>>,
    },

    /// Slice access: `obj[start:end]` or `obj[start:end:step]`
    Slice {
        object: Box<Spanned<Expr>>,
        start: Option<Box<Spanned<Expr>>>,
        end: Option<Box<Spanned<Expr>>>,
        step: Option<Box<Spanned<Expr>>>,
    },

    /// Tap-pipe: `expr |>> side_effect_expr` — evaluates side effect, passes value through
    TapPipe {
        left: Box<Spanned<Expr>>,
        right: Box<Spanned<Expr>>,
    },

    /// `given { cond1 => expr1, cond2 => expr2, otherwise => expr3 }`
    Given {
        arms: Vec<(Spanned<Expr>, Spanned<Expr>)>,
        otherwise: Option<Box<Spanned<Expr>>>,
    },

    /// `retry(n, delay: ms) { body }` — retry expression with optional delay
    Retry {
        count: Box<Spanned<Expr>>,
        delay: Option<Box<Spanned<Expr>>>,
        body: Vec<Spanned<Stmt>>,
    },

}

/// An entry in a record literal — either a named field or a spread.
#[derive(Debug, Clone, PartialEq)]
pub enum RecordEntry {
    /// `key: value`
    Field(String, Spanned<Expr>),
    /// `...expr` — spread another record's fields
    Spread(Spanned<Expr>),
}

/// A function/lambda parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_ann: Option<TypeAnnotation>,
    pub default: Option<Spanned<Expr>>,
    /// `...rest` parameter — collects remaining positional args into a List
    pub rest: bool,
}

/// A function call argument (positional or named).
#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    pub name: Option<String>,
    pub value: Spanned<Expr>,
    /// `...expr` — spread a List into positional args
    pub spread: bool,
}

/// A match arm: `pattern [if guard] => expr`
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Spanned<Pattern>,
    pub guard: Option<Box<Spanned<Expr>>>,
    pub body: Spanned<Expr>,
}

/// Match patterns.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard,
    Literal(Spanned<Expr>),
    Ident(String),
    /// `Variant` or `Variant(a, b)` — enum variant pattern
    EnumVariant {
        variant: String,
        bindings: Vec<String>,
    },
    /// `Int(n)`, `Str(_)`, `DNA(s)` — type-based pattern match
    TypePattern {
        type_name: String,
        binding: Option<String>,
    },
    /// `pattern1 | pattern2 | ...` — or-pattern (matches any alternative)
    Or(Vec<Spanned<Pattern>>),
}

/// An enum variant definition: `A`, `B(x)`, `C(x, y)`
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<String>,
}

/// A struct field definition
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub type_ann: Option<TypeAnnotation>,
    pub default: Option<Spanned<Expr>>,
}

/// A trait method signature
#[derive(Debug, Clone, PartialEq)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Param>,
}

/// Parts of a string interpolation expression.
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    /// Literal text segment
    Lit(String),
    /// Interpolated expression: `{expr}`
    Expr(Spanned<Expr>),
}

/// Pattern for for-loop variable binding.
#[derive(Debug, Clone, PartialEq)]
pub enum ForPattern {
    /// `for x in ...` — single variable binding
    Single(String),
    /// `for [a, b] in ...` — list destructuring
    ListDestr(Vec<String>),
    /// `for {x, y} in ...` — record destructuring
    RecordDestr(Vec<String>),
    /// `for (a, b) in zip(xs, ys)` — tuple destructuring
    TupleDestr(Vec<String>),
}

/// Destructuring pattern for `let [a, b] = ...` or `let {x, y} = ...`
#[derive(Debug, Clone, PartialEq)]
pub enum DestructPattern {
    /// `[a, b, c]` — bind elements by position
    List(Vec<String>),
    /// `[a, b, ...rest]` — list with rest capture
    ListWithRest {
        elements: Vec<String>,
        rest_name: String,
    },
    /// `{x, y}` — bind fields by name
    Record(Vec<String>),
    /// `{x, y, ...rest}` — record with rest capture
    RecordWithRest {
        fields: Vec<String>,
        rest_name: String,
    },
}

/// Type annotation (used in function params and return types).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeAnnotation {
    pub name: String,
    pub params: Vec<TypeAnnotation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    BitAnd,
    BitXor,
    Shl,
    Shr,
    Concat,
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Neg => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Sub => write!(f, "-"),
            BinaryOp::Mul => write!(f, "*"),
            BinaryOp::Div => write!(f, "/"),
            BinaryOp::Mod => write!(f, "%"),
            BinaryOp::Pow => write!(f, "**"),
            BinaryOp::Eq => write!(f, "=="),
            BinaryOp::Neq => write!(f, "!="),
            BinaryOp::Lt => write!(f, "<"),
            BinaryOp::Gt => write!(f, ">"),
            BinaryOp::Le => write!(f, "<="),
            BinaryOp::Ge => write!(f, ">="),
            BinaryOp::And => write!(f, "&&"),
            BinaryOp::Or => write!(f, "||"),
            BinaryOp::BitAnd => write!(f, "&"),
            BinaryOp::BitXor => write!(f, "^"),
            BinaryOp::Shl => write!(f, "<<"),
            BinaryOp::Shr => write!(f, ">>"),
            BinaryOp::Concat => write!(f, "++"),
        }
    }
}
