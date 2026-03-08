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

// 4.2: Lazy generators — values yielded on-demand
#[test]
fn test_lazy_generator_basic() {
    let code = r#"
fn* counting() {
    yield 1
    yield 2
    yield 3
}
let s = counting()
collect(s)
"#;
    assert_eq!(eval(code), Value::List(vec![
        Value::Int(1), Value::Int(2), Value::Int(3),
    ]));
}

#[test]
fn test_lazy_generator_for_loop() {
    let code = r#"
fn* range_gen(n) {
    let i = 0
    while i < n {
        yield i
        i = i + 1
    }
}
let total = 0
for x in range_gen(5) {
    total = total + x
}
total
"#;
    assert_eq!(eval(code), Value::Int(10)); // 0+1+2+3+4
}

#[test]
fn test_lazy_generator_partial_consume() {
    // Only consume first 3 items from a generator that yields many
    let code = r#"
fn* infinite_like() {
    let i = 0
    while i < 1000 {
        yield i
        i = i + 1
    }
}
let s = infinite_like()
let a = next(s)
let b = next(s)
let c = next(s)
[a, b, c]
"#;
    assert_eq!(eval(code), Value::List(vec![
        Value::Int(0), Value::Int(1), Value::Int(2),
    ]));
}

// 4.3: Stream pipeline chaining (lazy map/filter on streams)
#[test]
fn test_stream_map_filter_lazy() {
    let code = r#"
fn* nums() {
    yield 1
    yield 2
    yield 3
    yield 4
    yield 5
}
let s = nums()
let doubled = map(s, |x| x * 2)
let big = filter(doubled, |x| x > 4)
collect(big)
"#;
    assert_eq!(eval(code), Value::List(vec![
        Value::Int(6), Value::Int(8), Value::Int(10),
    ]));
}

#[test]
fn test_stream_ufcs_chaining() {
    let code = r#"
fn* items() {
    yield 10
    yield 20
    yield 30
}
items().map(|x| x + 1).filter(|x| x > 15).collect()
"#;
    assert_eq!(eval(code), Value::List(vec![
        Value::Int(21), Value::Int(31),
    ]));
}

#[test]
fn test_stream_reduce() {
    let code = r#"
fn* vals() {
    yield 1
    yield 2
    yield 3
    yield 4
}
vals().reduce(|a, b| a + b, 0)
"#;
    assert_eq!(eval(code), Value::Int(10));
}
