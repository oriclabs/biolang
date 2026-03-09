/// Comprehensive tests for ALL language constructs, focused on edge cases
/// that could cause parser hangs (infinite recursion / OOM).
///
/// Each test must complete in bounded time. A hang = test timeout = bug.

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

fn eval_err(code: &str) -> bool {
    let tokens = match Lexer::new(code).tokenize() {
        Ok(t) => t,
        Err(_) => return true,
    };
    let ast = match Parser::new(tokens).parse() {
        Ok(a) => a,
        Err(_) => return true,
    };
    let mut interp = Interpreter::new();
    interp.run(&ast.program).is_err()
}

fn parses_ok(code: &str) -> bool {
    let tokens = match Lexer::new(code).tokenize() {
        Ok(t) => t,
        Err(_) => return false,
    };
    Parser::new(tokens).parse().is_ok()
}

// ============================================================================
// given { ... } — newline-separated arms (was OOM bug)
// ============================================================================

#[test]
fn test_given_newline_arms() {
    let result = eval(r#"
let x = 42
given {
    x < 0     => "negative"
    x == 0    => "zero"
    x < 100   => "small"
    otherwise => "large"
}
"#);
    assert_eq!(result, Value::Str("small".into()));
}

#[test]
fn test_given_comma_arms() {
    let result = eval(r#"
let x = 42
given {
    x < 0 => "negative",
    x == 0 => "zero",
    x < 100 => "small",
    otherwise => "large",
}
"#);
    assert_eq!(result, Value::Str("small".into()));
}

#[test]
fn test_given_mixed_separators() {
    let result = eval(r#"
let x = -5
given {
    x < 0 => "negative",
    x == 0 => "zero"
    x < 100 => "small"
    otherwise => "large"
}
"#);
    assert_eq!(result, Value::Str("negative".into()));
}

#[test]
fn test_given_single_arm() {
    let result = eval(r#"
given {
    true => 42
}
"#);
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_given_otherwise_only() {
    let result = eval(r#"
given {
    false => 1
    otherwise => 99
}
"#);
    assert_eq!(result, Value::Int(99));
}

#[test]
fn test_given_no_match_returns_nil() {
    let result = eval(r#"
given {
    false => 1
    false => 2
}
"#);
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_given_with_complex_conditions() {
    let result = eval(r#"
let a = 10
let b = 20
given {
    a > 100 && b > 100 => "both big"
    a + b == 30        => "sum is 30"
    otherwise           => "other"
}
"#);
    assert_eq!(result, Value::Str("sum is 30".into()));
}

#[test]
fn test_given_nested_in_for() {
    let result = eval(r#"
let results = []
for x in [1, -1, 0] {
    let label = given {
        x > 0     => "pos"
        x < 0     => "neg"
        otherwise => "zero"
    }
    results = results + [label]
}
results
"#);
    assert_eq!(result, Value::List(vec![
        Value::Str("pos".into()),
        Value::Str("neg".into()),
        Value::Str("zero".into()),
    ]));
}

#[test]
fn test_given_in_let_binding() {
    let result = eval(r#"
let x = 5
let y = given {
    x > 10 => "big"
    otherwise => "small"
}
y
"#);
    assert_eq!(result, Value::Str("small".into()));
}

#[test]
fn test_given_as_function_body() {
    let result = eval(r#"
fn classify(n) {
    given {
        n < 0     => "negative"
        n == 0    => "zero"
        n < 10    => "small"
        otherwise => "large"
    }
}
classify(5)
"#);
    assert_eq!(result, Value::Str("small".into()));
}

// ============================================================================
// match { ... } — newline-separated arms
// ============================================================================

#[test]
fn test_match_newline_arms() {
    let result = eval(r#"
let x = "hello"
match x {
    "hi" => 1
    "hello" => 2
    _ => 0
}
"#);
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_match_comma_arms() {
    let result = eval(r#"
match 42 {
    1 => "one",
    42 => "answer",
    _ => "other",
}
"#);
    assert_eq!(result, Value::Str("answer".into()));
}

#[test]
fn test_match_with_guard() {
    let result = eval(r#"
let n = 15
match n {
    x if x > 10 => "big"
    _ => "small"
}
"#);
    assert_eq!(result, Value::Str("big".into()));
}

// ============================================================================
// Multi-line pipe chains (newline before |>)
// ============================================================================

#[test]
fn test_multiline_pipe_chain() {
    let result = eval(r#"
let data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
data
    |> filter(|n| n > 5)
    |> map(|n| n * 2)
"#);
    assert_eq!(result, Value::List(vec![
        Value::Int(12), Value::Int(14), Value::Int(16),
        Value::Int(18), Value::Int(20),
    ]));
}

#[test]
fn test_multiline_pipe_in_let() {
    let result = eval(r#"
let result = [1, 2, 3, 4, 5]
    |> filter(|n| n > 2)
    |> map(|n| n * 10)
result
"#);
    assert_eq!(result, Value::List(vec![
        Value::Int(30), Value::Int(40), Value::Int(50),
    ]));
}

#[test]
fn test_multiline_pipe_three_stages() {
    let result = eval(r#"
let x = [10, 20, 30, 40, 50]
    |> filter(|n| n >= 20)
    |> map(|n| n + 1)
    |> filter(|n| n < 42)
x
"#);
    assert_eq!(result, Value::List(vec![
        Value::Int(21), Value::Int(31), Value::Int(41),
    ]));
}

// ============================================================================
// Trailing lambda in pipes: |> each |x| expr
// ============================================================================

#[test]
fn test_pipe_trailing_lambda_each() {
    let result = eval(r#"
[1, 2, 3] |> each |n| n * 2
"#);
    assert_eq!(result, Value::List(vec![
        Value::Int(2), Value::Int(4), Value::Int(6),
    ]));
}

#[test]
fn test_pipe_trailing_lambda_map() {
    let result = eval(r#"
[10, 20, 30] |> map |n| n + 1
"#);
    assert_eq!(result, Value::List(vec![
        Value::Int(11), Value::Int(21), Value::Int(31),
    ]));
}

#[test]
fn test_pipe_trailing_lambda_filter() {
    let result = eval(r#"
[1, 2, 3, 4, 5] |> filter |n| n > 3
"#);
    assert_eq!(result, Value::List(vec![
        Value::Int(4), Value::Int(5),
    ]));
}

#[test]
fn test_pipe_trailing_lambda_last_in_chain() {
    // Trailing lambda works on the LAST pipe in a chain.
    // Earlier pipes need parens because the lambda body would
    // swallow subsequent |> operators.
    let result = eval(r#"
[1, 2, 3, 4, 5]
    |> filter(|n| n > 2)
    |> map |n| n * 10
"#);
    assert_eq!(result, Value::List(vec![
        Value::Int(30), Value::Int(40), Value::Int(50),
    ]));
}

// ============================================================================
// each — returns collected results (was returning Nil)
// ============================================================================

#[test]
fn test_each_returns_list() {
    let result = eval(r#"
[1, 2, 3] |> each(|n| n * 2)
"#);
    assert_eq!(result, Value::List(vec![
        Value::Int(2), Value::Int(4), Value::Int(6),
    ]));
}

#[test]
fn test_each_on_table_returns_list() {
    let result = eval(r#"
let t = from_records([{a: 1, b: 10}, {a: 2, b: 20}])
t |> each(|r| r.a + r.b)
"#);
    assert_eq!(result, Value::List(vec![Value::Int(11), Value::Int(22)]));
}

#[test]
fn test_each_with_record_transform() {
    let result = eval(r#"
let data = [{name: "a", val: 1}, {name: "b", val: 2}]
data |> each |r| r.name
"#);
    assert_eq!(result, Value::List(vec![
        Value::Str("a".into()), Value::Str("b".into()),
    ]));
}

#[test]
fn test_each_empty_list() {
    let result = eval(r#"
[] |> each(|n| n * 2)
"#);
    assert_eq!(result, Value::List(vec![]));
}

// ============================================================================
// variant() — positional constructor (new)
// ============================================================================

#[test]
fn test_variant_positional_4_args() {
    let result = eval(r#"
let v = variant("chr1", 100, "A", "G")
v.chrom
"#);
    assert_eq!(result, Value::Str("chr1".into()));
}

#[test]
fn test_variant_positional_ref_alt_access() {
    let result = eval(r#"
let v = variant("chr7", 55181378, "T", "A")
v.ref + ">" + v.alt
"#);
    assert_eq!(result, Value::Str("T>A".into()));
}

#[test]
fn test_variant_positional_2_args() {
    let result = eval(r#"
let v = variant("chr1", 500)
v.pos
"#);
    assert_eq!(result, Value::Int(500));
}

#[test]
fn test_variant_positional_3_args() {
    let result = eval(r#"
let v = variant("chr1", 100, "A")
v.ref
"#);
    assert_eq!(result, Value::Str("A".into()));
}

#[test]
fn test_variant_record_form() {
    let result = eval(r#"
let v = variant({chrom: "chr1", pos: 100, ref: "A", alt: "G"})
v.alt
"#);
    assert_eq!(result, Value::Str("G".into()));
}

#[test]
fn test_variant_is_snp() {
    let result = eval(r#"
let v = variant("chr1", 100, "A", "G")
is_snp(v)
"#);
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_variant_is_indel() {
    let result = eval(r#"
let v = variant("chr1", 100, "ACG", "A")
is_indel(v)
"#);
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_variant_type_classification() {
    let result = eval(r#"
let v = variant("chr1", 100, "A", "G")
variant_type(v)
"#);
    assert_eq!(result, Value::Str("Snp".into()));
}

// ============================================================================
// for loops — various forms
// ============================================================================

#[test]
fn test_for_with_given_body() {
    let result = eval(r#"
let results = []
for x in [1, -1, 0, 5, -3] {
    let label = given {
        x > 0     => "pos"
        x < 0     => "neg"
        otherwise => "zero"
    }
    results = results + [label]
}
results
"#);
    assert_eq!(result, Value::List(vec![
        Value::Str("pos".into()),
        Value::Str("neg".into()),
        Value::Str("zero".into()),
        Value::Str("pos".into()),
        Value::Str("neg".into()),
    ]));
}

#[test]
fn test_for_with_match_body() {
    let result = eval(r#"
let results = []
for x in ["a", "b", "c"] {
    let val = match x {
        "a" => 1
        "b" => 2
        _ => 0
    }
    results = results + [val]
}
results
"#);
    assert_eq!(result, Value::List(vec![
        Value::Int(1), Value::Int(2), Value::Int(0),
    ]));
}

#[test]
fn test_for_with_pipe_chain_body() {
    let result = eval(r#"
let all = []
for group in [[1,2,3], [4,5,6]] {
    let doubled = group |> map(|n| n * 10)
    let s = doubled |> reduce(|a, b| a + b, 0)
    all = all + [s]
}
all
"#);
    assert_eq!(result, Value::List(vec![Value::Int(60), Value::Int(150)]));
}

#[test]
fn test_for_when_guard() {
    let result = eval(r#"
let evens = []
for x in range(1, 11) when x % 2 == 0 {
    evens = evens + [x]
}
evens
"#);
    assert_eq!(result, Value::List(vec![
        Value::Int(2), Value::Int(4), Value::Int(6),
        Value::Int(8), Value::Int(10),
    ]));
}

// ============================================================================
// while loops
// ============================================================================

#[test]
fn test_while_basic() {
    let result = eval(r#"
let x = 0
while x < 5 {
    x = x + 1
}
x
"#);
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_while_with_break() {
    let result = eval(r#"
let x = 0
while true {
    x = x + 1
    if x >= 3 { break }
}
x
"#);
    assert_eq!(result, Value::Int(3));
}

// ============================================================================
// unless
// ============================================================================

#[test]
fn test_unless_false_runs_body() {
    let result = eval(r#"
let x = 10
unless x > 100 {
    x = x + 1
}
x
"#);
    assert_eq!(result, Value::Int(11));
}

#[test]
fn test_unless_true_skips_body() {
    let result = eval(r#"
let x = 10
unless x < 100 {
    x = x + 1
}
x
"#);
    assert_eq!(result, Value::Int(10));
}

// ============================================================================
// Destructuring
// ============================================================================

#[test]
fn test_destruct_list() {
    let result = eval(r#"
let [a, b, c] = [10, 20, 30]
a + b + c
"#);
    assert_eq!(result, Value::Int(60));
}

#[test]
fn test_destruct_record() {
    let result = eval(r#"
let {x, y} = {x: 1, y: 2}
x + y
"#);
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_destruct_list_with_rest() {
    let result = eval(r#"
let [first, ...rest] = [1, 2, 3, 4, 5]
len(rest)
"#);
    assert_eq!(result, Value::Int(4));
}

// ============================================================================
// String interpolation
// ============================================================================

#[test]
fn test_fstring_basic() {
    let code = "let x = 42\nf\"value is {x}\"";
    let result = eval(code);
    assert_eq!(result, Value::Str("value is 42".into()));
}

#[test]
fn test_fstring_expr() {
    let code = "let a = 3\nlet b = 4\nf\"sum = {a + b}\"";
    let result = eval(code);
    assert_eq!(result, Value::Str("sum = 7".into()));
}

#[test]
fn test_fstring_nested_function() {
    let code = "let items = [1, 2, 3]\nf\"count: {len(items)}\"";
    let result = eval(code);
    assert_eq!(result, Value::Str("count: 3".into()));
}

// ============================================================================
// Triple-quoted strings
// ============================================================================

#[test]
fn test_triple_quote_basic() {
    let result = eval(r#"
let s = """
hello
world
"""
len(s) > 0
"#);
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_triple_quote_with_quotes_inside() {
    let result = eval(r#"
let s = """He said "hello" to them"""
s
"#);
    assert_eq!(result, Value::Str(r#"He said "hello" to them"#.into()));
}

// ============================================================================
// Pipe edge cases
// ============================================================================

#[test]
fn test_pipe_single() {
    let result = eval(r#"
5 |> str()
"#);
    assert_eq!(result, Value::Str("5".into()));
}

#[test]
fn test_pipe_inserts_first_arg() {
    let result = eval(r#"
fn add(a, b) { a + b }
10 |> add(5)
"#);
    assert_eq!(result, Value::Int(15));
}

#[test]
fn test_tap_pipe() {
    // |>> calls RHS with value for side effects, returns ORIGINAL value
    let result = eval(r#"
42 |>> print
"#);
    assert_eq!(result, Value::Int(42));
}

// ============================================================================
// Combined constructs (the user's real-world scenario)
// ============================================================================

#[test]
fn test_variant_pipeline_with_given() {
    let result = eval(r#"
let examples = [
    variant("chr1", 100, "A", "G"),
    variant("chr1", 200, "A", "T"),
    variant("chr1", 300, "ACG", "A"),
]

let labels = []
for v in examples {
    let detail = given {
        is_snp(v) && is_transition(v)   => "transition"
        is_snp(v) && is_transversion(v) => "transversion"
        is_indel(v)                     => "indel"
        otherwise                       => "other"
    }
    labels = labels + [detail]
}
labels
"#);
    assert_eq!(result, Value::List(vec![
        Value::Str("transition".into()),
        Value::Str("transversion".into()),
        Value::Str("indel".into()),
    ]));
}

#[test]
fn test_each_pipe_with_record_transform() {
    let result = eval(r#"
let data = [{name: "Alice", age: 30}, {name: "Bob", age: 25}]
data |> each |p| p.name
"#);
    assert_eq!(result, Value::List(vec![
        Value::Str("Alice".into()),
        Value::Str("Bob".into()),
    ]));
}

#[test]
fn test_filter_map_chain_multiline() {
    let result = eval(r#"
let result = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    |> filter(|n| n % 2 == 0)
    |> map(|n| n * n)
    |> filter(|n| n > 20)
result
"#);
    assert_eq!(result, Value::List(vec![
        Value::Int(36), Value::Int(64), Value::Int(100),
    ]));
}

// ============================================================================
// Parser safety — these must NOT hang (regression tests for OOM bugs)
// ============================================================================

#[test]
fn test_given_five_arms_newlines() {
    // Regression: given with >2 newline-separated arms used to OOM
    let result = eval(r#"
let x = 3
given {
    x == 1 => "one"
    x == 2 => "two"
    x == 3 => "three"
    x == 4 => "four"
    otherwise => "other"
}
"#);
    assert_eq!(result, Value::Str("three".into()));
}

#[test]
fn test_given_with_block_bodies() {
    let result = eval(r#"
let x = 5
given {
    x > 10 => {
        let a = x * 2
        a
    }
    otherwise => {
        let b = x + 1
        b
    }
}
"#);
    assert_eq!(result, Value::Int(6));
}

#[test]
fn test_deeply_nested_given() {
    let result = eval(r#"
let x = 1
let y = 2
given {
    x == 1 => given {
        y == 1 => "x1y1"
        y == 2 => "x1y2"
        otherwise => "x1y?"
    }
    otherwise => "x?"
}
"#);
    assert_eq!(result, Value::Str("x1y2".into()));
}

#[test]
fn test_string_key_record_in_given() {
    // Regression: string-key records were another parser OOM trigger
    let result = eval(r#"
let r = {"key": "value"}
given {
    r.key == "value" => "found"
    otherwise => "missing"
}
"#);
    assert_eq!(result, Value::Str("found".into()));
}

// ============================================================================
// Edge cases that should produce errors, not hangs
// ============================================================================

#[test]
fn test_variant_wrong_type_errors() {
    assert!(eval_err(r#"variant(123, "not_a_pos")"#));
}

#[test]
fn test_each_non_collection_errors() {
    assert!(eval_err(r#"each(42, |n| n)"#));
}

#[test]
fn test_given_empty_block() {
    // given {} should parse and return nil
    let result = eval("given {}");
    assert_eq!(result, Value::Nil);
}

// ===== Bio type validation tests =====

// -- DNA literal validation --

#[test]
fn test_dna_rejects_u() {
    assert!(eval_err(r#"let x = dna"ATCGU""#));
}

#[test]
fn test_dna_rejects_numbers() {
    assert!(eval_err(r#"let x = dna"ATC1G""#));
}

#[test]
fn test_dna_accepts_iupac() {
    let result = eval(r#"dna"ACGTNRYWSMKBDHV""#);
    assert!(matches!(result, Value::DNA(_)));
}

// -- RNA literal validation --

#[test]
fn test_rna_rejects_t() {
    assert!(eval_err(r#"let x = rna"AUCGT""#));
}

#[test]
fn test_rna_accepts_iupac() {
    let result = eval(r#"rna"ACGUNRYWSMKBDHV""#);
    assert!(matches!(result, Value::RNA(_)));
}

// -- Protein literal validation --

#[test]
fn test_protein_rejects_numbers() {
    assert!(eval_err(r#"let x = protein"MVL1K""#));
}

#[test]
fn test_protein_accepts_stop_codon() {
    let result = eval(r#"protein"MVLK*""#);
    assert!(matches!(result, Value::Protein(_)));
}

// -- Interval validation --

#[test]
fn test_interval_valid() {
    let result = eval(r#"interval("chr1", 100, 200)"#);
    assert!(matches!(result, Value::Interval(_)));
}

#[test]
fn test_interval_rejects_end_before_start() {
    assert!(eval_err(r#"interval("chr1", 200, 100)"#));
}

#[test]
fn test_interval_rejects_empty_chrom() {
    assert!(eval_err(r#"interval("", 100, 200)"#));
}

#[test]
fn test_interval_rejects_negative_start() {
    assert!(eval_err(r#"interval("chr1", -1, 200)"#));
}

#[test]
fn test_interval_allows_equal_start_end() {
    // Zero-length intervals (point features) are valid in BED format
    let result = eval(r#"interval("chr1", 100, 100)"#);
    assert!(matches!(result, Value::Interval(_)));
}

// -- Variant validation --

#[test]
fn test_variant_valid() {
    let result = eval(r#"variant("chr7", 55181378, "T", "A")"#);
    assert!(matches!(result, Value::Variant { .. }));
}

#[test]
fn test_variant_rejects_negative_pos() {
    assert!(eval_err(r#"variant("chr7", -1, "T", "A")"#));
}

#[test]
fn test_variant_rejects_invalid_ref_allele() {
    assert!(eval_err(r#"variant("chr7", 100, "XYZ", "A")"#));
}

#[test]
fn test_variant_rejects_invalid_alt_allele() {
    assert!(eval_err(r#"variant("chr7", 100, "T", "123")"#));
}

#[test]
fn test_variant_accepts_iupac_alleles() {
    let result = eval(r#"variant("chr7", 100, "N", "R")"#);
    assert!(matches!(result, Value::Variant { .. }));
}

#[test]
fn test_variant_accepts_deletion_star() {
    let result = eval(r#"variant("chr7", 100, "AT", "*")"#);
    assert!(matches!(result, Value::Variant { .. }));
}

// -- Gene validation --

#[test]
fn test_gene_valid_from_string() {
    let result = eval(r#"gene("BRCA1")"#);
    assert!(matches!(result, Value::Gene { .. }));
}

#[test]
fn test_gene_rejects_empty_symbol() {
    assert!(eval_err(r#"gene("")"#));
}

#[test]
fn test_gene_record_rejects_empty_symbol() {
    // Use string keys to avoid `end` being a keyword token
    assert!(eval_err(r#"
let r = {"symbol": "", "chrom": "chr17", "start": 100, "end": 200}
gene(r)
"#));
}

#[test]
fn test_gene_record_rejects_end_before_start() {
    assert!(eval_err(r#"
let r = {"symbol": "BRCA1", "chrom": "chr17", "start": 200, "end": 100}
gene(r)
"#));
}

#[test]
fn test_gene_record_valid() {
    let result = eval(r#"
let r = {"symbol": "BRCA1", "chrom": "chr17", "start": 100, "end": 200}
gene(r)
"#);
    assert!(matches!(result, Value::Gene { .. }));
}

// -- AlignedRead validation --

#[test]
fn test_aligned_read_rejects_flag_out_of_range() {
    assert!(eval_err(r#"
let r = {qname: "r1", flag: 5000, seq: "ATCG", qual: "IIII"}
aligned_read(r)
"#));
}

#[test]
fn test_aligned_read_rejects_seq_qual_mismatch() {
    assert!(eval_err(r#"
let r = {qname: "r1", flag: 0, seq: "ATCG", qual: "II"}
aligned_read(r)
"#));
}

#[test]
fn test_aligned_read_valid() {
    let result = eval(r#"
let r = {qname: "r1", flag: 0, seq: "ATCG", qual: "IIII"}
aligned_read(r)
"#);
    assert!(matches!(result, Value::AlignedRead(_)));
}

// ===== Graph builtin tests =====

#[test]
fn test_graph_create_empty() {
    let result = eval("graph()");
    assert!(matches!(result, Value::Record(_)));
}

#[test]
fn test_graph_add_node() {
    let result = eval(r#"
        let g = graph()
        let g = add_node(g, "A")
        has_node(g, "A")
    "#);
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_graph_add_node_with_attrs() {
    let result = eval(r#"
        let g = graph()
        let g = add_node(g, "BRCA1", {biotype: "protein_coding"})
        node_attr(g, "BRCA1")
    "#);
    if let Value::Record(m) = result {
        assert_eq!(m.get("biotype"), Some(&Value::Str("protein_coding".into())));
    } else {
        panic!("expected Record");
    }
}

#[test]
fn test_graph_add_edge() {
    let result = eval(r#"
        let g = graph()
        let g = add_edge(g, "A", "B")
        has_edge(g, "A", "B")
    "#);
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_graph_auto_adds_nodes_on_edge() {
    let result = eval(r#"
        let g = graph()
        let g = add_edge(g, "X", "Y")
        has_node(g, "X")
    "#);
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_graph_neighbors() {
    let result = eval(r#"
        let g = graph()
        let g = add_edge(g, "A", "B")
        let g = add_edge(g, "A", "C")
        neighbors(g, "A")
    "#);
    if let Value::List(items) = result {
        let names: Vec<&str> = items.iter().filter_map(|v| v.as_str()).collect();
        assert!(names.contains(&"B"));
        assert!(names.contains(&"C"));
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_graph_undirected_neighbors() {
    let result = eval(r#"
        let g = graph()
        let g = add_edge(g, "A", "B")
        neighbors(g, "B")
    "#);
    if let Value::List(items) = result {
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].as_str(), Some("A"));
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_graph_directed_no_reverse_neighbor() {
    let result = eval(r#"
        let g = graph(true)
        let g = add_edge(g, "A", "B")
        neighbors(g, "B")
    "#);
    if let Value::List(items) = result {
        assert_eq!(items.len(), 0);
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_graph_degree() {
    let result = eval(r#"
        let g = graph()
        let g = add_edge(g, "A", "B")
        let g = add_edge(g, "A", "C")
        degree(g, "A")
    "#);
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_graph_shortest_path() {
    let result = eval(r#"
        let g = graph()
        let g = add_edge(g, "A", "B")
        let g = add_edge(g, "B", "C")
        let g = add_edge(g, "C", "D")
        shortest_path(g, "A", "D")
    "#);
    if let Value::List(items) = result {
        let path: Vec<&str> = items.iter().filter_map(|v| v.as_str()).collect();
        assert_eq!(path, vec!["A", "B", "C", "D"]);
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_graph_no_path_returns_nil() {
    let result = eval(r#"
        let g = graph()
        let g = add_node(g, "A")
        let g = add_node(g, "B")
        shortest_path(g, "A", "B")
    "#);
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_graph_nodes_list() {
    let result = eval(r#"
        let g = graph()
        let g = add_node(g, "B")
        let g = add_node(g, "A")
        nodes(g)
    "#);
    if let Value::List(items) = result {
        let names: Vec<&str> = items.iter().filter_map(|v| v.as_str()).collect();
        assert_eq!(names, vec!["A", "B"]); // sorted
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_graph_remove_node() {
    let result = eval(r#"
        let g = graph()
        let g = add_edge(g, "A", "B")
        let g = add_edge(g, "B", "C")
        let g = remove_node(g, "B")
        has_node(g, "B")
    "#);
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_graph_remove_node_removes_edges() {
    let result = eval(r#"
        let g = graph()
        let g = add_edge(g, "A", "B")
        let g = add_edge(g, "B", "C")
        let g = remove_node(g, "B")
        has_edge(g, "A", "B")
    "#);
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_graph_remove_edge() {
    let result = eval(r#"
        let g = graph()
        let g = add_edge(g, "A", "B")
        let g = remove_edge(g, "A", "B")
        has_edge(g, "A", "B")
    "#);
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_graph_edges_table() {
    let result = eval(r#"
        let g = graph()
        let g = add_edge(g, "A", "B", {weight: 0.9})
        edges(g)
    "#);
    assert!(matches!(result, Value::Table(_)));
}

#[test]
fn test_graph_subgraph() {
    let result = eval(r#"
        let g = graph()
        let g = add_edge(g, "A", "B")
        let g = add_edge(g, "B", "C")
        let g = add_edge(g, "C", "D")
        let sub = subgraph(g, ["A", "B"])
        has_edge(sub, "A", "B")
    "#);
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_graph_subgraph_excludes_outside_edges() {
    let result = eval(r#"
        let g = graph()
        let g = add_edge(g, "A", "B")
        let g = add_edge(g, "B", "C")
        let sub = subgraph(g, ["A", "B"])
        has_edge(sub, "B", "C")
    "#);
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_graph_connected_components() {
    let result = eval(r#"
        let g = graph()
        let g = add_edge(g, "A", "B")
        let g = add_node(g, "C")
        connected_components(g)
    "#);
    if let Value::List(components) = result {
        assert_eq!(components.len(), 2);
    } else {
        panic!("expected List");
    }
}
