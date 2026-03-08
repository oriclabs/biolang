use bl_core::value::Value;
use bl_lexer::Lexer;
use bl_parser::Parser;
use bl_runtime::Interpreter;

fn eval(code: &str) -> Value {
    let tokens = Lexer::new(code).tokenize().unwrap();
    let result = Parser::new(tokens).parse().unwrap();
    assert!(result.errors.is_empty(), "parse errors: {:?}", result.errors);
    let mut interp = Interpreter::new();
    interp.run(&result.program).unwrap()
}

// ── Feature 1.3: Default Function Arguments ─────────────────────────

#[test]
fn test_default_arg_basic() {
    assert_eq!(
        eval("fn greet(name = \"world\") { name }\ngreet()"),
        Value::Str("world".into())
    );
}

#[test]
fn test_default_arg_override_positional() {
    assert_eq!(
        eval("fn greet(name = \"world\") { name }\ngreet(\"Alice\")"),
        Value::Str("Alice".into())
    );
}

#[test]
fn test_default_arg_override_named() {
    assert_eq!(
        eval("fn greet(name = \"world\") { name }\ngreet(name: \"Bob\")"),
        Value::Str("Bob".into())
    );
}

#[test]
fn test_default_arg_mixed() {
    assert_eq!(
        eval("fn add(a, b = 10) { a + b }\nadd(5)"),
        Value::Int(15)
    );
}

#[test]
fn test_default_arg_mixed_both_provided() {
    assert_eq!(
        eval("fn add(a, b = 10) { a + b }\nadd(5, 20)"),
        Value::Int(25)
    );
}

#[test]
fn test_default_arg_expression() {
    // Default expression can reference earlier params (bound sequentially)
    assert_eq!(
        eval("fn f(x, y = x * 2) { y }\nf(3)"),
        Value::Int(6)
    );
}

#[test]
fn test_default_arg_multiple_defaults() {
    assert_eq!(
        eval("fn f(a = 1, b = 2, c = 3) { a + b + c }\nf()"),
        Value::Int(6)
    );
}

#[test]
fn test_default_arg_partial_override() {
    assert_eq!(
        eval("fn f(a = 1, b = 2, c = 3) { a + b + c }\nf(10)"),
        Value::Int(15)
    );
}

#[test]
fn test_missing_required_arg_errors() {
    let tokens = Lexer::new("fn f(a, b) { a + b }\nf(1)").tokenize().unwrap();
    let result = Parser::new(tokens).parse().unwrap();
    assert!(result.errors.is_empty());
    let mut interp = Interpreter::new();
    let err = interp.run(&result.program);
    assert!(err.is_err(), "should error on missing required arg");
}

// ── Feature 1.5: Multiline Lambda ───────────────────────────────────

#[test]
fn test_multiline_lambda_basic() {
    assert_eq!(
        eval("let f = |x| {\n  let y = x * 2\n  y + 1\n}\nf(5)"),
        Value::Int(11)
    );
}

#[test]
fn test_multiline_lambda_multiple_stmts() {
    assert_eq!(
        eval("let f = |a, b| {\n  let sum = a + b\n  let doubled = sum * 2\n  doubled\n}\nf(3, 4)"),
        Value::Int(14)
    );
}

#[test]
fn test_multiline_lambda_in_map() {
    let result = eval("[1, 2, 3] |> map(|x| {\n  let doubled = x * 2\n  doubled + 1\n})");
    match result {
        Value::List(items) => {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], Value::Int(3));
            assert_eq!(items[1], Value::Int(5));
            assert_eq!(items[2], Value::Int(7));
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_single_expr_lambda() {
    assert_eq!(
        eval("let double = |x| x * 2\ndouble(7)"),
        Value::Int(14)
    );
}

#[test]
fn test_lambda_in_filter() {
    let result = eval("[1, 2, 3, 4, 5] |> filter(|x| x > 3)");
    match result {
        Value::List(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], Value::Int(4));
            assert_eq!(items[1], Value::Int(5));
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_multiline_lambda_in_filter() {
    let result = eval("[1, 2, 3, 4, 5] |> filter(|x| {\n  let threshold = 3\n  x > threshold\n})");
    match result {
        Value::List(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], Value::Int(4));
            assert_eq!(items[1], Value::Int(5));
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_lambda_no_defaults() {
    // Lambdas don't support default params — passing fewer args than params should error
    let tokens = Lexer::new("let f = |a, b| a + b\nf(1)").tokenize().unwrap();
    let result = Parser::new(tokens).parse().unwrap();
    assert!(result.errors.is_empty());
    let mut interp = Interpreter::new();
    let err = interp.run(&result.program);
    assert!(err.is_err(), "lambda should error on missing arg (no defaults)");
}
