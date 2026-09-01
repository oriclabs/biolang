use bl_core::value::Value;
use bl_runtime::builtins::call_builtin;

#[test]
fn sequence_constructors_accept_iupac_ambiguity_codes_like_literals() {
    let dna = call_builtin("dna", vec![Value::Str("ATCGRYSWKMBDHVN".into())])
        .expect("IUPAC DNA should be accepted");
    assert!(matches!(dna, Value::DNA(sequence) if sequence.data == "ATCGRYSWKMBDHVN"));

    let rna = call_builtin("rna", vec![Value::Str("AUCGRYSWKMBDHVN".into())])
        .expect("IUPAC RNA should be accepted");
    assert!(matches!(rna, Value::RNA(sequence) if sequence.data == "AUCGRYSWKMBDHVN"));

    assert!(call_builtin("dna", vec![Value::Str("ATZG".into())]).is_err());
    assert!(call_builtin("rna", vec![Value::Str("AUZG".into())]).is_err());
}
