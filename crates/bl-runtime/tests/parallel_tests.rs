use bl_core::value::Value;
use bl_lexer::Lexer;
use bl_parser::Parser;
use bl_runtime::Interpreter;

fn eval(code: &str) -> Value {
    let tokens = Lexer::new(code).tokenize().unwrap();
    let result = Parser::new(tokens).parse().unwrap();
    let mut interp = Interpreter::new();
    interp.run(&result.program).unwrap()
}

// 3.1: par_map — true parallel execution
#[test]
fn test_par_map_basic() {
    let code = r#"
let xs = [1, 2, 3, 4, 5]
par_map(xs, |x| x * 2)
"#;
    assert_eq!(eval(code), Value::List(vec![
        Value::Int(2), Value::Int(4), Value::Int(6), Value::Int(8), Value::Int(10),
    ]));
}

#[test]
fn test_par_map_ufcs() {
    let code = r#"
[10, 20, 30].par_map(|x| x + 1)
"#;
    assert_eq!(eval(code), Value::List(vec![
        Value::Int(11), Value::Int(21), Value::Int(31),
    ]));
}

#[test]
fn test_par_map_large() {
    // Test with enough items to actually split across threads
    let code = r#"
let xs = range(1, 101)
let result = par_map(xs, |x| x * x)
result.len()
"#;
    assert_eq!(eval(code), Value::Int(100));
}

#[test]
fn test_par_map_preserves_order() {
    let code = r#"
let xs = [5, 4, 3, 2, 1]
par_map(xs, |x| x)
"#;
    assert_eq!(eval(code), Value::List(vec![
        Value::Int(5), Value::Int(4), Value::Int(3), Value::Int(2), Value::Int(1),
    ]));
}

// 3.1: par_filter — true parallel execution
#[test]
fn test_par_filter_basic() {
    let code = r#"
let xs = [1, 2, 3, 4, 5, 6]
par_filter(xs, |x| x % 2 == 0)
"#;
    assert_eq!(eval(code), Value::List(vec![
        Value::Int(2), Value::Int(4), Value::Int(6),
    ]));
}

#[test]
fn test_par_filter_ufcs() {
    let code = r#"
[10, 15, 20, 25, 30].par_filter(|x| x > 18)
"#;
    assert_eq!(eval(code), Value::List(vec![
        Value::Int(20), Value::Int(25), Value::Int(30),
    ]));
}

#[test]
fn test_par_filter_empty_result() {
    let code = r#"
par_filter([1, 2, 3], |x| x > 100)
"#;
    assert_eq!(eval(code), Value::List(vec![]));
}

// 3.2: Async/await
#[test]
fn test_async_await_basic() {
    let code = r#"
async fn compute(x) {
    x * x + 1
}
let f = compute(5)
await f
"#;
    assert_eq!(eval(code), Value::Int(26));
}

#[test]
fn test_async_await_multiple() {
    let code = r#"
async fn double(x) { x * 2 }
let a = double(10)
let b = double(20)
let c = double(30)
[await a, await b, await c]
"#;
    assert_eq!(eval(code), Value::List(vec![
        Value::Int(20), Value::Int(40), Value::Int(60),
    ]));
}

#[test]
fn test_await_all() {
    let code = r#"
async fn square(x) { x * x }
let futures = [square(2), square(3), square(4)]
await_all(futures)
"#;
    assert_eq!(eval(code), Value::List(vec![
        Value::Int(4), Value::Int(9), Value::Int(16),
    ]));
}

#[test]
fn test_await_non_future() {
    // await on non-future should pass through
    let code = r#"
await 42
"#;
    assert_eq!(eval(code), Value::Int(42));
}

#[test]
fn test_await_all_mixed() {
    // await_all should handle non-futures in the list
    let code = r#"
async fn double(x) { x * 2 }
await_all([double(5), 100, double(15)])
"#;
    assert_eq!(eval(code), Value::List(vec![
        Value::Int(10), Value::Int(100), Value::Int(30),
    ]));
}

// ── Additional par_map / par_filter tests ─────────────────────────

#[test]
fn test_par_map_correctness_vs_sequential() {
    // Verify par_map and map produce identical results
    let code = r#"
let data = range(1, 501)
let expected = data |> map(|x| x * x + x)
let actual = data |> par_map(|x| x * x + x)
expected == actual
"#;
    assert_eq!(eval(code), Value::Bool(true));
}

#[test]
fn test_par_filter_correctness_vs_sequential() {
    let code = r#"
let data = range(1, 501)
let expected = data |> filter(|x| x % 3 == 0 or x % 5 == 0)
let actual = data |> par_filter(|x| x % 3 == 0 or x % 5 == 0)
expected == actual
"#;
    assert_eq!(eval(code), Value::Bool(true));
}

#[test]
fn test_par_map_with_string_ops() {
    let code = r#"
let words = ["hello", "world", "foo", "bar"]
let result = words |> par_map(|w| upper(w))
result
"#;
    assert_eq!(eval(code), Value::List(vec![
        Value::Str("HELLO".into()),
        Value::Str("WORLD".into()),
        Value::Str("FOO".into()),
        Value::Str("BAR".into()),
    ]));
}

#[test]
fn test_par_map_large_dataset() {
    // 10000 items should distribute across threads
    let code = r#"
let result = range(1, 10001) |> par_map(|x| x * 2) |> len()
result
"#;
    assert_eq!(eval(code), Value::Int(10000));
}

#[test]
fn test_par_filter_large_dataset() {
    let code = r#"
let result = range(1, 10001) |> par_filter(|x| x % 10 == 0) |> len()
result
"#;
    assert_eq!(eval(code), Value::Int(1000));
}

#[test]
fn test_par_map_with_records() {
    let code = r#"
let items = [{x: 1}, {x: 2}, {x: 3}]
let result = items |> par_map(|r| {x: r.x * 10})
result |> map(|r| r.x)
"#;
    assert_eq!(eval(code), Value::List(vec![
        Value::Int(10), Value::Int(20), Value::Int(30),
    ]));
}

#[test]
fn test_par_map_two_items() {
    // Edge case: fewer items than threads
    let code = r#"
[1, 2] |> par_map(|x| x + 100)
"#;
    assert_eq!(eval(code), Value::List(vec![
        Value::Int(101), Value::Int(102),
    ]));
}

#[test]
fn test_async_fn_with_closure_capture() {
    let code = r#"
let factor = 7
async fn scaled(x) { x * factor }
let f = scaled(6)
await f
"#;
    assert_eq!(eval(code), Value::Int(42));
}

#[test]
fn test_await_all_large() {
    let code = r#"
async fn square(x) { x * x }
let futures = range(1, 101) |> map(|i| square(i))
let results = await_all(futures)
len(results)
"#;
    assert_eq!(eval(code), Value::Int(100));
}
