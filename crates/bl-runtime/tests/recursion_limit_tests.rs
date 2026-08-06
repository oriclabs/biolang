//! Runaway recursion must report an error, not kill the process.
//!
//! The interpreter spends several Rust frames per BioLang call - about 342 KB
//! per level, down from 477 KB once the fattest match arms were split out - so
//! `fn f(n) { f(n-1) }` used to abort with "thread
//! 'bl-main' has overflowed its stack" somewhere past a hundred levels. That is
//! an operating-system message: no line number, no span, nothing a program can
//! catch, and nothing telling the reader which function ran away.

use bl_core::error::ErrorKind;
use bl_lexer::Lexer;
use bl_parser::Parser;
use bl_runtime::Interpreter;

/// Run on a thread with the stack `bl-cli` gives the interpreter.
///
/// The test harness's own threads get 2 MB, and at ~342 KB of stack per BioLang
/// call that is exhausted after a handful of levels - long before
/// MAX_CALL_DEPTH is reached, so the guard would never be exercised and every
/// test here would die the way the guard exists to prevent.
///
/// That is worth stating plainly: the depth limit is calibrated to bl-cli's
/// stack, so any other embedder of this interpreter (the WASM build, the
/// workbench, a test harness) still overflows far earlier. The per-level stack
/// cost is the underlying defect; the guard only converts the crash into an
/// error for callers who provide the stack it assumes.
fn run(code: &str) -> Result<bl_core::value::Value, bl_core::error::BioLangError> {
    let source = code.to_string();
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(move || {
            let tokens = Lexer::new(&source).tokenize().unwrap();
            let parsed = Parser::new(tokens).parse().unwrap();
            Interpreter::new().run(&parsed.program)
        })
        .expect("spawn interpreter thread")
        .join()
        .expect("interpreter thread panicked")
}

#[test]
fn ordinary_recursion_still_works() {
    // Well under the limit, and the depth a tree walk plausibly reaches.
    let result = run("fn cd(n) { if n <= 0 { 0 } else { cd(n - 1) + 1 } }\ncd(300)").unwrap();
    assert_eq!(format!("{result}"), "300");
}

#[test]
fn runaway_recursion_errors_instead_of_aborting() {
    let err = run("fn boom(n) { boom(n + 1) }\nboom(0)").unwrap_err();
    assert_eq!(err.kind, ErrorKind::RecursionLimit);
    assert!(
        err.message.contains("recursion depth"),
        "unhelpful message: {}",
        err.message
    );
    // The span matters: it is what points at the offending call.
    assert!(err.span.is_some(), "recursion error carried no span");
}

#[test]
fn mutual_recursion_is_caught_too() {
    let err = run("fn ping(n) { pong(n + 1) }\nfn pong(n) { ping(n + 1) }\nping(0)").unwrap_err();
    assert_eq!(err.kind, ErrorKind::RecursionLimit);
}

#[test]
fn deep_traces_are_elided_rather_than_dumped() {
    // A 400-frame trace printed in full buries the error message hundreds of
    // lines above it, which is the one thing the reader needs to see.
    let err = run("fn boom(n) { boom(n + 1) }\nboom(0)").unwrap_err();
    let rendered = err.format_with_source("fn boom(n) { boom(n + 1) }\nboom(0)");
    let lines = rendered.lines().count();
    assert!(
        lines < 40,
        "error rendered as {lines} lines; deep traces should be elided"
    );
    assert!(
        rendered.contains("more frames"),
        "expected an elision marker in:\n{rendered}"
    );
}
