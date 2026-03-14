use crate::span::Span;
use crate::value::Value;
use std::fmt;

/// A frame in the call stack for error reporting.
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub function_name: String,
    pub span: Option<Span>,
    pub file: Option<String>,
}

/// All errors in BioLang.
#[derive(Debug, Clone)]
pub struct BioLangError {
    pub kind: ErrorKind,
    pub message: String,
    pub span: Option<Span>,
    /// Carries the actual Value for `return` statements (boxed to keep error size small).
    pub return_value: Option<Box<Value>>,
    /// Call stack snapshot at point of error
    pub call_stack: Vec<StackFrame>,
    /// Optional suggestions for fixing the error.
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    // Lexer errors
    UnexpectedChar,
    UnterminatedString,
    InvalidNumber,
    InvalidEscape,

    // Parser errors
    UnexpectedToken,
    ExpectedExpression,
    ExpectedToken,

    // Runtime errors
    TypeError,
    NameError,
    ArityError,
    DivisionByZero,
    IndexOutOfBounds,
    AssertionFailed,
    Return,
    Break,
    Continue,
    IOError,
    ImportError,
    PluginError,
}

impl BioLangError {
    pub fn new(kind: ErrorKind, message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            kind,
            message: message.into(),
            span,
            return_value: None,
            call_stack: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn lexer(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::UnexpectedChar, message, Some(span))
    }

    pub fn parser(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::UnexpectedToken, message, Some(span))
    }

    pub fn runtime(kind: ErrorKind, message: impl Into<String>, span: Option<Span>) -> Self {
        Self::new(kind, message, span)
    }

    pub fn type_error(message: impl Into<String>, span: Option<Span>) -> Self {
        Self::new(ErrorKind::TypeError, message, span)
    }

    pub fn name_error(message: impl Into<String>, span: Option<Span>) -> Self {
        Self::new(ErrorKind::NameError, message, span)
    }

    /// Create a Return "error" carrying an actual Value.
    pub fn return_val(value: Value, span: Option<Span>) -> Self {
        Self {
            kind: ErrorKind::Return,
            message: String::new(),
            span,
            return_value: Some(Box::new(value)),
            call_stack: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// Attach a call stack snapshot to this error.
    pub fn with_stack(mut self, stack: Vec<StackFrame>) -> Self {
        self.call_stack = stack;
        self
    }

    /// Add a suggestion for fixing this error.
    pub fn with_suggestion(mut self, s: impl Into<String>) -> Self {
        self.suggestions.push(s.into());
        self
    }

    pub fn import_error(message: impl Into<String>, span: Option<Span>) -> Self {
        Self::new(ErrorKind::ImportError, message, span)
    }

    pub fn plugin_error(message: impl Into<String>, span: Option<Span>) -> Self {
        Self::new(ErrorKind::PluginError, message, span)
    }

    /// Format error with source context.
    pub fn format_with_source(&self, source: &str) -> String {
        let mut result = format!("{self}");
        if let Some(span) = self.span {
            let (line, col) = offset_to_line_col(source, span.start);
            result.push_str(&format!("\n  at line {line}, column {col}"));

            // Show the source line
            if let Some(source_line) = source.lines().nth(line - 1) {
                result.push_str(&format!("\n  | {source_line}"));
                result.push_str(&format!("\n  | {}^", " ".repeat(col - 1)));
            }
        }

        // Print stack trace if available
        if !self.call_stack.is_empty() {
            result.push_str("\n\nStack trace (most recent call last):");
            for (i, frame) in self.call_stack.iter().enumerate() {
                let file = frame.file.as_deref().unwrap_or("<repl>");
                if let Some(span) = frame.span {
                    let (line, _col) = offset_to_line_col(source, span.start);
                    result.push_str(&format!(
                        "\n  #{} {} ({}:{})",
                        i, frame.function_name, file, line
                    ));
                } else {
                    result.push_str(&format!("\n  #{} {} ({})", i, frame.function_name, file));
                }
            }
        }

        // Print suggestions/hints
        if !self.suggestions.is_empty() {
            for s in &self.suggestions {
                result.push_str(&format!("\n  hint: {s}"));
            }
        }

        result
    }
}

impl fmt::Display for BioLangError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match &self.kind {
            ErrorKind::UnexpectedChar => "SyntaxError",
            ErrorKind::UnterminatedString => "SyntaxError",
            ErrorKind::InvalidNumber => "SyntaxError",
            ErrorKind::InvalidEscape => "SyntaxError",
            ErrorKind::UnexpectedToken => "ParseError",
            ErrorKind::ExpectedExpression => "ParseError",
            ErrorKind::ExpectedToken => "ParseError",
            ErrorKind::TypeError => "TypeError",
            ErrorKind::NameError => "NameError",
            ErrorKind::ArityError => "ArityError",
            ErrorKind::DivisionByZero => "DivisionByZero",
            ErrorKind::IndexOutOfBounds => "IndexOutOfBounds",
            ErrorKind::AssertionFailed => "AssertionFailed",
            ErrorKind::Return => "Return",
            ErrorKind::Break => "Break",
            ErrorKind::Continue => "Continue",
            ErrorKind::IOError => "IOError",
            ErrorKind::ImportError => "ImportError",
            ErrorKind::PluginError => "PluginError",
        };
        write!(f, "{kind}: {}", self.message)?;
        for s in &self.suggestions {
            write!(f, "\n  hint: {s}")?;
        }
        Ok(())
    }
}

impl std::error::Error for BioLangError {}

pub type Result<T> = std::result::Result<T, BioLangError>;

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}
