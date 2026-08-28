use bl_core::value::Value;
use bl_lexer::Lexer;
use bl_parser::Parser;
use bl_runtime::Interpreter;

fn program(source: &str) -> bl_core::ast::Program {
    let tokens = Lexer::new(source).tokenize().unwrap();
    let parsed = Parser::new(tokens).parse().unwrap();
    assert!(!parsed.has_errors(), "{:?}", parsed.errors);
    parsed.program
}

#[test]
fn native_interpreter_imports_registered_virtual_modules() {
    let mut interpreter = Interpreter::new();
    interpreter.register_virtual_module("embedded/math", "fn twice(value) { value * 2 }\n");

    let result = interpreter
        .run(&program(
            "import \"embedded/math\" as math\nmath.twice(21)\n",
        ))
        .unwrap();

    assert_eq!(result, Value::Int(42));
    assert!(interpreter.loaded_module_paths().is_empty());
}

#[test]
fn virtual_module_registration_survives_reset() {
    let mut interpreter = Interpreter::new();
    interpreter.register_virtual_module("probe", "let answer = 42\n");
    interpreter.reset();

    let result = interpreter
        .run(&program("import \"probe\" as module\nmodule.answer\n"))
        .unwrap();

    assert_eq!(result, Value::Int(42));
}
