use bl_lexer::{Lexer, TokenKind};

fn lex(input: &str) -> Vec<TokenKind> {
    Lexer::new(input)
        .tokenize()
        .unwrap()
        .into_iter()
        .map(|t| t.kind)
        .filter(|k| !matches!(k, TokenKind::Eof))
        .collect()
}

fn lex_err(input: &str) -> bool {
    Lexer::new(input).tokenize().is_err()
}

// ===== Migrated inline tests =====

#[test]
fn test_simple_let() {
    let tokens = lex("let x = 10");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Let,
            TokenKind::Ident("x".into()),
            TokenKind::Eq,
            TokenKind::Int(10),
        ]
    );
}

#[test]
fn test_pipe_operator() {
    let tokens = lex("x |> f(y)");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("x".into()),
            TokenKind::PipeOp,
            TokenKind::Ident("f".into()),
            TokenKind::LParen,
            TokenKind::Ident("y".into()),
            TokenKind::RParen,
        ]
    );
}

#[test]
fn test_lambda() {
    let tokens = lex("|n| n * 2");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Bar,
            TokenKind::Ident("n".into()),
            TokenKind::Bar,
            TokenKind::Ident("n".into()),
            TokenKind::Star,
            TokenKind::Int(2),
        ]
    );
}

#[test]
fn test_bio_literal() {
    let tokens = lex(r#"dna"ATCG""#);
    assert_eq!(tokens, vec![TokenKind::DnaLit("ATCG".into())]);
}

#[test]
fn test_string() {
    let tokens = lex(r#""hello world""#);
    assert_eq!(tokens, vec![TokenKind::Str("hello world".into())]);
}

#[test]
fn test_float() {
    let tokens = lex("3.14");
    assert_eq!(tokens, vec![TokenKind::Float(3.14)]);
}

#[test]
fn test_newline_as_terminator() {
    let tokens = lex("let x = 1\nlet y = 2");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Let,
            TokenKind::Ident("x".into()),
            TokenKind::Eq,
            TokenKind::Int(1),
            TokenKind::Newline,
            TokenKind::Let,
            TokenKind::Ident("y".into()),
            TokenKind::Eq,
            TokenKind::Int(2),
        ]
    );
}

#[test]
fn test_pipe_suppresses_newline() {
    let tokens = lex("x |>\nf()");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("x".into()),
            TokenKind::PipeOp,
            TokenKind::Ident("f".into()),
            TokenKind::LParen,
            TokenKind::RParen,
        ]
    );
}

#[test]
fn test_named_args() {
    let tokens = lex("f(key: value)");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("f".into()),
            TokenKind::LParen,
            TokenKind::Ident("key".into()),
            TokenKind::Colon,
            TokenKind::Ident("value".into()),
            TokenKind::RParen,
        ]
    );
}

#[test]
fn test_comment() {
    let tokens = lex("let x = 1 # this is a comment\nlet y = 2");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Let,
            TokenKind::Ident("x".into()),
            TokenKind::Eq,
            TokenKind::Int(1),
            TokenKind::Newline,
            TokenKind::Let,
            TokenKind::Ident("y".into()),
            TokenKind::Eq,
            TokenKind::Int(2),
        ]
    );
}

#[test]
fn test_comparison_operators() {
    let tokens = lex("a >= b && c != d");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Ge,
            TokenKind::Ident("b".into()),
            TokenKind::And,
            TokenKind::Ident("c".into()),
            TokenKind::Neq,
            TokenKind::Ident("d".into()),
        ]
    );
}

#[test]
fn test_milestone_expression() {
    let tokens = lex("let x = 10 |> |n| n * 2");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Let,
            TokenKind::Ident("x".into()),
            TokenKind::Eq,
            TokenKind::Int(10),
            TokenKind::PipeOp,
            TokenKind::Bar,
            TokenKind::Ident("n".into()),
            TokenKind::Bar,
            TokenKind::Ident("n".into()),
            TokenKind::Star,
            TokenKind::Int(2),
        ]
    );
}

// ===== Numbers =====

#[test]
fn test_integer_zero() {
    assert_eq!(lex("0"), vec![TokenKind::Int(0)]);
}

#[test]
fn test_large_integer() {
    assert_eq!(lex("9999999999"), vec![TokenKind::Int(9999999999)]);
}

#[test]
fn test_integer_overflow_is_error() {
    // i64 max is 9223372036854775807
    assert!(lex_err("99999999999999999999"));
}

#[test]
fn test_negative_number_is_minus_then_int() {
    // The lexer produces Minus + Int; negation is a parser concern
    let tokens = lex("-5");
    assert_eq!(tokens, vec![TokenKind::Minus, TokenKind::Int(5)]);
}

#[test]
fn test_negative_float_is_minus_then_float() {
    let tokens = lex("-3.14");
    assert_eq!(tokens, vec![TokenKind::Minus, TokenKind::Float(3.14)]);
}

#[test]
fn test_float_scientific_notation() {
    let tokens = lex("1.5e10");
    assert_eq!(tokens, vec![TokenKind::Float(1.5e10)]);
}

#[test]
fn test_float_scientific_negative_exponent() {
    let tokens = lex("2.5E-3");
    assert_eq!(tokens, vec![TokenKind::Float(2.5e-3)]);
}

#[test]
fn test_float_scientific_positive_exponent() {
    let tokens = lex("1e+5");
    assert_eq!(tokens, vec![TokenKind::Float(1e+5)]);
}

#[test]
fn test_integer_with_underscores() {
    assert_eq!(lex("1_000_000"), vec![TokenKind::Int(1_000_000)]);
}

#[test]
fn test_float_with_underscores() {
    assert_eq!(lex("1_000.500_1"), vec![TokenKind::Float(1000.5001)]);
}

#[test]
fn test_integer_scientific_becomes_float() {
    // 5e2 is parsed as float due to scientific notation
    assert_eq!(lex("5e2"), vec![TokenKind::Float(500.0)]);
}

// ===== Bio Literals =====

#[test]
fn test_rna_literal() {
    assert_eq!(lex(r#"rna"AUCG""#), vec![TokenKind::RnaLit("AUCG".into())]);
}

#[test]
fn test_protein_literal() {
    assert_eq!(
        lex(r#"protein"MVLK""#),
        vec![TokenKind::ProteinLit("MVLK".into())]
    );
}

#[test]
fn test_qual_literal() {
    assert_eq!(
        lex(r#"qual"IIIII""#),
        vec![TokenKind::QualLit("IIIII".into())]
    );
}

#[test]
fn test_empty_dna_literal() {
    assert_eq!(lex(r#"dna"""#), vec![TokenKind::DnaLit(String::new())]);
}

#[test]
fn test_unterminated_bio_literal() {
    assert!(lex_err(r#"dna"ATCG"#));
}

#[test]
fn test_bio_literal_newline_is_error() {
    assert!(lex_err("dna\"AT\nCG\""));
}

// ===== Strings =====

#[test]
fn test_empty_string() {
    assert_eq!(lex(r#""""#), vec![TokenKind::Str(String::new())]);
}

#[test]
fn test_string_escape_newline() {
    assert_eq!(lex(r#""\n""#), vec![TokenKind::Str("\n".into())]);
}

#[test]
fn test_string_escape_tab() {
    assert_eq!(lex(r#""\t""#), vec![TokenKind::Str("\t".into())]);
}

#[test]
fn test_string_escape_backslash() {
    assert_eq!(lex(r#""\\""#), vec![TokenKind::Str("\\".into())]);
}

#[test]
fn test_string_escape_quote() {
    assert_eq!(lex(r#""\"""#), vec![TokenKind::Str("\"".into())]);
}

#[test]
fn test_string_escape_carriage_return() {
    assert_eq!(lex(r#""\r""#), vec![TokenKind::Str("\r".into())]);
}

#[test]
fn test_string_escape_null() {
    assert_eq!(lex(r#""\0""#), vec![TokenKind::Str("\0".into())]);
}

#[test]
fn test_string_unicode_escape() {
    // \u{0041} = 'A'
    assert_eq!(lex(r#""\u{0041}""#), vec![TokenKind::Str("A".into())]);
}

#[test]
fn test_string_unicode_escape_emoji() {
    // \u{1F600} = grinning face
    let tokens = lex(r#""\u{1F600}""#);
    assert_eq!(tokens, vec![TokenKind::Str("\u{1F600}".into())]);
}

#[test]
fn test_string_invalid_unicode_escape_empty() {
    assert!(lex_err(r#""\u{}""#));
}

#[test]
fn test_string_invalid_unicode_escape_no_brace() {
    assert!(lex_err(r#""\u0041""#));
}

#[test]
fn test_string_unknown_escape_is_passthrough() {
    // Unknown escapes like \q are passed through literally as \q
    assert_eq!(lex(r#""\q""#), vec![TokenKind::Str("\\q".into())]);
}

#[test]
fn test_string_with_unicode_content() {
    assert_eq!(
        lex("\"hello \u{00e9} world\""),
        vec![TokenKind::Str("hello \u{00e9} world".into())]
    );
}

#[test]
fn test_unterminated_string() {
    assert!(lex_err(r#""hello"#));
}

#[test]
fn test_newline_in_regular_string_is_error() {
    assert!(lex_err("\"hello\nworld\""));
}

#[test]
fn test_triple_quoted_string() {
    let input = "\"\"\"hello\nworld\"\"\"";
    let tokens = lex(input);
    assert_eq!(tokens.len(), 1);
    if let TokenKind::Str(s) = &tokens[0] {
        assert!(s.contains("hello"));
        assert!(s.contains("world"));
    } else {
        panic!("expected Str, got {:?}", tokens[0]);
    }
}

#[test]
fn test_triple_quoted_string_dedent() {
    // The leading newline after """ is stripped, then dedent is applied
    let input = "\"\"\"\n    hello\n    world\n\"\"\"";
    let tokens = lex(input);
    assert_eq!(tokens.len(), 1);
    if let TokenKind::Str(s) = &tokens[0] {
        assert_eq!(s, "hello\nworld");
    } else {
        panic!("expected Str, got {:?}", tokens[0]);
    }
}

#[test]
fn test_unterminated_triple_quoted_string() {
    assert!(lex_err("\"\"\"hello world"));
}

// ===== F-strings =====

#[test]
fn test_fstring_simple() {
    let tokens = lex(r#"f"hello {x}""#);
    assert_eq!(tokens, vec![TokenKind::FStr("hello {x}".into())]);
}

#[test]
fn test_fstring_empty() {
    let tokens = lex(r#"f"""#);
    assert_eq!(tokens, vec![TokenKind::FStr(String::new())]);
}

#[test]
fn test_fstring_with_expression() {
    let tokens = lex(r#"f"result: {a + b}""#);
    assert_eq!(tokens, vec![TokenKind::FStr("result: {a + b}".into())]);
}

#[test]
fn test_fstring_unterminated() {
    assert!(lex_err(r#"f"hello {x}"#));
}

#[test]
fn test_fstring_newline_is_error() {
    assert!(lex_err("f\"hello\nworld\""));
}

// ===== Boolean Literals =====

#[test]
fn test_true_literal() {
    assert_eq!(lex("true"), vec![TokenKind::True]);
}

#[test]
fn test_false_literal() {
    assert_eq!(lex("false"), vec![TokenKind::False]);
}

// ===== Nil =====

#[test]
fn test_nil_literal() {
    assert_eq!(lex("nil"), vec![TokenKind::Nil]);
}

// ===== All Keywords =====

#[test]
fn test_keyword_let() {
    assert_eq!(lex("let"), vec![TokenKind::Let]);
}

#[test]
fn test_keyword_fn() {
    assert_eq!(lex("fn"), vec![TokenKind::Fn]);
}

#[test]
fn test_keyword_if() {
    assert_eq!(lex("if"), vec![TokenKind::If]);
}

#[test]
fn test_keyword_else() {
    assert_eq!(lex("else"), vec![TokenKind::Else]);
}

#[test]
fn test_keyword_for() {
    assert_eq!(lex("for"), vec![TokenKind::For]);
}

#[test]
fn test_keyword_in() {
    assert_eq!(lex("in"), vec![TokenKind::In]);
}

#[test]
fn test_keyword_while() {
    assert_eq!(lex("while"), vec![TokenKind::While]);
}

#[test]
fn test_keyword_return() {
    assert_eq!(lex("return"), vec![TokenKind::Return]);
}

#[test]
fn test_keyword_import() {
    assert_eq!(lex("import"), vec![TokenKind::Import]);
}

#[test]
fn test_keyword_break() {
    assert_eq!(lex("break"), vec![TokenKind::Break]);
}

#[test]
fn test_keyword_continue() {
    assert_eq!(lex("continue"), vec![TokenKind::Continue]);
}

#[test]
fn test_keyword_try() {
    assert_eq!(lex("try"), vec![TokenKind::Try]);
}

#[test]
fn test_keyword_catch() {
    assert_eq!(lex("catch"), vec![TokenKind::Catch]);
}

#[test]
fn test_keyword_struct() {
    assert_eq!(lex("struct"), vec![TokenKind::Struct]);
}

#[test]
fn test_keyword_impl() {
    assert_eq!(lex("impl"), vec![TokenKind::Impl]);
}

#[test]
fn test_keyword_match() {
    assert_eq!(lex("match"), vec![TokenKind::Match]);
}

#[test]
fn test_keyword_assert() {
    assert_eq!(lex("assert"), vec![TokenKind::Assert]);
}

#[test]
fn test_keyword_pipeline() {
    assert_eq!(lex("pipeline"), vec![TokenKind::Pipeline]);
}

#[test]
fn test_keyword_yield() {
    assert_eq!(lex("yield"), vec![TokenKind::Yield]);
}

#[test]
fn test_keyword_enum() {
    assert_eq!(lex("enum"), vec![TokenKind::Enum]);
}

#[test]
fn test_keyword_async() {
    assert_eq!(lex("async"), vec![TokenKind::Async]);
}

#[test]
fn test_keyword_await() {
    assert_eq!(lex("await"), vec![TokenKind::Await]);
}

#[test]
fn test_keyword_trait() {
    assert_eq!(lex("trait"), vec![TokenKind::Trait]);
}

#[test]
fn test_keyword_const() {
    assert_eq!(lex("const"), vec![TokenKind::Const]);
}

#[test]
fn test_keyword_with() {
    assert_eq!(lex("with"), vec![TokenKind::With]);
}

#[test]
fn test_keyword_as_is_keyword() {
    // 'as' is a keyword for type casting
    assert_eq!(lex("as"), vec![TokenKind::As]);
}

// ===== Arithmetic Operators =====

#[test]
fn test_plus() {
    assert_eq!(
        lex("a + b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Plus,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_minus() {
    assert_eq!(
        lex("a - b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Minus,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_star() {
    assert_eq!(
        lex("a * b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Star,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_slash_division() {
    // After an identifier (value-producing), / is division
    assert_eq!(
        lex("a / b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Slash,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_percent() {
    assert_eq!(
        lex("a % b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Percent,
            TokenKind::Ident("b".into()),
        ]
    );
}

// ===== Compound Assignment Operators =====

#[test]
fn test_plus_eq() {
    assert_eq!(
        lex("x += 1"),
        vec![
            TokenKind::Ident("x".into()),
            TokenKind::PlusEq,
            TokenKind::Int(1),
        ]
    );
}

#[test]
fn test_minus_eq() {
    assert_eq!(
        lex("x -= 1"),
        vec![
            TokenKind::Ident("x".into()),
            TokenKind::MinusEq,
            TokenKind::Int(1),
        ]
    );
}

#[test]
fn test_star_eq() {
    assert_eq!(
        lex("x *= 2"),
        vec![
            TokenKind::Ident("x".into()),
            TokenKind::StarEq,
            TokenKind::Int(2),
        ]
    );
}

#[test]
fn test_slash_eq() {
    assert_eq!(
        lex("x /= 2"),
        vec![
            TokenKind::Ident("x".into()),
            TokenKind::SlashEq,
            TokenKind::Int(2),
        ]
    );
}

// ===== Comparison Operators =====

#[test]
fn test_eq_eq() {
    assert_eq!(
        lex("a == b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::EqEq,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_neq() {
    assert_eq!(
        lex("a != b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Neq,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_lt() {
    assert_eq!(
        lex("a < b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Lt,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_gt() {
    assert_eq!(
        lex("a > b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Gt,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_le() {
    assert_eq!(
        lex("a <= b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Le,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_ge() {
    assert_eq!(
        lex("a >= b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Ge,
            TokenKind::Ident("b".into()),
        ]
    );
}

// ===== Logical Operators =====

#[test]
fn test_and() {
    assert_eq!(
        lex("a && b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::And,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_or() {
    assert_eq!(
        lex("a || b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Or,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_bang() {
    assert_eq!(
        lex("!x"),
        vec![TokenKind::Bang, TokenKind::Ident("x".into())]
    );
}

// ===== Range and Spread Operators =====

#[test]
fn test_dot_dot_range() {
    assert_eq!(
        lex("1..10"),
        vec![TokenKind::Int(1), TokenKind::DotDot, TokenKind::Int(10)]
    );
}

#[test]
fn test_dot_dot_eq_inclusive_range() {
    assert_eq!(
        lex("1..=10"),
        vec![TokenKind::Int(1), TokenKind::DotDotEq, TokenKind::Int(10)]
    );
}

#[test]
fn test_dot_dot_dot_spread() {
    assert_eq!(
        lex("...args"),
        vec![TokenKind::DotDotDot, TokenKind::Ident("args".into())]
    );
}

// ===== Tilde Operator =====

#[test]
fn test_tilde() {
    assert_eq!(
        lex("x ~ y"),
        vec![
            TokenKind::Ident("x".into()),
            TokenKind::Tilde,
            TokenKind::Ident("y".into()),
        ]
    );
}

// ===== Arrow Operators =====

#[test]
fn test_arrow() {
    assert_eq!(
        lex("a -> b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Arrow,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_fat_arrow() {
    assert_eq!(
        lex("a => b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::FatArrow,
            TokenKind::Ident("b".into()),
        ]
    );
}

// ===== Null-coalescing and Optional Chaining =====

#[test]
fn test_question_question() {
    assert_eq!(
        lex("a ?? b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::QuestionQuestion,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_question_dot() {
    assert_eq!(
        lex("a?.b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::QuestionDot,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_lone_question_mark_is_error() {
    assert!(lex_err("a ? b"));
}

// ===== At and HashLBrace =====

#[test]
fn test_at_operator() {
    assert_eq!(
        lex("@decorator"),
        vec![TokenKind::At, TokenKind::Ident("decorator".into())]
    );
}

#[test]
fn test_hash_lbrace() {
    assert_eq!(
        lex("#{x}"),
        vec![
            TokenKind::HashLBrace,
            TokenKind::Ident("x".into()),
            TokenKind::RBrace,
        ]
    );
}

// ===== Delimiters =====

#[test]
fn test_all_delimiters() {
    let tokens = lex("( ) { } [ ]");
    assert_eq!(
        tokens,
        vec![
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LBrace,
            TokenKind::RBrace,
            TokenKind::LBracket,
            TokenKind::RBracket,
        ]
    );
}

#[test]
fn test_parens() {
    assert_eq!(lex("()"), vec![TokenKind::LParen, TokenKind::RParen]);
}

#[test]
fn test_braces() {
    assert_eq!(lex("{}"), vec![TokenKind::LBrace, TokenKind::RBrace]);
}

#[test]
fn test_brackets() {
    assert_eq!(lex("[]"), vec![TokenKind::LBracket, TokenKind::RBracket]);
}

#[test]
fn test_double_bar_is_or() {
    // || is the logical OR operator, not two separate Bar tokens
    assert_eq!(lex("||"), vec![TokenKind::Or]);
}

#[test]
fn test_bar_separated_by_ident() {
    // |x| is Bar Ident Bar (lambda params)
    assert_eq!(
        lex("|x|"),
        vec![TokenKind::Bar, TokenKind::Ident("x".into()), TokenKind::Bar]
    );
}

// ===== Record Literal =====

#[test]
fn test_record_literal() {
    let tokens = lex("{key: value}");
    assert_eq!(
        tokens,
        vec![
            TokenKind::LBrace,
            TokenKind::Ident("key".into()),
            TokenKind::Colon,
            TokenKind::Ident("value".into()),
            TokenKind::RBrace,
        ]
    );
}

#[test]
fn test_record_literal_multiple_fields() {
    let tokens = lex("{a: 1, b: 2}");
    assert_eq!(
        tokens,
        vec![
            TokenKind::LBrace,
            TokenKind::Ident("a".into()),
            TokenKind::Colon,
            TokenKind::Int(1),
            TokenKind::Comma,
            TokenKind::Ident("b".into()),
            TokenKind::Colon,
            TokenKind::Int(2),
            TokenKind::RBrace,
        ]
    );
}

// ===== Index Access =====

#[test]
fn test_index_access() {
    let tokens = lex("list[0]");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("list".into()),
            TokenKind::LBracket,
            TokenKind::Int(0),
            TokenKind::RBracket,
        ]
    );
}

// ===== Dot Field Access =====

#[test]
fn test_dot_field_access() {
    let tokens = lex("obj.field");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("obj".into()),
            TokenKind::Dot,
            TokenKind::Ident("field".into()),
        ]
    );
}

// ===== Method Chaining =====

#[test]
fn test_method_chaining() {
    let tokens = lex("obj.method().field");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("obj".into()),
            TokenKind::Dot,
            TokenKind::Ident("method".into()),
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::Dot,
            TokenKind::Ident("field".into()),
        ]
    );
}

// ===== Colon Usage =====

#[test]
fn test_colon_in_named_arg_vs_record() {
    // Both named args and record fields use the same Colon token
    let named = lex("f(x: 1)");
    let record = lex("{x: 1}");
    assert!(named.contains(&TokenKind::Colon));
    assert!(record.contains(&TokenKind::Colon));
}

// ===== Multiple Pipes =====

#[test]
fn test_multiple_pipes() {
    let tokens = lex("a |> b() |> c()");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::PipeOp,
            TokenKind::Ident("b".into()),
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::PipeOp,
            TokenKind::Ident("c".into()),
            TokenKind::LParen,
            TokenKind::RParen,
        ]
    );
}

// ===== Multi-line Pipe Continuation =====

#[test]
fn test_multiline_pipe_continuation() {
    let tokens = lex("a |>\nb() |>\nc()");
    // PipeOp suppresses newline, so no Newline tokens
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::PipeOp,
            TokenKind::Ident("b".into()),
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::PipeOp,
            TokenKind::Ident("c".into()),
            TokenKind::LParen,
            TokenKind::RParen,
        ]
    );
}

// ===== Comments =====

#[test]
fn test_comment_at_end_of_line() {
    let tokens = lex("x = 1 # assign");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("x".into()),
            TokenKind::Eq,
            TokenKind::Int(1),
        ]
    );
}

#[test]
fn test_comment_only_line() {
    let tokens = lex("# just a comment");
    assert_eq!(tokens, vec![]);
}

#[test]
fn test_doc_comment() {
    let tokens = lex("## This is a doc comment");
    assert_eq!(
        tokens,
        vec![TokenKind::DocComment("This is a doc comment".into())]
    );
}

#[test]
fn test_doc_comment_no_space() {
    let tokens = lex("##nospc");
    assert_eq!(tokens, vec![TokenKind::DocComment("nospc".into())]);
}

// ===== Whitespace and Newlines =====

#[test]
fn test_blank_input() {
    let tokens = lex("");
    assert_eq!(tokens, vec![]);
}

#[test]
fn test_whitespace_only_input() {
    let tokens = lex("   \t  ");
    assert_eq!(tokens, vec![]);
}

#[test]
fn test_newline_only_input() {
    let tokens = lex("\n");
    // No preceding token, so newline is suppressed (last_suppresses_newline returns true for None)
    assert_eq!(tokens, vec![]);
}

#[test]
fn test_multiple_consecutive_newlines_collapsed() {
    let tokens = lex("x\n\n\ny");
    // Multiple newlines should collapse into a single Newline token
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("x".into()),
            TokenKind::Newline,
            TokenKind::Ident("y".into()),
        ]
    );
}

#[test]
fn test_carriage_return_newline() {
    let tokens = lex("x\r\ny");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("x".into()),
            TokenKind::Newline,
            TokenKind::Ident("y".into()),
        ]
    );
}

#[test]
fn test_tab_characters_are_whitespace() {
    let tokens = lex("let\tx\t=\t1");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Let,
            TokenKind::Ident("x".into()),
            TokenKind::Eq,
            TokenKind::Int(1),
        ]
    );
}

// ===== Newline Suppression =====

#[test]
fn test_comma_suppresses_newline() {
    let tokens = lex("a,\nb");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Comma,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_open_paren_suppresses_newline() {
    let tokens = lex("f(\nx)");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("f".into()),
            TokenKind::LParen,
            TokenKind::Ident("x".into()),
            TokenKind::RParen,
        ]
    );
}

#[test]
fn test_open_bracket_suppresses_newline() {
    let tokens = lex("[\n1]");
    assert_eq!(
        tokens,
        vec![TokenKind::LBracket, TokenKind::Int(1), TokenKind::RBracket]
    );
}

#[test]
fn test_open_brace_suppresses_newline() {
    let tokens = lex("{\nx}");
    assert_eq!(
        tokens,
        vec![
            TokenKind::LBrace,
            TokenKind::Ident("x".into()),
            TokenKind::RBrace,
        ]
    );
}

#[test]
fn test_eq_suppresses_newline() {
    let tokens = lex("let x =\n10");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Let,
            TokenKind::Ident("x".into()),
            TokenKind::Eq,
            TokenKind::Int(10),
        ]
    );
}

#[test]
fn test_plus_suppresses_newline() {
    let tokens = lex("a +\nb");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Plus,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_and_suppresses_newline() {
    let tokens = lex("a &&\nb");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::And,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_arrow_suppresses_newline() {
    let tokens = lex("fn f() ->\n1");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Fn,
            TokenKind::Ident("f".into()),
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::Arrow,
            TokenKind::Int(1),
        ]
    );
}

#[test]
fn test_colon_suppresses_newline() {
    let tokens = lex("key:\nvalue");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("key".into()),
            TokenKind::Colon,
            TokenKind::Ident("value".into()),
        ]
    );
}

// ===== Regex Literals =====

#[test]
fn test_regex_literal_simple() {
    // At start of input, / is not after a value-producing token, so it is a regex
    let tokens = lex("/abc/");
    assert_eq!(
        tokens,
        vec![TokenKind::RegexLit("abc".into(), String::new())]
    );
}

#[test]
fn test_regex_literal_with_flags() {
    let tokens = lex("/pattern/gim");
    assert_eq!(
        tokens,
        vec![TokenKind::RegexLit("pattern".into(), "gim".into())]
    );
}

#[test]
fn test_regex_literal_with_escape() {
    let tokens = lex(r#"/\d+/"#);
    assert_eq!(
        tokens,
        vec![TokenKind::RegexLit("\\d+".into(), String::new())]
    );
}

#[test]
fn test_regex_vs_division_disambiguation() {
    // After identifier, / is division
    let tokens = lex("a / b");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Slash,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_unterminated_regex() {
    assert!(lex_err("/abc\n"));
}

// ===== Bitwise Ampersand =====

#[test]
fn test_single_ampersand_is_bitwise_and() {
    let tokens = lex("a & b");
    assert_eq!(tokens[0], TokenKind::Ident("a".into()));
    assert_eq!(tokens[1], TokenKind::Amp);
    assert_eq!(tokens[2], TokenKind::Ident("b".into()));
}

// ===== Invalid Character =====

#[test]
fn test_invalid_character() {
    assert!(lex_err("`"));
}

#[test]
fn test_invalid_character_backtick() {
    assert!(lex_err("let x = `value`"));
}

// ===== Identifiers =====

#[test]
fn test_identifier_with_underscores() {
    assert_eq!(
        lex("my_var"),
        vec![TokenKind::Ident("my_var".into())]
    );
}

#[test]
fn test_identifier_leading_underscore() {
    assert_eq!(
        lex("_private"),
        vec![TokenKind::Ident("_private".into())]
    );
}

#[test]
fn test_identifier_with_digits() {
    assert_eq!(
        lex("var123"),
        vec![TokenKind::Ident("var123".into())]
    );
}

#[test]
fn test_identifier_not_keyword_prefix() {
    // "letter" starts with "let" but is not the keyword
    assert_eq!(lex("letter"), vec![TokenKind::Ident("letter".into())]);
}

#[test]
fn test_identifier_not_keyword_suffix() {
    // "returning" contains "return" but is not the keyword
    assert_eq!(lex("returning"), vec![TokenKind::Ident("returning".into())]);
}

#[test]
fn test_unicode_identifier_not_supported() {
    // Identifiers only support ASCII alpha + underscore start
    // A unicode char like a bare non-ASCII letter should error
    assert!(lex_err("\u{00e9}var"));
}

// ===== Complex Expressions =====

#[test]
fn test_if_else_expression() {
    let tokens = lex("if x > 0 { x } else { -x }");
    assert_eq!(
        tokens,
        vec![
            TokenKind::If,
            TokenKind::Ident("x".into()),
            TokenKind::Gt,
            TokenKind::Int(0),
            TokenKind::LBrace,
            TokenKind::Ident("x".into()),
            TokenKind::RBrace,
            TokenKind::Else,
            TokenKind::LBrace,
            TokenKind::Minus,
            TokenKind::Ident("x".into()),
            TokenKind::RBrace,
        ]
    );
}

#[test]
fn test_for_in_loop() {
    let tokens = lex("for x in items { x }");
    assert_eq!(
        tokens,
        vec![
            TokenKind::For,
            TokenKind::Ident("x".into()),
            TokenKind::In,
            TokenKind::Ident("items".into()),
            TokenKind::LBrace,
            TokenKind::Ident("x".into()),
            TokenKind::RBrace,
        ]
    );
}

#[test]
fn test_fn_definition() {
    let tokens = lex("fn add(a, b) { a + b }");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Fn,
            TokenKind::Ident("add".into()),
            TokenKind::LParen,
            TokenKind::Ident("a".into()),
            TokenKind::Comma,
            TokenKind::Ident("b".into()),
            TokenKind::RParen,
            TokenKind::LBrace,
            TokenKind::Ident("a".into()),
            TokenKind::Plus,
            TokenKind::Ident("b".into()),
            TokenKind::RBrace,
        ]
    );
}

#[test]
fn test_match_expression() {
    let tokens = lex("match x { 1 => a, 2 => b }");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Match,
            TokenKind::Ident("x".into()),
            TokenKind::LBrace,
            TokenKind::Int(1),
            TokenKind::FatArrow,
            TokenKind::Ident("a".into()),
            TokenKind::Comma,
            TokenKind::Int(2),
            TokenKind::FatArrow,
            TokenKind::Ident("b".into()),
            TokenKind::RBrace,
        ]
    );
}

#[test]
fn test_import_statement() {
    let tokens = lex(r#"import "path""#);
    assert_eq!(
        tokens,
        vec![TokenKind::Import, TokenKind::Str("path".into())]
    );
}

#[test]
fn test_struct_definition() {
    let tokens = lex("struct Point { x, y }");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Struct,
            TokenKind::Ident("Point".into()),
            TokenKind::LBrace,
            TokenKind::Ident("x".into()),
            TokenKind::Comma,
            TokenKind::Ident("y".into()),
            TokenKind::RBrace,
        ]
    );
}

#[test]
fn test_try_catch() {
    let tokens = lex("try { f() } catch { nil }");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Try,
            TokenKind::LBrace,
            TokenKind::Ident("f".into()),
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::RBrace,
            TokenKind::Catch,
            TokenKind::LBrace,
            TokenKind::Nil,
            TokenKind::RBrace,
        ]
    );
}

// ===== Span Correctness =====

#[test]
fn test_spans_are_correct() {
    let tokens = Lexer::new("let x = 10")
        .tokenize()
        .unwrap();
    // "let" spans 0..3
    assert_eq!(tokens[0].span.start, 0);
    assert_eq!(tokens[0].span.end, 3);
    // "x" spans 4..5
    assert_eq!(tokens[1].span.start, 4);
    assert_eq!(tokens[1].span.end, 5);
    // "=" spans 6..7
    assert_eq!(tokens[2].span.start, 6);
    assert_eq!(tokens[2].span.end, 7);
    // "10" spans 8..10
    assert_eq!(tokens[3].span.start, 8);
    assert_eq!(tokens[3].span.end, 10);
}

// ===== Eof Token =====

#[test]
fn test_eof_always_present() {
    let tokens = Lexer::new("x").tokenize().unwrap();
    assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
}

#[test]
fn test_empty_input_has_eof() {
    let tokens = Lexer::new("").tokenize().unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Eof);
}

// ===== Pipe + Bar Disambiguation =====

#[test]
fn test_pipe_op_vs_bar_vs_or() {
    let tokens = lex("a |> |x| x || y");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::PipeOp,
            TokenKind::Bar,
            TokenKind::Ident("x".into()),
            TokenKind::Bar,
            TokenKind::Ident("x".into()),
            TokenKind::Or,
            TokenKind::Ident("y".into()),
        ]
    );
}

// ===== Eq vs EqEq vs FatArrow =====

#[test]
fn test_eq_eqeq_fatarrow_disambiguation() {
    let tokens = lex("x = y == z => w");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("x".into()),
            TokenKind::Eq,
            TokenKind::Ident("y".into()),
            TokenKind::EqEq,
            TokenKind::Ident("z".into()),
            TokenKind::FatArrow,
            TokenKind::Ident("w".into()),
        ]
    );
}

// ===== Bang vs Neq =====

#[test]
fn test_bang_vs_neq() {
    let tokens = lex("!a != b");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Bang,
            TokenKind::Ident("a".into()),
            TokenKind::Neq,
            TokenKind::Ident("b".into()),
        ]
    );
}

// ===== Dot vs DotDot vs DotDotDot vs DotDotEq =====

#[test]
fn test_dot_dotdot_dotdotdot_disambiguation() {
    let tokens = lex("a.b 1..10 ...rest 1..=5");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Dot,
            TokenKind::Ident("b".into()),
            TokenKind::Int(1),
            TokenKind::DotDot,
            TokenKind::Int(10),
            TokenKind::DotDotDot,
            TokenKind::Ident("rest".into()),
            TokenKind::Int(1),
            TokenKind::DotDotEq,
            TokenKind::Int(5),
        ]
    );
}

// ===== Minus vs Arrow vs MinusEq =====

#[test]
fn test_minus_vs_arrow_vs_minuseq() {
    let tokens = lex("a - b -> c -= d");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Minus,
            TokenKind::Ident("b".into()),
            TokenKind::Arrow,
            TokenKind::Ident("c".into()),
            TokenKind::MinusEq,
            TokenKind::Ident("d".into()),
        ]
    );
}

// ===== Number edge cases =====

#[test]
fn test_number_followed_by_dot_not_digit() {
    // "42.x" should be Int(42) Dot Ident(x) -- dot followed by non-digit is not float
    let tokens = lex("42.x");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Int(42),
            TokenKind::Dot,
            TokenKind::Ident("x".into()),
        ]
    );
}

#[test]
fn test_number_dot_dot_is_range() {
    // "1..10": the number parser sees "1", then ".." is not a float (next after . is .)
    let tokens = lex("1..10");
    assert_eq!(
        tokens,
        vec![TokenKind::Int(1), TokenKind::DotDot, TokenKind::Int(10)]
    );
}

// ===== Various newline suppression edge cases =====

#[test]
fn test_leading_newlines_suppressed() {
    let tokens = lex("\n\nlet x = 1");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Let,
            TokenKind::Ident("x".into()),
            TokenKind::Eq,
            TokenKind::Int(1),
        ]
    );
}

#[test]
fn test_trailing_newlines_preserved() {
    let tokens = lex("x\n");
    // After "x" (value-producing), the newline should be emitted
    assert_eq!(
        tokens,
        vec![TokenKind::Ident("x".into()), TokenKind::Newline]
    );
}

#[test]
fn test_double_newline_after_value_collapsed() {
    let tokens = lex("x\n\n");
    // Multiple newlines after a value-producing token should collapse to one
    assert_eq!(
        tokens,
        vec![TokenKind::Ident("x".into()), TokenKind::Newline]
    );
}

// ===== Regression: all suppressing tokens =====

#[test]
fn test_star_suppresses_newline() {
    let tokens = lex("a *\nb");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Star,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_percent_suppresses_newline() {
    let tokens = lex("a %\nb");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Percent,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_minus_suppresses_newline() {
    let tokens = lex("a -\nb");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Minus,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_or_suppresses_newline() {
    let tokens = lex("a ||\nb");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Or,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_fat_arrow_suppresses_newline() {
    let tokens = lex("a =>\nb");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::FatArrow,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_dotdot_suppresses_newline() {
    let tokens = lex("1..\n10");
    assert_eq!(
        tokens,
        vec![TokenKind::Int(1), TokenKind::DotDot, TokenKind::Int(10)]
    );
}

#[test]
fn test_tilde_suppresses_newline() {
    let tokens = lex("a ~\nb");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Tilde,
            TokenKind::Ident("b".into()),
        ]
    );
}

#[test]
fn test_question_question_suppresses_newline() {
    let tokens = lex("a ??\nb");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::QuestionQuestion,
            TokenKind::Ident("b".into()),
        ]
    );
}

// ===== TokenKind Display =====

#[test]
fn test_tokenkind_display() {
    assert_eq!(format!("{}", TokenKind::PipeOp), "|>");
    assert_eq!(format!("{}", TokenKind::And), "&&");
    assert_eq!(format!("{}", TokenKind::Or), "||");
    assert_eq!(format!("{}", TokenKind::DotDotDot), "...");
    assert_eq!(format!("{}", TokenKind::FatArrow), "=>");
    assert_eq!(format!("{}", TokenKind::Int(42)), "42");
    assert_eq!(format!("{}", TokenKind::Str("hi".into())), "\"hi\"");
    assert_eq!(format!("{}", TokenKind::DnaLit("ATG".into())), "dna\"ATG\"");
    assert_eq!(format!("{}", TokenKind::Newline), "\\n");
    assert_eq!(format!("{}", TokenKind::Eof), "EOF");
}

// ===== Newline suppression before pipe operators =====

#[test]
fn test_newline_suppressed_before_pipe() {
    // `expr\n|> f()` should have NO newline between expr and |>
    let tokens = lex("x\n    |> f()");
    assert!(
        !tokens.contains(&TokenKind::Newline),
        "newline before |> should be suppressed, got: {:?}",
        tokens
    );
    assert!(tokens.contains(&TokenKind::PipeOp));
}

#[test]
fn test_newline_suppressed_before_tap_pipe() {
    let tokens = lex("x\n    |>> f()");
    assert!(
        !tokens.contains(&TokenKind::Newline),
        "newline before |>> should be suppressed, got: {:?}",
        tokens
    );
    assert!(tokens.contains(&TokenKind::TapPipe));
}

#[test]
fn test_multiple_newlines_before_pipe() {
    // Multiple blank lines before |> should all be suppressed
    let tokens = lex("x\n\n\n    |> f()");
    assert!(
        !tokens.contains(&TokenKind::Newline),
        "multiple newlines before |> should be suppressed, got: {:?}",
        tokens
    );
}

#[test]
fn test_newline_preserved_without_pipe() {
    // Normal newlines (not before pipe) should be preserved
    let tokens = lex("x\ny");
    assert!(
        tokens.contains(&TokenKind::Newline),
        "normal newline should be preserved, got: {:?}",
        tokens
    );
}

#[test]
fn test_pipe_chain_multiline() {
    let tokens = lex("data\n    |> filter(f)\n    |> map(g)");
    // Should have exactly 2 PipeOp tokens and 0 Newline tokens
    let pipe_count = tokens.iter().filter(|t| matches!(t, TokenKind::PipeOp)).count();
    let nl_count = tokens.iter().filter(|t| matches!(t, TokenKind::Newline)).count();
    assert_eq!(pipe_count, 2, "expected 2 pipe ops");
    assert_eq!(nl_count, 0, "expected 0 newlines (all before pipes)");
}
