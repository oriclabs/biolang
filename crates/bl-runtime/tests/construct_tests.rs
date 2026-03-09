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

// ============================================================================
// sort_by on Table
// ============================================================================

#[test]
fn test_sort_by_table_ascending() {
    let result = eval(r#"
let t = from_records([{name: "alice", score: 80}, {name: "bob", score: 95}, {name: "charlie", score: 70}])
let sorted = sort_by(t, |r| r.score)
sorted |> to_records |> map(|r| r.name)
    "#);
    assert_eq!(result, Value::List(vec![
        Value::Str("charlie".into()),
        Value::Str("alice".into()),
        Value::Str("bob".into()),
    ]));
}

#[test]
fn test_sort_by_table_descending() {
    let result = eval(r#"
let t = from_records([{name: "x", val: 3}, {name: "y", val: 1}, {name: "z", val: 2}])
let sorted = sort_by(t, |r| -r.val)
sorted |> to_records |> map(|r| r.name)
    "#);
    assert_eq!(result, Value::List(vec![
        Value::Str("x".into()),
        Value::Str("z".into()),
        Value::Str("y".into()),
    ]));
}

// ============================================================================
// kmer_count on List
// ============================================================================

#[test]
fn test_kmer_count_on_list() {
    let result = eval(r#"
let seqs = [dna"AAAA", dna"AAAA"]
let counts = kmer_count(seqs, 2)
# "AA" appears 3 times per "AAAA", so 6 total across 2 sequences
counts |> filter(|r| r.kmer == "AA") |> map(|r| r.count) |> first
    "#);
    assert_eq!(result, Value::Int(6));
}

// ============================================================================
// mean_phred on Str (ASCII quality)
// ============================================================================

#[test]
fn test_mean_phred_on_string() {
    // ASCII '!' = 33 = Phred 0, 'I' = 73 = Phred 40
    let result = eval(r#"
let q = "IIIII"
mean_phred(q)
    "#);
    if let Value::Float(v) = result {
        assert!((v - 40.0).abs() < 0.01, "expected ~40.0, got {}", v);
    } else {
        panic!("expected Float, got {:?}", result);
    }
}

// ============================================================================
// Raw strings
// ============================================================================

#[test]
fn test_raw_string_no_escapes() {
    let result = eval(r#"
r"hello\nworld"
    "#);
    assert_eq!(result, Value::Str("hello\\nworld".into()));
}

#[test]
fn test_raw_string_windows_path() {
    let result = eval(r#"
r"C:\Users\data\reads.fq"
    "#);
    assert_eq!(result, Value::Str("C:\\Users\\data\\reads.fq".into()));
}

// ============================================================================
// sort_by with 2-arg comparator (auto-detect)
// ============================================================================

#[test]
fn test_sort_by_comparator_list() {
    let result = eval(r#"
let nums = [3, 1, 4, 1, 5]
sort_by(nums, |a, b| a - b)
    "#);
    assert_eq!(result, Value::List(vec![
        Value::Int(1), Value::Int(1), Value::Int(3), Value::Int(4), Value::Int(5),
    ]));
}

#[test]
fn test_sort_by_comparator_descending() {
    let result = eval(r#"
let nums = [3, 1, 4, 1, 5]
sort_by(nums, |a, b| b - a)
    "#);
    assert_eq!(result, Value::List(vec![
        Value::Int(5), Value::Int(4), Value::Int(3), Value::Int(1), Value::Int(1),
    ]));
}

#[test]
fn test_sort_by_comparator_table() {
    let result = eval(r#"
let t = from_records([{name: "a", v: 3}, {name: "b", v: 1}, {name: "c", v: 2}])
let sorted = sort_by(t, |a, b| b.v - a.v)
sorted |> to_records |> map(|r| r.name)
    "#);
    assert_eq!(result, Value::List(vec![
        Value::Str("a".into()),
        Value::Str("c".into()),
        Value::Str("b".into()),
    ]));
}

// ============================================================================
// kmer_count on stream (via piped records)
// ============================================================================

#[test]
fn test_kmer_count_on_record_list() {
    // Simulate stream records with "seq" field
    let result = eval(r#"
let records = [{seq: dna"AAAA"}, {seq: dna"AAAA"}]
let counts = kmer_count(records, 2)
counts |> filter(|r| r.kmer == "AA") |> map(|r| r.count) |> first
    "#);
    assert_eq!(result, Value::Int(6));
}

// ============================================================================
// min_phred and error_rate on Str
// ============================================================================

#[test]
fn test_min_phred_on_string() {
    let result = eval(r#"
let q = "I!I"
min_phred(q)
    "#);
    // '!' = 33 - 33 = Phred 0
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_error_rate_on_string() {
    let result = eval(r#"
let q = "IIIII"
error_rate(q)
    "#);
    if let Value::Float(v) = result {
        assert!(v < 0.001, "Phred 40 error rate should be tiny, got {}", v);
    } else {
        panic!("expected Float");
    }
}

// ============================================================================
// Raw string in function call
// ============================================================================

#[test]
fn test_raw_string_in_expression() {
    let result = eval(r#"
let p = r"C:\test\new"
len(p)
    "#);
    assert_eq!(result, Value::Int(11));
}

// ── Pipe-at-start-of-line (multi-line pipeline) tests ──

#[test]
fn test_pipe_at_start_of_next_line() {
    // Lexer strips newlines before |> so this is a single pipeline expression
    let result = eval(r#"
[3, 1, 2]
|> sort()
|> first()
    "#);
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_pipe_at_start_with_indentation() {
    let result = eval(r#"
[5, 10, 15, 20]
  |> filter(|x| x > 8)
  |> len()
    "#);
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_pipe_at_start_mixed_with_trailing() {
    // Mix of trailing |> on some lines and leading |> on others
    let result = eval(r#"
[1, 2, 3, 4, 5] |>
  filter(|x| x > 2)
  |> map(|x| x * 10)
  |> sum()
    "#);
    assert_eq!(result, Value::Int(120));
}

#[test]
fn test_pipe_at_start_does_not_affect_non_pipe_statements() {
    // Separate statements without pipes remain independent
    let result = eval(r#"
let x = 10
let y = 20
x + y
    "#);
    assert_eq!(result, Value::Int(30));
}

#[test]
fn test_tap_pipe_at_start_of_line() {
    // ~ (tap pipe) at start of line should also work
    let result = eval(r#"
let x = [1, 2, 3]
  ~ len()
last(x)
    "#);
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_multiline_pipeline_with_lambda() {
    let result = eval(r#"
["hello", "world", "foo"]
  |> filter(|s| len(s) > 3)
  |> map(|s| upper(s))
  |> len()
    "#);
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_sort_by_on_stream() {
    // sort_by should collect a stream into a list, then sort
    let result = eval(r#"
let s = to_stream([3, 1, 4, 1, 5])
sort_by(s, |x| x) |> first()
    "#);
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_sort_by_stream_comparator() {
    // sort_by with 2-arg comparator on a stream
    let result = eval(r#"
let s = to_stream([{name: "b", val: 2}, {name: "a", val: 1}, {name: "c", val: 3}])
sort_by(s, |a, b| a.val - b.val) |> first()
    "#);
    assert!(matches!(result, Value::Record(_)));
    if let Value::Record(rec) = result {
        assert_eq!(rec.get("name"), Some(&Value::Str("a".into())));
    }
}

#[test]
fn test_kmer_count_top_n() {
    // kmer_count with top N parameter: kmer_count(seq, k, top)
    let result = eval(r#"
kmer_count(dna"AAAAAACCCCCCGGGGGGTTTTTT", 2, 2)
    "#);
    // Should return only top 2 k-mers by count (sorted descending)
    if let Value::Table(tbl) = result {
        assert_eq!(tbl.rows.len(), 2);
        // Counts should be > 0 and rows sorted descending
        if let (Value::Int(c1), Value::Int(c2)) = (&tbl.rows[0][1], &tbl.rows[1][1]) {
            assert!(*c1 >= *c2, "top N should be sorted descending");
            assert!(*c1 > 0);
        }
    } else {
        panic!("expected Table");
    }
}

// ============================================================
// Table operations — all List ops should also work on Table
// ============================================================

fn make_table_code() -> &'static str {
    r#"let t = table([{name: "Alice", age: 30, score: 95}, {name: "Bob", age: 25, score: 88}, {name: "Charlie", age: 35, score: 72}, {name: "Diana", age: 28, score: 91}])"#
}

#[test]
fn table_first() {
    let code = format!("{}\nfirst(t)", make_table_code());
    let val = eval(&code);
    if let Value::Record(r) = val {
        assert_eq!(r.get("name"), Some(&Value::Str("Alice".into())));
        assert_eq!(r.get("age"), Some(&Value::Int(30)));
    } else {
        panic!("expected Record, got {:?}", val);
    }
}

#[test]
fn table_first_empty() {
    let val = eval(r#"let t = table([])
first(t)"#);
    assert_eq!(val, Value::Nil);
}

#[test]
fn table_last() {
    let code = format!("{}\nlast(t)", make_table_code());
    let val = eval(&code);
    if let Value::Record(r) = val {
        assert_eq!(r.get("name"), Some(&Value::Str("Diana".into())));
        assert_eq!(r.get("score"), Some(&Value::Int(91)));
    } else {
        panic!("expected Record, got {:?}", val);
    }
}

#[test]
fn table_last_empty() {
    let val = eval(r#"let t = table([])
last(t)"#);
    assert_eq!(val, Value::Nil);
}

#[test]
fn table_reverse() {
    let code = format!("{}\nlet r = reverse(t)\nfirst(r)", make_table_code());
    let val = eval(&code);
    if let Value::Record(r) = val {
        assert_eq!(r.get("name"), Some(&Value::Str("Diana".into())));
    } else {
        panic!("expected Record, got {:?}", val);
    }
}

#[test]
fn table_reverse_preserves_columns() {
    let code = format!("{}\nlet r = reverse(t)\nlen(r)", make_table_code());
    let val = eval(&code);
    assert_eq!(val, Value::Int(4));
}

#[test]
fn table_take() {
    let code = format!("{}\nlet r = take(t, 2)\nlen(r)", make_table_code());
    let val = eval(&code);
    assert_eq!(val, Value::Int(2));
}

#[test]
fn table_take_returns_table() {
    let code = format!("{}\nlet r = take(t, 2)\nfirst(r)", make_table_code());
    let val = eval(&code);
    if let Value::Record(r) = val {
        assert_eq!(r.get("name"), Some(&Value::Str("Alice".into())));
    } else {
        panic!("expected Record, got {:?}", val);
    }
}

#[test]
fn table_drop() {
    let code = format!("{}\nlet r = drop(t, 2)\nlen(r)", make_table_code());
    let val = eval(&code);
    assert_eq!(val, Value::Int(2));
}

#[test]
fn table_drop_first_record() {
    let code = format!("{}\nlet r = drop(t, 2)\nfirst(r)", make_table_code());
    let val = eval(&code);
    if let Value::Record(r) = val {
        assert_eq!(r.get("name"), Some(&Value::Str("Charlie".into())));
    } else {
        panic!("expected Record, got {:?}", val);
    }
}

#[test]
fn table_collect_passthrough() {
    let code = format!("{}\nlet r = collect(t)\nlen(r)", make_table_code());
    let val = eval(&code);
    assert_eq!(val, Value::Int(4));
}

#[test]
fn table_to_stream_and_collect() {
    let code = format!("{}\nlet s = to_stream(t)\nlet items = collect(s)\nlen(items)", make_table_code());
    let val = eval(&code);
    assert_eq!(val, Value::Int(4));
}

#[test]
fn table_enumerate() {
    let code = format!("{}\nlet pairs = enumerate(t)\nlet p = first(pairs)\nfirst(p)", make_table_code());
    let val = eval(&code);
    assert_eq!(val, Value::Int(0));
}

#[test]
fn table_enumerate_second_element_is_record() {
    let code = format!("{}\nlet pairs = enumerate(t)\nlet p = first(pairs)\nlast(p)", make_table_code());
    let val = eval(&code);
    if let Value::Record(r) = val {
        assert_eq!(r.get("name"), Some(&Value::Str("Alice".into())));
    } else {
        panic!("expected Record, got {:?}", val);
    }
}

#[test]
fn table_chunk() {
    let code = format!("{}\nlet chunks = chunk(t, 2)\nlen(chunks)", make_table_code());
    let val = eval(&code);
    assert_eq!(val, Value::Int(2));
}

#[test]
fn table_chunk_inner_size() {
    let code = format!("{}\nlet chunks = chunk(t, 3)\nlen(first(chunks))", make_table_code());
    let val = eval(&code);
    assert_eq!(val, Value::Int(3));
}

#[test]
fn table_window() {
    let code = format!("{}\nlet wins = window(t, 2)\nlen(wins)", make_table_code());
    let val = eval(&code);
    assert_eq!(val, Value::Int(3));
}

#[test]
fn table_window_inner_size() {
    let code = format!("{}\nlet wins = window(t, 3)\nlen(first(wins))", make_table_code());
    let val = eval(&code);
    assert_eq!(val, Value::Int(3));
}

#[test]
fn table_frequencies() {
    let code = r#"let t = table([{val: 1}, {val: 2}, {val: 1}, {val: 1}, {val: 2}])
let f = frequencies(t)
len(f)"#;
    let val = eval(code);
    // Should be a Table with 2 rows (two unique record values)
    assert_eq!(val, Value::Int(2));
}

#[test]
fn table_zip_two_tables() {
    let code = r#"let a = table([{x: 1}, {x: 2}, {x: 3}])
let b = table([{y: 10}, {y: 20}, {y: 30}])
let z = zip(a, b)
len(z)"#;
    let val = eval(code);
    assert_eq!(val, Value::Int(3));
}

#[test]
fn table_zip_table_with_list() {
    let code = r#"let t = table([{x: 1}, {x: 2}, {x: 3}])
let z = zip(t, [10, 20, 30])
let pair = first(z)
first(pair)"#;
    let val = eval(code);
    // First element of first pair should be a Record {x: 1}
    if let Value::Record(r) = val {
        assert_eq!(r.get("x"), Some(&Value::Int(1)));
    } else {
        panic!("expected Record, got {:?}", val);
    }
}

#[test]
fn table_join_to_string() {
    let code = r#"let t = table([{name: "A"}, {name: "B"}, {name: "C"}])
join(t, ", ")"#;
    let val = eval(code);
    // join on table converts rows to record strings
    if let Value::Str(s) = val {
        assert!(s.contains("A"), "join output should contain field values: {}", s);
    } else {
        panic!("expected Str, got {:?}", val);
    }
}

#[test]
fn table_flatten() {
    let code = r#"let t = table([{items: [1, 2]}, {items: [3, 4]}])
let f = flatten(t)
len(f)"#;
    let val = eval(code);
    // Each row becomes a Record; flatten doesn't unwrap records, so 2 records
    assert_eq!(val, Value::Int(2));
}

#[test]
fn table_pipe_first_last() {
    // Test pipe syntax with table operations
    let code = format!("{}\nt |> first()", make_table_code());
    let val = eval(&code);
    if let Value::Record(r) = val {
        assert_eq!(r.get("name"), Some(&Value::Str("Alice".into())));
    } else {
        panic!("expected Record, got {:?}", val);
    }
}

#[test]
fn table_pipe_take_first() {
    let code = format!("{}\nt |> take(2) |> first()", make_table_code());
    let val = eval(&code);
    if let Value::Record(r) = val {
        assert_eq!(r.get("name"), Some(&Value::Str("Alice".into())));
    } else {
        panic!("expected Record, got {:?}", val);
    }
}

#[test]
fn table_pipe_reverse_first() {
    let code = format!("{}\nt |> reverse() |> first()", make_table_code());
    let val = eval(&code);
    if let Value::Record(r) = val {
        assert_eq!(r.get("name"), Some(&Value::Str("Diana".into())));
    } else {
        panic!("expected Record, got {:?}", val);
    }
}

#[test]
fn table_pipe_drop_len() {
    let code = format!("{}\nt |> drop(3) |> len()", make_table_code());
    let val = eval(&code);
    assert_eq!(val, Value::Int(1));
}

#[test]
fn table_filter_map_on_table() {
    // filter and map on Table already work via HOF dispatch, but verify
    let code = format!("{}\nt |> filter(|r| r.age > 27) |> len()", make_table_code());
    let val = eval(&code);
    assert_eq!(val, Value::Int(3)); // Alice(30), Charlie(35), Diana(28)
}

#[test]
fn table_sort_by() {
    let code = format!("{}\nt |> sort_by(|r| r.score) |> first()", make_table_code());
    let val = eval(&code);
    if let Value::Record(r) = val {
        // sort ascending: lowest score first = Charlie (72)
        assert_eq!(r.get("name"), Some(&Value::Str("Charlie".into())));
    } else {
        panic!("expected Record, got {:?}", val);
    }
}

// ============================================================
// `and` / `or` keyword aliases for && / ||
// ============================================================

#[test]
fn and_keyword_basic() {
    let val = eval("true and true");
    assert_eq!(val, Value::Bool(true));
}

#[test]
fn and_keyword_false() {
    let val = eval("true and false");
    assert_eq!(val, Value::Bool(false));
}

#[test]
fn or_keyword_basic() {
    let val = eval("false or true");
    assert_eq!(val, Value::Bool(true));
}

#[test]
fn or_keyword_false() {
    let val = eval("false or false");
    assert_eq!(val, Value::Bool(false));
}

#[test]
fn and_or_combined() {
    let val = eval("true and false or true");
    assert_eq!(val, Value::Bool(true));
}

#[test]
fn and_in_filter_expression() {
    let val = eval(r#"[1, 2, 3, 4, 5] |> filter(|x| x > 1 and x < 5) |> len()"#);
    assert_eq!(val, Value::Int(3));
}

#[test]
fn or_in_filter_expression() {
    let val = eval(r#"[1, 2, 3, 4, 5] |> filter(|x| x == 1 or x == 5) |> len()"#);
    assert_eq!(val, Value::Int(2));
}

#[test]
fn and_with_comparison() {
    let val = eval(r#"let x = 10
x > 5 and x < 20"#);
    assert_eq!(val, Value::Bool(true));
}

#[test]
fn and_multiline() {
    // `and` should suppress newlines like &&
    let val = eval("true and\ntrue");
    assert_eq!(val, Value::Bool(true));
}

// ============================================================
// Pipe-into — |> into name
// ============================================================

#[test]
fn pipe_into_basic() {
    let val = eval(r#"[1, 2, 3] |> len() |> into count
count"#);
    assert_eq!(val, Value::Int(3));
}

#[test]
fn pipe_into_creates_binding() {
    let val = eval(r#"[10, 20, 30] |> map(|x| x * 2) |> into doubled
doubled"#);
    assert_eq!(val, Value::List(vec![Value::Int(20), Value::Int(40), Value::Int(60)]));
}

#[test]
fn pipe_into_returns_value() {
    // |> into should also return the value (for chaining or display)
    let val = eval(r#"42 |> into answer"#);
    assert_eq!(val, Value::Int(42));
}

#[test]
fn pipe_into_chain() {
    let val = eval(r#"[1, 2, 3, 4, 5]
  |> filter(|x| x > 2)
  |> into filtered
len(filtered)"#);
    assert_eq!(val, Value::Int(3));
}

#[test]
fn pipe_into_shadows_existing() {
    let val = eval(r#"let x = 10
[1, 2, 3] |> len() |> into x
x"#);
    assert_eq!(val, Value::Int(3));
}

// ============================================================
// VCF INFO parsing — parse_vcf_info builtin
// ============================================================

#[test]
fn parse_vcf_info_basic() {
    let val = eval(r#"let info = parse_vcf_info("DP=30;AF=0.5;MQ=60")
info.DP"#);
    assert_eq!(val, Value::Int(30));
}

#[test]
fn parse_vcf_info_float() {
    let val = eval(r#"let info = parse_vcf_info("DP=30;AF=0.5;MQ=60")
info.AF"#);
    assert_eq!(val, Value::Float(0.5));
}

#[test]
fn parse_vcf_info_flag() {
    let val = eval(r#"let info = parse_vcf_info("DP=30;DB")
info.DB"#);
    assert_eq!(val, Value::Bool(true));
}

#[test]
fn parse_vcf_info_empty() {
    let val = eval(r#"let info = parse_vcf_info(".")
len(keys(info))"#);
    assert_eq!(val, Value::Int(0));
}

#[test]
fn parse_vcf_info_multi_value() {
    // Comma-separated values become a List
    let val = eval(r#"let info = parse_vcf_info("AF=0.45,0.12;DP=30")
len(info.AF)"#);
    assert_eq!(val, Value::Int(2));
}

#[test]
fn parse_vcf_info_multi_value_first() {
    let val = eval(r#"let info = parse_vcf_info("AF=0.45,0.12")
first(info.AF)"#);
    assert_eq!(val, Value::Float(0.45));
}

#[test]
fn parse_vcf_info_single_value_is_scalar() {
    // Single value (no comma) stays as a scalar, not a List
    let val = eval(r#"let info = parse_vcf_info("AF=0.45;DP=30")
info.AF >= 0.01"#);
    assert_eq!(val, Value::Bool(true));
}

#[test]
fn parse_vcf_info_multi_value_compare_uses_first() {
    // Multi-value AF like "0.35,0.10" becomes List; comparison auto-uses first element
    let val = eval(r#"let info = parse_vcf_info("AF=0.35,0.10")
info.AF >= 0.01"#);
    assert_eq!(val, Value::Bool(true));
}

#[test]
fn compare_nil_returns_false() {
    // Missing VCF fields (nil) should not throw — just return false
    let val = eval("nil >= 10");
    assert_eq!(val, Value::Bool(false));
}

// ============================================================
// Distribution functions
// ============================================================

#[test]
fn dnorm_standard() {
    let val = eval("dnorm(0)");
    if let Value::Float(f) = val {
        assert!((f - 0.3989).abs() < 0.001, "dnorm(0) ≈ 0.3989, got {f}");
    } else { panic!("expected Float"); }
}

#[test]
fn pnorm_standard() {
    let val = eval("pnorm(0)");
    if let Value::Float(f) = val {
        assert!((f - 0.5).abs() < 0.001, "pnorm(0) = 0.5, got {f}");
    } else { panic!("expected Float"); }
}

#[test]
fn pnorm_with_mean_sd() {
    let val = eval("pnorm(10, 10, 1)");
    if let Value::Float(f) = val {
        assert!((f - 0.5).abs() < 0.001, "pnorm(10, 10, 1) = 0.5, got {f}");
    } else { panic!("expected Float"); }
}

#[test]
fn qnorm_standard() {
    let val = eval("qnorm(0.975)");
    if let Value::Float(f) = val {
        assert!((f - 1.96).abs() < 0.02, "qnorm(0.975) ≈ 1.96, got {f}");
    } else { panic!("expected Float"); }
}

#[test]
fn dbinom_basic() {
    let val = eval("dbinom(3, 10, 0.5)");
    if let Value::Float(f) = val {
        assert!((f - 0.1172).abs() < 0.001, "dbinom(3,10,0.5) ≈ 0.1172, got {f}");
    } else { panic!("expected Float"); }
}

#[test]
fn pbinom_basic() {
    let val = eval("pbinom(5, 10, 0.5)");
    if let Value::Float(f) = val {
        assert!((f - 0.6230).abs() < 0.001, "pbinom(5,10,0.5) ≈ 0.623, got {f}");
    } else { panic!("expected Float"); }
}

#[test]
fn dpois_basic() {
    let val = eval("dpois(3, 2.0)");
    if let Value::Float(f) = val {
        assert!((f - 0.1804).abs() < 0.001, "dpois(3,2) ≈ 0.1804, got {f}");
    } else { panic!("expected Float"); }
}

#[test]
fn ppois_basic() {
    let val = eval("ppois(3, 2.0)");
    if let Value::Float(f) = val {
        assert!((f - 0.8571).abs() < 0.001, "ppois(3,2) ≈ 0.857, got {f}");
    } else { panic!("expected Float"); }
}

#[test]
fn dunif_in_range() {
    let val = eval("dunif(0.5, 0, 1)");
    assert_eq!(val, Value::Float(1.0));
}

#[test]
fn punif_half() {
    let val = eval("punif(0.5, 0, 1)");
    assert_eq!(val, Value::Float(0.5));
}

#[test]
fn dexp_basic() {
    let val = eval("dexp(1.0, 1.0)");
    if let Value::Float(f) = val {
        assert!((f - 0.3679).abs() < 0.001, "dexp(1,1) ≈ 0.368, got {f}");
    } else { panic!("expected Float"); }
}

#[test]
fn pexp_basic() {
    let val = eval("pexp(1.0, 1.0)");
    if let Value::Float(f) = val {
        assert!((f - 0.6321).abs() < 0.001, "pexp(1,1) ≈ 0.632, got {f}");
    } else { panic!("expected Float"); }
}

#[test]
fn rnorm_returns_list() {
    let val = eval("len(rnorm(100))");
    assert_eq!(val, Value::Int(100));
}

#[test]
fn rbinom_returns_list() {
    let val = eval("len(rbinom(50, 10, 0.5))");
    assert_eq!(val, Value::Int(50));
}

#[test]
fn rpois_returns_list() {
    let val = eval("len(rpois(50, 3.0))");
    assert_eq!(val, Value::Int(50));
}

// ============================================================
// K-means clustering
// ============================================================

#[test]
fn kmeans_basic() {
    let code = r#"let data = [[0,0],[0,1],[1,0],[1,1],[10,10],[10,11],[11,10],[11,11]]
let result = kmeans(data, 2)
len(result.clusters)"#;
    let val = eval(code);
    assert_eq!(val, Value::Int(8));
}

#[test]
fn kmeans_returns_centroids() {
    let code = r#"let data = [[0,0],[0,1],[1,0],[1,1],[10,10],[10,11],[11,10],[11,11]]
let result = kmeans(data, 2)
nrow(result.centroids)"#;
    let val = eval(code);
    assert_eq!(val, Value::Int(2));
}

#[test]
fn kmeans_iterations() {
    let code = r#"let data = [[0,0],[10,10]]
let result = kmeans(data, 2)
result.iterations"#;
    let val = eval(code);
    if let Value::Int(n) = val {
        assert!(n >= 1 && n <= 10, "should converge quickly, got {n} iterations");
    } else { panic!("expected Int"); }
}

// ============================================================
// GLM (logistic regression)
// ============================================================

#[test]
fn glm_logistic_basic() {
    let code = r#"let t = table([
  {x: 1, y: 0}, {x: 2, y: 0}, {x: 3, y: 0}, {x: 4, y: 0}, {x: 5, y: 0},
  {x: 6, y: 1}, {x: 7, y: 1}, {x: 8, y: 1}, {x: 9, y: 1}, {x: 10, y: 1}
])
let result = glm(~y ~ x, t, "binomial")
len(result.coefficients)"#;
    let val = eval(code);
    assert_eq!(val, Value::Int(2)); // intercept + 1 coefficient
}

#[test]
fn glm_has_aic() {
    let code = r#"let t = table([
  {x: 1, y: 0}, {x: 2, y: 0}, {x: 3, y: 0}, {x: 4, y: 0}, {x: 5, y: 0},
  {x: 6, y: 1}, {x: 7, y: 1}, {x: 8, y: 1}, {x: 9, y: 1}, {x: 10, y: 1}
])
let result = glm(~y ~ x, t, "binomial")
result.aic"#;
    let val = eval(code);
    if let Value::Float(f) = val {
        assert!(f > 0.0, "AIC should be positive, got {f}");
    } else { panic!("expected Float"); }
}
