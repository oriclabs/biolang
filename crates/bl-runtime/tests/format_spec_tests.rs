//! f-string format specifiers: `{value:.3f}`, `{value:>10}`, `{p:.2e}`.
//!
//! Before these existed the parser accepted a spec and threw it away, so
//! `{mu:.3f}` silently printed all 17 digits of the double. Every expectation
//! below was checked against CPython's `format()` and must stay byte-identical
//! to it — the audience reads these numbers next to Python output.

use bl_core::value::Value;
use bl_lexer::Lexer;
use bl_parser::Parser;
use bl_runtime::Interpreter;

fn eval(code: &str) -> String {
    let tokens = Lexer::new(code).tokenize().unwrap();
    let result = Parser::new(tokens).parse().unwrap();
    let mut interp = Interpreter::new();
    match interp.run(&result.program).unwrap() {
        Value::Str(s) => s,
        other => panic!("expected a string, got {other:?}"),
    }
}

/// The parser recovers rather than bailing, so a rejected spec shows up as a
/// collected diagnostic. The CLI refuses to run a program with any.
fn parse_errors(code: &str) -> String {
    let tokens = Lexer::new(code).tokenize().unwrap();
    let result = Parser::new(tokens).parse().expect("lexes and parses");
    assert!(result.has_errors(), "expected `{code}` to be rejected");
    result
        .errors
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join(
            "
",
        )
}

#[test]
fn fixed_precision_rounds_and_keeps_trailing_zeros() {
    assert_eq!(eval(r#"f"{0.5087592529939176:.3f}""#), "0.509");
    // The trailing zero is the whole point: round() would give "0.5" here and
    // a column of numbers would not line up.
    assert_eq!(eval(r#"f"{0.5:.3f}""#), "0.500");
    assert_eq!(eval(r#"f"{0.05925528:.4f}""#), "0.0593");
    assert_eq!(eval(r#"f"{2.675:.2f}""#), "2.67");
    assert_eq!(eval(r#"f"{1.5:.0f}""#), "2");
}

#[test]
fn precision_applies_to_ints_too() {
    assert_eq!(eval(r#"f"{42:.2f}""#), "42.00");
}

#[test]
fn percent_multiplies_by_a_hundred() {
    assert_eq!(eval(r#"f"{0.4567:.2%}""#), "45.67%");
    assert_eq!(eval(r#"f"{1.0:.1%}""#), "100.0%");
    assert_eq!(eval(r#"f"{0.005:.0%}""#), "0%");
}

#[test]
fn exponent_form_matches_python_not_rust() {
    // Rust's own `{:e}` gives `1.20e-5`. Python gives `1.20e-05`, and p-values
    // in the biostatistics books are read alongside Python's.
    assert_eq!(eval(r#"f"{0.000012:.2e}""#), "1.20e-05");
    assert_eq!(eval(r#"f"{12345.6789:.2e}""#), "1.23e+04");
    assert_eq!(eval(r#"f"{0.0:.2e}""#), "0.00e+00");
    assert_eq!(eval(r#"f"{-0.00034:.2e}""#), "-3.40e-04");
}

#[test]
fn width_and_alignment() {
    assert_eq!(eval(r#"f"[{0.5087:>10.3f}]""#), "[     0.509]");
    assert_eq!(eval(r#"f"[{0.5087:<10.3f}]""#), "[0.509     ]");
    assert_eq!(eval(r#"f"[{0.5087:^10.3f}]""#), "[  0.509   ]");
    // Width applies to non-numbers; precision and type do not.
    assert_eq!(eval(r#"f"[{"abc":>8}]""#), "[     abc]");
}

#[test]
fn width_narrower_than_the_value_does_not_truncate() {
    assert_eq!(eval(r#"f"[{123456:>3}]""#), "[123456]");
}

#[test]
fn a_spec_is_optional_and_absent_means_unchanged() {
    assert_eq!(eval(r#"f"{0.5087592529939176}""#), "0.5087592529939176");
}

#[test]
fn colons_inside_the_expression_are_not_specs() {
    // A dict literal and a slice both contain a colon that must not be read as
    // the start of a format spec.
    assert_eq!(eval(r#"f"{ {"a": 2}["a"] }""#), "2");
    assert_eq!(eval(r#"f"{"abcdef"[1:3]}""#), "bc");
}

#[test]
fn a_colon_in_a_string_literal_is_not_a_spec() {
    assert_eq!(eval(r#"f"{"12:30"}""#), "12:30");
}

#[test]
fn unsupported_specs_are_rejected_rather_than_ignored() {
    // Silently ignoring the spec is the bug this feature fixes, so an
    // unrecognised one has to be loud.
    assert!(parse_errors(r#"f"{1.0:.3q}""#).contains("not a supported format type"));
    assert!(parse_errors(r#"f"{1.0:zz}""#).contains("not a supported format type"));
}

#[test]
fn a_bare_precision_means_decimal_places_not_significant_digits() {
    // The one deliberate divergence from Python, which reads `.1` as one
    // significant digit and renders 95.23 as `1e+02`.
    assert_eq!(eval(r#"f"{0.9523:.4}""#), "0.9523");
    assert_eq!(eval(r#"f"{95.23:.1}""#), "95.2");
}
