use bl_core::span::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Int(i64),
    Float(f64),
    Str(String),

    // Bio literals
    DnaLit(String),
    RnaLit(String),
    ProteinLit(String),
    QualLit(String),

    // Identifier
    Ident(String),

    // Keywords
    Let,
    Fn,
    If,
    Else,
    For,
    In,
    While,
    Break,
    Continue,
    Match,
    Return,
    Assert,
    Try,
    Catch,
    Pipeline,
    Import,
    True,
    False,
    Nil,
    Yield,
    Enum,
    Struct,
    Async,
    Await,
    Trait,
    Impl,
    Const,
    With,
    Then,
    Unless,
    Guard,
    Do,
    End,
    When,
    Defer,
    As,
    Stage,
    Parallel,
    Not,
    From,
    Given,
    Otherwise,
    Retry,
    Where,

    // Regex literal: /pattern/flags
    RegexLit(String, String),

    // F-string literal: f"hello {name}"
    FStr(String),

    // Doc comment: ## ...
    DocComment(String),

    // Operators
    Plus,      // +
    Minus,     // -
    Star,      // *
    StarStar,  // **
    Slash,     // /
    Percent,   // %
    DotDot,    // ..
    DotDotEq,  // ..=
    DotDotDot, // ...
    PlusEq,    // +=
    MinusEq,   // -=
    StarEq,    // *=
    SlashEq,   // /=
    QuestionQuestion, // ??
    QuestionDot,      // ?.
    QuestionEq,       // ?=
    HashLBrace,       // #{
    At,               // @
    EqEq,    // ==
    Neq,      // !=
    Lt,       // <
    Gt,       // >
    Le,       // <=
    Ge,       // >=
    Amp,      // & (bitwise AND)
    And,      // &&
    Or,       // ||
    Bang,     // !
    Caret,    // ^ (bitwise XOR)
    Shl,      // <<
    Shr,      // >>
    Eq,       // =
    PipeOp,    // |>
    TapPipe,   // |>>
    Tilde,    // ~
    Dot,      // .
    Colon,    // :
    Comma,    // ,
    Arrow,    // ->
    FatArrow, // =>

    // Delimiters
    LParen,   // (
    RParen,   // )
    LBrace,   // {
    RBrace,   // }
    LBracket, // [
    RBracket, // ]
    Bar,      // | (for lambda params)

    // Special
    Newline,
    Eof,
}

impl TokenKind {
    pub fn keyword_from_str(s: &str) -> Option<TokenKind> {
        match s {
            "let" => Some(TokenKind::Let),
            "fn" => Some(TokenKind::Fn),
            "if" => Some(TokenKind::If),
            "else" => Some(TokenKind::Else),
            "for" => Some(TokenKind::For),
            "in" => Some(TokenKind::In),
            "while" => Some(TokenKind::While),
            "break" => Some(TokenKind::Break),
            "continue" => Some(TokenKind::Continue),
            "match" => Some(TokenKind::Match),
            "return" => Some(TokenKind::Return),
            "assert" => Some(TokenKind::Assert),
            "try" => Some(TokenKind::Try),
            "catch" => Some(TokenKind::Catch),
            "pipeline" => Some(TokenKind::Pipeline),
            "import" => Some(TokenKind::Import),
            "true" => Some(TokenKind::True),
            "false" => Some(TokenKind::False),
            "nil" => Some(TokenKind::Nil),
            "yield" => Some(TokenKind::Yield),
            "enum" => Some(TokenKind::Enum),
            "struct" => Some(TokenKind::Struct),
            "async" => Some(TokenKind::Async),
            "await" => Some(TokenKind::Await),
            "trait" => Some(TokenKind::Trait),
            "impl" => Some(TokenKind::Impl),
            "const" => Some(TokenKind::Const),
            "with" => Some(TokenKind::With),
            "then" => Some(TokenKind::Then),
            "unless" => Some(TokenKind::Unless),
            "guard" => Some(TokenKind::Guard),
            "do" => Some(TokenKind::Do),
            "end" => Some(TokenKind::End),
            "when" => Some(TokenKind::When),
            "defer" => Some(TokenKind::Defer),
            "as" => Some(TokenKind::As),
            "stage" => Some(TokenKind::Stage),
            "parallel" => Some(TokenKind::Parallel),
            "not" => Some(TokenKind::Not),
            "from" => Some(TokenKind::From),
            "given" => Some(TokenKind::Given),
            "otherwise" => Some(TokenKind::Otherwise),
            "retry" => Some(TokenKind::Retry),
            "where" => Some(TokenKind::Where),
            _ => None,
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Int(n) => write!(f, "{n}"),
            TokenKind::Float(v) => write!(f, "{v}"),
            TokenKind::Str(s) => write!(f, "\"{s}\""),
            TokenKind::DnaLit(s) => write!(f, "dna\"{s}\""),
            TokenKind::RnaLit(s) => write!(f, "rna\"{s}\""),
            TokenKind::ProteinLit(s) => write!(f, "protein\"{s}\""),
            TokenKind::QualLit(s) => write!(f, "qual\"{s}\""),
            TokenKind::Ident(s) => write!(f, "{s}"),
            TokenKind::Let => write!(f, "let"),
            TokenKind::Fn => write!(f, "fn"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::For => write!(f, "for"),
            TokenKind::In => write!(f, "in"),
            TokenKind::While => write!(f, "while"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::Assert => write!(f, "assert"),
            TokenKind::Try => write!(f, "try"),
            TokenKind::Catch => write!(f, "catch"),
            TokenKind::Pipeline => write!(f, "pipeline"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Nil => write!(f, "nil"),
            TokenKind::Yield => write!(f, "yield"),
            TokenKind::Enum => write!(f, "enum"),
            TokenKind::Struct => write!(f, "struct"),
            TokenKind::Async => write!(f, "async"),
            TokenKind::Await => write!(f, "await"),
            TokenKind::Trait => write!(f, "trait"),
            TokenKind::Impl => write!(f, "impl"),
            TokenKind::Const => write!(f, "const"),
            TokenKind::With => write!(f, "with"),
            TokenKind::Then => write!(f, "then"),
            TokenKind::Unless => write!(f, "unless"),
            TokenKind::Guard => write!(f, "guard"),
            TokenKind::Do => write!(f, "do"),
            TokenKind::End => write!(f, "end"),
            TokenKind::When => write!(f, "when"),
            TokenKind::Defer => write!(f, "defer"),
            TokenKind::As => write!(f, "as"),
            TokenKind::Stage => write!(f, "stage"),
            TokenKind::Parallel => write!(f, "parallel"),
            TokenKind::Not => write!(f, "not"),
            TokenKind::From => write!(f, "from"),
            TokenKind::Given => write!(f, "given"),
            TokenKind::Otherwise => write!(f, "otherwise"),
            TokenKind::Retry => write!(f, "retry"),
            TokenKind::Where => write!(f, "where"),
            TokenKind::RegexLit(pat, flags) => write!(f, "/{pat}/{flags}"),
            TokenKind::DocComment(s) => write!(f, "##{s}"),
            TokenKind::FStr(s) => write!(f, "f\"{s}\""),
            TokenKind::DotDot => write!(f, ".."),
            TokenKind::DotDotEq => write!(f, "..="),
            TokenKind::DotDotDot => write!(f, "..."),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::StarStar => write!(f, "**"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::PlusEq => write!(f, "+="),
            TokenKind::MinusEq => write!(f, "-="),
            TokenKind::StarEq => write!(f, "*="),
            TokenKind::SlashEq => write!(f, "/="),
            TokenKind::QuestionQuestion => write!(f, "??"),
            TokenKind::QuestionDot => write!(f, "?."),
            TokenKind::QuestionEq => write!(f, "?="),
            TokenKind::HashLBrace => write!(f, "#{{"),
            TokenKind::At => write!(f, "@"),
            TokenKind::EqEq => write!(f, "=="),
            TokenKind::Neq => write!(f, "!="),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::Le => write!(f, "<="),
            TokenKind::Ge => write!(f, ">="),
            TokenKind::Amp => write!(f, "&"),
            TokenKind::And => write!(f, "&&"),
            TokenKind::Or => write!(f, "||"),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::Caret => write!(f, "^"),
            TokenKind::Shl => write!(f, "<<"),
            TokenKind::Shr => write!(f, ">>"),
            TokenKind::Eq => write!(f, "="),
            TokenKind::PipeOp => write!(f, "|>"),
            TokenKind::TapPipe => write!(f, "|>>"),
            TokenKind::Tilde => write!(f, "~"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::Bar => write!(f, "|"),
            TokenKind::Newline => write!(f, "\\n"),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}
