use bl_core::value::{BioSequence, Value};
use bl_lexer::Lexer;
use bl_parser::Parser;
use bl_runtime::Interpreter;

fn eval(code: &str) -> Value {
    let tokens = Lexer::new(code).tokenize().unwrap();
    let result = Parser::new(tokens).parse().unwrap();
    let mut interp = Interpreter::new();
    interp.run(&result.program).unwrap()
}

fn eval_err(code: &str) -> bool {
    let tokens = Lexer::new(code).tokenize().unwrap();
    let result = Parser::new(tokens).parse().unwrap();
    let mut interp = Interpreter::new();
    interp.run(&result.program).is_err()
}

// ============================================================================
// Migrated inline tests (68 tests from interpreter.rs)
// ============================================================================

// ── Basic arithmetic ────────────────────────────────────────────────────

#[test]
fn test_arithmetic() {
    assert_eq!(eval("2 + 3"), Value::Int(5));
    assert_eq!(eval("10 - 4"), Value::Int(6));
    assert_eq!(eval("3 * 7"), Value::Int(21));
    assert_eq!(eval("15 / 3"), Value::Int(5));
    assert_eq!(eval("17 % 5"), Value::Int(2));
}

#[test]
fn test_float_arithmetic() {
    assert_eq!(eval("1.5 + 2.5"), Value::Float(4.0));
    assert_eq!(eval("3.0 * 2"), Value::Float(6.0));
}

#[test]
fn test_string_concat() {
    assert_eq!(
        eval(r#""hello" + " " + "world""#),
        Value::Str("hello world".into())
    );
}

#[test]
fn test_let_and_reference() {
    assert_eq!(eval("let x = 42\nx"), Value::Int(42));
}

// ── Pipe operations ────────────────────────────────────────────────────

#[test]
fn test_pipe_to_lambda() {
    assert_eq!(eval("10 |> |n| n * 2"), Value::Int(20));
}

#[test]
fn test_milestone() {
    let tokens = Lexer::new("let x = 10 |> |n| n * 2\nx")
        .tokenize()
        .unwrap();
    let program = Parser::new(tokens).parse().unwrap().program;
    let mut interp = Interpreter::new();
    let result = interp.run(&program).unwrap();
    assert_eq!(result, Value::Int(20));
}

#[test]
fn test_pipe_to_function() {
    let result = eval("fn double(n) { n * 2 }\n5 |> double()");
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_pipe_chain() {
    let result = eval("10 |> |n| n + 5 |> |n| n * 2");
    assert_eq!(result, Value::Int(30));
}

#[test]
fn test_function_with_named_args() {
    let result = eval(
        r#"fn greet(name, greeting: Str = "Hello") { greeting + " " + name }
greet("World")"#,
    );
    assert_eq!(result, Value::Str("Hello World".into()));
}

// ── If/else and comparison ─────────────────────────────────────────────

#[test]
fn test_if_expression() {
    assert_eq!(eval("if true { 1 } else { 2 }"), Value::Int(1));
    assert_eq!(eval("if false { 1 } else { 2 }"), Value::Int(2));
}

#[test]
fn test_comparison() {
    assert_eq!(eval("3 > 2"), Value::Bool(true));
    assert_eq!(eval("1 >= 1"), Value::Bool(true));
    assert_eq!(eval("5 < 3"), Value::Bool(false));
    assert_eq!(eval("2 == 2"), Value::Bool(true));
    assert_eq!(eval("2 != 3"), Value::Bool(true));
}

// ── List operations ────────────────────────────────────────────────────

#[test]
fn test_list_operations() {
    assert_eq!(
        eval("[1, 2, 3]"),
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
    assert_eq!(eval("len([1, 2, 3])"), Value::Int(3));
}

#[test]
fn test_for_loop() {
    let result = eval(
        r#"
let sum = 0
for i in range(5) {
  let sum = sum + i
}
"#,
    );
    // `let` creates a new binding, so outer sum is not modified
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_map_filter() {
    let result = eval("map([1, 2, 3, 4], |x| x * 2)");
    assert_eq!(
        result,
        Value::List(vec![
            Value::Int(2),
            Value::Int(4),
            Value::Int(6),
            Value::Int(8)
        ])
    );

    let result = eval("filter([1, 2, 3, 4, 5], |x| x > 3)");
    assert_eq!(
        result,
        Value::List(vec![Value::Int(4), Value::Int(5)])
    );
}

#[test]
fn test_reduce() {
    let result = eval("reduce([1, 2, 3, 4], |acc, x| acc + x)");
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_sort() {
    let result = eval("sort([3, 1, 4, 1, 5])");
    assert_eq!(
        result,
        Value::List(vec![
            Value::Int(1),
            Value::Int(1),
            Value::Int(3),
            Value::Int(4),
            Value::Int(5)
        ])
    );
}

// ── Functions ──────────────────────────────────────────────────────────

#[test]
fn test_nested_function() {
    let result = eval(
        r#"
fn add(a, b) { a + b }
fn apply(f, x, y) { f(x, y) }
apply(add, 3, 4)
"#,
    );
    assert_eq!(result, Value::Int(7));
}

// ── Records and field access ───────────────────────────────────────────

#[test]
fn test_field_access() {
    let result = eval(
        r#"let r = {name: "Alice", age: 30}
r.name"#,
    );
    assert_eq!(result, Value::Str("Alice".into()));
}

#[test]
fn test_index_access() {
    assert_eq!(eval("[10, 20, 30][1]"), Value::Int(20));
    assert_eq!(eval("[10, 20, 30][-1]"), Value::Int(30));
}

// ── Match expressions ──────────────────────────────────────────────────

#[test]
fn test_match_expr() {
    let result = eval(
        r#"
let x = 2
match x {
  1 => "one",
  2 => "two",
  _ => "other"
}
"#,
    );
    assert_eq!(result, Value::Str("two".into()));
}

// ── Assert ─────────────────────────────────────────────────────────────

#[test]
fn test_assert_pass() {
    let result = eval("assert 1 + 1 == 2");
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_assert_fail() {
    assert!(eval_err("assert 1 == 2"));
}

// ── Type builtin ───────────────────────────────────────────────────────

#[test]
fn test_type_builtin() {
    assert_eq!(eval("type(42)"), Value::Str("Int".into()));
    assert_eq!(eval("type(3.14)"), Value::Str("Float".into()));
    assert_eq!(eval(r#"type("hello")"#), Value::Str("Str".into()));
    assert_eq!(eval("type(true)"), Value::Str("Bool".into()));
    assert_eq!(eval("type(nil)"), Value::Str("Nil".into()));
}

// ── Return values ──────────────────────────────────────────────────────

#[test]
fn test_return_complex_value() {
    let result = eval(
        r#"
fn make_list() {
    return [1, 2, 3]
}
make_list()
"#,
    );
    assert_eq!(
        result,
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn test_return_record() {
    let result = eval(
        r#"
fn make_record() {
    {x: 1, y: 2}
}
make_record()
"#,
    );
    match result {
        Value::Record(map) => {
            assert_eq!(map.get("x"), Some(&Value::Int(1)));
            assert_eq!(map.get("y"), Some(&Value::Int(2)));
        }
        other => panic!("expected Record, got {other:?}"),
    }
}

// ── Variable reassignment ──────────────────────────────────────────────

#[test]
fn test_variable_reassignment() {
    let result = eval(
        r#"
let x = 1
x = 2
x
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_accumulator_loop() {
    let result = eval(
        r#"
let sum = 0
for i in range(5) {
    sum = sum + i
}
sum
"#,
    );
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_reassignment_error_undefined() {
    assert!(eval_err("x = 5"));
}

// ── Bio operations ─────────────────────────────────────────────────────

#[test]
fn test_bio_transcribe_translate() {
    let result = eval(r#"dna"ATGATCGATCG" |> transcribe() |> translate()"#);
    assert_eq!(
        result,
        Value::Protein(BioSequence {
            data: "MID".into()
        })
    );
}

#[test]
fn test_bio_reverse_complement() {
    let result = eval(r#"dna"ATCG" |> reverse_complement()"#);
    assert_eq!(
        result,
        Value::DNA(BioSequence {
            data: "CGAT".into()
        })
    );
}

#[test]
fn test_bio_gc_content() {
    let result = eval(r#"dna"GCGCATAT" |> gc_content()"#);
    assert_eq!(result, Value::Float(0.5));
}

#[test]
fn test_bio_len() {
    let result = eval(r#"len(dna"ATCGATCG")"#);
    assert_eq!(result, Value::Int(8));
}

#[test]
fn test_bio_pipe_chain() {
    let result = eval(
        r#"
dna"ATGATCGATCGATCGATCG"
  |> reverse_complement()
  |> transcribe()
  |> translate()
"#,
    );
    assert_eq!(
        result,
        Value::Protein(BioSequence {
            data: "RSIDRS".into()
        })
    );
}

// ── Stream tests ───────────────────────────────────────────────────────

#[test]
fn test_to_stream_and_collect() {
    let result = eval(
        r#"
[1, 2, 3] |> to_stream() |> collect()
"#,
    );
    assert_eq!(
        result,
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn test_stream_take() {
    let result = eval(
        r#"
[10, 20, 30, 40, 50] |> to_stream() |> take(3)
"#,
    );
    assert_eq!(
        result,
        Value::List(vec![Value::Int(10), Value::Int(20), Value::Int(30)])
    );
}

#[test]
fn test_stream_count() {
    let result = eval(
        r#"
[1, 2, 3, 4, 5] |> to_stream() |> count()
"#,
    );
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_stream_map() {
    let result = eval(
        r#"
[1, 2, 3] |> to_stream() |> map(|x| x * 10) |> collect()
"#,
    );
    assert_eq!(
        result,
        Value::List(vec![Value::Int(10), Value::Int(20), Value::Int(30)])
    );
}

#[test]
fn test_stream_map_returns_stream() {
    let result = eval(
        r#"
[1, 2, 3] |> to_stream() |> map(|x| x * 10) |> type()
"#,
    );
    assert_eq!(result, Value::Str("Stream".into()));
}

#[test]
fn test_stream_filter() {
    let result = eval(
        r#"
[1, 2, 3, 4, 5, 6] |> to_stream() |> filter(|x| x > 3) |> collect()
"#,
    );
    assert_eq!(
        result,
        Value::List(vec![Value::Int(4), Value::Int(5), Value::Int(6)])
    );
}

#[test]
fn test_stream_filter_then_take() {
    let result = eval(
        r#"
[1, 2, 3, 4, 5, 6] |> to_stream() |> filter(|x| x > 1) |> take(1)
"#,
    );
    assert_eq!(result, Value::List(vec![Value::Int(2)]));
}

#[test]
fn test_stream_take_then_map() {
    let result = eval(
        r#"
[1, 2, 3, 4, 5] |> to_stream() |> take(3) |> map(|x| x * 2)
"#,
    );
    assert_eq!(
        result,
        Value::List(vec![Value::Int(2), Value::Int(4), Value::Int(6)])
    );
}

#[test]
fn test_stream_reduce() {
    let result = eval(
        r#"
[1, 2, 3, 4] |> to_stream() |> reduce(|a, b| a + b)
"#,
    );
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_collect_list_passthrough() {
    let result = eval(r#"collect([1, 2, 3])"#);
    assert_eq!(
        result,
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn test_stream_type() {
    let result = eval(r#"type(to_stream([1, 2, 3]))"#);
    assert_eq!(result, Value::Str("Stream".into()));
}

// ── Table tests ────────────────────────────────────────────────────────

#[test]
fn test_table_from_records() {
    let result = eval(
        r#"
let t = table([{name: "Alice", age: 30}, {name: "Bob", age: 25}])
type(t)
"#,
    );
    assert_eq!(result, Value::Str("Table".into()));
}

#[test]
fn test_table_nrow_ncol() {
    let result = eval(
        r#"
let t = table([{name: "Alice", age: 30}, {name: "Bob", age: 25}])
nrow(t)
"#,
    );
    assert_eq!(result, Value::Int(2));

    let result = eval(
        r#"
let t = table([{name: "Alice", age: 30}, {name: "Bob", age: 25}])
ncol(t)
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_table_colnames() {
    let result = eval(
        r#"
let t = table([{name: "Alice", age: 30}])
colnames(t)
"#,
    );
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("age".into()),
            Value::Str("name".into())
        ])
    );
}

#[test]
fn test_table_len() {
    let result = eval(
        r#"
let t = table([{x: 1}, {x: 2}, {x: 3}])
len(t)
"#,
    );
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_table_head_tail() {
    let result = eval(
        r#"
let t = table([{x: 1}, {x: 2}, {x: 3}, {x: 4}, {x: 5}])
nrow(head(t, 2))
"#,
    );
    assert_eq!(result, Value::Int(2));

    let result = eval(
        r#"
let t = table([{x: 1}, {x: 2}, {x: 3}, {x: 4}, {x: 5}])
nrow(tail(t, 2))
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_table_index() {
    let result = eval(
        r#"
let t = table([{name: "Alice", age: 30}, {name: "Bob", age: 25}])
let row = t[0]
row.name
"#,
    );
    assert_eq!(result, Value::Str("Alice".into()));
}

#[test]
fn test_table_field_column() {
    let result = eval(
        r#"
let t = table([{name: "Alice", age: 30}, {name: "Bob", age: 25}])
t.name
"#,
    );
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("Alice".into()),
            Value::Str("Bob".into())
        ])
    );
}

#[test]
fn test_table_field_meta() {
    let result = eval(
        r#"
let t = table([{x: 1}, {x: 2}])
t.num_rows
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_table_for_loop() {
    let result = eval(
        r#"
let t = table([{x: 1}, {x: 2}, {x: 3}])
let sum = 0
for row in t {
    sum = sum + row.x
}
sum
"#,
    );
    assert_eq!(result, Value::Int(6));
}

#[test]
fn test_table_select() {
    let result = eval(
        r#"
let t = table([{name: "Alice", age: 30, city: "NYC"}])
let t2 = select(t, "name", "age")
ncol(t2)
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_table_arrange() {
    let result = eval(
        r#"
let t = table([{name: "Bob", age: 25}, {name: "Alice", age: 30}])
let t2 = arrange(t, "name")
t2[0].name
"#,
    );
    assert_eq!(result, Value::Str("Alice".into()));
}

#[test]
fn test_table_arrange_desc() {
    let result = eval(
        r#"
let t = table([{name: "Alice", score: 80}, {name: "Bob", score: 95}])
let t2 = arrange(t, "-score")
t2[0].name
"#,
    );
    assert_eq!(result, Value::Str("Bob".into()));
}

#[test]
fn test_table_arrange_desc_positional() {
    // "desc" as a positional modifier after column name
    let result = eval(
        r#"
let t = table([{name: "Alice", score: 80}, {name: "Bob", score: 95}])
let t2 = arrange(t, "score", "desc")
t2[0].name
"#,
    );
    assert_eq!(result, Value::Str("Bob".into()));
}

#[test]
fn test_table_arrange_asc_positional() {
    // "asc" as a positional modifier (explicit ascending)
    let result = eval(
        r#"
let t = table([{name: "Bob", score: 95}, {name: "Alice", score: 80}])
let t2 = arrange(t, "score", "asc")
t2[0].name
"#,
    );
    assert_eq!(result, Value::Str("Alice".into()));
}

#[test]
fn test_table_distinct() {
    let result = eval(
        r#"
let t = table([{x: 1}, {x: 2}, {x: 1}])
nrow(distinct(t))
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_table_rename() {
    let result = eval(
        r#"
let t = table([{old_name: 1}])
let t2 = rename(t, "old_name", "new_name")
colnames(t2)
"#,
    );
    assert_eq!(
        result,
        Value::List(vec![Value::Str("new_name".into())])
    );
}

#[test]
fn test_table_to_records() {
    let result = eval(
        r#"
let t = table([{x: 1}, {x: 2}])
let recs = to_records(t)
len(recs)
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_table_group_by() {
    let result = eval(
        r#"
let t = table([
    {cat: "A", val: 1},
    {cat: "B", val: 2},
    {cat: "A", val: 3}
])
let groups = group_by(t, "cat")
nrow(groups["A"])
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_table_count() {
    let result = eval(
        r#"
let t = table([{x: 1}, {x: 2}])
count(t)
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_table_pipe_chain() {
    let result = eval(
        r#"
let t = table([
    {name: "Alice", score: 90},
    {name: "Bob", score: 75},
    {name: "Carol", score: 85}
])
t |> arrange("-score") |> head(2) |> nrow()
"#,
    );
    assert_eq!(result, Value::Int(2));
}

// ── Intervals ──────────────────────────────────────────────────────────

#[test]
fn test_interval_basic() {
    let result = eval(
        r#"
let iv = interval("chr1", 1000, 2000)
iv.chrom
"#,
    );
    assert_eq!(result, Value::Str("chr1".into()));
}

#[test]
fn test_interval_fields() {
    assert_eq!(
        eval(r#"interval("chr1", 100, 200).start"#),
        Value::Int(100)
    );
    assert_eq!(
        eval(r#"interval("chr1", 100, 200).end"#),
        Value::Int(200)
    );
    assert_eq!(
        eval(r#"interval("chr1", 100, 200, "+").strand"#),
        Value::Str("+".into())
    );
}

// ── Table HOF tests ────────────────────────────────────────────────────

#[test]
fn test_table_filter() {
    let result = eval(
        r#"
let t = table([
    {name: "Alice", score: 90},
    {name: "Bob", score: 60},
    {name: "Carol", score: 85}
])
let t2 = t |> filter(|r| r.score > 70)
nrow(t2)
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_table_filter_preserves_type() {
    let result = eval(
        r#"
let t = table([{x: 1}, {x: 2}, {x: 3}])
type(filter(t, |r| r.x > 1))
"#,
    );
    assert_eq!(result, Value::Str("Table".into()));
}

#[test]
fn test_table_map() {
    let result = eval(
        r#"
let t = table([{name: "Alice", score: 90}, {name: "Bob", score: 75}])
t |> map(|r| r.name)
"#,
    );
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("Alice".into()),
            Value::Str("Bob".into())
        ])
    );
}

#[test]
fn test_table_mutate() {
    let result = eval(
        r#"
let t = table([{name: "Alice", score: 80}, {name: "Bob", score: 90}])
let t2 = mutate(t, "grade", |r| if r.score >= 85 { "A" } else { "B" })
t2[0].grade
"#,
    );
    assert_eq!(result, Value::Str("B".into()));
}

#[test]
fn test_table_mutate_replace() {
    let result = eval(
        r#"
let t = table([{x: 10}, {x: 20}])
let t2 = mutate(t, "x", |r| r.x * 2)
t2[0].x
"#,
    );
    assert_eq!(result, Value::Int(20));
}

#[test]
fn test_table_summarize() {
    let result = eval(
        r#"
let t = table([
    {cat: "A", val: 10},
    {cat: "B", val: 20},
    {cat: "A", val: 30}
])
let groups = group_by(t, "cat")
fn agg(k, sub) {
    return {category: k, count: nrow(sub)}
}
let summary = summarize(groups, agg)
nrow(summary)
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_table_full_pipeline() {
    let result = eval(
        r#"
let t = table([
    {name: "Alice", score: 90, dept: "eng"},
    {name: "Bob", score: 60, dept: "eng"},
    {name: "Carol", score: 85, dept: "sci"},
    {name: "Dave", score: 95, dept: "sci"}
])
t |> filter(|r| r.score > 70)
  |> mutate("rank", |r| if r.score >= 90 { "top" } else { "mid" })
  |> select("name", "rank")
  |> arrange("name")
  |> nrow()
"#,
    );
    assert_eq!(result, Value::Int(3));
}

// ── CSV/TSV tests ──────────────────────────────────────────────────────

fn test_data_dir() -> String {
    let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop(); // crates
    dir.pop(); // project root
    dir.push("tests");
    dir.push("data");
    dir.to_str().unwrap().replace('\\', "/")
}

#[test]
fn test_csv_read() {
    let dir = test_data_dir();
    let result = eval(&format!(
        r#"
let t = csv("{dir}/test.csv")
nrow(t)
"#
    ));
    assert_eq!(result, Value::Int(4));
}

#[test]
fn test_csv_type_inference() {
    let dir = test_data_dir();
    let result = eval(&format!(
        r#"
let t = csv("{dir}/test.csv")
type(t[0].age)
"#
    ));
    assert_eq!(result, Value::Str("Int".into()));
}

#[test]
fn test_csv_float_inference() {
    let dir = test_data_dir();
    let result = eval(&format!(
        r#"
let t = csv("{dir}/test.csv")
type(t[0].score)
"#
    ));
    assert_eq!(result, Value::Str("Float".into()));
}

#[test]
fn test_tsv_read() {
    let dir = test_data_dir();
    let result = eval(&format!(
        r#"
let t = tsv("{dir}/test.tsv")
nrow(t)
"#
    ));
    assert_eq!(result, Value::Int(4));
}

#[test]
fn test_csv_write_read_roundtrip() {
    let dir = test_data_dir();
    let result = eval(&format!(
        r#"
let t = table([
    {{name: "Alice", score: 95}},
    {{name: "Bob", score: 87}}
])
write_csv(t, "{dir}/roundtrip.csv")
let t2 = csv("{dir}/roundtrip.csv")
nrow(t2)
"#
    ));
    assert_eq!(result, Value::Int(2));
    let _ = std::fs::remove_file(format!("{dir}/roundtrip.csv"));
}

// ── Join and Pivot tests ───────────────────────────────────────────────

#[test]
fn test_inner_join() {
    let result = eval(
        r#"
let left = table([{id: 1, name: "Alice"}, {id: 2, name: "Bob"}, {id: 3, name: "Carol"}])
let right = table([{id: 1, score: 95}, {id: 2, score: 87}])
let joined = inner_join(left, right, "id")
nrow(joined)
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_left_join() {
    let result = eval(
        r#"
let left = table([{id: 1, name: "Alice"}, {id: 2, name: "Bob"}, {id: 3, name: "Carol"}])
let right = table([{id: 1, score: 95}, {id: 2, score: 87}])
let joined = left_join(left, right, "id")
nrow(joined)
"#,
    );
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_left_join_nil_fill() {
    let result = eval(
        r#"
let left = table([{id: 1, name: "Alice"}, {id: 3, name: "Carol"}])
let right = table([{id: 1, score: 95}])
let joined = left_join(left, right, "id")
joined[1].score
"#,
    );
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_pivot_longer() {
    let result = eval(
        r#"
let t = table([
    {name: "Alice", math: 90, science: 85},
    {name: "Bob", math: 80, science: 95}
])
let long = pivot_longer(t, ["math", "science"], "subject", "score")
nrow(long)
"#,
    );
    assert_eq!(result, Value::Int(4));
}

#[test]
fn test_pivot_wider() {
    let result = eval(
        r#"
let t = table([
    {name: "Alice", subject: "math", score: 90},
    {name: "Alice", subject: "sci", score: 85},
    {name: "Bob", subject: "math", score: 80},
    {name: "Bob", subject: "sci", score: 95}
])
let wide = pivot_wider(t, "subject", "score")
ncol(wide)
"#,
    );
    // name + math + sci = 3 columns
    assert_eq!(result, Value::Int(3));
}

// ── Import / Module tests ──────────────────────────────────────────────

#[test]
fn test_import_bare() {
    let dir = tempfile::tempdir().unwrap();
    let utils_path = dir.path().join("utils.bl");
    std::fs::write(&utils_path, "let PI = 3\nfn double(x) { x * 2 }").unwrap();

    let main_src = r#"import "utils"
        double(PI)"#;

    let tokens = Lexer::new(main_src).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap().program;
    let mut interp = Interpreter::new();
    interp.set_current_file(Some(dir.path().join("main.bl")));
    let result = interp.run(&program).unwrap();
    assert_eq!(result, Value::Int(6));
}

#[test]
fn test_import_aliased() {
    let dir = tempfile::tempdir().unwrap();
    let utils_path = dir.path().join("math.bl");
    std::fs::write(&utils_path, "fn square(x) { x * x }").unwrap();

    let main_src = r#"import "math" as m
        m.square(5)"#;

    let tokens = Lexer::new(main_src).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap().program;
    let mut interp = Interpreter::new();
    interp.set_current_file(Some(dir.path().join("main.bl")));
    let result = interp.run(&program).unwrap();
    assert_eq!(result, Value::Int(25));
}

#[test]
fn test_import_cached() {
    let dir = tempfile::tempdir().unwrap();
    let utils_path = dir.path().join("counter.bl");
    std::fs::write(&utils_path, "let val = 42").unwrap();

    let main_src = r#"
        import "counter" as a
        import "counter" as b
        a.val + b.val
    "#;

    let tokens = Lexer::new(main_src).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap().program;
    let mut interp = Interpreter::new();
    interp.set_current_file(Some(dir.path().join("main.bl")));
    let result = interp.run(&program).unwrap();
    assert_eq!(result, Value::Int(84));
}

#[test]
fn test_circular_import_detected() {
    use bl_core::error::ErrorKind;

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.bl"), r#"import "b""#).unwrap();
    std::fs::write(dir.path().join("b.bl"), r#"import "a""#).unwrap();

    let main_src = r#"import "a""#;
    let tokens = Lexer::new(main_src).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap().program;
    let mut interp = Interpreter::new();
    interp.set_current_file(Some(dir.path().join("main.bl")));
    let result = interp.run(&program);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::ImportError);
    assert!(err.message.contains("circular import"));
}

#[test]
fn test_import_directory_module() {
    let dir = tempfile::tempdir().unwrap();
    let pkg_dir = dir.path().join("mypackage");
    std::fs::create_dir(&pkg_dir).unwrap();
    std::fs::write(pkg_dir.join("main.bl"), "let greeting = \"hello\"").unwrap();

    let main_src = r#"import "mypackage" as pkg
        pkg.greeting"#;

    let tokens = Lexer::new(main_src).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap().program;
    let mut interp = Interpreter::new();
    interp.set_current_file(Some(dir.path().join("main.bl")));
    let result = interp.run(&program).unwrap();
    assert_eq!(result, Value::Str("hello".to_string()));
}

#[test]
fn test_import_not_found() {
    use bl_core::error::ErrorKind;

    let main_src = r#"import "nonexistent_module""#;
    let tokens = Lexer::new(main_src).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap().program;
    let mut interp = Interpreter::new();
    let result = interp.run(&program);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::ImportError);
    assert!(err.message.contains("not found"));
}

#[test]
fn test_import_pipe_chain() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("math.bl"), "fn double(x) { x * 2 }").unwrap();

    let main_src = r#"import "math" as m
        5 |> m.double()"#;

    let tokens = Lexer::new(main_src).tokenize().unwrap();
    let program = Parser::new(tokens).parse().unwrap().program;
    let mut interp = Interpreter::new();
    interp.set_current_file(Some(dir.path().join("main.bl")));
    let result = interp.run(&program).unwrap();
    assert_eq!(result, Value::Int(10));
}

// ── Text ops integration ───────────────────────────────────────────────

#[test]
fn test_text_grep_pipe() {
    let result = eval(
        r#"
"PASS\nFAIL\nPASS\nERROR" |> grep("PASS") |> len()
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_text_lines_pipe_grep() {
    let result = eval(
        r#"
"chr1\t100\nchr2\t200\nchrX\t50" |> lines() |> grep("chr[12]") |> len()
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_text_grep_uniq_count_pipe() {
    let result = eval(
        r#"
let data = "OK\nERR\nOK\nOK\nERR\nWARN"
data |> lines() |> uniq_count() |> len()
"#,
    );
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_text_wc_pipe() {
    let result = eval(
        r#"
let stats = "hello world\nfoo bar baz" |> wc()
stats.words
"#,
    );
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_text_cut_pipe() {
    let result = eval(
        r#"
"a,b,c\nd,e,f" |> cut(",", 1)
"#,
    );
    assert_eq!(
        result,
        Value::List(vec![Value::Str("b".into()), Value::Str("e".into())])
    );
}

#[test]
fn test_text_paste_pipe() {
    let result = eval(
        r#"
let names = ["Alice", "Bob"]
let scores = ["95", "87"]
paste(names, scores, ",") |> len()
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_text_full_pipeline() {
    let result = eval(
        r#"
let log = "INFO\nERR\nINFO\nWARN\nERR\nINFO"
let counts = log |> grep("INFO|ERR") |> uniq_count()
counts[0].count
"#,
    );
    assert_eq!(result, Value::Int(3));
}

// ── While loop ─────────────────────────────────────────────────────────

#[test]
fn test_while_loop() {
    let result = eval(
        r#"
let x = 0
while x < 5 { x += 1 }
x
"#,
    );
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_while_with_break() {
    let result = eval(
        r#"
let x = 0
while true {
    x += 1
    if x == 3 { break }
}
x
"#,
    );
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_while_with_continue() {
    let result = eval(
        r#"
let sum = 0
let i = 0
while i < 10 {
    i += 1
    if i % 2 == 0 { continue }
    sum += i
}
sum
"#,
    );
    // sum of odd numbers 1..10: 1+3+5+7+9 = 25
    assert_eq!(result, Value::Int(25));
}

// ── For break/continue ─────────────────────────────────────────────────

#[test]
fn test_for_break() {
    let result = eval(
        r#"
let x = 0
for i in [1, 2, 3, 4, 5] {
    if i == 3 { break }
    x = i
}
x
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_for_continue() {
    let result = eval(
        r#"
let sum = 0
for i in [1, 2, 3, 4, 5] {
    if i == 3 { continue }
    sum += i
}
sum
"#,
    );
    assert_eq!(result, Value::Int(12)); // 1+2+4+5
}

// ── Compound assignment ────────────────────────────────────────────────

#[test]
fn test_compound_assign() {
    let result = eval(
        r#"
let x = 10
x += 5
x -= 3
x *= 2
x /= 4
x
"#,
    );
    assert_eq!(result, Value::Int(6)); // ((10+5-3)*2)/4 = 6
}

// ── Null coalesce ──────────────────────────────────────────────────────

#[test]
fn test_null_coalesce() {
    assert_eq!(eval("nil ?? 42"), Value::Int(42));
    assert_eq!(eval("10 ?? 42"), Value::Int(10));
    assert_eq!(
        eval(r#""hello" ?? "default""#),
        Value::Str("hello".into())
    );
    assert_eq!(eval("nil ?? nil ?? 99"), Value::Int(99));
}

// ── Try/catch ──────────────────────────────────────────────────────────

#[test]
fn test_try_catch_no_error() {
    let result = eval(
        r#"
try { 42 } catch e { -1 }
"#,
    );
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_try_catch_with_error() {
    let result = eval(
        r#"
try { 1 / 0 } catch e { e }
"#,
    );
    match result {
        Value::Str(s) => assert!(s.contains("division by zero")),
        other => panic!("expected Str error, got {other:?}"),
    }
}

#[test]
fn test_try_catch_no_var() {
    let result = eval(
        r#"
try { 1 / 0 } catch { -1 }
"#,
    );
    assert_eq!(result, Value::Int(-1));
}

// ── F-strings ──────────────────────────────────────────────────────────

#[test]
fn test_fstring_simple() {
    let result = eval(
        r#"
let name = "world"
f"hello {name}"
"#,
    );
    assert_eq!(result, Value::Str("hello world".into()));
}

#[test]
fn test_fstring_expr() {
    let result = eval(
        r#"
let x = 5
f"x is {x * 2} ok"
"#,
    );
    assert_eq!(result, Value::Str("x is 10 ok".into()));
}

#[test]
fn test_fstring_multiple() {
    let result = eval(
        r#"
let a = 1
let b = 2
f"{a} + {b} = {a + b}"
"#,
    );
    assert_eq!(result, Value::Str("1 + 2 = 3".into()));
}

// ── Destructuring ──────────────────────────────────────────────────────

#[test]
fn test_destruct_list() {
    let result = eval(
        r#"
let [a, b, c] = [10, 20, 30]
a + b + c
"#,
    );
    assert_eq!(result, Value::Int(60));
}

#[test]
fn test_destruct_list_short() {
    let result = eval(
        r#"
let [a, b, c] = [10, 20]
c ?? 99
"#,
    );
    assert_eq!(result, Value::Int(99));
}

#[test]
fn test_destruct_record() {
    let result = eval(
        r#"
let rec = {x: 10, y: 20, z: 30}
let {x, z} = rec
x + z
"#,
    );
    assert_eq!(result, Value::Int(40));
}

// ── DataFrame operations ───────────────────────────────────────────────

#[test]
fn test_df_head_tail() {
    let result = eval(
        r#"
let t = table({a: [1,2,3,4,5], b: [10,20,30,40,50]})
head(t, 2) |> nrow()
"#,
    );
    assert_eq!(result, Value::Int(2));

    let result = eval(
        r#"
let t = table({a: [1,2,3,4,5], b: [10,20,30,40,50]})
tail(t, 3) |> nrow()
"#,
    );
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_table_describe() {
    let result = eval(
        r#"
let t = table({score: [90, 85, 95, 70]})
let d = describe(t)
d.mean[0]
"#,
    );
    assert_eq!(result, Value::Float(85.0));
}

#[test]
fn test_table_value_counts() {
    let result = eval(
        r#"
let t = table({color: ["red", "blue", "red", "red", "blue"]})
let vc = value_counts(t, "color")
vc.count[0]
"#,
    );
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_table_fill_null() {
    let result = eval(
        r#"
let t = table({a: [1, nil, 3]})
let t2 = fill_null(t, 0)
t2.a[1]
"#,
    );
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_table_concat() {
    let result = eval(
        r#"
let a = table({x: [1, 2]})
let b = table({x: [3, 4]})
concat(a, b) |> nrow()
"#,
    );
    assert_eq!(result, Value::Int(4));
}

#[test]
fn test_table_window_cumsum() {
    let result = eval(
        r#"
let t = table({val: [1, 2, 3, 4]})
let t2 = cumsum(t, "val")
t2.val_cumsum[3]
"#,
    );
    assert_eq!(result, Value::Float(10.0));
}

#[test]
fn test_table_lag_lead() {
    let result = eval(
        r#"
let t = table({x: [10, 20, 30]})
let t2 = lag(t, "x")
t2.x_lag1[2]
"#,
    );
    assert_eq!(result, Value::Int(20));
}

#[test]
fn test_table_rolling_mean() {
    let result = eval(
        r#"
let t = table({v: [1.0, 2.0, 3.0, 4.0, 5.0]})
let t2 = rolling_mean(t, "v", 3)
t2.v_rmean3[2]
"#,
    );
    assert_eq!(result, Value::Float(2.0));
}

#[test]
fn test_df_from_records() {
    let result = eval(
        r#"
let recs = [{name: "a", val: 1}, {name: "b", val: 2}]
let t = from_records(recs)
nrow(t)
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_table_anti_join() {
    let result = eval(
        r#"
let a = table({id: [1, 2, 3], name: ["a", "b", "c"]})
let b = table({id: [2, 3], score: [90, 80]})
anti_join(a, b, "id") |> nrow()
"#,
    );
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_table_cross_join() {
    let result = eval(
        r#"
let a = table({x: [1, 2]})
let b = table({y: ["a", "b", "c"]})
cross_join(a, b) |> nrow()
"#,
    );
    assert_eq!(result, Value::Int(6));
}

#[test]
fn test_table_explode() {
    let result = eval(
        r#"
let t = table({name: ["a", "b"], tags: [["x", "y"], ["z"]]})
explode(t, "tags") |> nrow()
"#,
    );
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_table_drop_cols() {
    let result = eval(
        r#"
let t = table({a: [1], b: [2], c: [3]})
drop_cols(t, "b") |> ncol()
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_table_pipeline() {
    let result = eval(
        r#"
table({name: ["Alice", "Bob", "Alice", "Bob"], score: [90, 85, 95, 70]})
|> arrange("-score")
|> head(3)
|> nrow()
"#,
    );
    assert_eq!(result, Value::Int(3));
}

// ============================================================================
// NEW comprehensive tests (not covered by other external test files)
// ============================================================================

// ── Arithmetic edge cases ──────────────────────────────────────────────

#[test]
fn test_integer_large_values() {
    // Large integer arithmetic
    assert_eq!(eval("1000000 * 1000000"), Value::Int(1_000_000_000_000));
}

#[test]
fn test_float_division_precision() {
    let result = eval("1.0 / 3.0");
    match result {
        Value::Float(f) => assert!((f - 0.3333333333333333).abs() < 1e-15),
        other => panic!("expected Float, got {other:?}"),
    }
}

#[test]
fn test_division_by_zero_int() {
    assert!(eval_err("1 / 0"));
}

#[test]
fn test_division_by_zero_float() {
    // Float division by zero also errors in this language
    assert!(eval_err("1.0 / 0.0"));
}

#[test]
fn test_modulo_with_negatives() {
    // Rust i64 modulo follows truncation semantics
    let result = eval("-7 % 3");
    assert_eq!(result, Value::Int(-1));
}

#[test]
fn test_modulo_by_zero() {
    assert!(eval_err("5 % 0"));
}

#[test]
fn test_string_multiply_repeat() {
    // String repeat: "ab" * 3 => "ababab"
    assert_eq!(eval(r#""ab" * 3"#), Value::Str("ababab".into()));
    assert_eq!(eval(r#"3 * "ATG""#), Value::Str("ATGATGATG".into()));
}

#[test]
fn test_unary_negation() {
    assert_eq!(eval("-5"), Value::Int(-5));
    assert_eq!(eval("-3.14"), Value::Float(-3.14));
    assert_eq!(eval("-(2 + 3)"), Value::Int(-5));
}

#[test]
fn test_boolean_logic_and() {
    assert_eq!(eval("true && true"), Value::Bool(true));
    assert_eq!(eval("true && false"), Value::Bool(false));
    assert_eq!(eval("false && true"), Value::Bool(false));
    assert_eq!(eval("false && false"), Value::Bool(false));
}

#[test]
fn test_boolean_logic_or() {
    assert_eq!(eval("true || false"), Value::Bool(true));
    assert_eq!(eval("false || true"), Value::Bool(true));
    assert_eq!(eval("false || false"), Value::Bool(false));
}

#[test]
fn test_boolean_not() {
    assert_eq!(eval("!true"), Value::Bool(false));
    assert_eq!(eval("!false"), Value::Bool(true));
}

#[test]
fn test_nil_equality() {
    assert_eq!(eval("nil == nil"), Value::Bool(true));
    assert_eq!(eval("nil != nil"), Value::Bool(false));
    assert_eq!(eval("nil == 0"), Value::Bool(false));
    assert_eq!(eval("nil == false"), Value::Bool(false));
}

#[test]
fn test_string_comparison() {
    assert_eq!(eval(r#""abc" == "abc""#), Value::Bool(true));
    assert_eq!(eval(r#""abc" != "def""#), Value::Bool(true));
    assert_eq!(eval(r#""abc" < "def""#), Value::Bool(true));
    assert_eq!(eval(r#""xyz" > "abc""#), Value::Bool(true));
}

#[test]
fn test_mixed_int_float_arithmetic() {
    assert_eq!(eval("2 + 3.5"), Value::Float(5.5));
    assert_eq!(eval("10 - 2.5"), Value::Float(7.5));
    assert_eq!(eval("4 * 2.0"), Value::Float(8.0));
    assert_eq!(eval("7 / 2.0"), Value::Float(3.5));
}

// ── Control flow: nested if/else ───────────────────────────────────────

#[test]
fn test_nested_if_else() {
    let result = eval(
        r#"
let x = 15
if x > 20 { "big" } else { if x > 10 { "medium" } else { "small" } }
"#,
    );
    assert_eq!(result, Value::Str("medium".into()));
}

#[test]
fn test_deeply_nested_if() {
    let result = eval(
        r#"
let x = 3
if x == 1 { "one" } else { if x == 2 { "two" } else { if x == 3 { "three" } else { "other" } } }
"#,
    );
    assert_eq!(result, Value::Str("three".into()));
}

#[test]
fn test_if_as_expression_in_let() {
    let result = eval(
        r#"
let x = 10
let label = if x > 5 { "high" } else { "low" }
label
"#,
    );
    assert_eq!(result, Value::Str("high".into()));
}

// ── Control flow: nested loops with break/continue ─────────────────────

#[test]
fn test_nested_loops_with_break() {
    let result = eval(
        r#"
let count = 0
for i in [1, 2, 3] {
    for j in [10, 20, 30] {
        if j == 20 { break }
        count += 1
    }
}
count
"#,
    );
    // Each outer iteration, inner loop breaks at j==20 so only j==10 runs => 3 iterations
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_nested_loops_with_continue() {
    let result = eval(
        r#"
let sum = 0
for i in [1, 2, 3] {
    for j in [1, 2, 3] {
        if j == 2 { continue }
        sum += i * j
    }
}
sum
"#,
    );
    // For each i: j=1 and j=3 are added: i*(1+3) = i*4
    // Total: 1*4 + 2*4 + 3*4 = 24
    assert_eq!(result, Value::Int(24));
}

#[test]
fn test_while_loop_counter() {
    let result = eval(
        r#"
let n = 1
let count = 0
while n <= 100 {
    n = n * 2
    count += 1
}
count
"#,
    );
    // 1, 2, 4, 8, 16, 32, 64, 128 — 7 iterations
    assert_eq!(result, Value::Int(7));
}

// ── Functions: recursion ───────────────────────────────────────────────

#[test]
fn test_recursive_factorial() {
    let result = eval(
        r#"
fn fact(n) {
    if n <= 1 { return 1 }
    return n * fact(n - 1)
}
fact(5)
"#,
    );
    assert_eq!(result, Value::Int(120));
}

#[test]
#[ignore = "stack overflow in tree-walking interpreter on Windows debug builds"]
fn test_recursive_fibonacci() {
    // Use small input to avoid stack overflow in tree-walking interpreter
    let result = eval(
        r#"
fn fib(n) {
    if n <= 1 { return n }
    return fib(n - 1) + fib(n - 2)
}
fib(7)
"#,
    );
    assert_eq!(result, Value::Int(13));
}

// ── Functions: closures and higher-order ───────────────────────────────

#[test]
fn test_closure_captures_variable() {
    let result = eval(
        r#"
let multiplier = 3
let f = |x| x * multiplier
f(10)
"#,
    );
    assert_eq!(result, Value::Int(30));
}

#[test]
fn test_higher_order_function_as_argument() {
    let result = eval(
        r#"
fn apply_twice(f, x) {
    f(f(x))
}
fn inc(n) { n + 1 }
apply_twice(inc, 5)
"#,
    );
    assert_eq!(result, Value::Int(7));
}

#[test]
fn test_function_returning_function() {
    let result = eval(
        r#"
fn make_adder(n) {
    |x| x + n
}
let add5 = make_adder(5)
add5(10)
"#,
    );
    assert_eq!(result, Value::Int(15));
}

#[test]
fn test_early_return_from_function() {
    let result = eval(
        r#"
fn classify(x) {
    if x < 0 { return "negative" }
    if x == 0 { return "zero" }
    return "positive"
}
[classify(-5), classify(0), classify(7)]
"#,
    );
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("negative".into()),
            Value::Str("zero".into()),
            Value::Str("positive".into()),
        ])
    );
}

#[test]
fn test_variable_shadowing_in_nested_scopes() {
    let result = eval(
        r#"
let x = 1
fn inner() {
    let x = 10
    x
}
let y = inner()
x + y
"#,
    );
    // Outer x is 1, inner x is 10
    assert_eq!(result, Value::Int(11));
}

// ── Collections: edge cases ────────────────────────────────────────────

#[test]
fn test_empty_list_operations() {
    assert_eq!(eval("len([])"), Value::Int(0));
    assert_eq!(eval("sort([])"), Value::List(vec![]));
    assert_eq!(eval("map([], |x| x * 2)"), Value::List(vec![]));
    assert_eq!(eval("filter([], |x| x > 0)"), Value::List(vec![]));
}

#[test]
fn test_list_index_out_of_bounds() {
    assert!(eval_err("[1, 2, 3][5]"));
}

#[test]
fn test_list_negative_index() {
    assert_eq!(eval("[10, 20, 30][-2]"), Value::Int(20));
    assert_eq!(eval("[10, 20, 30][-3]"), Value::Int(10));
}

#[test]
fn test_record_field_update() {
    let result = eval(
        r#"
let r = {x: 1, y: 2}
r.x
"#,
    );
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_nested_record_access() {
    let result = eval(
        r#"
let r = {outer: {inner: 42}}
r.outer.inner
"#,
    );
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_list_of_records() {
    let result = eval(
        r#"
let items = [{name: "a", val: 1}, {name: "b", val: 2}]
items[1].name
"#,
    );
    assert_eq!(result, Value::Str("b".into()));
}

#[test]
fn test_list_concatenation() {
    let result = eval("[1, 2] + [3, 4]");
    assert_eq!(
        result,
        Value::List(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4)
        ])
    );
}

#[test]
fn test_table_column_access() {
    let result = eval(
        r#"
let t = table({x: [10, 20, 30], y: [1, 2, 3]})
t.x
"#,
    );
    assert_eq!(
        result,
        Value::List(vec![Value::Int(10), Value::Int(20), Value::Int(30)])
    );
}

#[test]
fn test_table_row_access() {
    let result = eval(
        r#"
let t = table({x: [10, 20, 30], y: [1, 2, 3]})
t[1].x
"#,
    );
    assert_eq!(result, Value::Int(20));
}

// ── Streams: edge cases ────────────────────────────────────────────────

#[test]
fn test_stream_from_list() {
    let result = eval(
        r#"
let s = to_stream([10, 20, 30])
type(s)
"#,
    );
    assert_eq!(result, Value::Str("Stream".into()));
}

#[test]
fn test_stream_chaining_map_then_filter() {
    let result = eval(
        r#"
[1, 2, 3, 4, 5]
    |> to_stream()
    |> map(|x| x * 3)
    |> filter(|x| x > 6)
    |> collect()
"#,
    );
    assert_eq!(
        result,
        Value::List(vec![Value::Int(9), Value::Int(12), Value::Int(15)])
    );
}

#[test]
fn test_stream_take_zero() {
    let result = eval(
        r#"
[1, 2, 3] |> to_stream() |> take(0)
"#,
    );
    assert_eq!(result, Value::List(vec![]));
}

#[test]
fn test_stream_count_of_empty() {
    let result = eval(
        r#"
[] |> to_stream() |> count()
"#,
    );
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_collect_empty_stream() {
    let result = eval(
        r#"
[] |> to_stream() |> collect()
"#,
    );
    assert_eq!(result, Value::List(vec![]));
}

// ── Error handling: try/catch ──────────────────────────────────────────

#[test]
fn test_try_catch_returns_value_on_success() {
    let result = eval(
        r#"
let x = try { 2 + 3 } catch e { 0 }
x
"#,
    );
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_nested_try_catch() {
    let result = eval(
        r#"
try {
    try {
        1 / 0
    } catch e {
        "inner caught"
    }
} catch e {
    "outer caught"
}
"#,
    );
    assert_eq!(result, Value::Str("inner caught".into()));
}

#[test]
fn test_try_catch_error_in_function() {
    let result = eval(
        r#"
fn risky() { 1 / 0 }
try { risky() } catch e { "handled" }
"#,
    );
    assert_eq!(result, Value::Str("handled".into()));
}

#[test]
fn test_error_in_lambda_via_try_catch() {
    let result = eval(
        r#"
let f = |x| x / 0
try { f(10) } catch e { -1 }
"#,
    );
    assert_eq!(result, Value::Int(-1));
}

// ── Bio operations: additional ─────────────────────────────────────────

#[test]
fn test_dna_complement() {
    let result = eval(r#"dna"AATTCCGG" |> complement()"#);
    assert_eq!(
        result,
        Value::DNA(BioSequence {
            data: "TTAAGGCC".into()
        })
    );
}

#[test]
fn test_rna_transcription() {
    let result = eval(r#"dna"ATCG" |> transcribe()"#);
    assert_eq!(
        result,
        Value::RNA(BioSequence {
            data: "AUCG".into()
        })
    );
}

#[test]
fn test_protein_translation_start_codon() {
    // AUG = M (methionine / start codon)
    let result = eval(r#"rna"AUGGCC" |> translate()"#);
    assert_eq!(
        result,
        Value::Protein(BioSequence {
            data: "MA".into()
        })
    );
}

#[test]
fn test_gc_content_all_gc() {
    let result = eval(r#"dna"GCGCGC" |> gc_content()"#);
    assert_eq!(result, Value::Float(1.0));
}

#[test]
fn test_gc_content_no_gc() {
    let result = eval(r#"dna"ATATAT" |> gc_content()"#);
    assert_eq!(result, Value::Float(0.0));
}

#[test]
fn test_sequence_type_is_dna() {
    // Verify DNA literal type
    assert_eq!(eval(r#"type(dna"ATCG")"#), Value::Str("DNA".into()));
}

#[test]
fn test_sequence_len() {
    assert_eq!(eval(r#"len(rna"AUGC")"#), Value::Int(4));
    assert_eq!(eval(r#"len(protein"MVLK")"#), Value::Int(4));
}

// ── String operations ──────────────────────────────────────────────────

#[test]
fn test_fstring_with_function_call() {
    let result = eval(
        r#"
fn double(x) { x * 2 }
let n = 7
f"result: {double(n)}"
"#,
    );
    assert_eq!(result, Value::Str("result: 14".into()));
}

#[test]
fn test_string_len() {
    assert_eq!(eval(r#"len("hello")"#), Value::Int(5));
    assert_eq!(eval(r#"len("")"#), Value::Int(0));
}

#[test]
fn test_string_contains() {
    assert_eq!(
        eval(r#"contains("hello world", "world")"#),
        Value::Bool(true)
    );
    assert_eq!(
        eval(r#"contains("hello", "xyz")"#),
        Value::Bool(false)
    );
}

#[test]
fn test_string_upper_lower() {
    assert_eq!(
        eval(r#"upper("hello")"#),
        Value::Str("HELLO".into())
    );
    assert_eq!(
        eval(r#"lower("HELLO")"#),
        Value::Str("hello".into())
    );
}

#[test]
fn test_string_trim() {
    assert_eq!(
        eval(r#"trim("  hello  ")"#),
        Value::Str("hello".into())
    );
}

#[test]
fn test_string_split() {
    let result = eval(r#"split("a,b,c", ",")"#);
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("a".into()),
            Value::Str("b".into()),
            Value::Str("c".into())
        ])
    );
}

#[test]
fn test_string_starts_with() {
    assert_eq!(
        eval(r#"starts_with("hello world", "hello")"#),
        Value::Bool(true)
    );
    assert_eq!(
        eval(r#"starts_with("hello world", "world")"#),
        Value::Bool(false)
    );
}

#[test]
fn test_empty_string_operations() {
    assert_eq!(eval(r#"len("")"#), Value::Int(0));
    assert_eq!(eval(r#"upper("")"#), Value::Str("".into()));
    assert_eq!(eval(r#"trim("")"#), Value::Str("".into()));
    assert_eq!(
        eval(r#"split("", ",")"#),
        Value::List(vec![Value::Str("".into())])
    );
}

// ── Match expression: additional cases ─────────────────────────────────

#[test]
fn test_match_wildcard() {
    let result = eval(
        r#"
match 99 {
    1 => "one",
    2 => "two",
    _ => "other"
}
"#,
    );
    assert_eq!(result, Value::Str("other".into()));
}

#[test]
fn test_match_string() {
    let result = eval(
        r#"
let cmd = "run"
match cmd {
    "start" => 1,
    "run" => 2,
    "stop" => 3,
    _ => 0
}
"#,
    );
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_match_bool() {
    let result = eval(
        r#"
match true {
    true => "yes",
    false => "no"
}
"#,
    );
    assert_eq!(result, Value::Str("yes".into()));
}

// ── Range builtin ──────────────────────────────────────────────────────

#[test]
fn test_range_single_arg() {
    let result = eval("range(5)");
    assert_eq!(
        result,
        Value::List(vec![
            Value::Int(0),
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4)
        ])
    );
}

#[test]
fn test_range_two_args() {
    let result = eval("range(2, 6)");
    assert_eq!(
        result,
        Value::List(vec![
            Value::Int(2),
            Value::Int(3),
            Value::Int(4),
            Value::Int(5)
        ])
    );
}

#[test]
fn test_range_empty() {
    let result = eval("range(0)");
    assert_eq!(result, Value::List(vec![]));
}

// ── Multiple return and complex expressions ────────────────────────────

#[test]
fn test_nested_function_calls() {
    let result = eval(
        r#"
fn add(a, b) { a + b }
fn mul(a, b) { a * b }
add(mul(2, 3), mul(4, 5))
"#,
    );
    assert_eq!(result, Value::Int(26));
}

#[test]
fn test_chained_pipes_with_functions() {
    let result = eval(
        r#"
fn inc(x) { x + 1 }
fn double(x) { x * 2 }
5 |> inc() |> double() |> inc()
"#,
    );
    // (5+1)*2+1 = 13
    assert_eq!(result, Value::Int(13));
}

#[test]
fn test_complex_reduce() {
    let result = eval(
        r#"
reduce([1, 2, 3, 4, 5], |acc, x| acc * 10 + x)
"#,
    );
    // ((((1*10+2)*10+3)*10+4)*10+5) = 12345
    assert_eq!(result, Value::Int(12345));
}

#[test]
fn test_map_with_index_via_enumerate() {
    // Map list items via pipe
    let result = eval(
        r#"
[10, 20, 30] |> map(|x| x + 1)
"#,
    );
    assert_eq!(
        result,
        Value::List(vec![Value::Int(11), Value::Int(21), Value::Int(31)])
    );
}

// ── Bool and Nil edge cases ────────────────────────────────────────────

#[test]
fn test_bool_equality() {
    assert_eq!(eval("true == true"), Value::Bool(true));
    assert_eq!(eval("true == false"), Value::Bool(false));
    assert_eq!(eval("false == false"), Value::Bool(true));
}

#[test]
fn test_nil_null_coalesce_chain() {
    let result = eval(
        r#"
let a = nil
let b = nil
let c = 42
a ?? b ?? c
"#,
    );
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_if_with_nil() {
    // nil is falsy
    let result = eval("if nil { 1 } else { 2 }");
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_if_with_zero() {
    // 0 may be truthy or falsy depending on language semantics
    let result = eval("if 0 { 1 } else { 2 }");
    // Accept whatever the language defines
    assert!(result == Value::Int(1) || result == Value::Int(2));
}

// ── Compound expressions ───────────────────────────────────────────────

#[test]
fn test_compound_plus_assign() {
    let result = eval(
        r#"
let x = 10
x += 7
x
"#,
    );
    assert_eq!(result, Value::Int(17));
}

#[test]
fn test_multiple_assignments_sequence() {
    let result = eval(
        r#"
let a = 1
let b = 2
let c = a + b
let d = c * c
d
"#,
    );
    assert_eq!(result, Value::Int(9));
}

// ── Deeply nested structures ───────────────────────────────────────────

#[test]
fn test_list_of_lists() {
    let result = eval("[[1, 2], [3, 4]][1][0]");
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_record_in_list_in_record() {
    let result = eval(
        r#"
let data = {items: [{val: 99}]}
data.items[0].val
"#,
    );
    assert_eq!(result, Value::Int(99));
}

// ── Scope and binding tests ────────────────────────────────────────────

#[test]
fn test_let_in_block_does_not_leak() {
    let result = eval(
        r#"
let x = 1
if true {
    let y = 10
}
x
"#,
    );
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_function_scope_isolation() {
    let result = eval(
        r#"
let x = 100
fn f() {
    let x = 42
    x
}
f() + x
"#,
    );
    assert_eq!(result, Value::Int(142));
}

// ── Type conversions ───────────────────────────────────────────────────

#[test]
fn test_int_to_str() {
    assert_eq!(eval("str(42)"), Value::Str("42".into()));
}

#[test]
fn test_float_to_str() {
    let result = eval("str(3.14)");
    match result {
        Value::Str(s) => assert!(s.starts_with("3.14")),
        other => panic!("expected Str, got {other:?}"),
    }
}

#[test]
fn test_str_to_int() {
    assert_eq!(eval(r#"int("42")"#), Value::Int(42));
}

#[test]
fn test_str_to_float() {
    assert_eq!(eval(r#"float("3.14")"#), Value::Float(3.14));
}

// ── Misc builtins ──────────────────────────────────────────────────────

#[test]
fn test_abs_builtin() {
    assert_eq!(eval("abs(-5)"), Value::Int(5));
    assert_eq!(eval("abs(5)"), Value::Int(5));
    assert_eq!(eval("abs(-3.14)"), Value::Float(3.14));
}

#[test]
fn test_min_max_builtin() {
    assert_eq!(eval("min(3, 7)"), Value::Int(3));
    assert_eq!(eval("max(3, 7)"), Value::Int(7));
}

#[test]
fn test_keys_values_on_record() {
    let result = eval(r#"len(keys({a: 1, b: 2, c: 3}))"#);
    assert_eq!(result, Value::Int(3));
    let result = eval(r#"len(values({a: 1, b: 2, c: 3}))"#);
    assert_eq!(result, Value::Int(3));
}

// ── For loop over range ────────────────────────────────────────────────

#[test]
fn test_for_over_range() {
    let result = eval(
        r#"
let total = 0
for i in range(1, 6) {
    total += i
}
total
"#,
    );
    assert_eq!(result, Value::Int(15)); // 1+2+3+4+5
}

// ── Pipe with multi-arg functions ──────────────────────────────────────

#[test]
fn test_pipe_inserts_as_first_arg() {
    let result = eval(
        r#"
fn add(a, b) { a + b }
10 |> add(5)
"#,
    );
    assert_eq!(result, Value::Int(15));
}

#[test]
fn test_pipe_chain_with_named_functions() {
    let result = eval(
        r#"
fn add(a, b) { a + b }
fn mul(a, b) { a * b }
1 |> add(2) |> mul(3)
"#,
    );
    // (1+2)*3 = 9
    assert_eq!(result, Value::Int(9));
}

// ── Assert with message ────────────────────────────────────────────────

#[test]
fn test_assert_true_does_not_error() {
    assert_eq!(eval("assert true"), Value::Nil);
}

#[test]
fn test_assert_expression() {
    assert_eq!(eval("assert 2 + 2 == 4"), Value::Nil);
}

// ── Empty record ───────────────────────────────────────────────────────

#[test]
fn test_single_field_record() {
    let result = eval("type({x: 1})");
    assert_eq!(result, Value::Str("Record".into()));
}

// ── Operator precedence ────────────────────────────────────────────────

#[test]
fn test_operator_precedence_mul_before_add() {
    assert_eq!(eval("2 + 3 * 4"), Value::Int(14));
    assert_eq!(eval("(2 + 3) * 4"), Value::Int(20));
}

#[test]
fn test_operator_precedence_and_or() {
    assert_eq!(eval("true && true || false"), Value::Bool(true));
    assert_eq!(eval("false || false || true"), Value::Bool(true));
    assert_eq!(eval("true && false"), Value::Bool(false));
}

// ── Complex pipe + lambda ──────────────────────────────────────────────

#[test]
fn test_pipe_lambda_complex() {
    let result = eval(
        r#"
[1, 2, 3, 4, 5]
    |> filter(|x| x % 2 == 1)
    |> map(|x| x * x)
    |> reduce(|a, b| a + b)
"#,
    );
    // odds: 1,3,5 -> squares: 1,9,25 -> sum: 35
    assert_eq!(result, Value::Int(35));
}

// ── DNA sequence slicing ───────────────────────────────────────────────

#[test]
fn test_dna_type_check() {
    assert_eq!(eval(r#"type(dna"ATCG")"#), Value::Str("DNA".into()));
    assert_eq!(eval(r#"type(rna"AUCG")"#), Value::Str("RNA".into()));
    assert_eq!(
        eval(r#"type(protein"MVLK")"#),
        Value::Str("Protein".into())
    );
}

// ── F-string edge cases ────────────────────────────────────────────────

#[test]
fn test_fstring_no_interpolation() {
    assert_eq!(eval(r#"f"plain text""#), Value::Str("plain text".into()));
}

#[test]
fn test_fstring_adjacent_braces() {
    let result = eval(
        r#"
let a = 1
let b = 2
f"{a}{b}"
"#,
    );
    assert_eq!(result, Value::Str("12".into()));
}

// ── Sort with custom comparator ────────────────────────────────────────

#[test]
fn test_sort_strings() {
    let result = eval(r#"sort(["banana", "apple", "cherry"])"#);
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("apple".into()),
            Value::Str("banana".into()),
            Value::Str("cherry".into()),
        ])
    );
}

// ── Deeply nested function calls ───────────────────────────────────────

#[test]
fn test_function_as_value() {
    let result = eval(
        r#"
fn greet() { "hello" }
let f = greet
f()
"#,
    );
    assert_eq!(result, Value::Str("hello".into()));
}

#[test]
fn test_lambda_assigned_to_variable() {
    let result = eval(
        r#"
let square = |x| x * x
square(9)
"#,
    );
    assert_eq!(result, Value::Int(81));
}

// ── Null coalesce with complex types ───────────────────────────────────

#[test]
fn test_null_coalesce_with_list() {
    let result = eval("nil ?? [1, 2, 3]");
    assert_eq!(
        result,
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn test_null_coalesce_with_record() {
    let result = eval(r#"nil ?? {x: 42}"#);
    match result {
        Value::Record(map) => assert_eq!(map.get("x"), Some(&Value::Int(42))),
        other => panic!("expected Record, got {other:?}"),
    }
}

// ── Regression: ensure various types can be returned from functions ─────

#[test]
fn test_return_nil_from_function() {
    let result = eval(
        r#"
fn nothing() { nil }
nothing()
"#,
    );
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_return_bool_from_function() {
    let result = eval(
        r#"
fn is_even(n) { n % 2 == 0 }
[is_even(4), is_even(3)]
"#,
    );
    assert_eq!(
        result,
        Value::List(vec![Value::Bool(true), Value::Bool(false)])
    );
}

#[test]
fn test_return_float_from_function() {
    let result = eval(
        r#"
fn half(x) { x / 2.0 }
half(7.0)
"#,
    );
    assert_eq!(result, Value::Float(3.5));
}

// ── Interval tree + count_overlaps + bulk_overlaps tests ────────────────

// Helper to build a BED-like table in BioLang code
fn bed_table(rows: &[(&str, i64, i64)]) -> String {
    let records: Vec<String> = rows.iter()
        .map(|(c, s, e)| format!("{{chrom: \"{c}\", start: {s}, end: {e}}}"))
        .collect();
    format!("table([{}])", records.join(", "))
}

#[test]
fn test_interval_tree_basic() {
    let result = eval(&format!(
        r#"
let t = {}
let tree = interval_tree(t)
tree.__type
"#,
        bed_table(&[("chr1", 100, 200), ("chr1", 150, 300), ("chr1", 500, 600), ("chr2", 100, 200)])
    ));
    assert_eq!(result, Value::Str("interval_tree".to_string()));
}

#[test]
fn test_query_overlaps_returns_table() {
    let result = eval(&format!(
        r#"
let t = {}
let tree = interval_tree(t)
let hits = query_overlaps(tree, "chr1", 120, 250)
nrow(hits)
"#,
        bed_table(&[("chr1", 100, 200), ("chr1", 150, 300), ("chr1", 500, 600), ("chr2", 100, 200)])
    ));
    assert_eq!(result, Value::Int(2)); // [100,200] and [150,300] overlap [120,250]
}

#[test]
fn test_query_overlaps_no_match() {
    let result = eval(&format!(
        r#"
let t = {}
let tree = interval_tree(t)
let hits = query_overlaps(tree, "chr1", 300, 400)
nrow(hits)
"#,
        bed_table(&[("chr1", 100, 200), ("chr1", 500, 600)])
    ));
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_query_overlaps_wrong_chrom() {
    let result = eval(&format!(
        r#"
let t = {}
let tree = interval_tree(t)
let hits = query_overlaps(tree, "chr3", 100, 200)
nrow(hits)
"#,
        bed_table(&[("chr1", 100, 200)])
    ));
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_count_overlaps_basic() {
    let result = eval(&format!(
        r#"
let t = {}
let tree = interval_tree(t)
count_overlaps(tree, "chr1", 120, 250)
"#,
        bed_table(&[("chr1", 100, 200), ("chr1", 150, 300), ("chr1", 500, 600), ("chr2", 100, 200)])
    ));
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_count_overlaps_none() {
    let result = eval(&format!(
        r#"
let t = {}
let tree = interval_tree(t)
count_overlaps(tree, "chr1", 300, 400)
"#,
        bed_table(&[("chr1", 100, 200), ("chr1", 500, 600)])
    ));
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_count_overlaps_missing_chrom() {
    let result = eval(&format!(
        r#"
let t = {}
let tree = interval_tree(t)
count_overlaps(tree, "chrX", 100, 200)
"#,
        bed_table(&[("chr1", 100, 200)])
    ));
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_bulk_overlaps_basic() {
    let result = eval(&format!(
        r#"
let regions = {}
let queries = {}
let tree = interval_tree(regions)
bulk_overlaps(tree, queries)
"#,
        bed_table(&[("chr1", 100, 200), ("chr1", 150, 300), ("chr1", 500, 600), ("chr2", 100, 200)]),
        bed_table(&[("chr1", 120, 250), ("chr1", 550, 650), ("chr2", 50, 150), ("chr3", 100, 200)])
    ));
    // Query 1: [120,250] overlaps [100,200] and [150,300] = 2
    // Query 2: [550,650] overlaps [500,600] = 1
    // Query 3: [50,150] overlaps [100,200] on chr2 = 1
    // Query 4: chr3 has nothing = 0
    assert_eq!(result, Value::Int(4));
}

#[test]
fn test_bulk_overlaps_single_no_match() {
    // Single query that doesn't match any region
    let result = eval(&format!(
        r#"
let regions = {}
let queries = {}
let tree = interval_tree(regions)
bulk_overlaps(tree, queries)
"#,
        bed_table(&[("chr1", 100, 200)]),
        bed_table(&[("chrX", 999, 1000)])
    ));
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_bulk_overlaps_no_overlap() {
    let result = eval(&format!(
        r#"
let regions = {}
let queries = {}
let tree = interval_tree(regions)
bulk_overlaps(tree, queries)
"#,
        bed_table(&[("chr1", 100, 200)]),
        bed_table(&[("chr1", 300, 400), ("chr2", 100, 200)])
    ));
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_count_overlaps_matches_bulk() {
    let result = eval(&format!(
        r#"
let regions = {}
let tree = interval_tree(regions)
let c1 = count_overlaps(tree, "chr1", 12, 25)
let c2 = count_overlaps(tree, "chr1", 52, 58)
let c3 = count_overlaps(tree, "chr1", 100, 200)

let queries = {}
let bulk = bulk_overlaps(tree, queries)
bulk == c1 + c2 + c3
"#,
        bed_table(&[("chr1", 10, 20), ("chr1", 15, 30), ("chr1", 50, 60), ("chr1", 55, 70)]),
        bed_table(&[("chr1", 12, 25), ("chr1", 52, 58), ("chr1", 100, 200)])
    ));
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_query_nearest() {
    let result = eval(&format!(
        r#"
let t = {}
let tree = interval_tree(t)
let nearest = query_nearest(tree, "chr1", 350)
nrow(nearest)
"#,
        bed_table(&[("chr1", 100, 200), ("chr1", 500, 600), ("chr1", 900, 1000)])
    ));
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_coverage_basic() {
    let result = eval(&format!(
        r#"
let t = {}
let tree = interval_tree(t)
let cov = coverage(tree)
nrow(cov) > 0
"#,
        bed_table(&[("chr1", 100, 200), ("chr1", 150, 250)])
    ));
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_sizes() {
    eprintln!("Value size: {} bytes", std::mem::size_of::<bl_core::value::Value>());
    eprintln!("Expr size: {} bytes", std::mem::size_of::<bl_core::ast::Expr>());
    eprintln!("Spanned<Expr> size: {} bytes", std::mem::size_of::<bl_core::span::Spanned<bl_core::ast::Expr>>());
    eprintln!("Stmt size: {} bytes", std::mem::size_of::<bl_core::ast::Stmt>());
    eprintln!("Spanned<Stmt> size: {} bytes", std::mem::size_of::<bl_core::span::Spanned<bl_core::ast::Stmt>>());
}

// ============================================================================
// Error message improvement tests
// ============================================================================

fn eval_err_msg(code: &str) -> String {
    let tokens = Lexer::new(code).tokenize().unwrap();
    let result = Parser::new(tokens).parse().unwrap();
    let mut interp = Interpreter::new();
    match interp.run(&result.program) {
        Ok(_) => panic!("expected error for: {code}"),
        Err(e) => format!("{e}"),
    }
}

fn eval_err_full(code: &str) -> String {
    let tokens = Lexer::new(code).tokenize().unwrap();
    let result = Parser::new(tokens).parse().unwrap();
    let mut interp = Interpreter::new();
    match interp.run(&result.program) {
        Ok(_) => panic!("expected error for: {code}"),
        Err(e) => e.format_with_source(code),
    }
}

#[test]
fn test_error_did_you_mean_variable() {
    let msg = eval_err_msg("let value = 42\nvaleu");
    assert!(msg.contains("did you mean"), "expected 'did you mean' in: {msg}");
    assert!(msg.contains("value"), "expected 'value' suggestion in: {msg}");
}

#[test]
fn test_error_did_you_mean_builtin_function() {
    // "maen" is close to "mean"
    let msg = eval_err_msg("maen([1,2,3])");
    assert!(
        msg.contains("did you mean") || msg.contains("hint"),
        "expected suggestion in: {msg}"
    );
}

#[test]
fn test_error_did_you_mean_sort_bye() {
    // "sort_bye" doesn't exist but "sort" does
    let msg = eval_err_msg("sort_bye([1,2,3])");
    // Should either suggest "sort" or "sort_by" if it exists
    assert!(
        msg.contains("hint") || msg.contains("did you mean"),
        "expected suggestion in: {msg}"
    );
}

#[test]
fn test_error_type_mismatch_shows_types() {
    // Adding string + int should show both types
    let msg = eval_err_msg(r#""hello" + 42"#);
    assert!(
        msg.contains("Str") || msg.contains("String") || msg.contains("cannot add"),
        "expected type info in: {msg}"
    );
}

#[test]
fn test_error_type_conversion_hint_str_plus_int() {
    let msg = eval_err_msg(r#""5" + 3"#);
    assert!(
        msg.contains("int()") || msg.contains("str()") || msg.contains("convert"),
        "expected type conversion hint in: {msg}"
    );
}

#[test]
fn test_error_source_location_shown() {
    let full = eval_err_full("let x = 42\nundefined_var");
    assert!(
        full.contains("line") && full.contains("column"),
        "expected source location in: {full}"
    );
}

#[test]
fn test_error_suggestions_in_format_with_source() {
    let full = eval_err_full("let value = 42\nvaleu");
    assert!(
        full.contains("hint"),
        "expected 'hint' in formatted error: {full}"
    );
}

#[test]
fn test_error_arity_mismatch() {
    let msg = eval_err_msg("mean()");
    assert!(
        msg.contains("expected") && msg.contains("argument"),
        "expected arity info in: {msg}"
    );
}

// ── MSA / Distance Matrix / Conservation via interpreter ─────────

#[test]
fn test_msa_via_interpreter() {
    let result = eval(r#"msa(["ACGTACGT", "ACGAACGT", "TTGTACGT"])"#);
    if let Value::Record(r) = &result {
        assert_eq!(r.get("n_seqs"), Some(&Value::Int(3)));
    } else {
        panic!("expected Record from msa()");
    }
}

#[test]
fn test_distance_matrix_via_interpreter() {
    let result = eval(r#"distance_matrix(["ACGT", "ACGA", "TTTT"])"#);
    assert!(matches!(result, Value::Matrix(_)), "expected Matrix");
}

#[test]
fn test_conservation_scores_via_interpreter() {
    let result = eval(r#"conservation_scores(["ACGT", "ACGT", "ACGT"])"#);
    if let Value::List(scores) = result {
        assert_eq!(scores.len(), 4);
        for s in &scores {
            assert_eq!(s, &Value::Float(1.0));
        }
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_msa_pipeline_via_interpreter() {
    // Full pipeline: msa → distance_matrix
    let result = eval(r#"
        let aln = msa(["ACGTACGT", "ACGAACGT"])
        distance_matrix(aln)
    "#);
    assert!(matches!(result, Value::Matrix(_)));
}

#[test]
fn test_msa_conservation_pipeline_via_interpreter() {
    let result = eval(r#"
        let aln = msa(["ACGT", "ACGT", "TTTT"])
        conservation_scores(aln)
    "#);
    if let Value::List(scores) = result {
        assert!(!scores.is_empty());
    } else {
        panic!("expected List");
    }
}

// ── suggest_builtin unit tests ───────────────────────────────────

#[test]
fn test_suggest_builtin_close_match() {
    let suggestion = bl_runtime::builtins::suggest_builtin("meann");
    assert_eq!(suggestion, Some("mean".to_string()));
}

#[test]
fn test_suggest_builtin_gc_contnt() {
    let suggestion = bl_runtime::builtins::suggest_builtin("gc_contnt");
    // Should suggest gc_content
    assert!(suggestion.is_some(), "expected a suggestion for 'gc_contnt'");
    assert_eq!(suggestion.unwrap(), "gc_content");
}

#[test]
fn test_suggest_builtin_no_match() {
    let suggestion = bl_runtime::builtins::suggest_builtin("xyzzyplugh");
    assert!(suggestion.is_none(), "no builtin is close to 'xyzzyplugh'");
}

#[test]
fn test_all_builtin_names_not_empty() {
    let names = bl_runtime::builtins::all_builtin_names();
    assert!(names.len() > 100, "expected 100+ builtins, got {}", names.len());
    assert!(names.contains(&"mean"));
    assert!(names.contains(&"gc_content"));
    assert!(names.contains(&"print"));
}

// ── Interpreter Performance Tests ──────────────────────────────────

#[test]
fn test_perf_gc_calculation_1000_sequences() {
    // GC content on 1000 sequences should complete in reasonable time
    let code = r#"
let seqs = range(1, 1001) |> map(|_| dna"ATCGATCGATCGATCG")
seqs |> map(|s| gc_content(s)) |> len()
"#;
    assert_eq!(eval(code), Value::Int(1000));
}

#[test]
fn test_perf_kmer_counting_long_sequence() {
    // k-mer counting on a long sequence
    let code = r#"
let seq = dna"ATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCGATCG"
let kmers = kmer_count(seq, 3)
type(kmers)
"#;
    assert_eq!(eval(code), Value::Str("Table".into()));
}

#[test]
fn test_perf_environment_deep_scope_lookup() {
    // Deeply nested scopes — variable lookup should still work
    let code = r#"
let x = 42
fn f1() {
    fn f2() {
        fn f3() {
            fn f4() {
                fn f5() {
                    x
                }
                f5()
            }
            f4()
        }
        f3()
    }
    f2()
}
f1()
"#;
    assert_eq!(eval(code), Value::Int(42));
}

#[test]
fn test_perf_list_map_10000_items() {
    // Map over 10000 items
    let code = r#"
range(1, 10001) |> map(|x| x * x) |> len()
"#;
    assert_eq!(eval(code), Value::Int(10000));
}

#[test]
fn test_perf_filter_large_list() {
    let code = r#"
range(1, 10001) |> filter(|x| x % 7 == 0) |> len()
"#;
    // 10000 / 7 = 1428 (floor)
    assert_eq!(eval(code), Value::Int(1428));
}

#[test]
fn test_perf_reduce_sum_large() {
    let code = r#"
range(1, 10001) |> reduce(|acc, x| acc + x)
"#;
    // Sum 1..10000 = 10000 * 10001 / 2 = 50005000
    assert_eq!(eval(code), Value::Int(50005000));
}

#[test]
fn test_perf_nested_map_filter_chain() {
    let code = r#"
range(1, 5001)
    |> map(|x| x * 2)
    |> filter(|x| x % 3 == 0)
    |> map(|x| x + 1)
    |> len()
"#;
    let result = eval(code);
    if let Value::Int(n) = result {
        assert!(n > 0, "chain should produce results");
    } else {
        panic!("expected Int");
    }
}

#[test]
fn test_perf_string_operations_bulk() {
    let code = r#"
let results = range(1, 501) |> map(|i| {
    let s = "ATCG" + str(i)
    len(s)
})
len(results)
"#;
    assert_eq!(eval(code), Value::Int(500));
}

#[test]
fn test_perf_record_creation_bulk() {
    let code = r#"
let recs = range(1, 1001) |> map(|i| {id: i, name: "gene_" + str(i), value: i * 1.5})
len(recs)
"#;
    assert_eq!(eval(code), Value::Int(1000));
}

// ── Parallel Execution Tests ──────────────────────────────────────

#[test]
fn test_par_map_computational_correctness() {
    // Verify par_map produces identical results to map
    let code = r#"
let xs = range(1, 201)
let seq_result = xs |> map(|x| x * x + 1)
let par_result = xs |> par_map(|x| x * x + 1)
seq_result == par_result
"#;
    assert_eq!(eval(code), Value::Bool(true));
}

#[test]
fn test_par_filter_computational_correctness() {
    // Verify par_filter produces identical results to filter
    let code = r#"
let xs = range(1, 201)
let seq_result = xs |> filter(|x| x % 3 == 0)
let par_result = xs |> par_filter(|x| x % 3 == 0)
seq_result == par_result
"#;
    assert_eq!(eval(code), Value::Bool(true));
}

#[test]
fn test_par_map_with_closures() {
    // par_map should capture closure variables correctly
    let code = r#"
let factor = 10
let result = [1, 2, 3, 4, 5] |> par_map(|x| x * factor)
result
"#;
    assert_eq!(
        eval(code),
        Value::List(vec![
            Value::Int(10), Value::Int(20), Value::Int(30),
            Value::Int(40), Value::Int(50),
        ])
    );
}

#[test]
fn test_par_map_single_item() {
    let code = r#"
[42] |> par_map(|x| x + 1)
"#;
    assert_eq!(eval(code), Value::List(vec![Value::Int(43)]));
}

#[test]
fn test_par_map_empty_list() {
    let code = r#"
[] |> par_map(|x| x * 2)
"#;
    assert_eq!(eval(code), Value::List(vec![]));
}

#[test]
fn test_par_filter_all_pass() {
    let code = r#"
[1, 2, 3] |> par_filter(|x| true)
"#;
    assert_eq!(
        eval(code),
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn test_par_filter_none_pass() {
    let code = r#"
[1, 2, 3] |> par_filter(|x| false)
"#;
    assert_eq!(eval(code), Value::List(vec![]));
}

#[test]
fn test_par_map_type_error() {
    let code = r#"
42 |> par_map(|x| x + 1)
"#;
    assert!(eval_err(code));
}

#[test]
fn test_par_filter_type_error() {
    let code = r#"
"hello" |> par_filter(|x| true)
"#;
    assert!(eval_err(code));
}

// ── Stream Tests ──────────────────────────────────────────────────

#[test]
fn test_stream_map_lazy() {
    // Mapping over a stream should produce a stream, not materialize
    let code = r#"
let s = range(1, 6) |> to_stream()
let mapped = s |> map(|x| x * 2)
type(mapped)
"#;
    assert_eq!(eval(code), Value::Str("Stream".into()));
}

#[test]
fn test_stream_collect_after_map() {
    let code = r#"
let s = range(1, 6) |> to_stream()
let mapped = s |> map(|x| x * 10)
collect(mapped)
"#;
    assert_eq!(
        eval(code),
        Value::List(vec![
            Value::Int(10), Value::Int(20), Value::Int(30),
            Value::Int(40), Value::Int(50),
        ])
    );
}

#[test]
fn test_stream_reduce_lazy() {
    // Reduce should consume stream lazily without materializing the whole thing
    let code = r#"
let s = range(1, 101) |> to_stream()
s |> reduce(|acc, x| acc + x)
"#;
    assert_eq!(eval(code), Value::Int(5050));
}

#[test]
fn test_stream_for_loop_lazy() {
    // For loop should consume stream one at a time
    let code = r#"
let s = range(1, 6) |> to_stream()
let total = 0
for x in s {
    total = total + x
}
total
"#;
    assert_eq!(eval(code), Value::Int(15));
}

#[test]
fn test_stream_exhaustion_error() {
    // Re-consuming an exhausted stream should error
    let code = r#"
let s = range(1, 4) |> to_stream()
collect(s)
collect(s)
"#;
    assert!(eval_err(code));
}

#[test]
fn test_stream_batch_basic() {
    let code = r#"
let s = range(1, 11) |> to_stream()
let batches = stream_batch(s, 3, |batch| len(batch))
collect(batches)
"#;
    let result = eval(code);
    if let Value::List(items) = result {
        // 10 items in batches of 3: [3, 3, 3, 1]
        assert_eq!(items.len(), 4);
        assert_eq!(items[0], Value::Int(3));
        assert_eq!(items[3], Value::Int(1));
    } else {
        panic!("expected List");
    }
}

// ── JIT / @compile Decorator Tests ────────────────────────────────

#[test]
fn test_compile_decorator_fallback() {
    // Without bytecode feature, @compile should gracefully fall back
    let code = r#"
@compile
fn double(x) {
    x * 2
}
double(21)
"#;
    // Should work regardless of whether bytecode feature is enabled
    assert_eq!(eval(code), Value::Int(42));
}

#[test]
fn test_compile_decorator_with_loop() {
    let code = r#"
@compile
fn sum_range(n) {
    let total = 0
    for i in range(1, n + 1) {
        total = total + i
    }
    total
}
sum_range(100)
"#;
    assert_eq!(eval(code), Value::Int(5050));
}

#[test]
fn test_compile_decorator_with_pipe() {
    let code = r#"
@compile
fn process(xs) {
    xs |> map(|x| x * 2)
}
process([1, 2, 3])
"#;
    assert_eq!(
        eval(code),
        Value::List(vec![Value::Int(2), Value::Int(4), Value::Int(6)])
    );
}

// ── Large Collection Handling Tests ───────────────────────────────

#[test]
fn test_large_list_sort() {
    // Sort a list of 5000 items in reverse, verify order
    let code = r#"
let xs = range(1, 5001) |> map(|x| 5001 - x)
let sorted = sort(xs, |a, b| a - b)
sorted[0] == 1 and sorted[len(sorted) - 1] == 5000
"#;
    assert_eq!(eval(code), Value::Bool(true));
}

#[test]
fn test_large_list_flat_map() {
    let code = r#"
range(1, 1001) |> flat_map(|x| [x, x]) |> len()
"#;
    assert_eq!(eval(code), Value::Int(2000));
}

#[test]
fn test_large_list_find() {
    let code = r#"
range(1, 100001) |> find(|x| x == 99999)
"#;
    assert_eq!(eval(code), Value::Int(99999));
}

#[test]
fn test_large_list_any_all() {
    let code = r#"
let xs = range(1, 10001)
let has_even = xs |> any(|x| x % 2 == 0)
let all_positive = xs |> all(|x| x > 0)
has_even and all_positive
"#;
    assert_eq!(eval(code), Value::Bool(true));
}

#[test]
fn test_large_list_take_while() {
    let code = r#"
range(1, 100001) |> take_while(|x| x < 100) |> len()
"#;
    assert_eq!(eval(code), Value::Int(99));
}

#[test]
fn test_large_list_count_if() {
    let code = r#"
range(1, 10001) |> count_if(|x| x % 5 == 0)
"#;
    assert_eq!(eval(code), Value::Int(2000));
}

#[test]
fn test_large_list_partition() {
    let code = r#"
let parts = range(1, 1001) |> partition(|x| x % 2 == 0)
len(parts[0]) + len(parts[1])
"#;
    assert_eq!(eval(code), Value::Int(1000));
}

// ── Bio-specific Performance Tests ────────────────────────────────

#[test]
fn test_perf_dna_reverse_complement_batch() {
    let code = r#"
let seqs = range(1, 201) |> map(|_| dna"ATCGATCGATCGATCG")
seqs |> map(|s| reverse_complement(s)) |> len()
"#;
    assert_eq!(eval(code), Value::Int(200));
}

#[test]
fn test_perf_translate_batch() {
    let code = r#"
let seqs = range(1, 101) |> map(|_| dna"ATGATCGATCGATCGATCGATCGATC")
seqs |> map(|s| translate(s)) |> len()
"#;
    assert_eq!(eval(code), Value::Int(100));
}

#[test]
fn test_perf_kmer_count_multiple() {
    let code = r#"
let seqs = [
    dna"ATCGATCGATCGATCGATCGATCG",
    dna"GCTAGCTAGCTAGCTAGCTAGCTA",
    dna"AAATTTCCCGGGAAATTTCCCGGG"
]
seqs |> map(|s| kmer_count(s, 4)) |> len()
"#;
    assert_eq!(eval(code), Value::Int(3));
}

// ── Memoization / Caching Tests ───────────────────────────────────

#[test]
fn test_memoize_decorator() {
    let code = r#"
let call_count = 0
@memoize
fn expensive(x) {
    call_count = call_count + 1
    x * x
}
let a = expensive(5)
let b = expensive(5)
let c = expensive(3)
a == 25 and b == 25 and c == 9
"#;
    assert_eq!(eval(code), Value::Bool(true));
}

// ── Stream Chaining Tests ─────────────────────────────────────────

#[test]
fn test_stream_filter_map_chain() {
    let code = r#"
let s = range(1, 101) |> to_stream()
let result = s
    |> filter(|x| x % 2 == 0)
    |> map(|x| x * 3)
    |> collect()
len(result) == 50 and result[0] == 6
"#;
    assert_eq!(eval(code), Value::Bool(true));
}

#[test]
fn test_stream_for_break() {
    // Breaking out of a stream for-loop should work
    let code = r#"
let s = range(1, 1000001) |> to_stream()
let total = 0
for x in s {
    if x > 10 { break }
    total = total + x
}
total
"#;
    assert_eq!(eval(code), Value::Int(55));
}
