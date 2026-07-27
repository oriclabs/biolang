use bl_core::value::Value;
use bl_lexer::Lexer;
use bl_parser::Parser;
use bl_runtime::Interpreter;

fn eval(code: &str) -> bl_core::error::Result<Value> {
    let tokens = Lexer::new(code).tokenize().unwrap();
    let result = Parser::new(tokens).parse().unwrap();
    Interpreter::new().run(&result.program)
}

#[test]
fn rank_dispatches_by_input_type() {
    assert_eq!(
        eval("rank(matrix([[1.0, 0.0], [0.0, 1.0]]))").unwrap(),
        Value::Int(2)
    );
    assert_eq!(
        eval(
            r#"
let values = table({score: [3, 1, 2]})
rank(values, "score") |> col("rank")
"#,
        )
        .unwrap(),
        Value::List(vec![Value::Int(3), Value::Int(1), Value::Int(2)].into())
    );
}

#[test]
fn shared_string_registration_preserves_optional_substr_length() {
    assert_eq!(
        eval(r#"substr("ATCGATCG", 4)"#).unwrap(),
        Value::Str("ATCG".into())
    );
    assert_eq!(
        eval(r#"substr("ATCGATCG", 4, 2)"#).unwrap(),
        Value::Str("AT".into())
    );
}

#[test]
fn trim_quality_dispatches_quality_values_to_the_stats_implementation() {
    assert_eq!(
        eval(r#"trim_quality(qual"II!!", 20)"#).unwrap(),
        Value::Quality(vec![40, 40, 0, 0])
    );
}

#[test]
fn biological_windows_preserve_the_input_sequence_type() {
    assert_eq!(
        eval(r#"windows(dna"ATCG", 2, 2)[0].seq"#).unwrap(),
        Value::DNA(bl_core::value::BioSequence {
            data: "AT".to_string()
        })
    );
}

#[test]
fn intervals_have_a_length_and_can_build_an_interval_tree() {
    assert_eq!(
        eval(r#"len(interval("chr1", 100, 250))"#).unwrap(),
        Value::Int(150)
    );
    assert_eq!(
        eval(
            r#"
let regions = [
  interval("chr1", 100, 200),
  interval("chr1", 150, 250),
]
query_overlaps(interval_tree(regions), "chr1", 175, 180) |> len()
"#,
        )
        .unwrap(),
        Value::Int(2)
    );
}

#[test]
fn date_parse_accepts_date_only_formats() {
    assert_eq!(
        eval(r#"date_parse("2026-07-26", "%Y-%m-%d")"#).unwrap(),
        Value::Str("2026-07-26T00:00:00+00:00".to_string())
    );
}

#[test]
fn json_pretty_accepts_structured_values() {
    assert_eq!(
        eval(r#"json_pretty({gene: "BRCA1"})"#).unwrap(),
        Value::Str("{\n  \"gene\": \"BRCA1\"\n}".to_string())
    );
}

#[test]
fn debug_passes_its_input_through() {
    assert_eq!(
        eval("[1, 2, 3] |> debug").unwrap(),
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)].into())
    );
}

#[cfg(feature = "native")]
#[test]
fn trim_quality_dispatches_paths_to_the_native_fastq_implementation() {
    let error = eval(r#"trim_quality("__missing__.fastq", "unused.fastq", 20)"#).unwrap_err();
    assert_eq!(error.kind, bl_core::error::ErrorKind::IOError);
}
