//! Piping into a user-defined function must not cost more than calling it.
//!
//! `x |> f()` reaches the function through `call_value`, while `f(x)` goes
//! through `eval_call`. `call_value` probed for a `__named_returns_{name}`
//! binding using `env.get`, which runs a "did you mean?" search over every name
//! in scope when it misses - and it misses on every ordinary function. With
//! roughly a thousand builtins bound, that search cost about 9ms per call, so
//! `range(0, 300) |> map(|i| i |> f())` took six seconds where the direct call
//! took three milliseconds: a factor of about 1750.
//!
//! Only named functions were affected, because lambdas carry no name and skipped
//! the probe entirely - which is what made the asymmetry visible.
//!
//! These are wall-clock tests, which are ordinarily a poor idea. They are
//! justified here because the regression they guard against is three orders of
//! magnitude, and the bounds below are loose enough that ordinary machine noise
//! or a debug build cannot trip them.

use bl_lexer::Lexer;
use bl_parser::Parser;
use bl_runtime::Interpreter;
use std::time::{Duration, Instant};

fn time_source(code: &str) -> Duration {
    let tokens = Lexer::new(code).tokenize().unwrap();
    let parsed = Parser::new(tokens).parse().unwrap();
    let mut interp = Interpreter::new();
    let start = Instant::now();
    interp.run(&parsed.program).unwrap();
    start.elapsed()
}

const CALLS: usize = 2000;

#[test]
fn piping_into_a_named_function_is_not_pathologically_slow() {
    let code =
        format!("fn f(x) {{ x + 1 }}\nlet r = range(0, {CALLS}) |> map(|i| i |> f())\nlen(r)");
    let elapsed = time_source(&code);
    // Before the fix this took roughly 40 seconds for 2000 calls.
    assert!(
        elapsed < Duration::from_secs(5),
        "{CALLS} piped calls took {elapsed:?}; a direct call takes milliseconds"
    );
}

#[test]
fn piping_costs_about_the_same_as_calling() {
    let piped = time_source(&format!(
        "fn f(x) {{ x + 1 }}\nlet r = range(0, {CALLS}) |> map(|i| i |> f())\nlen(r)"
    ));
    let direct = time_source(&format!(
        "fn f(x) {{ x + 1 }}\nlet r = range(0, {CALLS}) |> map(|i| f(i))\nlen(r)"
    ));

    // A generous ceiling: the two paths differ by a little dispatch overhead,
    // never by orders of magnitude. Timings this small are noisy, so compare
    // against a floor rather than dividing by a possibly sub-millisecond value.
    let allowed = direct.max(Duration::from_millis(50)) * 20;
    assert!(
        piped < allowed,
        "piped {piped:?} vs direct {direct:?}: piping should not be dramatically slower"
    );
}

#[test]
fn piping_into_a_lambda_stays_fast() {
    // Lambdas were always fast, because they have no name to probe for. Guard
    // it so a future fix for named functions cannot regress the unnamed path.
    let code = format!("let g = |x| x + 1\nlet r = range(0, {CALLS}) |> map(|i| i |> g())\nlen(r)");
    let elapsed = time_source(&code);
    assert!(
        elapsed < Duration::from_secs(5),
        "{CALLS} piped lambda calls took {elapsed:?}"
    );
}

#[test]
fn piping_still_produces_the_right_answer() {
    // Speed is worthless if the semantics moved.
    let tokens = Lexer::new("fn double(x) { x * 2 }\n[1, 2, 3] |> map(|i| i |> double())")
        .tokenize()
        .unwrap();
    let parsed = Parser::new(tokens).parse().unwrap();
    let mut interp = Interpreter::new();
    let result = interp.run(&parsed.program).unwrap();
    assert_eq!(format!("{result}"), "[2, 4, 6]");
}
