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

// 5.2: Tuples — not yet parseable as syntax, test via runtime constructor
// Tuples will be parsed from (a, b) syntax once parser supports it
// For now test through list-based operations

// 5.3: For destructuring
#[test]
fn test_for_destructuring_list() {
    let code = r#"
let pairs = [[1, 2], [3, 4], [5, 6]]
let total = 0
for [a, b] in pairs {
    total = total + a + b
}
total
"#;
    assert_eq!(eval(code), Value::Int(21));
}

#[test]
fn test_for_destructuring_record() {
    let code = r#"
let items = [{name: "Alice", age: 30}, {name: "Bob", age: 25}]
let names = []
for {name, age} in items {
    names = names + [name]
}
names
"#;
    assert_eq!(eval(code), Value::List(vec![
        Value::Str("Alice".into()), Value::Str("Bob".into()),
    ]));
}

// 5.5: Slicing syntax
#[test]
fn test_list_slicing() {
    let code = r#"
let xs = [10, 20, 30, 40, 50]
xs[1..3]
"#;
    assert_eq!(eval(code), Value::List(vec![Value::Int(20), Value::Int(30)]));
}

#[test]
fn test_list_slicing_inclusive() {
    let code = r#"
let xs = [10, 20, 30, 40, 50]
xs[1..=3]
"#;
    assert_eq!(eval(code), Value::List(vec![
        Value::Int(20), Value::Int(30), Value::Int(40),
    ]));
}

#[test]
fn test_string_slicing() {
    assert_eq!(eval(r#""hello world"[0..5]"#), Value::Str("hello".into()));
}

#[test]
fn test_dna_slicing() {
    let code = r#"
let seq = dna"ATCGATCG"
seq[2..6]
"#;
    let result = eval(code);
    assert_eq!(format!("{result}"), "DNA(CGAT)");
}

// 5.6: Operator overloading
#[test]
fn test_operator_overloading_add() {
    let code = r#"
struct Vec2 { x, y }
impl Vec2 {
    fn add(self, other) {
        Vec2 { x: self.x + other.x, y: self.y + other.y }
    }
}
let a = Vec2 { x: 1, y: 2 }
let b = Vec2 { x: 3, y: 4 }
let c = a + b
[c.x, c.y]
"#;
    assert_eq!(eval(code), Value::List(vec![Value::Int(4), Value::Int(6)]));
}

#[test]
fn test_operator_overloading_mul() {
    let code = r#"
struct Vec2 { x, y }
impl Vec2 {
    fn mul(self, other) {
        Vec2 { x: self.x * other.x, y: self.y * other.y }
    }
}
let a = Vec2 { x: 2, y: 3 }
let b = Vec2 { x: 4, y: 5 }
let c = a * b
c.x + c.y
"#;
    assert_eq!(eval(code), Value::Int(23)); // 8 + 15
}

// For loop with simple variable (regression test)
#[test]
fn test_for_simple_var() {
    let code = r#"
let sum = 0
for x in [1, 2, 3] {
    sum = sum + x
}
sum
"#;
    assert_eq!(eval(code), Value::Int(6));
}
