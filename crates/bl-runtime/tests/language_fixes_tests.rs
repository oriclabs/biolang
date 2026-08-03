//! Regressions for four language problems found by writing the Rosalind packs.
//!
//! Two of them produced wrong answers rather than errors, which is why they
//! survived so long: the pipe precedence bug turned a comparison into an extra
//! argument, and `str()` on a sequence returned the type wrapper, so printed
//! results silently disagreed with the values beside them.

use bl_core::value::Value;
use bl_lexer::Lexer;
use bl_parser::Parser;
use bl_runtime::Interpreter;

fn eval(code: &str) -> Value {
    let tokens = Lexer::new(code).tokenize().unwrap();
    let parsed = Parser::new(tokens).parse().unwrap();
    assert!(
        !parsed.has_errors(),
        "unexpected parse errors: {:?}",
        parsed.errors
    );
    let mut interp = Interpreter::new();
    interp.run(&parsed.program).unwrap()
}

fn eval_str(code: &str) -> String {
    match eval(code) {
        Value::Str(s) => s,
        other => panic!("expected a string, got {other:?}"),
    }
}

fn eval_int(code: &str) -> i64 {
    match eval(code) {
        Value::Int(n) => n,
        other => panic!("expected an int, got {other:?}"),
    }
}

// ── A pipe binds tighter than any binary operator ───────────────────

#[test]
fn pipe_result_is_the_left_side_of_a_comparison() {
    // Parsed as `[1,2,3] |> (len() == 3)` before, which reached len() with an
    // extra argument and failed on arity.
    assert_eq!(eval("[1, 2, 3] |> len() == 3"), Value::Bool(true));
    assert_eq!(eval("[1, 2, 3] |> len() == 9"), Value::Bool(false));
}

#[test]
fn pipe_result_is_the_left_side_of_arithmetic() {
    assert_eq!(eval_int("[1, 2, 3, 4] |> sum() % 7"), 3);
    assert_eq!(eval_int("[1, 2, 3] |> len() + 10"), 13);
    assert_eq!(eval_int("[1, 2, 3] |> len() * 2"), 6);
}

#[test]
fn chained_pipes_still_compose() {
    assert_eq!(eval_int("[1, 2, 3, 4] |> filter(|v| v > 1) |> len()"), 3);
    assert_eq!(eval_int("[1, 2, 3] |> map(|v| v * 2) |> sum()"), 12);
}

// ── str() converts, it does not describe ────────────────────────────

#[test]
fn str_of_a_sequence_is_its_residues() {
    // Returned "DNA(ACGT)" before, so substr() sliced the wrapper.
    assert_eq!(eval_str(r#"str(dna"ACGT")"#), "ACGT");
    assert_eq!(eval_str(r#"str(rna"ACGU")"#), "ACGU");
    assert_eq!(eval_str(r#"str(protein"MAK")"#), "MAK");
}

#[test]
fn str_of_a_sequence_composes_with_string_builtins() {
    assert_eq!(eval_str(r#"substr(str(dna"ACGTAC"), 1, 3)"#), "CGT");
    assert_eq!(eval_int(r#"len(str(dna"ACGT"))"#), 4);
}

#[test]
fn str_of_other_values_is_unchanged() {
    assert_eq!(eval_str("str(42)"), "42");
    assert_eq!(eval_str("str(true)"), "true");
}

// ── A line may open with an operator that cannot start an expression ─

#[test]
fn expressions_continue_across_newlines() {
    assert_eq!(eval_int("let a = 1\n    + 2\n    + 3\na"), 6);
    assert_eq!(eval_int("let a = 2\n    * 3\n    * 4\na"), 24);
    assert_eq!(eval("let ok = true\n    and 1 < 2\nok"), Value::Bool(true));
}

#[test]
fn a_leading_minus_still_starts_a_new_statement() {
    // `-5` is a valid expression, so this line must not be swallowed as a
    // continuation of the previous one.
    assert_eq!(eval_int("let n = 5\n-3\nn"), 5);
}

// ── Indexed assignment ──────────────────────────────────────────────

#[test]
fn list_elements_can_be_assigned() {
    assert_eq!(eval_str("let xs = [1, 2, 3]\nxs[0] = 9\nstr(xs[0])"), "9");
    assert_eq!(eval_int("let xs = [1, 2, 3]\nxs[2] = 7\nxs[2]"), 7);
}

#[test]
fn assigning_in_a_loop_fills_a_row() {
    let code = "let row = range(0, 5) |> map(|_| 0)\n\
                let i = 0\n\
                while i < 5 {\n\
                    row[i] = i * i\n\
                    i = i + 1\n\
                }\n\
                row[4]";
    assert_eq!(eval_int(code), 16);
}

#[test]
fn map_entries_can_be_assigned() {
    let code = "let counts = {a: 1, b: 2}\ncounts[\"a\"] = counts[\"a\"] + 4\ncounts[\"a\"]";
    assert_eq!(eval_int(code), 5);
}

#[test]
fn assigning_out_of_range_is_an_error() {
    let code = "let xs = [1, 2, 3]\nxs[9] = 1";
    let tokens = Lexer::new(code).tokenize().unwrap();
    let parsed = Parser::new(tokens).parse().unwrap();
    let mut interp = Interpreter::new();
    let error = interp.run(&parsed.program).unwrap_err();
    let text = format!("{error}");
    assert!(
        text.contains("out of bounds"),
        "expected a bounds error, got: {text}"
    );
}

#[test]
fn assigning_into_a_non_container_is_an_error() {
    let code = "let n = 5\nn[0] = 1";
    let tokens = Lexer::new(code).tokenize().unwrap();
    let parsed = Parser::new(tokens).parse().unwrap();
    let mut interp = Interpreter::new();
    let error = interp.run(&parsed.program).unwrap_err();
    let text = format!("{error}");
    assert!(
        text.contains("only lists, maps and records"),
        "expected a container error, got: {text}"
    );
}

// `{}` parsed as an empty block and evaluated to nil, so the natural way to
// start a tally produced a value of the wrong type. Nothing failed at the
// literal: the program ran on until the first use and reported a type error
// naming Nil, which appears nowhere in the source.

#[test]
fn empty_braces_are_an_empty_map() {
    let code = "let counts = {}\nlen(keys(counts))";
    assert_eq!(eval_int(code), 0);
}

#[test]
fn an_empty_map_can_be_filled_in() {
    let code =
        "let counts = {}\ncounts[\"a\"] = 1\ncounts[\"b\"] = counts[\"a\"] + 1\ncounts[\"b\"]";
    assert_eq!(eval_int(code), 2);
}

#[test]
fn empty_statement_bodies_are_still_blocks() {
    // The dispatch this fix changed is reached from expression position only.
    // An empty `if` body must not silently become a map literal.
    let code = "let hits = 0\nif hits == 0 { }\nwhile false { }\nhits + 7";
    assert_eq!(eval_int(code), 7);
}

#[test]
fn an_empty_function_body_still_returns_nil() {
    let code = "fn nothing() { }\nstr(nothing())";
    assert_eq!(eval_str(code), "nil");
}

// Assigning into a container updates it through a mutable borrow so the binding
// stays its only owner and Arc::make_mut writes in place. Reading the value out
// and storing it back copied the whole container on every element write, which
// made a sorting loop cubic. These check that the sharing rules did not change
// with it: a second name for the same list must not see the update.

#[test]
fn assigning_through_one_name_leaves_the_other_alone() {
    let code = "let a = [1, 2, 3]\nlet b = a\nb[0] = 99\na[0]";
    assert_eq!(eval_int(code), 1);
}

#[test]
fn the_assigned_name_does_see_the_update() {
    let code = "let a = [1, 2, 3]\nlet b = a\nb[0] = 99\nb[0]";
    assert_eq!(eval_int(code), 99);
}

#[test]
fn map_assignment_does_not_leak_through_a_shared_binding() {
    let code = "let m = {\"k\": 1}\nlet n = m\nn[\"k\"] = 42\nm[\"k\"]";
    assert_eq!(eval_int(code), 1);
}

#[test]
fn assigning_into_an_unbound_name_still_reports_it() {
    let code = "missing[0] = 1";
    let tokens = Lexer::new(code).tokenize().unwrap();
    let parsed = Parser::new(tokens).parse().unwrap();
    let mut interp = Interpreter::new();
    let error = interp.run(&parsed.program).unwrap_err();
    let text = format!("{error}");
    assert!(
        text.contains("undefined variable"),
        "expected an undefined-variable error, got: {text}"
    );
}

// Scopes are held in a Vec and named by index, so leaving one could not simply
// drop it: a closure keeps the index of the scope it was defined in. Nothing
// was ever reclaimed as a result, and a loop kept every scope it had entered —
// about 900 bytes an iteration, so a million iterations left most of a gigabyte
// behind. A scope is now reclaimed when no id was handed out during its
// lifetime, which is exactly when no closure can name it.

fn run_and_count_scopes(code: &str) -> usize {
    let tokens = Lexer::new(code).tokenize().unwrap();
    let parsed = Parser::new(tokens).parse().unwrap();
    assert!(
        !parsed.has_errors(),
        "unexpected parse errors: {:?}",
        parsed.errors
    );
    let mut interp = Interpreter::new();
    interp.run(&parsed.program).unwrap();
    interp.env().scope_count()
}

#[test]
fn a_loop_that_captures_nothing_reclaims_its_scopes() {
    let few = run_and_count_scopes(
        "let t = 0\nlet i = 0\nwhile i < 50 {\n let x = i\n t = t + x\n i = i + 1\n}",
    );
    let many = run_and_count_scopes(
        "let t = 0\nlet i = 0\nwhile i < 5000 {\n let x = i\n t = t + x\n i = i + 1\n}",
    );
    // A hundredfold more iterations must not mean more scopes retained.
    assert_eq!(
        few, many,
        "scope count grew with the iteration count: {few} then {many}"
    );
}

#[test]
fn a_closure_made_in_a_loop_still_reads_its_own_capture() {
    // The scopes behind these closures must survive the loop that made them.
    let code = "let fns = []\nfor i in range(0, 5) {\n fns = push(fns, |x| x + i)\n}\nlet applied = fns |> map(|f| f(100))\nsum(applied)";
    // 100+0 .. 104 summed is 510.
    assert_eq!(eval_int(code), 510);
}

#[test]
fn a_returned_closure_outlives_the_call_that_made_it() {
    let code = "fn make_adder(n) {\n |x| x + n\n}\nlet add7 = make_adder(7)\nlet add100 = make_adder(100)\nadd7(1) + add100(1)";
    assert_eq!(eval_int(code), 109);
}

#[test]
fn nested_recursion_still_resolves_each_frame() {
    // Run on a thread with a generous stack. The interpreter recurses on the
    // Rust stack, and an unoptimised build of this test binary needs far more of
    // it per level than the CLI does — 2 MB, a test thread's default, does not
    // survive even ten levels here, while a debug CLI on 8 MB manages a hundred.
    // Why the two differ by that much is not established.
    //
    // What is established: the ceiling is not this fix. A debug CLI manages 100
    // levels and not 300 both with and without scope reclamation, checked by
    // reverting env.rs and rebuilding.
    let worker = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            let code = "fn depth(n) {\n if n == 0 then 0 else 1 + depth(n - 1)\n}\ndepth(50)";
            eval_int(code)
        })
        .expect("spawn a worker with a larger stack");
    assert_eq!(worker.join().expect("the worker panicked"), 50);
}

// `||` is lexed as one Or token, so a lambda taking no arguments was a parse
// error and had to be written `|_| expr` with an argument nobody wanted. Meeting
// Or where an expression must begin can only be an empty parameter list, since
// Or is binary and cannot start one.

#[test]
fn a_lambda_can_take_no_arguments() {
    let code = "let counter = 7\nlet read = || counter\nread()";
    assert_eq!(eval_int(code), 7);
}

#[test]
fn a_zero_argument_lambda_can_have_a_block_body() {
    let code = "let make = || {\n let inner = 5\n inner * 2\n}\nmake()";
    assert_eq!(eval_int(code), 10);
}

#[test]
fn logical_or_still_works_everywhere_else() {
    assert_eq!(eval("false || true"), Value::Bool(true));
    assert_eq!(eval("false || false"), Value::Bool(false));
    assert_eq!(eval_int("if false || true then 1 else 0"), 1);
}
