use bl_core::value::Value;
use bl_lexer::Lexer;
use bl_parser::Parser;
use bl_runtime::{builtins, Interpreter};
use std::collections::HashMap;

fn eval(source: &str) -> Value {
    let tokens = Lexer::new(source).tokenize().unwrap();
    let parsed = Parser::new(tokens).parse().unwrap();
    let mut interpreter = Interpreter::new();
    interpreter.run(&parsed.program).unwrap()
}

#[test]
fn run_parameters_are_typed_and_have_an_explicit_default() {
    builtins::set_run_parameters(HashMap::from([
        ("count".to_string(), Value::Int(7)),
        ("label".to_string(), Value::Str("treated".to_string())),
    ]));
    assert_eq!(
        eval("[run_param(\"count\"), run_param(\"label\"), run_param(\"missing\", 42)]"),
        Value::List(vec![Value::Int(7), Value::Str("treated".into()), Value::Int(42)].into())
    );
    builtins::clear_run_parameters();
    assert_eq!(eval("run_param(\"count\")"), Value::Nil);
}

#[test]
fn run_param_rejects_a_non_string_name() {
    let tokens = Lexer::new("run_param(3)").tokenize().unwrap();
    let parsed = Parser::new(tokens).parse().unwrap();
    let mut interpreter = Interpreter::new();
    assert!(interpreter.run(&parsed.program).is_err());
}
