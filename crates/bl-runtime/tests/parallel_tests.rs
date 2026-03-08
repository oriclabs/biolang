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
