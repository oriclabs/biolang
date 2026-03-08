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

// 2.2: Struct/Trait dispatch
#[test]
fn test_impl_method_dispatch() {
    let code = r#"
struct Point { x, y }
impl Point {
    fn sum(self) {
        self.x + self.y
    }
}
let p = Point { x: 3, y: 7 }
p.sum()
"#;
    assert_eq!(eval(code), Value::Int(10));
}

#[test]
fn test_impl_method_with_args() {
    let code = r#"
struct Counter { val }
impl Counter {
    fn add(self, n) {
        Counter { val: self.val + n }
    }
}
let c = Counter { val: 0 }
let c2 = c.add(5)
c2.val
"#;
    assert_eq!(eval(code), Value::Int(5));
}

// 2.3: UFCS
#[test]
fn test_ufcs_string_upper() {
    assert_eq!(eval(r#""hello".upper()"#), Value::Str("HELLO".into()));
}

#[test]
fn test_ufcs_list_len() {
    assert_eq!(eval("[1, 2, 3].len()"), Value::Int(3));
}

#[test]
fn test_ufcs_chaining() {
    assert_eq!(eval(r#""hello world".upper().len()"#), Value::Int(11));
}

// 2.4: Type Pattern Matching
#[test]
fn test_type_pattern_int() {
    let code = r#"
let x = 42
match x {
    Int(n) => n * 2,
    Str(s) => len(s),
    _ => 0
}
"#;
    assert_eq!(eval(code), Value::Int(84));
}

#[test]
fn test_type_pattern_str() {
    let code = r#"
let x = "hello"
match x {
    Int(n) => n * 2,
    Str(s) => len(s),
    _ => 0
}
"#;
    assert_eq!(eval(code), Value::Int(5));
}

#[test]
fn test_type_pattern_wildcard_binding() {
    let code = r#"
match dna"ATCG" {
    DNA(_) => "is dna",
    _ => "not dna"
}
"#;
    assert_eq!(eval(code), Value::Str("is dna".into()));
}

#[test]
fn test_type_pattern_no_binding() {
    let code = r#"
match 3.14 {
    Int(_) => "int",
    Float(_) => "float",
    _ => "other"
}
"#;
    assert_eq!(eval(code), Value::Str("float".into()));
}
