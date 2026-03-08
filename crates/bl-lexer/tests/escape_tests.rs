use bl_lexer::Lexer;
use bl_lexer::token::TokenKind;

#[test]
fn test_basic_escapes() {
    let tokens = Lexer::new(r#""\n\t\r\\\"\0""#).tokenize().unwrap();
    match &tokens[0].kind {
        TokenKind::Str(s) => assert_eq!(s, "\n\t\r\\\"\0"),
        other => panic!("expected Str, got {other:?}"),
    }
}

#[test]
fn test_unicode_escape() {
    let tokens = Lexer::new(r#""\u{0041}\u{03B1}\u{1F600}""#).tokenize().unwrap();
    match &tokens[0].kind {
        TokenKind::Str(s) => assert_eq!(s, "A\u{03B1}\u{1F600}"),
        other => panic!("expected Str, got {other:?}"),
    }
}

#[test]
fn test_unicode_escape_short() {
    let tokens = Lexer::new(r#""\u{41}""#).tokenize().unwrap();
    match &tokens[0].kind {
        TokenKind::Str(s) => assert_eq!(s, "A"),
        other => panic!("expected Str, got {other:?}"),
    }
}

#[test]
fn test_invalid_unicode_escape() {
    let result = Lexer::new(r#""\u{ZZZZ}""#).tokenize();
    assert!(result.is_err());
}

#[test]
fn test_unicode_missing_open_brace() {
    let result = Lexer::new(r#""\u0041}""#).tokenize();
    assert!(result.is_err());
}

#[test]
fn test_unicode_missing_close_brace() {
    let result = Lexer::new(r#""\u{0041""#).tokenize();
    assert!(result.is_err());
}

#[test]
fn test_unicode_empty_braces() {
    let result = Lexer::new(r#""\u{}""#).tokenize();
    assert!(result.is_err());
}

#[test]
fn test_unicode_invalid_code_point() {
    // U+D800 is a surrogate, not a valid char
    let result = Lexer::new(r#""\u{D800}""#).tokenize();
    assert!(result.is_err());
}

#[test]
fn test_null_escape() {
    let tokens = Lexer::new(r#""\0""#).tokenize().unwrap();
    match &tokens[0].kind {
        TokenKind::Str(s) => assert_eq!(s, "\0"),
        other => panic!("expected Str, got {other:?}"),
    }
}

#[test]
fn test_unicode_in_triple_string() {
    let input = r#""""\u{03B1}\u{03B2}""""#;
    let tokens = Lexer::new(input).tokenize().unwrap();
    match &tokens[0].kind {
        TokenKind::Str(s) => assert_eq!(s, "\u{03B1}\u{03B2}"),
        other => panic!("expected Str, got {other:?}"),
    }
}

#[test]
fn test_null_in_fstring() {
    let tokens = Lexer::new(r#"f"hello\0world""#).tokenize().unwrap();
    match &tokens[0].kind {
        TokenKind::FStr(s) => assert_eq!(s, "hello\0world"),
        other => panic!("expected FStr, got {other:?}"),
    }
}

#[test]
fn test_unicode_in_fstring() {
    let tokens = Lexer::new(r#"f"\u{0041}""#).tokenize().unwrap();
    match &tokens[0].kind {
        TokenKind::FStr(s) => assert_eq!(s, "A"),
        other => panic!("expected FStr, got {other:?}"),
    }
}
