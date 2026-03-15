use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::span::Span;

use crate::token::{Token, TokenKind};

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    tokens: Vec<Token>,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            pos: 0,
            tokens: Vec::new(),
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>> {
        while !self.is_at_end() {
            self.skip_whitespace_not_newline();
            if self.is_at_end() {
                break;
            }

            let ch = self.current();

            match ch {
                '\n' | '\r' => {
                    // Collapse consecutive newlines into one
                    let start = self.pos;
                    while !self.is_at_end() && (self.current() == '\n' || self.current() == '\r') {
                        self.advance();
                    }
                    // Only emit newline if the last token isn't already a newline
                    // and isn't a token that suppresses newlines
                    if !self.tokens.is_empty() && !self.last_suppresses_newline() {
                        self.tokens
                            .push(Token::new(TokenKind::Newline, Span::new(start, self.pos)));
                    }
                }
                '@' => self.single(TokenKind::At),
                '#' => {
                    let start = self.pos;
                    self.advance(); // consume first #
                    if !self.is_at_end() && self.current() == '{' {
                        self.advance();
                        self.tokens.push(Token::new(TokenKind::HashLBrace, Span::new(start, self.pos)));
                    } else if !self.is_at_end() && self.current() == '#' {
                        // Doc comment: ## ...
                        self.advance(); // consume second #
                        // Skip one optional leading space
                        if !self.is_at_end() && self.current() == ' ' {
                            self.advance();
                        }
                        let doc_start = self.pos;
                        while !self.is_at_end() && self.current() != '\n' {
                            self.advance();
                        }
                        let text: String = self.source[doc_start..self.pos].iter().collect();
                        self.tokens.push(Token::new(
                            TokenKind::DocComment(text),
                            Span::new(start, self.pos),
                        ));
                    } else {
                        // Regular line comment
                        while !self.is_at_end() && self.current() != '\n' {
                            self.advance();
                        }
                    }
                }
                '"' => self.string()?,
                '(' => self.single(TokenKind::LParen),
                ')' => self.single(TokenKind::RParen),
                '{' => self.single(TokenKind::LBrace),
                '}' => self.single(TokenKind::RBrace),
                '[' => self.single(TokenKind::LBracket),
                ']' => self.single(TokenKind::RBracket),
                '+' => {
                    let start = self.pos;
                    self.advance();
                    if !self.is_at_end() && self.current() == '+' {
                        self.advance();
                        self.tokens.push(Token::new(TokenKind::PlusPlus, Span::new(start, self.pos)));
                    } else if !self.is_at_end() && self.current() == '=' {
                        self.advance();
                        self.tokens.push(Token::new(TokenKind::PlusEq, Span::new(start, self.pos)));
                    } else {
                        self.tokens.push(Token::new(TokenKind::Plus, Span::new(start, self.pos)));
                    }
                }
                '*' => {
                    let start = self.pos;
                    self.advance();
                    if !self.is_at_end() && self.current() == '*' {
                        self.advance();
                        self.tokens.push(Token::new(TokenKind::StarStar, Span::new(start, self.pos)));
                    } else if !self.is_at_end() && self.current() == '=' {
                        self.advance();
                        self.tokens.push(Token::new(TokenKind::StarEq, Span::new(start, self.pos)));
                    } else {
                        self.tokens.push(Token::new(TokenKind::Star, Span::new(start, self.pos)));
                    }
                }
                '%' => self.single(TokenKind::Percent),
                '?' => {
                    let start = self.pos;
                    self.advance();
                    if !self.is_at_end() && self.current() == '?' {
                        self.advance();
                        self.tokens.push(Token::new(TokenKind::QuestionQuestion, Span::new(start, self.pos)));
                    } else if !self.is_at_end() && self.current() == '.' {
                        self.advance();
                        self.tokens.push(Token::new(TokenKind::QuestionDot, Span::new(start, self.pos)));
                    } else if !self.is_at_end() && self.current() == '=' {
                        self.advance();
                        self.tokens.push(Token::new(TokenKind::QuestionEq, Span::new(start, self.pos)));
                    } else {
                        return Err(BioLangError::new(
                            ErrorKind::UnexpectedChar,
                            "expected '??', '?.' or '?='",
                            Some(Span::new(start, self.pos)),
                        ));
                    }
                }
                '~' => self.single(TokenKind::Tilde),
                ',' => self.single(TokenKind::Comma),
                '.' => {
                    let start = self.pos;
                    self.advance();
                    if !self.is_at_end() && self.current() == '.' {
                        self.advance();
                        if !self.is_at_end() && self.current() == '.' {
                            self.advance();
                            self.tokens.push(Token::new(TokenKind::DotDotDot, Span::new(start, self.pos)));
                        } else if !self.is_at_end() && self.current() == '=' {
                            self.advance();
                            self.tokens.push(Token::new(TokenKind::DotDotEq, Span::new(start, self.pos)));
                        } else {
                            self.tokens.push(Token::new(TokenKind::DotDot, Span::new(start, self.pos)));
                        }
                    } else {
                        self.tokens.push(Token::new(TokenKind::Dot, Span::new(start, self.pos)));
                    }
                }
                ':' => self.single(TokenKind::Colon),
                '-' => {
                    let start = self.pos;
                    self.advance();
                    if !self.is_at_end() && self.current() == '>' {
                        self.advance();
                        self.tokens
                            .push(Token::new(TokenKind::Arrow, Span::new(start, self.pos)));
                    } else if !self.is_at_end() && self.current() == '=' {
                        self.advance();
                        self.tokens
                            .push(Token::new(TokenKind::MinusEq, Span::new(start, self.pos)));
                    } else {
                        self.tokens
                            .push(Token::new(TokenKind::Minus, Span::new(start, self.pos)));
                    }
                }
                '/' => {
                    let start = self.pos;
                    if self.last_is_value_producing() {
                        // Division context
                        self.advance();
                        if !self.is_at_end() && self.current() == '=' {
                            self.advance();
                            self.tokens.push(Token::new(TokenKind::SlashEq, Span::new(start, self.pos)));
                        } else {
                            self.tokens.push(Token::new(TokenKind::Slash, Span::new(start, self.pos)));
                        }
                    } else {
                        // Regex literal context
                        self.regex_literal()?;
                    }
                }
                '|' => {
                    let start = self.pos;
                    self.advance();
                    if !self.is_at_end() && self.current() == '>' {
                        self.advance();
                        if !self.is_at_end() && self.current() == '>' {
                            self.advance();
                            self.tokens
                                .push(Token::new(TokenKind::TapPipe, Span::new(start, self.pos)));
                        } else {
                            self.tokens
                                .push(Token::new(TokenKind::PipeOp, Span::new(start, self.pos)));
                        }
                    } else if !self.is_at_end() && self.current() == '|' {
                        self.advance();
                        self.tokens
                            .push(Token::new(TokenKind::Or, Span::new(start, self.pos)));
                    } else {
                        self.tokens
                            .push(Token::new(TokenKind::Bar, Span::new(start, self.pos)));
                    }
                }
                '&' => {
                    let start = self.pos;
                    self.advance();
                    if !self.is_at_end() && self.current() == '&' {
                        self.advance();
                        self.tokens
                            .push(Token::new(TokenKind::And, Span::new(start, self.pos)));
                    } else {
                        self.tokens
                            .push(Token::new(TokenKind::Amp, Span::new(start, self.pos)));
                    }
                }
                '^' => self.single(TokenKind::Caret),
                '=' => {
                    let start = self.pos;
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        self.advance();
                        self.tokens
                            .push(Token::new(TokenKind::EqEq, Span::new(start, self.pos)));
                    } else if !self.is_at_end() && self.current() == '>' {
                        self.advance();
                        self.tokens
                            .push(Token::new(TokenKind::FatArrow, Span::new(start, self.pos)));
                    } else {
                        self.tokens
                            .push(Token::new(TokenKind::Eq, Span::new(start, self.pos)));
                    }
                }
                '!' => {
                    let start = self.pos;
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        self.advance();
                        self.tokens
                            .push(Token::new(TokenKind::Neq, Span::new(start, self.pos)));
                    } else {
                        self.tokens
                            .push(Token::new(TokenKind::Bang, Span::new(start, self.pos)));
                    }
                }
                '<' => {
                    let start = self.pos;
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        self.advance();
                        self.tokens
                            .push(Token::new(TokenKind::Le, Span::new(start, self.pos)));
                    } else if !self.is_at_end() && self.current() == '<' {
                        self.advance();
                        self.tokens
                            .push(Token::new(TokenKind::Shl, Span::new(start, self.pos)));
                    } else {
                        self.tokens
                            .push(Token::new(TokenKind::Lt, Span::new(start, self.pos)));
                    }
                }
                '>' => {
                    let start = self.pos;
                    self.advance();
                    if !self.is_at_end() && self.current() == '=' {
                        self.advance();
                        self.tokens
                            .push(Token::new(TokenKind::Ge, Span::new(start, self.pos)));
                    } else if !self.is_at_end() && self.current() == '>' {
                        self.advance();
                        self.tokens
                            .push(Token::new(TokenKind::Shr, Span::new(start, self.pos)));
                    } else {
                        self.tokens
                            .push(Token::new(TokenKind::Gt, Span::new(start, self.pos)));
                    }
                }
                _ if ch.is_ascii_digit() => self.number()?,
                _ if ch.is_ascii_alphabetic() || ch == '_' => self.identifier()?,
                _ => {
                    let start = self.pos;
                    self.advance();
                    return Err(BioLangError::new(
                        ErrorKind::UnexpectedChar,
                        format!("unexpected character: '{ch}'"),
                        Some(Span::new(start, self.pos)),
                    ));
                }
            }
        }

        self.tokens
            .push(Token::new(TokenKind::Eof, Span::new(self.pos, self.pos)));

        // Post-processing: remove newlines before pipe operators so that
        //   expr\n  |> f()   is treated as   expr |> f()
        let mut i = 0;
        while i < self.tokens.len() {
            if self.tokens[i].kind == TokenKind::Newline {
                // Check if next non-newline token is a pipe operator
                let mut j = i + 1;
                while j < self.tokens.len() && self.tokens[j].kind == TokenKind::Newline {
                    j += 1;
                }
                if j < self.tokens.len()
                    && matches!(self.tokens[j].kind, TokenKind::PipeOp | TokenKind::TapPipe)
                {
                    // Remove all newlines from i..j
                    self.tokens.drain(i..j);
                    continue; // re-check at same index
                }
            }
            i += 1;
        }

        Ok(self.tokens)
    }

    /// Whether the previous token could produce a value (used for regex vs division disambiguation).
    fn last_is_value_producing(&self) -> bool {
        matches!(
            self.tokens.last().map(|t| &t.kind),
            Some(
                TokenKind::Int(_)
                    | TokenKind::Float(_)
                    | TokenKind::Str(_)
                    | TokenKind::Ident(_)
                    | TokenKind::RParen
                    | TokenKind::RBracket
                    | TokenKind::RBrace
                    | TokenKind::True
                    | TokenKind::False
                    | TokenKind::Nil
                    | TokenKind::DnaLit(_)
                    | TokenKind::RnaLit(_)
                    | TokenKind::ProteinLit(_)
                    | TokenKind::FStr(_)
                    | TokenKind::RegexLit(_, _)
            )
        )
    }

    fn regex_literal(&mut self) -> Result<()> {
        let start = self.pos;
        self.advance(); // consume opening /
        let mut pattern = String::new();
        loop {
            if self.is_at_end() || self.current() == '\n' {
                return Err(BioLangError::new(
                    ErrorKind::UnterminatedString,
                    "unterminated regex literal",
                    Some(Span::new(start, self.pos)),
                ));
            }
            if self.current() == '/' {
                self.advance(); // consume closing /
                break;
            }
            if self.current() == '\\' {
                pattern.push(self.current());
                self.advance();
                if !self.is_at_end() && self.current() != '\n' {
                    pattern.push(self.current());
                    self.advance();
                }
            } else {
                pattern.push(self.current());
                self.advance();
            }
        }
        // Collect optional flags: i, g, m, s
        let mut flags = String::new();
        while !self.is_at_end() && matches!(self.current(), 'i' | 'g' | 'm' | 's') {
            flags.push(self.current());
            self.advance();
        }
        self.tokens.push(Token::new(
            TokenKind::RegexLit(pattern, flags),
            Span::new(start, self.pos),
        ));
        Ok(())
    }

    fn last_suppresses_newline(&self) -> bool {
        matches!(
            self.tokens.last().map(|t| &t.kind),
            Some(
                TokenKind::PipeOp
                    | TokenKind::TapPipe
                    | TokenKind::Comma
                    | TokenKind::LParen
                    | TokenKind::LBrace
                    | TokenKind::LBracket
                    | TokenKind::Newline
                    | TokenKind::Plus
                    | TokenKind::PlusPlus
                    | TokenKind::Minus
                    | TokenKind::Star
                    | TokenKind::StarStar
                    | TokenKind::Slash
                    | TokenKind::Percent
                    | TokenKind::Amp
                    | TokenKind::Caret
                    | TokenKind::Shl
                    | TokenKind::Shr
                    | TokenKind::PlusEq
                    | TokenKind::MinusEq
                    | TokenKind::StarEq
                    | TokenKind::SlashEq
                    | TokenKind::QuestionQuestion
                    | TokenKind::EqEq
                    | TokenKind::Neq
                    | TokenKind::Lt
                    | TokenKind::Gt
                    | TokenKind::Le
                    | TokenKind::Ge
                    | TokenKind::And
                    | TokenKind::Or
                    | TokenKind::Eq
                    | TokenKind::Arrow
                    | TokenKind::FatArrow
                    | TokenKind::Tilde
                    | TokenKind::Colon
                    | TokenKind::DotDot
                    | TokenKind::DotDotEq
                    | TokenKind::DotDotDot
                    | TokenKind::QuestionDot
                    | TokenKind::HashLBrace
                    | TokenKind::At
                    | TokenKind::Bar
                    | TokenKind::Else
                    | TokenKind::Do
                    | TokenKind::When,
            ) | None
        )
    }

    fn current(&self) -> char {
        self.source[self.pos]
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> char {
        let ch = self.source[self.pos];
        self.pos += 1;
        ch
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn skip_whitespace_not_newline(&mut self) {
        while !self.is_at_end() {
            let ch = self.current();
            if ch == ' ' || ch == '\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn single(&mut self, kind: TokenKind) {
        let start = self.pos;
        self.advance();
        self.tokens.push(Token::new(kind, Span::new(start, self.pos)));
    }

    fn string(&mut self) -> Result<()> {
        let start = self.pos;
        self.advance(); // consume opening "

        // Check for triple-quote: """..."""
        if !self.is_at_end() && self.current() == '"' {
            if self.peek() == Some('"') {
                // Triple-quoted string
                self.advance(); // consume second "
                self.advance(); // consume third "
                return self.triple_string(start);
            } else {
                // Empty string ""
                self.advance(); // consume closing "
                self.tokens
                    .push(Token::new(TokenKind::Str(String::new()), Span::new(start, self.pos)));
                return Ok(());
            }
        }

        let mut value = String::new();
        while !self.is_at_end() && self.current() != '"' {
            if self.current() == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(BioLangError::new(
                        ErrorKind::UnterminatedString,
                        "unterminated string",
                        Some(Span::new(start, self.pos)),
                    ));
                }
                self.parse_escape_char(&mut value, start)?;
            } else if self.current() == '\n' {
                return Err(BioLangError::new(
                    ErrorKind::UnterminatedString,
                    "unterminated string (newline in string)",
                    Some(Span::new(start, self.pos)),
                ));
            } else {
                value.push(self.current());
            }
            self.advance();
        }

        if self.is_at_end() {
            return Err(BioLangError::new(
                ErrorKind::UnterminatedString,
                "unterminated string",
                Some(Span::new(start, self.pos)),
            ));
        }

        self.advance(); // consume closing "
        self.tokens
            .push(Token::new(TokenKind::Str(value), Span::new(start, self.pos)));
        Ok(())
    }

    fn triple_string(&mut self, start: usize) -> Result<()> {
        // Skip optional leading newline after opening """
        if !self.is_at_end() && self.current() == '\n' {
            self.advance();
        } else if !self.is_at_end() && self.current() == '\r' {
            self.advance();
            if !self.is_at_end() && self.current() == '\n' {
                self.advance();
            }
        }

        let mut value = String::new();
        loop {
            if self.is_at_end() {
                return Err(BioLangError::new(
                    ErrorKind::UnterminatedString,
                    "unterminated triple-quoted string",
                    Some(Span::new(start, self.pos)),
                ));
            }
            // Check for closing """
            if self.current() == '"'
                && self.peek() == Some('"')
                && self.source.get(self.pos + 2).copied() == Some('"')
            {
                self.advance(); // "
                self.advance(); // "
                self.advance(); // "
                break;
            }
            if self.current() == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(BioLangError::new(
                        ErrorKind::UnterminatedString,
                        "unterminated triple-quoted string",
                        Some(Span::new(start, self.pos)),
                    ));
                }
                self.parse_escape_char(&mut value, start)?;
            } else {
                value.push(self.current());
            }
            self.advance();
        }

        // Dedent: strip common leading whitespace (Python-style)
        let dedented = self.dedent_string(&value);

        self.tokens
            .push(Token::new(TokenKind::Str(dedented), Span::new(start, self.pos)));
        Ok(())
    }

    fn dedent_string(&self, s: &str) -> String {
        let lines: Vec<&str> = s.lines().collect();
        if lines.is_empty() {
            return String::new();
        }
        // Find minimum indentation of non-empty lines
        let min_indent = lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.len() - line.trim_start().len())
            .min()
            .unwrap_or(0);
        // Strip common prefix and rejoin
        let result: Vec<&str> = lines
            .iter()
            .map(|line| {
                if line.len() >= min_indent {
                    &line[min_indent..]
                } else {
                    line.trim()
                }
            })
            .collect();
        // Remove trailing empty line (from newline before closing """)
        let mut result = result.join("\n");
        if result.ends_with('\n') {
            result.pop();
        }
        result
    }

    /// Parse an escape character after consuming `\`. The current char is the escape specifier.
    /// After this returns, `self.pos` still points at the last consumed char of the escape
    /// (the caller will `advance()` past it).
    fn parse_escape_char(&mut self, value: &mut String, _string_start: usize) -> Result<()> {
        match self.current() {
            'n' => value.push('\n'),
            't' => value.push('\t'),
            'r' => value.push('\r'),
            '0' => value.push('\0'),
            '\\' => value.push('\\'),
            '"' => value.push('"'),
            'u' => {
                let esc_start = self.pos - 1; // position of the backslash
                self.advance(); // consume 'u'
                if self.is_at_end() || self.current() != '{' {
                    return Err(BioLangError::new(
                        ErrorKind::InvalidEscape,
                        "expected '{' after \\u",
                        Some(Span::new(esc_start, self.pos)),
                    ));
                }
                self.advance(); // consume '{'
                let hex_start = self.pos;
                let mut hex_len = 0;
                while !self.is_at_end() && self.current() != '}' && hex_len < 6 {
                    if !self.current().is_ascii_hexdigit() {
                        return Err(BioLangError::new(
                            ErrorKind::InvalidEscape,
                            format!("invalid hex digit '{}' in unicode escape", self.current()),
                            Some(Span::new(self.pos, self.pos + 1)),
                        ));
                    }
                    self.advance();
                    hex_len += 1;
                }
                if hex_len == 0 {
                    return Err(BioLangError::new(
                        ErrorKind::InvalidEscape,
                        "empty unicode escape \\u{}",
                        Some(Span::new(esc_start, self.pos)),
                    ));
                }
                if self.is_at_end() || self.current() != '}' {
                    return Err(BioLangError::new(
                        ErrorKind::InvalidEscape,
                        "expected '}' to close unicode escape",
                        Some(Span::new(esc_start, self.pos)),
                    ));
                }
                let hex_str: String = self.source[hex_start..self.pos].iter().collect();
                let code_point = u32::from_str_radix(&hex_str, 16).map_err(|_| {
                    BioLangError::new(
                        ErrorKind::InvalidEscape,
                        format!("invalid unicode escape: \\u{{{hex_str}}}"),
                        Some(Span::new(esc_start, self.pos + 1)),
                    )
                })?;
                let ch = char::from_u32(code_point).ok_or_else(|| {
                    BioLangError::new(
                        ErrorKind::InvalidEscape,
                        format!("invalid unicode code point: U+{code_point:04X}"),
                        Some(Span::new(esc_start, self.pos + 1)),
                    )
                })?;
                value.push(ch);
                // self.pos is now on '}', caller will advance past it
            }
            other => {
                value.push('\\');
                value.push(other);
            }
        }
        Ok(())
    }

    fn number(&mut self) -> Result<()> {
        let start = self.pos;
        let mut is_float = false;

        while !self.is_at_end() && (self.current().is_ascii_digit() || self.current() == '_') {
            self.advance();
        }

        // Check for decimal point
        if !self.is_at_end() && self.current() == '.' {
            if let Some(next) = self.peek() {
                if next.is_ascii_digit() {
                    is_float = true;
                    self.advance(); // consume '.'
                    while !self.is_at_end()
                        && (self.current().is_ascii_digit() || self.current() == '_')
                    {
                        self.advance();
                    }
                }
            }
        }

        // Check for scientific notation
        if !self.is_at_end() && (self.current() == 'e' || self.current() == 'E') {
            is_float = true;
            self.advance();
            if !self.is_at_end() && (self.current() == '+' || self.current() == '-') {
                self.advance();
            }
            while !self.is_at_end() && self.current().is_ascii_digit() {
                self.advance();
            }
        }

        let text: String = self.source[start..self.pos]
            .iter()
            .filter(|c| **c != '_')
            .collect();
        let span = Span::new(start, self.pos);

        if is_float {
            let value: f64 = text.parse().map_err(|_| {
                BioLangError::new(ErrorKind::InvalidNumber, format!("invalid float: {text}"), Some(span))
            })?;
            self.tokens.push(Token::new(TokenKind::Float(value), span));
        } else {
            let value: i64 = text.parse().map_err(|_| {
                BioLangError::new(ErrorKind::InvalidNumber, format!("invalid integer: {text}"), Some(span))
            })?;
            self.tokens.push(Token::new(TokenKind::Int(value), span));
        }
        Ok(())
    }

    fn identifier(&mut self) -> Result<()> {
        let start = self.pos;
        while !self.is_at_end() && (self.current().is_ascii_alphanumeric() || self.current() == '_')
        {
            self.advance();
        }

        let text: String = self.source[start..self.pos].iter().collect();
        let span = Span::new(start, self.pos);

        // Check for bio literals: dna"...", rna"...", protein"..."
        // Check for f-string: f"..."
        // Check for raw string: r"..." (no escape processing, useful for Windows paths)
        if !self.is_at_end() && self.current() == '"' {
            match text.as_str() {
                "dna" => return self.bio_literal(start, TokenKind::DnaLit),
                "rna" => return self.bio_literal(start, TokenKind::RnaLit),
                "protein" => return self.bio_literal(start, TokenKind::ProteinLit),
                "qual" => return self.bio_literal(start, TokenKind::QualLit),
                "f" => return self.fstring_literal(start),
                "r" => return self.raw_string(start),
                _ => {}
            }
        }

        // Check for keyword
        if let Some(keyword) = TokenKind::keyword_from_str(&text) {
            self.tokens.push(Token::new(keyword, span));
        } else {
            self.tokens.push(Token::new(TokenKind::Ident(text), span));
        }
        Ok(())
    }

    fn fstring_literal(&mut self, start: usize) -> Result<()> {
        self.advance(); // consume opening "
        let mut value = String::new();
        let mut depth = 0;

        while !self.is_at_end() && (self.current() != '"' || depth > 0) {
            if self.current() == '{' {
                depth += 1;
                value.push('{');
            } else if self.current() == '}' {
                if depth > 0 {
                    depth -= 1;
                }
                value.push('}');
            } else if self.current() == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(BioLangError::new(
                        ErrorKind::UnterminatedString,
                        "unterminated f-string",
                        Some(Span::new(start, self.pos)),
                    ));
                }
                match self.current() {
                    '{' => value.push('{'),
                    '}' => value.push('}'),
                    _ => self.parse_escape_char(&mut value, start)?,
                }
            } else if self.current() == '\n' {
                return Err(BioLangError::new(
                    ErrorKind::UnterminatedString,
                    "unterminated f-string (newline)",
                    Some(Span::new(start, self.pos)),
                ));
            } else {
                value.push(self.current());
            }
            self.advance();
        }

        if self.is_at_end() {
            return Err(BioLangError::new(
                ErrorKind::UnterminatedString,
                "unterminated f-string",
                Some(Span::new(start, self.pos)),
            ));
        }

        self.advance(); // consume closing "
        self.tokens
            .push(Token::new(TokenKind::FStr(value), Span::new(start, self.pos)));
        Ok(())
    }

    fn raw_string(&mut self, start: usize) -> Result<()> {
        self.advance(); // consume opening "
        let mut value = String::new();
        while !self.is_at_end() && self.current() != '"' {
            if self.current() == '\n' {
                return Err(BioLangError::new(
                    ErrorKind::UnterminatedString,
                    "unterminated raw string (newline)",
                    Some(Span::new(start, self.pos)),
                ));
            }
            value.push(self.current());
            self.advance();
        }
        if self.is_at_end() {
            return Err(BioLangError::new(
                ErrorKind::UnterminatedString,
                "unterminated raw string",
                Some(Span::new(start, self.pos)),
            ));
        }
        self.advance(); // consume closing "
        self.tokens
            .push(Token::new(TokenKind::Str(value), Span::new(start, self.pos)));
        Ok(())
    }

    fn bio_literal(&mut self, start: usize, make_token: fn(String) -> TokenKind) -> Result<()> {
        self.advance(); // consume opening "
        let mut value = String::new();

        while !self.is_at_end() && self.current() != '"' {
            if self.current() == '\n' {
                return Err(BioLangError::new(
                    ErrorKind::UnterminatedString,
                    "unterminated bio literal",
                    Some(Span::new(start, self.pos)),
                ));
            }
            value.push(self.current());
            self.advance();
        }

        if self.is_at_end() {
            return Err(BioLangError::new(
                ErrorKind::UnterminatedString,
                "unterminated bio literal",
                Some(Span::new(start, self.pos)),
            ));
        }

        self.advance(); // consume closing "

        // Validate bio literal characters
        let kind_tag = make_token("".into());
        let validation: Option<(&str, Box<dyn Fn(char) -> bool>)> = match &kind_tag {
            TokenKind::DnaLit(_) => Some((
                "DNA",
                Box::new(|c: char| "ACGTNRYWSMKBDHVacgtnrywsmkbdhv".contains(c)),
            )),
            TokenKind::RnaLit(_) => Some((
                "RNA",
                Box::new(|c: char| "ACGUNRYWSMKBDHVacgunrywsmkbdhv".contains(c)),
            )),
            TokenKind::ProteinLit(_) => Some((
                "protein",
                Box::new(|c: char| "ACDEFGHIKLMNPQRSTVWYXBZJUOacdefghiklmnpqrstvwyxbzjuo*".contains(c)),
            )),
            TokenKind::QualLit(_) => Some((
                "quality",
                Box::new(|c: char| c.is_ascii() && (c as u8) >= 33 && (c as u8) <= 126),
            )),
            _ => None,
        };

        if let Some((kind, is_valid)) = validation {
            for (i, ch) in value.chars().enumerate() {
                if !is_valid(ch) {
                    let msg = format!(
                        "invalid character '{}' at position {} in {} literal",
                        ch, i + 1, kind
                    );
                    return Err(BioLangError::new(
                        ErrorKind::UnexpectedChar,
                        &msg,
                        Some(Span::new(start, self.pos)),
                    ));
                }
            }
        }

        self.tokens
            .push(Token::new(make_token(value), Span::new(start, self.pos)));
        Ok(())
    }
}
