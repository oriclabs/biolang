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
    /// Recursion nested deeper than the interpreter's stack can carry. Raised
    /// deliberately so a runaway recursion reports a span instead of aborting
    /// the process with "overflowed its stack".
    RecursionLimit,
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
    ///
    /// Deliberately not built on `Display`, which prints the suggestions
    /// itself: starting from it and then appending them again below the source
    /// excerpt printed every "did you mean" twice, once above the location and
    /// once below it.
    pub fn format_with_source(&self, source: &str) -> String {
        let mut result = format!("{}: {}", self.kind_label(), self.message);
        if let Some(span) = self.span {
            let (line, col) = offset_to_line_col(source, span.start);
            result.push_str(&format!("\n  at line {line}, column {col}"));

            // Show the source line
            if let Some(source_line) = source.lines().nth(line - 1) {
                result.push_str(&format!("\n  | {source_line}"));
                result.push_str(&format!("\n  | {}^", " ".repeat(col - 1)));
            }
        }

        // Print stack trace if available.
        //
        // Deep traces are elided in the middle. A runaway recursion produces
        // hundreds of identical frames, and printing all of them buries the
        // error message itself hundreds of lines up the terminal - the one thing
        // the reader needs. The ends are what carry information: where it
        // started and where it stopped.
        if !self.call_stack.is_empty() {
            const HEAD: usize = 5;
            const TAIL: usize = 5;
            result.push_str("\n\nStack trace (most recent call last):");

            let total = self.call_stack.len();
            let mut render = |i: usize, frame: &StackFrame, out: &mut String| {
                let file = frame.file.as_deref().unwrap_or("<repl>");
                if let Some(span) = frame.span {
                    let (line, _col) = offset_to_line_col(source, span.start);
                    out.push_str(&format!(
                        "\n  #{} {} ({}:{})",
                        i, frame.function_name, file, line
                    ));
                } else {
                    out.push_str(&format!("\n  #{} {} ({})", i, frame.function_name, file));
                }
            };

            if total <= HEAD + TAIL + 1 {
                for (i, frame) in self.call_stack.iter().enumerate() {
                    render(i, frame, &mut result);
                }
            } else {
                for (i, frame) in self.call_stack.iter().enumerate().take(HEAD) {
                    render(i, frame, &mut result);
                }
                result.push_str(&format!("\n  ... {} more frames ...", total - HEAD - TAIL));
                for (i, frame) in self.call_stack.iter().enumerate().skip(total - TAIL) {
                    render(i, frame, &mut result);
                }
            }
        }

        // An explicit suggestion wins; otherwise derive one centrally, so an
        // error raised anywhere still says something useful.
        if self.suggestions.is_empty() {
            for hint in crate::hints::suggest(&self.kind, &self.message) {
                result.push_str(&format!("\n  hint: {}", hint.text));
            }
        } else {
            for s in &self.suggestions {
                result.push_str(&format!("\n  hint: {s}"));
            }
        }

        result
    }
}

impl BioLangError {
    /// The name a reader sees for this kind of error.
    fn kind_label(&self) -> &'static str {
        match &self.kind {
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
            ErrorKind::RecursionLimit => "RecursionLimit",
        }
    }
}

impl fmt::Display for BioLangError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind_label(), self.message)?;
        for s in &self.suggestions {
            write!(f, "\n  hint: {s}")?;
        }
        Ok(())
    }
}

impl std::error::Error for BioLangError {}

pub type Result<T> = std::result::Result<T, BioLangError>;

/// Resolve a span offset to a 1-based line and column.
///
/// `offset` is a **character** index, not a byte index: the lexer works over
/// `source.chars().collect()`, so every span it produces counts characters.
/// This used to compare against `char_indices()` byte positions, which made
/// every error in a file containing non-ASCII text point at the wrong line —
/// and the `# ──────` section headers used throughout the examples are three
/// bytes per character, so the reported line drifted further the longer the
/// file got.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for ch in source.chars().take(offset) {
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[cfg(test)]
mod line_col_tests {
    use super::*;

    /// Spans come from the lexer, which indexes `source.chars().collect()`, so
    /// offsets count characters. Resolving them as byte offsets made every
    /// error in a file containing non-ASCII point at the wrong line.
    #[test]
    fn resolves_character_offsets_not_byte_offsets() {
        // Each `─` is one character but three bytes in UTF-8.
        let source = "# ───\nlet a = 1\nlet b = 2\n";
        let offset = source.chars().position(|c| c == 'b').unwrap();
        assert_eq!(offset_to_line_col(source, offset), (3, 5));
    }

    #[test]
    fn ascii_only_source_is_unaffected() {
        let source = "let a = 1\nlet b = 2\n";
        let offset = source.chars().position(|c| c == 'b').unwrap();
        assert_eq!(offset_to_line_col(source, offset), (2, 5));
    }

    /// The drift used to grow with each multi-byte line, so a header-heavy file
    /// misreported by several lines rather than one.
    #[test]
    fn drift_does_not_accumulate_over_many_multibyte_lines() {
        let header = "# ──────────\n";
        let mut source = header.repeat(6);
        source.push_str("let bad = 1\n");
        let offset = source.chars().position(|c| c == 'b').unwrap();
        assert_eq!(offset_to_line_col(&source, offset), (7, 5));
    }

    #[test]
    fn start_of_source_is_line_one_column_one() {
        assert_eq!(offset_to_line_col("let a = 1", 0), (1, 1));
    }

    /// A span pointing past the end must not panic or wrap.
    #[test]
    fn offset_beyond_end_clamps_to_the_last_position() {
        let source = "let a = 1\n";
        let (line, _) = offset_to_line_col(source, 9_999);
        assert_eq!(line, 2);
    }
}

#[cfg(test)]
mod suggestion_rendering_tests {
    use super::*;

    // `format_with_source` used to start from the `Display` output, which
    // prints the suggestions itself, and then print them again after the source
    // excerpt. Every "did you mean" therefore appeared twice with the file
    // location wedged between them.

    fn errored() -> BioLangError {
        BioLangError::new(ErrorKind::NameError, "undefined variable 'nrows'", None)
            .with_suggestion("did you mean 'nrow'?")
    }

    #[test]
    fn a_suggestion_is_printed_once_with_source_context() {
        let error = errored();
        let rendered = error.format_with_source("println(nrows(t))\n");
        assert_eq!(
            rendered.matches("did you mean 'nrow'?").count(),
            1,
            "rendered twice:\n{rendered}"
        );
    }

    #[test]
    fn a_suggestion_is_printed_once_without_source_context() {
        assert_eq!(
            format!("{}", errored())
                .matches("did you mean 'nrow'?")
                .count(),
            1
        );
    }

    #[test]
    fn the_kind_and_message_survive_both_ways() {
        let error = errored();
        for rendered in [format!("{error}"), error.format_with_source("x\n")] {
            assert!(
                rendered.starts_with("NameError: undefined variable 'nrows'"),
                "lost the header: {rendered}"
            );
        }
    }

    #[test]
    fn several_suggestions_are_each_printed_once() {
        let error = BioLangError::new(ErrorKind::TypeError, "two ways to fix this", None)
            .with_suggestion("first")
            .with_suggestion("second");
        let rendered = error.format_with_source("x\n");
        assert_eq!(rendered.matches("hint: first").count(), 1);
        assert_eq!(rendered.matches("hint: second").count(), 1);
    }

    #[test]
    fn a_derived_hint_still_appears_when_nothing_explicit_was_attached() {
        // With no explicit suggestion the central hints table is consulted, and
        // that path must not have been broken by moving the explicit one.
        let rendered = BioLangError::new(ErrorKind::ArityError, "ttest() expected 2, got 4", None)
            .format_with_source("ttest(a, b, c, d)\n");
        assert!(rendered.contains("hint: "), "no derived hint: {rendered}");
    }
}
