//! The Python converter emitted things BioLang does not have, while reporting
//! that there was nothing to review. Both were found by following a clippy
//! warning about three `replace` calls that replaced text with itself.

use bl_import::convert;

fn python(source: &str) -> String {
    convert(source, "python", "test.py")
}

#[test]
fn none_becomes_nil_not_null() {
    // BioLang has no `null`. Emitting it produced a program that failed on an
    // undefined variable at the first use.
    let out = python("x = None\n");
    assert!(out.contains("nil"), "expected nil in:\n{out}");
    assert!(!out.contains("null"), "null should not survive:\n{out}");
}

#[test]
fn a_bare_return_becomes_nil() {
    let out = python("def f():\n    return\n");
    assert!(!out.contains("null"), "null should not survive:\n{out}");
}

#[test]
fn is_none_becomes_an_equality_test() {
    // `is` is not a BioLang operator, so this used to be a parse error.
    let out = python("def f(x):\n    if x is None:\n        return 1\n");
    assert!(out.contains("== nil"), "expected `== nil` in:\n{out}");
    assert!(!out.contains(" is "), "`is` should not survive:\n{out}");
}

#[test]
fn is_not_none_becomes_an_inequality_test() {
    // Order matters: replacing ` is ` first would leave a stray `not`.
    let out = python("def f(x):\n    if x is not None:\n        return 1\n");
    assert!(out.contains("!= nil"), "expected `!= nil` in:\n{out}");
    assert!(!out.contains(" is "), "`is` should not survive:\n{out}");
}

#[test]
fn python_boolean_operators_carry_over_unchanged() {
    // These are spelled the same in both languages, which is why the three
    // replacements that named them were doing nothing.
    let out = python("x = a and b or not c\n");
    assert!(out.contains("and"), "expected `and` in:\n{out}");
    assert!(out.contains("or"), "expected `or` in:\n{out}");
    assert!(out.contains("not"), "expected `not` in:\n{out}");
}

#[test]
fn true_and_false_are_lowercased() {
    let out = python("x = True\ny = False\n");
    assert!(out.contains("true"), "expected true in:\n{out}");
    assert!(out.contains("false"), "expected false in:\n{out}");
}
