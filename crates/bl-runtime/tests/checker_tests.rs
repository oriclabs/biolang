use bl_lexer::Lexer;
use bl_parser::Parser;
use bl_runtime::checker::Checker;

fn check(code: &str) -> Vec<bl_runtime::checker::TypeWarning> {
    let tokens = Lexer::new(code).tokenize().unwrap();
    let result = Parser::new(tokens).parse().unwrap();
    let mut checker = Checker::new();
    checker.check(&result.program)
}

#[test]
fn test_correct_annotation_no_warning() {
    let warnings = check("let x: Int = 42");
    assert!(warnings.is_empty());
}

#[test]
fn test_wrong_annotation_warns() {
    let warnings = check("let x: Int = \"hello\"");
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("type mismatch"));
}

#[test]
fn test_no_annotation_no_check() {
    let warnings = check("let x = \"hello\"");
    assert!(warnings.is_empty());
}

#[test]
fn test_float_accepts_int() {
    let warnings = check("let x: Float = 42");
    assert!(warnings.is_empty());
}

#[test]
fn test_fn_param_default_mismatch() {
    let warnings = check("fn f(x: Int = \"bad\") { x }");
    assert_eq!(warnings.len(), 1);
}

#[test]
fn test_dna_literal_type() {
    let warnings = check("let s: DNA = dna\"ATCG\"");
    assert!(warnings.is_empty());
}

#[test]
fn test_dna_wrong_type() {
    let warnings = check("let s: DNA = 42");
    assert_eq!(warnings.len(), 1);
}
