use bl_core::ast::*;
use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::span::{Span, Spanned};
use bl_lexer::token::{Token, TokenKind};

/// Result of parsing: contains the (possibly partial) program and any collected errors.
pub struct ParseResult {
    pub program: Program,
    pub errors: Vec<BioLangError>,
}

impl ParseResult {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<BioLangError>,
}

/// Operator precedence levels (higher = tighter binding).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    None,
    Pipe,           // |>
    NullCoalesce,   // ??
    Or,             // ||
    And,            // &&
    BitXor,         // ^
    BitAnd,         // &
    Equality,       // == != in
    Comparison,     // < > <= >=
    Shift,          // << >>
    TypeCast,       // as
    Range,          // .. ..=
    Addition,       // + -
    Multiply,       // * / %
    Power,          // **
    Unary,          // - ! not
    Call,           // () . []
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0, errors: Vec::new() }
    }

    pub fn parse(mut self) -> Result<ParseResult> {
        let mut stmts = Vec::new();
        self.skip_newlines();

        while !self.is_at_end() {
            let before = self.pos;
            match self.parse_stmt() {
                Ok(stmt) => stmts.push(stmt),
                Err(e) => {
                    self.errors.push(e);
                    self.synchronize();
                }
            }
            self.skip_newlines();
            // Safety: if no progress was made, force advance to prevent infinite loop
            if self.pos == before && !self.is_at_end() {
                self.advance();
            }
        }

        Ok(ParseResult {
            program: Program { stmts },
            errors: self.errors,
        })
    }

    /// Advance past tokens until we reach a synchronization point,
    /// so parsing can continue after an error.
    fn synchronize(&mut self) {
        while !self.is_at_end() {
            // If the previous token was a newline, we are synced
            if matches!(
                self.tokens.get(self.pos.saturating_sub(1)).map(|t| &t.kind),
                Some(TokenKind::Newline)
            ) {
                return;
            }
            match self.current_kind() {
                TokenKind::Fn
                | TokenKind::Let
                | TokenKind::Const
                | TokenKind::With
                | TokenKind::For
                | TokenKind::While
                | TokenKind::Import
                | TokenKind::From
                | TokenKind::Return
                | TokenKind::Enum
                | TokenKind::Struct
                | TokenKind::Trait
                | TokenKind::Impl
                | TokenKind::Given => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    // ── Statement parsing ──────────────────────────────────────────────

    fn parse_stmt(&mut self) -> Result<Spanned<Stmt>> {
        match self.current_kind() {
            TokenKind::Let => {
                // Check for destructuring: let [ or let {ident,
                if self.is_destruct_let() {
                    self.parse_destruct_let()
                } else {
                    self.parse_let()
                }
            }
            TokenKind::Const => self.parse_const(),
            TokenKind::With => self.parse_with(),
            TokenKind::DocComment(_) => {
                // Collect doc comments, then expect fn
                return self.parse_doc_fn();
            }
            TokenKind::At => return self.parse_decorated_fn(),
            TokenKind::Async => return self.parse_async_fn(),
            TokenKind::Fn => self.parse_fn_with_doc(None, Vec::new(), false),
            TokenKind::Struct => return self.parse_struct(),
            TokenKind::Trait => return self.parse_trait(),
            TokenKind::Impl => return self.parse_impl(),
            TokenKind::Yield => self.parse_yield(),
            TokenKind::Enum => self.parse_enum(),
            TokenKind::Return => self.parse_return(),
            TokenKind::For => self.parse_for(),
            TokenKind::While => self.parse_while(),
            TokenKind::Break => self.parse_break(),
            TokenKind::Continue => self.parse_continue(),
            TokenKind::Assert => self.parse_assert(),
            TokenKind::Pipeline => self.parse_pipeline(),
            TokenKind::Import => self.parse_import(),
            TokenKind::From => self.parse_from_import(),
            TokenKind::Unless => self.parse_unless(),
            TokenKind::Guard => self.parse_guard(),
            TokenKind::Defer => self.parse_defer(),
            TokenKind::Parallel => self.parse_parallel_for(),
            TokenKind::Stage => self.parse_stage(),
            TokenKind::Ident(s) if s == "type" && self.is_type_alias() => self.parse_type_alias(),
            TokenKind::Ident(_) if self.is_nil_assign() => self.parse_nil_assign(),
            TokenKind::Ident(_) if self.is_compound_assignment() => self.parse_compound_assign(),
            TokenKind::Ident(_) if self.is_assignment() => self.parse_assign(),
            _ => self.parse_expr_stmt(),
        }
    }

    /// Check if current position is `type Name = ...` (type alias).
    fn is_type_alias(&self) -> bool {
        self.pos + 2 < self.tokens.len()
            && matches!(self.tokens[self.pos + 1].kind, TokenKind::Ident(_))
            && matches!(self.tokens[self.pos + 2].kind, TokenKind::Eq)
    }

    /// Check if current position is `ident ?=`
    fn is_nil_assign(&self) -> bool {
        self.pos + 1 < self.tokens.len()
            && matches!(self.tokens[self.pos + 1].kind, TokenKind::QuestionEq)
    }

    fn parse_nil_assign(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        let name = self.expect_ident()?;
        self.expect(TokenKind::QuestionEq)?;
        let value = self.parse_expr()?;
        let span = start.merge(value.span);
        Ok(Spanned::new(Stmt::NilAssign { name, value }, span))
    }

    /// Check if current position is `ident =` (assignment, not `==`).
    fn is_assignment(&self) -> bool {
        self.pos + 1 < self.tokens.len()
            && matches!(self.tokens[self.pos + 1].kind, TokenKind::Eq)
    }

    fn parse_assign(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        let name = self.expect_ident()?;
        self.expect(TokenKind::Eq)?;
        let value = self.parse_expr()?;
        let span = start.merge(value.span);
        Ok(Spanned::new(Stmt::Assign { name, value }, span))
    }

    /// Check if current position is `ident +=` / `-=` / `*=` / `/=`
    fn is_compound_assignment(&self) -> bool {
        self.pos + 1 < self.tokens.len()
            && matches!(
                self.tokens[self.pos + 1].kind,
                TokenKind::PlusEq | TokenKind::MinusEq | TokenKind::StarEq | TokenKind::SlashEq
            )
    }

    /// Desugar `x += expr` into `x = x + expr`
    fn parse_compound_assign(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        let name = self.expect_ident()?;
        let op = match self.current_kind() {
            TokenKind::PlusEq => BinaryOp::Add,
            TokenKind::MinusEq => BinaryOp::Sub,
            TokenKind::StarEq => BinaryOp::Mul,
            TokenKind::SlashEq => BinaryOp::Div,
            _ => unreachable!(),
        };
        self.advance(); // consume operator
        let rhs = self.parse_expr()?;
        let rhs_span = rhs.span;
        // Desugar: x += e  →  x = x + e
        let ident_expr = Spanned::new(Expr::Ident(name.clone()), start);
        let binary = Spanned::new(
            Expr::Binary {
                op,
                left: Box::new(ident_expr),
                right: Box::new(rhs),
            },
            start.merge(rhs_span),
        );
        Ok(Spanned::new(
            Stmt::Assign {
                name,
                value: binary,
            },
            start.merge(rhs_span),
        ))
    }

    fn parse_while(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::While)?;
        let condition = self.parse_expr()?;
        self.expect(TokenKind::LBrace)?;
        let body = self.parse_block_body()?;
        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;
        Ok(Spanned::new(
            Stmt::While { condition, body },
            start.merge(end),
        ))
    }

    fn parse_break(&mut self) -> Result<Spanned<Stmt>> {
        let span = self.current_span();
        self.expect(TokenKind::Break)?;
        Ok(Spanned::new(Stmt::Break, span))
    }

    fn parse_continue(&mut self) -> Result<Spanned<Stmt>> {
        let span = self.current_span();
        self.expect(TokenKind::Continue)?;
        Ok(Spanned::new(Stmt::Continue, span))
    }

    /// Check for destructuring let: `let [` or `let {ident,`
    fn is_destruct_let(&self) -> bool {
        // let [
        if self.pos + 1 < self.tokens.len()
            && matches!(self.tokens[self.pos + 1].kind, TokenKind::LBracket)
        {
            return true;
        }
        // let {ident, ...} — must distinguish from let x = {record}
        // We detect: let { ident , or let { ident }
        if self.pos + 3 < self.tokens.len()
            && matches!(self.tokens[self.pos + 1].kind, TokenKind::LBrace)
            && matches!(self.tokens[self.pos + 2].kind, TokenKind::Ident(_))
            && matches!(
                self.tokens[self.pos + 3].kind,
                TokenKind::Comma | TokenKind::RBrace
            )
        {
            return true;
        }
        false
    }

    fn parse_destruct_let(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Let)?;

        let pattern = if self.check(&TokenKind::LBracket) {
            // let [a, b, c] = ... or let [a, b, ...rest] = ...
            self.advance();
            let mut names = Vec::new();
            let mut rest_name = None;
            while !self.check(&TokenKind::RBracket) && !self.is_at_end() {
                if self.check(&TokenKind::DotDotDot) {
                    self.advance(); // consume ...
                    rest_name = Some(self.expect_ident()?);
                    break;
                }
                names.push(self.expect_ident()?);
                if self.check(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(TokenKind::RBracket)?;
            if let Some(rest) = rest_name {
                DestructPattern::ListWithRest { elements: names, rest_name: rest }
            } else {
                DestructPattern::List(names)
            }
        } else {
            // let {x, y} = ... or let {x, y, ...rest} = ...
            self.expect(TokenKind::LBrace)?;
            let mut names = Vec::new();
            let mut rest_name = None;
            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                if self.check(&TokenKind::DotDotDot) {
                    self.advance(); // consume ...
                    rest_name = Some(self.expect_ident()?);
                    break;
                }
                names.push(self.expect_ident()?);
                if self.check(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(TokenKind::RBrace)?;
            if let Some(rest) = rest_name {
                DestructPattern::RecordWithRest { fields: names, rest_name: rest }
            } else {
                DestructPattern::Record(names)
            }
        };

        self.expect(TokenKind::Eq)?;
        let value = self.parse_expr()?;
        let span = start.merge(value.span);
        Ok(Spanned::new(Stmt::DestructLet { pattern, value }, span))
    }

    fn parse_let(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Let)?;
        let name = self.expect_ident()?;

        let type_ann = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        self.expect(TokenKind::Eq)?;
        let value = self.parse_expr()?;
        let span = start.merge(value.span);

        Ok(Spanned::new(Stmt::Let { name, type_ann, value }, span))
    }

    fn parse_const(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Const)?;
        let name = self.expect_ident()?;

        let type_ann = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        self.expect(TokenKind::Eq)?;
        let value = self.parse_expr()?;
        let span = start.merge(value.span);

        Ok(Spanned::new(Stmt::Const { name, type_ann, value }, span))
    }

    fn parse_with(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::With)?;
        let expr = self.parse_expr()?;
        self.expect(TokenKind::LBrace)?;
        let body = self.parse_block_body()?;
        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;
        Ok(Spanned::new(
            Stmt::With { expr, body },
            start.merge(end),
        ))
    }

    fn parse_doc_fn(&mut self) -> Result<Spanned<Stmt>> {
        let mut doc_lines = Vec::new();
        while let TokenKind::DocComment(text) = self.current_kind() {
            doc_lines.push(text.clone());
            self.advance();
            self.skip_newlines();
        }
        let doc = if doc_lines.is_empty() {
            None
        } else {
            Some(doc_lines.join("\n"))
        };
        self.parse_fn_with_doc(doc, Vec::new(), false)
    }

    fn parse_decorated_fn(&mut self) -> Result<Spanned<Stmt>> {
        let mut decorators = Vec::new();
        while self.check(&TokenKind::At) {
            self.advance(); // @
            let name = self.expect_ident()?;
            decorators.push(name);
            self.skip_newlines();
        }
        let is_async = if self.check(&TokenKind::Async) {
            self.advance();
            true
        } else {
            false
        };
        self.parse_fn_with_doc(None, decorators, is_async)
    }

    fn parse_async_fn(&mut self) -> Result<Spanned<Stmt>> {
        self.advance(); // consume 'async'
        self.parse_fn_with_doc(None, Vec::new(), true)
    }

    fn parse_fn_with_doc(&mut self, doc: Option<String>, decorators: Vec<String>, is_async: bool) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Fn)?;

        // Check for generator: fn* name(...)
        let is_generator = if self.check(&TokenKind::Star) {
            self.advance();
            true
        } else {
            false
        };

        let name = self.expect_ident()?;

        self.expect(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen)?;

        let mut return_type = None;
        let mut named_returns = Vec::new();

        if self.check(&TokenKind::Arrow) {
            self.advance();
            // Check for named tuple return: -> (name: Type, name2: Type)
            if self.check(&TokenKind::LParen) && self.is_named_tuple_return() {
                self.advance(); // consume (
                while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                    let field_name = self.expect_ident()?;
                    self.expect(TokenKind::Colon)?;
                    let field_type = self.parse_type_annotation()?;
                    named_returns.push((field_name, field_type));
                    if self.check(&TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(TokenKind::RParen)?;
            } else {
                return_type = Some(self.parse_type_annotation()?);
            }
        }

        // Parse optional where clause: `where expr`
        let where_clause = if self.check(&TokenKind::Where) {
            self.advance(); // consume 'where'
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.expect(TokenKind::LBrace)?;
        let body = self.parse_block_body()?;
        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;

        Ok(Spanned::new(
            Stmt::Fn {
                name,
                params,
                return_type,
                body,
                doc,
                is_generator,
                decorators,
                is_async,
                named_returns,
                where_clause,
            },
            start.merge(end),
        ))
    }

    fn parse_struct(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Struct)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let field_name = self.expect_ident()?;
            let type_ann = if self.check(&TokenKind::Colon) {
                self.advance();
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            let default = if self.check(&TokenKind::Eq) {
                self.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };
            fields.push(StructField { name: field_name, type_ann, default });
            self.skip_newlines();
            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            } else {
                self.skip_newlines();
            }
        }

        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;
        Ok(Spanned::new(Stmt::Struct { name, fields }, start.merge(end)))
    }

    fn parse_trait(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Trait)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            self.expect(TokenKind::Fn)?;
            let method_name = self.expect_ident()?;
            self.expect(TokenKind::LParen)?;
            let params = self.parse_params()?;
            self.expect(TokenKind::RParen)?;
            methods.push(TraitMethod { name: method_name, params });
            self.skip_newlines();
        }

        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;
        Ok(Spanned::new(Stmt::Trait { name, methods }, start.merge(end)))
    }

    fn parse_impl(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Impl)?;
        let first_name = self.expect_ident()?;

        // Check for `impl Trait for Type` vs `impl Type`
        let (trait_name, type_name) = if let TokenKind::Ident(s) = self.current_kind() {
            if s == "for" {
                self.advance(); // consume "for"
                let tn = self.expect_ident()?;
                (Some(first_name), tn)
            } else {
                (None, first_name)
            }
        } else {
            (None, first_name)
        };

        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let stmt = self.parse_fn_with_doc(None, Vec::new(), false)?;
            methods.push(stmt);
            self.skip_newlines();
        }

        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;
        Ok(Spanned::new(
            Stmt::Impl { type_name, trait_name, methods },
            start.merge(end),
        ))
    }

    fn parse_yield(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Yield)?;
        let expr = self.parse_expr()?;
        let span = start.merge(expr.span);
        Ok(Spanned::new(Stmt::Yield(expr), span))
    }

    fn parse_enum(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Enum)?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut variants = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let variant_name = self.expect_ident()?;
            let fields = if self.check(&TokenKind::LParen) {
                self.advance();
                let mut fields = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                    fields.push(self.expect_ident()?);
                    if self.check(&TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(TokenKind::RParen)?;
                fields
            } else {
                Vec::new()
            };
            variants.push(EnumVariant { name: variant_name, fields });
            self.skip_newlines();
            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            } else {
                self.skip_newlines();
            }
        }

        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;
        Ok(Spanned::new(Stmt::Enum { name, variants }, start.merge(end)))
    }

    fn parse_return(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Return)?;

        let value = if self.is_at_line_end() {
            None
        } else {
            Some(self.parse_expr()?)
        };

        let span = match &value {
            Some(v) => start.merge(v.span),
            None => start,
        };

        Ok(Spanned::new(Stmt::Return(value), span))
    }

    fn parse_for(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::For)?;

        // Parse pattern: `x`, `[a, b]`, or `{x, y}`
        let pattern = if self.check(&TokenKind::LBracket) {
            self.advance();
            let mut names = Vec::new();
            loop {
                names.push(self.expect_ident()?);
                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
            self.expect(TokenKind::RBracket)?;
            ForPattern::ListDestr(names)
        } else if self.check(&TokenKind::LBrace) {
            self.advance();
            let mut names = Vec::new();
            loop {
                names.push(self.expect_ident()?);
                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
            self.expect(TokenKind::RBrace)?;
            ForPattern::RecordDestr(names)
        } else if self.check(&TokenKind::LParen) {
            self.advance();
            let mut names = Vec::new();
            loop {
                names.push(self.expect_ident()?);
                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
            self.expect(TokenKind::RParen)?;
            ForPattern::TupleDestr(names)
        } else {
            ForPattern::Single(self.expect_ident()?)
        };

        self.expect(TokenKind::In)?;
        let iter = self.parse_expr()?;
        // Optional `when` guard: `for x in list when x > 0 { ... }`
        let when_guard = if matches!(self.current_kind(), TokenKind::When) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect(TokenKind::LBrace)?;
        let body = self.parse_block_body()?;
        let mut end = self.current_span();
        self.expect(TokenKind::RBrace)?;

        // Optional `else { ... }` block (runs if loop completed without break)
        let else_body = if self.peek_past_newlines_is(&TokenKind::Else) {
            self.skip_newlines();
            self.advance(); // consume 'else'
            self.expect(TokenKind::LBrace)?;
            let eb = self.parse_block_body()?;
            end = self.current_span();
            self.expect(TokenKind::RBrace)?;
            Some(eb)
        } else {
            None
        };

        Ok(Spanned::new(Stmt::For { pattern, iter, when_guard, body, else_body }, start.merge(end)))
    }

    fn parse_assert(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Assert)?;
        let condition = self.parse_expr()?;

        let message = if self.check(&TokenKind::Comma) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        let span = match &message {
            Some(m) => start.merge(m.span),
            None => start.merge(condition.span),
        };

        Ok(Spanned::new(Stmt::Assert { condition, message }, span))
    }

    fn parse_pipeline(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Pipeline)?;

        // Accept both identifier and string as pipeline name
        let name = match self.current_kind() {
            TokenKind::Ident(s) => {
                let n = s.clone();
                self.advance();
                n
            }
            TokenKind::Str(s) => {
                let n = s.clone();
                self.advance();
                n
            }
            _ => {
                return Err(BioLangError::parser(
                    "expected pipeline name",
                    self.current_span(),
                ))
            }
        };

        // Optional parameter list: pipeline name(param1, param2) { ... }
        let params = if self.check(&TokenKind::LParen) {
            self.advance();
            let params = self.parse_params()?;
            self.expect(TokenKind::RParen)?;
            params
        } else {
            Vec::new()
        };

        self.expect(TokenKind::LBrace)?;
        let body = self.parse_block_body()?;
        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;

        Ok(Spanned::new(Stmt::Pipeline { name, params, body }, start.merge(end)))
    }

    fn parse_import(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Import)?;

        let path = match self.current_kind() {
            TokenKind::Str(s) => {
                let p = s.clone();
                self.advance();
                p
            }
            _ => {
                return Err(BioLangError::parser(
                    "expected string path after 'import'",
                    self.current_span(),
                ))
            }
        };

        // Check for `as alias` keyword
        let alias = if matches!(self.current_kind(), TokenKind::As) {
            self.advance(); // consume "as"
            let name = self.expect_ident()?;
            Some(name)
        } else if let TokenKind::Ident(s) = self.current_kind() {
            if s == "as" {
                self.advance();
                let name = self.expect_ident()?;
                Some(name)
            } else {
                None
            }
        } else {
            None
        };

        let end = self.tokens[self.pos - 1].span;
        Ok(Spanned::new(Stmt::Import { path, alias }, start.merge(end)))
    }

    /// `from "module" import name1, name2`
    fn parse_from_import(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::From)?;

        let path = match self.current_kind() {
            TokenKind::Str(s) => {
                let p = s.clone();
                self.advance();
                p
            }
            _ => {
                return Err(BioLangError::parser(
                    "expected string path after 'from'",
                    self.current_span(),
                ))
            }
        };

        self.expect(TokenKind::Import)?;

        let mut names = Vec::new();
        loop {
            names.push(self.expect_ident()?);
            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        let end = self.prev_span();
        Ok(Spanned::new(Stmt::FromImport { path, names }, start.merge(end)))
    }

    /// `unless condition { body }`
    fn parse_unless(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Unless)?;
        let condition = self.parse_expr()?;
        self.expect(TokenKind::LBrace)?;
        let body = self.parse_block_body()?;
        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;
        Ok(Spanned::new(Stmt::Unless { condition, body }, start.merge(end)))
    }

    /// `guard condition else { fallback }` or `guard condition else return expr`
    fn parse_guard(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Guard)?;
        let condition = self.parse_expr()?;
        self.expect(TokenKind::Else)?;
        let else_body = if self.check(&TokenKind::LBrace) {
            self.advance();
            let body = self.parse_block_body()?;
            self.expect(TokenKind::RBrace)?;
            body
        } else {
            // Single statement: `guard x else return nil`
            let stmt = self.parse_stmt()?;
            vec![stmt]
        };
        let end = self.prev_span();
        Ok(Spanned::new(Stmt::Guard { condition, else_body }, start.merge(end)))
    }

    /// `defer expr`
    fn parse_defer(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Defer)?;
        let expr = self.parse_expr()?;
        let span = start.merge(expr.span);
        Ok(Spanned::new(Stmt::Defer(expr), span))
    }

    /// `parallel for var in iter { body }`
    fn parse_parallel_for(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Parallel)?;
        self.expect(TokenKind::For)?;

        let pattern = if self.check(&TokenKind::LBracket) {
            self.advance();
            let mut names = Vec::new();
            loop {
                names.push(self.expect_ident()?);
                if !self.check(&TokenKind::Comma) { break; }
                self.advance();
            }
            self.expect(TokenKind::RBracket)?;
            ForPattern::ListDestr(names)
        } else if self.check(&TokenKind::LBrace) {
            self.advance();
            let mut names = Vec::new();
            loop {
                names.push(self.expect_ident()?);
                if !self.check(&TokenKind::Comma) { break; }
                self.advance();
            }
            self.expect(TokenKind::RBrace)?;
            ForPattern::RecordDestr(names)
        } else if self.check(&TokenKind::LParen) {
            self.advance();
            let mut names = Vec::new();
            loop {
                names.push(self.expect_ident()?);
                if !self.check(&TokenKind::Comma) { break; }
                self.advance();
            }
            self.expect(TokenKind::RParen)?;
            ForPattern::TupleDestr(names)
        } else {
            ForPattern::Single(self.expect_ident()?)
        };

        self.expect(TokenKind::In)?;
        let iter = self.parse_expr()?;
        self.expect(TokenKind::LBrace)?;
        let body = self.parse_block_body()?;
        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;

        Ok(Spanned::new(Stmt::ParallelFor { pattern, iter, body }, start.merge(end)))
    }

    /// `stage "name" -> expr` inside a pipeline block
    fn parse_stage(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.expect(TokenKind::Stage)?;
        let name = match self.current_kind() {
            TokenKind::Str(s) => {
                let n = s.clone();
                self.advance();
                n
            }
            TokenKind::Ident(s) => {
                let n = s.clone();
                self.advance();
                n
            }
            _ => return Err(BioLangError::new(
                ErrorKind::ExpectedExpression,
                "expected stage name (string or identifier)",
                Some(self.current_span()),
            )),
        };
        self.expect(TokenKind::Arrow)?;
        let expr = self.parse_expr()?;
        let span = start.merge(expr.span);
        Ok(Spanned::new(Stmt::Stage { name, expr }, span))
    }

    /// `type Name = TypeExpr`
    fn parse_type_alias(&mut self) -> Result<Spanned<Stmt>> {
        let start = self.current_span();
        self.advance(); // consume `type` identifier (contextual keyword)
        let name = self.expect_ident()?;
        self.expect(TokenKind::Eq)?;
        let target = self.parse_type_annotation()?;
        let end = self.current_span();
        Ok(Spanned::new(Stmt::TypeAlias { name, target }, start.merge(end)))
    }

    /// `given { cond => expr, ... otherwise => expr }`
    fn parse_given_expr(&mut self) -> Result<Spanned<Expr>> {
        let start = self.current_span();
        self.expect(TokenKind::Given)?;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut arms = Vec::new();
        let mut otherwise = None;

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.check(&TokenKind::Otherwise) {
                self.advance();
                self.expect(TokenKind::FatArrow)?;
                let body = self.parse_expr()?;
                otherwise = Some(Box::new(body));
                self.skip_newlines();
                if self.check(&TokenKind::Comma) {
                    self.advance();
                    self.skip_newlines();
                }
                break;
            }
            let condition = self.parse_expr()?;
            self.expect(TokenKind::FatArrow)?;
            let body = self.parse_expr()?;
            arms.push((condition, body));
            self.skip_newlines();
            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            }
            // Newlines alone are sufficient arm separators — don't break
        }
        self.skip_newlines();
        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;
        Ok(Spanned::new(Expr::Given { arms, otherwise }, start.merge(end)))
    }

    /// `retry(count, delay: ms) { body }`
    fn parse_retry_expr(&mut self) -> Result<Spanned<Expr>> {
        let start = self.current_span();
        self.expect(TokenKind::Retry)?;
        self.expect(TokenKind::LParen)?;
        let count = self.parse_expr()?;
        let delay = if self.check(&TokenKind::Comma) {
            self.advance();
            // optional `delay:` keyword
            if let TokenKind::Ident(s) = self.current_kind() {
                if s == "delay" {
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                }
            }
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::LBrace)?;
        let body = self.parse_block_body()?;
        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;
        Ok(Spanned::new(
            Expr::Retry { count: Box::new(count), delay, body },
            start.merge(end),
        ))
    }

    fn parse_expr_stmt(&mut self) -> Result<Spanned<Stmt>> {
        let expr = self.parse_expr()?;
        let span = expr.span;
        Ok(Spanned::new(Stmt::Expr(expr), span))
    }

    fn parse_block_body(&mut self) -> Result<Vec<Spanned<Stmt>>> {
        let mut stmts = Vec::new();
        self.skip_newlines();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            stmts.push(self.parse_stmt()?);
            self.skip_newlines();
        }

        Ok(stmts)
    }

    // ── Expression parsing (Pratt) ────────────────────────────────────

    fn parse_expr(&mut self) -> Result<Spanned<Expr>> {
        let expr = self.parse_precedence(Precedence::None)?;
        // Check for ternary: `value if condition else other`
        self.maybe_ternary(expr)
    }

    fn parse_precedence(&mut self, min_prec: Precedence) -> Result<Spanned<Expr>> {
        let mut left = self.parse_prefix()?;

        while !self.is_at_end() {
            // Check for struct literal: `Name { field: val }` before normal precedence
            if matches!(self.current_kind(), TokenKind::LBrace) && self.is_struct_literal(&left) {
                if Precedence::Call <= min_prec {
                    break;
                }
                left = self.parse_struct_literal(left)?;
                continue;
            }
            // Look past newlines for pipe or dot continuation
            let prec = if matches!(self.current_kind(), TokenKind::Newline) {
                if self.peek_past_newlines_is_pipe() {
                    self.skip_newlines();
                    self.current_precedence()
                } else if self.peek_past_newlines_is(&TokenKind::Dot) {
                    self.skip_newlines();
                    self.current_precedence()
                } else {
                    Precedence::None
                }
            } else {
                self.current_precedence()
            };
            if prec <= min_prec {
                break;
            }
            left = self.parse_infix(left, prec)?;
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Spanned<Expr>> {
        match self.current_kind() {
            TokenKind::Nil => {
                let span = self.current_span();
                self.advance();
                Ok(Spanned::new(Expr::Nil, span))
            }
            TokenKind::True => {
                let span = self.current_span();
                self.advance();
                Ok(Spanned::new(Expr::Bool(true), span))
            }
            TokenKind::False => {
                let span = self.current_span();
                self.advance();
                Ok(Spanned::new(Expr::Bool(false), span))
            }
            TokenKind::Int(n) => {
                let n = *n;
                let span = self.current_span();
                self.advance();
                Ok(Spanned::new(Expr::Int(n), span))
            }
            TokenKind::Float(f) => {
                let f = *f;
                let span = self.current_span();
                self.advance();
                Ok(Spanned::new(Expr::Float(f), span))
            }
            TokenKind::Str(s) => {
                let s = s.clone();
                let span = self.current_span();
                self.advance();
                Ok(Spanned::new(Expr::Str(s), span))
            }
            TokenKind::DnaLit(s) => {
                let s = s.clone();
                let span = self.current_span();
                self.advance();
                Ok(Spanned::new(Expr::DnaLit(s), span))
            }
            TokenKind::RnaLit(s) => {
                let s = s.clone();
                let span = self.current_span();
                self.advance();
                Ok(Spanned::new(Expr::RnaLit(s), span))
            }
            TokenKind::ProteinLit(s) => {
                let s = s.clone();
                let span = self.current_span();
                self.advance();
                Ok(Spanned::new(Expr::ProteinLit(s), span))
            }
            TokenKind::QualLit(s) => {
                let s = s.clone();
                let span = self.current_span();
                self.advance();
                Ok(Spanned::new(Expr::QualLit(s), span))
            }
            TokenKind::Ident(_) => self.parse_ident_expr(),
            // `into` is a keyword after `|>`, but also a builtin function name.
            // In prefix position (start of expression), treat it as an identifier.
            TokenKind::Into => {
                let span = self.current_span();
                self.advance();
                Ok(Spanned::new(Expr::Ident("into".to_string()), span))
            }
            TokenKind::Minus => {
                let start = self.current_span();
                self.advance();
                let expr = self.parse_precedence(Precedence::Unary)?;
                let span = start.merge(expr.span);
                Ok(Spanned::new(
                    Expr::Unary {
                        op: UnaryOp::Neg,
                        expr: Box::new(expr),
                    },
                    span,
                ))
            }
            TokenKind::Bang | TokenKind::Not => {
                let start = self.current_span();
                self.advance();
                let expr = self.parse_precedence(Precedence::Unary)?;
                let span = start.merge(expr.span);
                Ok(Spanned::new(
                    Expr::Unary {
                        op: UnaryOp::Not,
                        expr: Box::new(expr),
                    },
                    span,
                ))
            }
            TokenKind::LParen => self.parse_grouped(),
            TokenKind::LBracket => self.parse_list(),
            TokenKind::LBrace => self.parse_block_or_record(),
            TokenKind::Bar => self.parse_lambda(),
            TokenKind::Tilde => self.parse_formula(),
            TokenKind::If => self.parse_if_expr(),
            TokenKind::Match => self.parse_match_expr(),
            TokenKind::Given => self.parse_given_expr(),
            TokenKind::Retry => self.parse_retry_expr(),
            TokenKind::Try => self.parse_try_catch(),
            TokenKind::FStr(_) => self.parse_fstring(),
            TokenKind::HashLBrace => self.parse_set_literal(),
            TokenKind::RegexLit(_, _) => {
                let span = self.current_span();
                if let TokenKind::RegexLit(pat, flags) = self.current_kind().clone() {
                    self.advance();
                    Ok(Spanned::new(Expr::Regex { pattern: pat, flags }, span))
                } else {
                    unreachable!()
                }
            }
            TokenKind::Await => {
                let start = self.current_span();
                self.advance();
                let expr = self.parse_precedence(Precedence::Unary)?;
                let span = start.merge(expr.span);
                Ok(Spanned::new(Expr::Await(Box::new(expr)), span))
            }
            TokenKind::Do => self.parse_do_block(),
            _ => Err(BioLangError::new(
                ErrorKind::ExpectedExpression,
                format!("expected expression, found '{}'", self.current_kind()),
                Some(self.current_span()),
            )),
        }
    }

    fn parse_set_literal(&mut self) -> Result<Spanned<Expr>> {
        let start = self.current_span();
        self.expect(TokenKind::HashLBrace)?;
        self.skip_newlines();

        let mut items = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            items.push(self.parse_expr()?);
            self.skip_newlines();
            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            } else {
                break;
            }
        }

        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;
        Ok(Spanned::new(Expr::SetLiteral(items), start.merge(end)))
    }

    fn parse_ident_expr(&mut self) -> Result<Spanned<Expr>> {
        let name = self.expect_ident()?;
        let span = self.tokens[self.pos - 1].span;
        Ok(Spanned::new(Expr::Ident(name), span))
    }

    fn parse_grouped(&mut self) -> Result<Spanned<Expr>> {
        let start = self.current_span();
        self.expect(TokenKind::LParen)?;
        self.skip_newlines();
        let expr = self.parse_expr()?;
        self.skip_newlines();
        let end = self.current_span();
        self.expect(TokenKind::RParen)?;
        // Preserve span of the grouping
        Ok(Spanned::new(expr.node, start.merge(end)))
    }

    fn parse_list(&mut self) -> Result<Spanned<Expr>> {
        let start = self.current_span();
        self.expect(TokenKind::LBracket)?;
        self.skip_newlines();

        // Empty list
        if self.check(&TokenKind::RBracket) {
            let end = self.current_span();
            self.advance();
            return Ok(Spanned::new(Expr::List(Vec::new()), start.merge(end)));
        }

        // Parse first expression
        let first = self.parse_expr()?;
        self.skip_newlines();

        // Check for list comprehension: [expr for x in iter if cond]
        if self.check(&TokenKind::For) {
            self.advance();
            let var = self.expect_ident()?;
            self.expect(TokenKind::In)?;
            let iter = self.parse_expr()?;
            self.skip_newlines();

            let condition = if self.check(&TokenKind::If) {
                self.advance();
                Some(Box::new(self.parse_expr()?))
            } else {
                None
            };
            self.skip_newlines();
            let end = self.current_span();
            self.expect(TokenKind::RBracket)?;
            return Ok(Spanned::new(
                Expr::ListComp {
                    expr: Box::new(first),
                    var,
                    iter: Box::new(iter),
                    condition,
                },
                start.merge(end),
            ));
        }

        // Regular list
        let mut items = vec![first];
        if self.check(&TokenKind::Comma) {
            self.advance();
            self.skip_newlines();
            while !self.check(&TokenKind::RBracket) && !self.is_at_end() {
                items.push(self.parse_expr()?);
                self.skip_newlines();
                if self.check(&TokenKind::Comma) {
                    self.advance();
                    self.skip_newlines();
                } else {
                    break;
                }
            }
        }

        let end = self.current_span();
        self.expect(TokenKind::RBracket)?;
        Ok(Spanned::new(Expr::List(items), start.merge(end)))
    }

    fn parse_block_or_record(&mut self) -> Result<Spanned<Expr>> {
        // Lookahead to determine if this is a record literal or a block
        // Record: { ident: expr, ... }
        // Block: { stmts }
        if self.is_record_literal() {
            self.parse_record()
        } else {
            self.parse_block_expr()
        }
    }

    fn is_record_literal(&self) -> bool {
        // { key: ...} is a record — key can be ident, string, or keyword (e.g. "end", "type")
        if self.pos + 2 < self.tokens.len() {
            let key_is_valid = Self::token_is_record_key(&self.tokens[self.pos + 1].kind);
            if key_is_valid && matches!(&self.tokens[self.pos + 2].kind, TokenKind::Colon) {
                return true;
            }
        }
        // {...expr} is a record spread
        if self.pos + 1 < self.tokens.len() {
            if matches!(&self.tokens[self.pos + 1].kind, TokenKind::DotDotDot) {
                return true;
            }
        }
        false
    }

    /// Check if a token can serve as a record key (identifiers, strings, and keywords)
    fn token_is_record_key(kind: &TokenKind) -> bool {
        matches!(kind, TokenKind::Ident(_) | TokenKind::Str(_))
            || Self::keyword_as_name(kind).is_some()
    }

    /// Extract a name from a keyword token for use as a record key
    fn keyword_as_name(kind: &TokenKind) -> Option<&'static str> {
        match kind {
            TokenKind::End => Some("end"),
            TokenKind::Into => Some("into"),
            TokenKind::As => Some("as"),
            TokenKind::From => Some("from"),
            TokenKind::Where => Some("where"),
            TokenKind::With => Some("with"),
            TokenKind::Not => Some("not"),
            TokenKind::And => Some("and"),
            TokenKind::Or => Some("or"),
            TokenKind::Do => Some("do"),
            TokenKind::If => Some("if"),
            TokenKind::Then => Some("then"),
            TokenKind::Else => Some("else"),
            TokenKind::For => Some("for"),
            TokenKind::In => Some("in"),
            TokenKind::While => Some("while"),
            TokenKind::Match => Some("match"),
            TokenKind::Return => Some("return"),
            TokenKind::True => Some("true"),
            TokenKind::False => Some("false"),
            TokenKind::Nil => Some("nil"),
            TokenKind::Import => Some("import"),
            TokenKind::Try => Some("try"),
            TokenKind::Catch => Some("catch"),
            TokenKind::Break => Some("break"),
            TokenKind::Continue => Some("continue"),
            TokenKind::Fn => Some("fn"),
            TokenKind::Let => Some("let"),
            TokenKind::Const => Some("const"),
            TokenKind::Struct => Some("struct"),
            TokenKind::Enum => Some("enum"),
            TokenKind::Trait => Some("trait"),
            TokenKind::Impl => Some("impl"),
            TokenKind::Async => Some("async"),
            TokenKind::Await => Some("await"),
            TokenKind::Yield => Some("yield"),
            TokenKind::Stage => Some("stage"),
            TokenKind::Parallel => Some("parallel"),
            TokenKind::Pipeline => Some("pipeline"),
            TokenKind::Assert => Some("assert"),
            TokenKind::Given => Some("given"),
            TokenKind::Otherwise => Some("otherwise"),
            TokenKind::Guard => Some("guard"),
            TokenKind::Unless => Some("unless"),
            TokenKind::When => Some("when"),
            TokenKind::Defer => Some("defer"),
            TokenKind::Retry => Some("retry"),
            _ => None,
        }
    }

    fn parse_record(&mut self) -> Result<Spanned<Expr>> {
        let start = self.current_span();
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();

        // Check if first entry is a spread `...expr`
        let mut spreads: Vec<Spanned<Expr>> = Vec::new();
        let mut fields: Vec<(String, Spanned<Expr>)> = Vec::new();
        let mut has_spread = false;

        // Parse entries: either `...expr` or `key: value`
        loop {
            if self.check(&TokenKind::RBrace) || self.is_at_end() {
                break;
            }
            if self.check(&TokenKind::DotDotDot) {
                has_spread = true;
                self.advance(); // consume ...
                let spread_expr = self.parse_expr()?;
                spreads.push(spread_expr);
            } else {
                let key_name = self.expect_record_key()?;
                self.expect(TokenKind::Colon)?;
                let value = self.parse_expr()?;

                // Check for map comprehension on first field: {k: v for x in iter [if cond]}
                if fields.is_empty() && spreads.is_empty() && self.check(&TokenKind::For) {
                    self.advance();
                    let var = self.expect_ident()?;
                    self.expect(TokenKind::In)?;
                    let iter = self.parse_expr()?;
                    self.skip_newlines();
                    let condition = if self.check(&TokenKind::If) {
                        self.advance();
                        Some(Box::new(self.parse_expr()?))
                    } else {
                        None
                    };
                    self.skip_newlines();
                    let end = self.current_span();
                    self.expect(TokenKind::RBrace)?;
                    let key_expr = Spanned::new(Expr::Ident(key_name), start);
                    return Ok(Spanned::new(
                        Expr::MapComp {
                            key: Box::new(key_expr),
                            value: Box::new(value),
                            var,
                            iter: Box::new(iter),
                            condition,
                        },
                        start.merge(end),
                    ));
                }

                fields.push((key_name, value));
            }
            self.skip_newlines();
            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            } else {
                break;
            }
        }

        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;

        if has_spread {
            Ok(Spanned::new(Expr::RecordSpread { spreads, fields }, start.merge(end)))
        } else {
            Ok(Spanned::new(Expr::Record(fields), start.merge(end)))
        }
    }

    fn parse_block_expr(&mut self) -> Result<Spanned<Expr>> {
        let start = self.current_span();
        self.expect(TokenKind::LBrace)?;
        let body = self.parse_block_body()?;
        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;
        Ok(Spanned::new(Expr::Block(body), start.merge(end)))
    }

    fn parse_lambda(&mut self) -> Result<Spanned<Expr>> {
        let start = self.current_span();
        self.expect(TokenKind::Bar)?;

        let mut params = Vec::new();
        while !self.check(&TokenKind::Bar) && !self.is_at_end() {
            let rest = if self.check(&TokenKind::DotDotDot) {
                self.advance();
                true
            } else {
                false
            };
            let name = self.expect_ident()?;
            let type_ann = if self.check(&TokenKind::Colon) {
                self.advance();
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            params.push(Param {
                name,
                type_ann,
                default: None,
                rest,
            });
            if self.check(&TokenKind::Comma) {
                self.advance();
            }
            if rest {
                break;
            }
        }

        self.expect(TokenKind::Bar)?;

        let body = if self.check(&TokenKind::FatArrow) {
            // `|x| => { ... }` — multi-line lambda with fat arrow
            self.advance(); // consume =>
            if self.check(&TokenKind::LBrace) {
                self.parse_block_expr()?
            } else {
                self.parse_expr()?
            }
        } else if self.check(&TokenKind::LBrace) {
            self.parse_block_or_record()?
        } else if self.is_lambda_block_start() {
            // Multi-statement lambda body without braces (e.g. |x| let y = ...\n y + 1)
            self.parse_implicit_lambda_block()?
        } else {
            self.parse_expr()?
        };

        let span = start.merge(body.span);
        Ok(Spanned::new(
            Expr::Lambda {
                params,
                body: Box::new(body),
            },
            span,
        ))
    }

    /// Check if the current token starts a statement (not an expression),
    /// indicating the lambda body needs multi-statement parsing.
    fn is_lambda_block_start(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::Let | TokenKind::Const | TokenKind::For | TokenKind::While
        )
    }

    /// Parse an implicit multi-statement lambda body (no braces).
    /// Collects statements until we hit `)` or a dedent/unmatched closer.
    fn parse_implicit_lambda_block(&mut self) -> Result<Spanned<Expr>> {
        let start = self.current_span();
        let mut stmts = Vec::new();

        loop {
            self.skip_newlines();
            // Stop at closers that belong to the enclosing call
            if self.is_at_end()
                || self.check(&TokenKind::RParen)
                || self.check(&TokenKind::RBrace)
                || self.check(&TokenKind::RBracket)
            {
                break;
            }
            stmts.push(self.parse_stmt()?);
            self.skip_newlines();
        }

        let span = if let Some(last) = stmts.last() {
            start.merge(last.span)
        } else {
            start
        };
        Ok(Spanned::new(Expr::Block(stmts), span))
    }

    fn parse_formula(&mut self) -> Result<Spanned<Expr>> {
        let start = self.current_span();
        self.advance(); // consume leading ~
        let lhs = self.parse_precedence(Precedence::Addition)?;
        // Check for ~ separator (R-style: ~response ~ predictor1 + predictor2)
        if self.check(&TokenKind::Tilde) {
            self.advance(); // consume inner ~
            let rhs = self.parse_precedence(Precedence::Addition)?;
            let span = start.merge(rhs.span);
            // Use Binary with Sub as a structural separator — parse_formula_columns
            // only inspects left/right, not the op
            let inner = Spanned::new(
                Expr::Binary {
                    op: BinaryOp::Sub,
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                },
                span,
            );
            Ok(Spanned::new(Expr::Formula(Box::new(inner)), span))
        } else {
            let span = start.merge(lhs.span);
            Ok(Spanned::new(Expr::Formula(Box::new(lhs)), span))
        }
    }

    fn parse_if_expr(&mut self) -> Result<Spanned<Expr>> {
        let start = self.current_span();
        self.expect(TokenKind::If)?;

        let condition = self.parse_expr()?;

        // `if cond then value/stmt [else value/stmt]` — inline form
        // Supports both expression ternary: `if x > 0 then 1 else 0`
        // and statement form: `if cond then x = 42`
        if self.check(&TokenKind::Then) {
            self.advance(); // consume 'then'
            self.skip_newlines(); // allow then-branch on next line
            let then_body = vec![self.parse_then_branch()?];

            // Peek past newlines to check for `else`, but only consume
            // them if `else` is actually present — otherwise the newline
            // acts as a statement terminator.
            let else_body = if self.peek_past_newlines_is(&TokenKind::Else) {
                self.skip_newlines();
                self.advance(); // consume 'else'
                self.skip_newlines();
                if self.check(&TokenKind::If) {
                    let elif = self.parse_if_expr()?;
                    Some(vec![Spanned::new(Stmt::Expr(elif.clone()), elif.span)])
                } else {
                    Some(vec![self.parse_then_branch()?])
                }
            } else {
                None
            };

            let end = self.tokens[self.pos - 1].span;
            return Ok(Spanned::new(
                Expr::If {
                    condition: Box::new(condition),
                    then_body,
                    else_body,
                },
                start.merge(end),
            ));
        }

        // `if cond { ... } [else { ... }]` — block form
        self.expect(TokenKind::LBrace)?;
        let then_body = self.parse_block_body()?;
        self.expect(TokenKind::RBrace)?;

        let else_body = if self.check(&TokenKind::Else) {
            self.advance();
            if self.check(&TokenKind::If) {
                // else if — wrap in block
                let elif = self.parse_if_expr()?;
                Some(vec![Spanned::new(Stmt::Expr(elif.clone()), elif.span)])
            } else {
                self.expect(TokenKind::LBrace)?;
                let body = self.parse_block_body()?;
                self.expect(TokenKind::RBrace)?;
                Some(body)
            }
        } else {
            None
        };

        let end = self.tokens[self.pos - 1].span;
        Ok(Spanned::new(
            Expr::If {
                condition: Box::new(condition),
                then_body,
                else_body,
            },
            start.merge(end),
        ))
    }

    /// Parse a single statement for `then`/`else` branches.
    /// Handles assignments (`x = expr`), compound assignments (`x += expr`),
    /// let bindings, and plain expressions.
    fn parse_then_branch(&mut self) -> Result<Spanned<Stmt>> {
        match self.current_kind() {
            TokenKind::Let => {
                if self.is_destruct_let() {
                    self.parse_destruct_let()
                } else {
                    self.parse_let()
                }
            }
            TokenKind::Const => self.parse_const(),
            TokenKind::Ident(_) if self.is_nil_assign() => self.parse_nil_assign(),
            TokenKind::Ident(_) if self.is_compound_assignment() => self.parse_compound_assign(),
            TokenKind::Ident(_) if self.is_assignment() => self.parse_assign(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_try_catch(&mut self) -> Result<Spanned<Expr>> {
        let start = self.current_span();
        self.expect(TokenKind::Try)?;
        self.expect(TokenKind::LBrace)?;
        let body = self.parse_block_body()?;
        self.expect(TokenKind::RBrace)?;

        self.expect(TokenKind::Catch)?;

        // Optional error variable name
        let error_var = if !self.check(&TokenKind::LBrace) {
            Some(self.expect_ident()?)
        } else {
            None
        };

        self.expect(TokenKind::LBrace)?;
        let catch_body = self.parse_block_body()?;
        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;

        Ok(Spanned::new(
            Expr::TryCatch {
                body,
                error_var,
                catch_body,
            },
            start.merge(end),
        ))
    }

    fn parse_fstring(&mut self) -> Result<Spanned<Expr>> {
        let span = self.current_span();
        let template = match self.current_kind().clone() {
            TokenKind::FStr(s) => s,
            _ => unreachable!(),
        };
        self.advance();

        let mut parts = Vec::new();
        let mut text = String::new();
        let chars: Vec<char> = template.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '{' {
                // Flush accumulated text
                if !text.is_empty() {
                    parts.push(StringPart::Lit(std::mem::take(&mut text)));
                }
                // Collect expression text until matching }
                i += 1;
                let mut expr_text = String::new();
                let mut depth = 1;
                while i < chars.len() && depth > 0 {
                    if chars[i] == '{' {
                        depth += 1;
                    } else if chars[i] == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    expr_text.push(chars[i]);
                    i += 1;
                }
                i += 1; // skip closing }

                // Parse the sub-expression
                let tokens = bl_lexer::Lexer::new(&expr_text).tokenize().map_err(|e| {
                    BioLangError::parser(
                        format!("in f-string interpolation: {}", e.message),
                        span,
                    )
                })?;
                let expr = Parser::new(tokens).parse_single_expr().map_err(|e| {
                    BioLangError::parser(
                        format!("in f-string interpolation: {}", e.message),
                        span,
                    )
                })?;
                parts.push(StringPart::Expr(expr));
            } else {
                text.push(chars[i]);
                i += 1;
            }
        }

        // Flush trailing text
        if !text.is_empty() {
            parts.push(StringPart::Lit(text));
        }

        Ok(Spanned::new(Expr::StringInterp(parts), span))
    }

    /// Parse a single expression (used for f-string sub-parsing).
    fn parse_single_expr(mut self) -> Result<Spanned<Expr>> {
        self.skip_newlines();
        let expr = self.parse_expr()?;
        Ok(expr)
    }

    fn parse_match_expr(&mut self) -> Result<Spanned<Expr>> {
        let start = self.current_span();
        self.expect(TokenKind::Match)?;

        let expr = self.parse_expr()?;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut arms = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let pattern = self.parse_or_pattern()?;
            // Parse optional guard: `if condition`
            let guard = if self.check(&TokenKind::If) {
                self.advance();
                Some(Box::new(self.parse_expr()?))
            } else {
                None
            };
            self.expect(TokenKind::FatArrow)?;
            let body = self.parse_expr()?;
            arms.push(MatchArm { pattern, guard, body });
            self.skip_newlines();
            if self.check(&TokenKind::Comma) {
                self.advance();
            }
            self.skip_newlines();
        }

        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;

        Ok(Spanned::new(
            Expr::Match {
                expr: Box::new(expr),
                arms,
            },
            start.merge(end),
        ))
    }

    /// Parse a pattern that may be an or-pattern: `pattern1 | pattern2 | ...`
    fn parse_or_pattern(&mut self) -> Result<Spanned<Pattern>> {
        let first = self.parse_pattern()?;
        if !self.check(&TokenKind::Bar) {
            return Ok(first);
        }
        let start_span = first.span;
        let mut alternatives = vec![first];
        while self.check(&TokenKind::Bar) {
            self.advance();
            self.skip_newlines();
            alternatives.push(self.parse_pattern()?);
        }
        let end_span = alternatives.last().unwrap().span;
        Ok(Spanned::new(Pattern::Or(alternatives), start_span.merge(end_span)))
    }

    fn parse_pattern(&mut self) -> Result<Spanned<Pattern>> {
        let span = self.current_span();
        match self.current_kind() {
            TokenKind::Ident(name) if name == "_" => {
                self.advance();
                Ok(Spanned::new(Pattern::Wildcard, span))
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                // Check for type pattern or enum variant: Name(binding)
                if self.check(&TokenKind::LParen) {
                    // Known type names → TypePattern
                    const TYPE_NAMES: &[&str] = &[
                        "Int", "Float", "Str", "Bool", "Nil", "List", "Map", "Record",
                        "DNA", "RNA", "Protein", "Table", "Stream", "Set", "Matrix",
                        "Interval", "Range", "Regex", "Kmer", "Function", "Future",
                        "SparseMatrix",
                    ];
                    if TYPE_NAMES.contains(&name.as_str()) {
                        self.advance(); // consume (
                        let binding = if self.check(&TokenKind::RParen) {
                            None
                        } else {
                            let b = self.expect_ident()?;
                            if b == "_" { None } else { Some(b) }
                        };
                        let end = self.current_span();
                        self.expect(TokenKind::RParen)?;
                        Ok(Spanned::new(
                            Pattern::TypePattern { type_name: name, binding },
                            span.merge(end),
                        ))
                    } else {
                        // Enum variant pattern: Variant(a, b)
                        self.advance(); // consume (
                        let mut bindings = Vec::new();
                        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                            bindings.push(self.expect_ident()?);
                            if self.check(&TokenKind::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        let end = self.current_span();
                        self.expect(TokenKind::RParen)?;
                        Ok(Spanned::new(
                            Pattern::EnumVariant { variant: name, bindings },
                            span.merge(end),
                        ))
                    }
                } else {
                    Ok(Spanned::new(Pattern::Ident(name), span))
                }
            }
            TokenKind::Int(_) | TokenKind::Float(_) | TokenKind::Str(_) | TokenKind::True
            | TokenKind::False | TokenKind::Nil => {
                let lit = self.parse_prefix()?;
                let span = lit.span;
                Ok(Spanned::new(Pattern::Literal(lit), span))
            }
            _ => Err(BioLangError::parser(
                format!("expected pattern, found '{}'", self.current_kind()),
                span,
            )),
        }
    }

    // ── Infix (binary) parsing ────────────────────────────────────────

    fn parse_infix(&mut self, left: Spanned<Expr>, prec: Precedence) -> Result<Spanned<Expr>> {
        match self.current_kind() {
            TokenKind::DotDot => {
                self.advance();
                let right = self.parse_precedence(prec)?;
                let span = left.span.merge(right.span);
                Ok(Spanned::new(
                    Expr::Range {
                        start: Box::new(left),
                        end: Box::new(right),
                        inclusive: false,
                    },
                    span,
                ))
            }
            TokenKind::DotDotEq => {
                self.advance();
                let right = self.parse_precedence(prec)?;
                let span = left.span.merge(right.span);
                Ok(Spanned::new(
                    Expr::Range {
                        start: Box::new(left),
                        end: Box::new(right),
                        inclusive: true,
                    },
                    span,
                ))
            }
            TokenKind::PipeOp => {
                self.advance();
                self.skip_newlines();
                // Check for `|> into name` (pipe-into binding)
                if matches!(self.current_kind(), TokenKind::Into) {
                    self.advance();
                    let name = self.expect_ident()?;
                    let span = left.span.merge(self.prev_span());
                    return Ok(Spanned::new(
                        Expr::PipeInto {
                            value: Box::new(left),
                            name,
                        },
                        span,
                    ));
                }
                // Check for `|> then var -> expr` (destructuring pipe)
                if matches!(self.current_kind(), TokenKind::Then) {
                    self.advance();
                    let var = self.expect_ident()?;
                    self.expect(TokenKind::Arrow)?;
                    let right = self.parse_precedence(prec)?;
                    let span = left.span.merge(right.span);
                    return Ok(Spanned::new(
                        Expr::ThenPipe {
                            left: Box::new(left),
                            var,
                            right: Box::new(right),
                        },
                        span,
                    ));
                }
                let right = self.parse_precedence(prec)?;
                // Trailing lambda: `|> each |p| expr` → `|> each(|p| expr)`
                let right = if matches!(right.node, Expr::Ident(_))
                    && self.check(&TokenKind::Bar)
                {
                    let lambda = self.parse_lambda()?;
                    let call_span = right.span.merge(lambda.span);
                    Spanned::new(
                        Expr::Call {
                            callee: Box::new(right),
                            args: vec![Arg { name: None, value: lambda, spread: false }],
                        },
                        call_span,
                    )
                } else {
                    right
                };
                let span = left.span.merge(right.span);
                Ok(Spanned::new(
                    Expr::Pipe {
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    span,
                ))
            }
            TokenKind::TapPipe => {
                self.advance();
                self.skip_newlines();
                let right = self.parse_precedence(prec)?;
                // Trailing lambda: `|>> each |p| expr` → `|>> each(|p| expr)`
                let right = if matches!(right.node, Expr::Ident(_))
                    && self.check(&TokenKind::Bar)
                {
                    let lambda = self.parse_lambda()?;
                    let call_span = right.span.merge(lambda.span);
                    Spanned::new(
                        Expr::Call {
                            callee: Box::new(right),
                            args: vec![Arg { name: None, value: lambda, spread: false }],
                        },
                        call_span,
                    )
                } else {
                    right
                };
                let span = left.span.merge(right.span);
                Ok(Spanned::new(
                    Expr::TapPipe {
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    span,
                ))
            }
            TokenKind::QuestionQuestion => {
                self.advance();
                let right = self.parse_precedence(prec)?;
                let span = left.span.merge(right.span);
                Ok(Spanned::new(
                    Expr::NullCoalesce {
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    span,
                ))
            }
            TokenKind::In => {
                self.advance();
                let right = self.parse_precedence(prec)?;
                let span = left.span.merge(right.span);
                Ok(Spanned::new(
                    Expr::In {
                        left: Box::new(left),
                        right: Box::new(right),
                        negated: false,
                    },
                    span,
                ))
            }
            TokenKind::As => {
                self.advance();
                let target = self.expect_ident()?;
                let span = left.span.merge(self.prev_span());
                Ok(Spanned::new(
                    Expr::TypeCast {
                        expr: Box::new(left),
                        target,
                    },
                    span,
                ))
            }
            TokenKind::Not => {
                // `not in` — negated membership test
                self.advance();
                self.expect(TokenKind::In)?;
                let right = self.parse_precedence(prec)?;
                let span = left.span.merge(right.span);
                return Ok(Spanned::new(
                    Expr::In {
                        left: Box::new(left),
                        right: Box::new(right),
                        negated: true,
                    },
                    span,
                ));
            }
            TokenKind::StarStar => {
                // Right-associative exponentiation
                self.advance();
                // Use one level lower for right-associativity
                let right = self.parse_precedence(Precedence::Multiply)?;
                let span = left.span.merge(right.span);
                return Ok(Spanned::new(
                    Expr::Binary {
                        op: BinaryOp::Pow,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    span,
                ));
            }
            TokenKind::LParen => self.parse_call(left),
            TokenKind::Dot => self.parse_field_access(left),
            TokenKind::QuestionDot => self.parse_optional_field_access(left),
            TokenKind::LBracket => self.parse_index_access(left),
            _ => {
                let op = self.parse_binary_op()?;
                let right = self.parse_precedence(prec)?;
                // Check for chained comparison: a < b < c
                if self.is_comparison_op(self.current_kind()) && self.is_comparison_binary_op(&op) {
                    return self.parse_chained_cmp(left, op, right);
                }
                let span = left.span.merge(right.span);
                Ok(Spanned::new(
                    Expr::Binary {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    span,
                ))
            }
        }
    }

    fn parse_call(&mut self, callee: Spanned<Expr>) -> Result<Spanned<Expr>> {
        let callee_span = callee.span;
        self.expect(TokenKind::LParen)?;
        self.skip_newlines();

        let mut args = Vec::new();
        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            let arg = self.parse_arg()?;
            args.push(arg);
            self.skip_newlines();
            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            } else {
                break;
            }
        }

        let end = self.current_span();
        self.expect(TokenKind::RParen)?;

        Ok(Spanned::new(
            Expr::Call {
                callee: Box::new(callee),
                args,
            },
            callee_span.merge(end),
        ))
    }

    /// Check if the current `{` should be parsed as a struct literal `Name { field: val }`.
    /// The left expression must be an Ident and the lookahead must show `{ ident : ...`.
    fn is_struct_literal(&self, left: &Spanned<Expr>) -> bool {
        if !matches!(left.node, Expr::Ident(_)) {
            return false;
        }
        // lookahead: LBrace Ident Colon
        if self.pos + 2 < self.tokens.len() {
            matches!(
                (&self.tokens[self.pos].kind, &self.tokens[self.pos + 1].kind, &self.tokens[self.pos + 2].kind),
                (TokenKind::LBrace, TokenKind::Ident(_), TokenKind::Colon)
            )
        } else {
            false
        }
    }

    /// Parse `Name { field: val, ... }` as a StructLit expression.
    fn parse_struct_literal(&mut self, name_expr: Spanned<Expr>) -> Result<Spanned<Expr>> {
        let name = if let Expr::Ident(ref n) = name_expr.node {
            n.clone()
        } else {
            unreachable!()
        };
        let start = name_expr.span;
        self.expect(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let key = self.expect_ident()?;
            self.expect(TokenKind::Colon)?;
            let value = self.parse_expr()?;
            fields.push((key, value));
            self.skip_newlines();
            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            } else {
                break;
            }
        }
        self.skip_newlines();
        let end = self.current_span();
        self.expect(TokenKind::RBrace)?;
        Ok(Spanned::new(
            Expr::StructLit { name, fields },
            start.merge(end),
        ))
    }

    fn parse_arg(&mut self) -> Result<Arg> {
        // Check for spread argument: ...expr
        if self.check(&TokenKind::DotDotDot) {
            self.advance();
            let value = self.parse_expr()?;
            return Ok(Arg { name: None, value, spread: true });
        }

        // Check if this is a named argument: ident : expr
        if let TokenKind::Ident(name) = self.current_kind() {
            if self.pos + 1 < self.tokens.len()
                && matches!(self.tokens[self.pos + 1].kind, TokenKind::Colon)
            {
                let name = name.clone();
                self.advance(); // ident
                self.advance(); // :
                let value = self.parse_expr()?;
                return Ok(Arg {
                    name: Some(name),
                    value,
                    spread: false,
                });
            }
        }

        let value = self.parse_expr()?;
        Ok(Arg { name: None, value, spread: false })
    }

    fn parse_field_access(&mut self, object: Spanned<Expr>) -> Result<Spanned<Expr>> {
        let object_span = object.span;
        self.expect(TokenKind::Dot)?;
        let field = self.expect_field_name()?;
        let end = self.tokens[self.pos - 1].span;

        Ok(Spanned::new(
            Expr::Field {
                object: Box::new(object),
                field,
                optional: false,
            },
            object_span.merge(end),
        ))
    }

    fn parse_optional_field_access(&mut self, object: Spanned<Expr>) -> Result<Spanned<Expr>> {
        let object_span = object.span;
        self.expect(TokenKind::QuestionDot)?;
        let field = self.expect_field_name()?;
        let end = self.tokens[self.pos - 1].span;

        Ok(Spanned::new(
            Expr::Field {
                object: Box::new(object),
                field,
                optional: true,
            },
            object_span.merge(end),
        ))
    }

    fn parse_index_access(&mut self, object: Spanned<Expr>) -> Result<Spanned<Expr>> {
        let object_span = object.span;
        self.expect(TokenKind::LBracket)?;

        // Check for slice syntax: [start:end] or [:end] or [start:] or [start:end:step]
        // Detect slice by checking if current is `:` (empty start) or next token is `:`
        let is_slice_start = self.check(&TokenKind::Colon);
        let is_slice_after_expr = if !is_slice_start && !self.check(&TokenKind::RBracket) {
            // Peek: parse first expr, then check for `:` — but we need lookahead
            // Use a simpler approach: parse the first expr, then check for `:`
            false
        } else {
            false
        };
        let _ = is_slice_after_expr; // suppress warning

        if is_slice_start {
            // [:end] or [:end:step] or [:]
            let start = None;
            self.advance(); // consume `:`
            let end_expr = if !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Colon) {
                Some(Box::new(self.parse_expr()?))
            } else {
                None
            };
            let step = if self.check(&TokenKind::Colon) {
                self.advance();
                if !self.check(&TokenKind::RBracket) {
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                }
            } else {
                None
            };
            let end_span = self.current_span();
            self.expect(TokenKind::RBracket)?;
            return Ok(Spanned::new(
                Expr::Slice { object: Box::new(object), start, end: end_expr, step },
                object_span.merge(end_span),
            ));
        }

        let index = self.parse_expr()?;

        // Check if this is a slice: `expr : ...`
        if self.check(&TokenKind::Colon) {
            self.advance();
            let end_expr = if !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Colon) {
                Some(Box::new(self.parse_expr()?))
            } else {
                None
            };
            let step = if self.check(&TokenKind::Colon) {
                self.advance();
                if !self.check(&TokenKind::RBracket) {
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                }
            } else {
                None
            };
            let end_span = self.current_span();
            self.expect(TokenKind::RBracket)?;
            return Ok(Spanned::new(
                Expr::Slice {
                    object: Box::new(object),
                    start: Some(Box::new(index)),
                    end: end_expr,
                    step,
                },
                object_span.merge(end_span),
            ));
        }

        let end = self.current_span();
        self.expect(TokenKind::RBracket)?;

        Ok(Spanned::new(
            Expr::Index {
                object: Box::new(object),
                index: Box::new(index),
            },
            object_span.merge(end),
        ))
    }

    fn parse_binary_op(&mut self) -> Result<BinaryOp> {
        let op = match self.current_kind() {
            TokenKind::Plus => BinaryOp::Add,
            TokenKind::Minus => BinaryOp::Sub,
            TokenKind::Star => BinaryOp::Mul,
            TokenKind::StarStar => BinaryOp::Pow,
            TokenKind::Slash => BinaryOp::Div,
            TokenKind::Percent => BinaryOp::Mod,
            TokenKind::EqEq => BinaryOp::Eq,
            TokenKind::Neq => BinaryOp::Neq,
            TokenKind::Lt => BinaryOp::Lt,
            TokenKind::Gt => BinaryOp::Gt,
            TokenKind::Le => BinaryOp::Le,
            TokenKind::Ge => BinaryOp::Ge,
            TokenKind::And => BinaryOp::And,
            TokenKind::Or => BinaryOp::Or,
            TokenKind::Amp => BinaryOp::BitAnd,
            TokenKind::Caret => BinaryOp::BitXor,
            TokenKind::Shl => BinaryOp::Shl,
            TokenKind::Shr => BinaryOp::Shr,
            _ => {
                return Err(BioLangError::parser(
                    format!("expected operator, found '{}'", self.current_kind()),
                    self.current_span(),
                ))
            }
        };
        self.advance();
        Ok(op)
    }

    fn current_precedence(&self) -> Precedence {
        match self.current_kind() {
            TokenKind::PipeOp | TokenKind::TapPipe => Precedence::Pipe,
            TokenKind::QuestionQuestion => Precedence::NullCoalesce,
            TokenKind::Or => Precedence::Or,
            TokenKind::And => Precedence::And,
            TokenKind::Caret => Precedence::BitXor,
            TokenKind::Amp => Precedence::BitAnd,
            TokenKind::EqEq | TokenKind::Neq | TokenKind::In | TokenKind::Not => Precedence::Equality,
            TokenKind::Lt | TokenKind::Gt | TokenKind::Le | TokenKind::Ge => {
                Precedence::Comparison
            }
            TokenKind::Shl | TokenKind::Shr => Precedence::Shift,
            TokenKind::As => Precedence::TypeCast,
            TokenKind::DotDot | TokenKind::DotDotEq => Precedence::Range,
            TokenKind::Plus | TokenKind::Minus => Precedence::Addition,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Multiply,
            TokenKind::StarStar => Precedence::Power,
            TokenKind::LParen | TokenKind::Dot | TokenKind::LBracket | TokenKind::QuestionDot => Precedence::Call,
            _ => Precedence::None,
        }
    }

    // ── Type annotations ──────────────────────────────────────────────

    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation> {
        let name = self.expect_ident()?;
        let mut params = Vec::new();

        if self.check(&TokenKind::Lt) {
            self.advance();
            loop {
                params.push(self.parse_type_annotation()?);
                if self.check(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(TokenKind::Gt)?;
        }

        Ok(TypeAnnotation { name, params })
    }

    // ── Parameter list ────────────────────────────────────────────────

    fn parse_params(&mut self) -> Result<Vec<Param>> {
        let mut params = Vec::new();
        self.skip_newlines();

        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            // Check for rest parameter: ...name
            let rest = if self.check(&TokenKind::DotDotDot) {
                self.advance();
                true
            } else {
                false
            };

            let name = self.expect_ident()?;

            let type_ann = if self.check(&TokenKind::Colon) {
                self.advance();
                Some(self.parse_type_annotation()?)
            } else {
                None
            };

            let default = if self.check(&TokenKind::Eq) {
                self.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };

            params.push(Param {
                name,
                type_ann,
                default,
                rest,
            });

            self.skip_newlines();
            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            } else {
                break;
            }

            // Rest param must be last
            if rest {
                break;
            }
        }

        Ok(params)
    }

    // ── Chained comparisons ─────────────────────────────────────────

    fn is_comparison_op(&self, kind: &TokenKind) -> bool {
        matches!(kind, TokenKind::Lt | TokenKind::Gt | TokenKind::Le | TokenKind::Ge
            | TokenKind::EqEq | TokenKind::Neq)
    }

    fn is_comparison_binary_op(&self, op: &BinaryOp) -> bool {
        matches!(op, BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Le | BinaryOp::Ge
            | BinaryOp::Eq | BinaryOp::Neq)
    }

    fn parse_chained_cmp(
        &mut self,
        first: Spanned<Expr>,
        first_op: BinaryOp,
        second: Spanned<Expr>,
    ) -> Result<Spanned<Expr>> {
        let mut operands = vec![first, second];
        let mut ops = vec![first_op];
        while self.is_comparison_op(self.current_kind()) {
            let op = self.parse_binary_op()?;
            let next = self.parse_precedence(Precedence::Comparison)?;
            ops.push(op);
            operands.push(next);
        }
        let span = operands.first().unwrap().span.merge(operands.last().unwrap().span);
        Ok(Spanned::new(Expr::ChainedCmp { operands, ops }, span))
    }

    // ── Ternary expressions ──────────────────────────────────────────

    /// After parsing a full expression, check for ternary: `value if cond else other`
    fn maybe_ternary(&mut self, expr: Spanned<Expr>) -> Result<Spanned<Expr>> {
        if self.check(&TokenKind::If) {
            let start = expr.span;
            self.advance(); // consume 'if'
            let condition = self.parse_precedence(Precedence::Or)?;
            self.expect(TokenKind::Else)?;
            let else_value = self.parse_precedence(Precedence::Pipe)?;
            let span = start.merge(else_value.span);
            Ok(Spanned::new(
                Expr::Ternary {
                    value: Box::new(expr),
                    condition: Box::new(condition),
                    else_value: Box::new(else_value),
                },
                span,
            ))
        } else {
            Ok(expr)
        }
    }

    // ── Helpers ────────────────────────────────────────────────────────

    /// Look past newline tokens to see if a pipe operator follows.
    fn peek_past_newlines_is_pipe(&self) -> bool {
        let mut i = self.pos;
        while i < self.tokens.len() && matches!(self.tokens[i].kind, TokenKind::Newline) {
            i += 1;
        }
        i < self.tokens.len() && matches!(self.tokens[i].kind, TokenKind::PipeOp)
    }

    fn current_kind(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn current_span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn is_at_end(&self) -> bool {
        matches!(self.current_kind(), TokenKind::Eof)
    }

    fn is_at_line_end(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::Newline | TokenKind::Eof | TokenKind::RBrace
        )
    }

    fn advance(&mut self) -> &Token {
        let token = &self.tokens[self.pos];
        if !self.is_at_end() {
            self.pos += 1;
        }
        token
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.current_kind()) == std::mem::discriminant(kind)
    }

    fn expect(&mut self, expected: TokenKind) -> Result<&Token> {
        if self.check(&expected) {
            Ok(self.advance())
        } else {
            Err(BioLangError::new(
                ErrorKind::ExpectedToken,
                format!(
                    "expected '{}', found '{}'",
                    expected,
                    self.current_kind()
                ),
                Some(self.current_span()),
            ))
        }
    }

    fn expect_ident(&mut self) -> Result<String> {
        match self.current_kind().clone() {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(name)
            }
            other => Err(BioLangError::new(
                ErrorKind::ExpectedToken,
                format!("expected identifier, found '{other}'"),
                Some(self.current_span()),
            )),
        }
    }

    /// Accept an identifier or a keyword as a field name (e.g., `.end`, `.in`, `.as`).
    fn expect_field_name(&mut self) -> Result<String> {
        let name = match self.current_kind().clone() {
            TokenKind::Ident(name) => name,
            // Allow keywords as field names after `.`
            ref kw => {
                let s = format!("{kw}");
                if s.chars().all(|c| c.is_ascii_alphabetic() || c == '_') && !s.is_empty() {
                    s
                } else {
                    return Err(BioLangError::new(
                        ErrorKind::ExpectedToken,
                        format!("expected field name, found '{kw}'"),
                        Some(self.current_span()),
                    ));
                }
            }
        };
        self.advance();
        Ok(name)
    }

    /// Lookahead to check if `(name: Type, ...)` follows — used for named tuple returns.
    fn is_named_tuple_return(&self) -> bool {
        // We're at `(`, check for `( Ident Colon ...`
        self.pos + 2 < self.tokens.len()
            && matches!(&self.tokens[self.pos + 1].kind, TokenKind::Ident(_))
            && matches!(&self.tokens[self.pos + 2].kind, TokenKind::Colon)
    }

    fn expect_record_key(&mut self) -> Result<String> {
        let kind = self.current_kind().clone();
        match kind {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(name)
            }
            TokenKind::Str(s) => {
                self.advance();
                Ok(s)
            }
            ref k if Self::keyword_as_name(k).is_some() => {
                let name = Self::keyword_as_name(k).unwrap().to_string();
                self.advance();
                Ok(name)
            }
            other => Err(BioLangError::new(
                ErrorKind::ExpectedToken,
                format!("expected identifier or string key, found '{other}'"),
                Some(self.current_span()),
            )),
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.current_kind(), TokenKind::Newline) {
            self.advance();
        }
    }

    /// Peek past any newlines without consuming them, check if the token matches.
    fn peek_past_newlines_is(&self, kind: &TokenKind) -> bool {
        let mut i = self.pos;
        while i < self.tokens.len() && matches!(self.tokens[i].kind, TokenKind::Newline) {
            i += 1;
        }
        i < self.tokens.len() && std::mem::discriminant(&self.tokens[i].kind) == std::mem::discriminant(kind)
    }

    /// Span of the previously consumed token.
    fn prev_span(&self) -> Span {
        self.tokens[self.pos.saturating_sub(1)].span
    }

    /// `do |params| stmts... end` — multi-statement lambda expression
    fn parse_do_block(&mut self) -> Result<Spanned<Expr>> {
        let start = self.current_span();
        self.expect(TokenKind::Do)?;

        // Optional parameter list: `do |x, y| ... end`
        let params = if self.check(&TokenKind::Bar) {
            self.advance();
            let mut params = Vec::new();
            while !self.check(&TokenKind::Bar) && !self.is_at_end() {
                let rest = if self.check(&TokenKind::DotDotDot) {
                    self.advance();
                    true
                } else {
                    false
                };
                let name = self.expect_ident()?;
                let type_ann = if self.check(&TokenKind::Colon) {
                    self.advance();
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };
                params.push(Param { name, type_ann, default: None, rest });
                if self.check(&TokenKind::Comma) {
                    self.advance();
                }
                if rest { break; }
            }
            self.expect(TokenKind::Bar)?;
            params
        } else {
            Vec::new()
        };

        self.skip_newlines();
        let mut body = Vec::new();
        while !self.check(&TokenKind::End) && !self.is_at_end() {
            body.push(self.parse_stmt()?);
            self.skip_newlines();
        }

        let end = self.current_span();
        self.expect(TokenKind::End)?;

        Ok(Spanned::new(Expr::DoBlock { params, body }, start.merge(end)))
    }
}
