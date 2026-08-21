use bl_core::value::{Table, Value};
use bl_runtime::chipseq::call_chipseq_builtin;

fn peak_table(rows: &[(&str, i64, i64)]) -> Value {
    Value::Table(Table::new(
        vec!["chrom".to_string(), "start".to_string(), "end".to_string()],
        rows.iter()
            .map(|(chrom, start, end)| {
                vec![
                    Value::Str((*chrom).to_string()),
                    Value::Int(*start),
                    Value::Int(*end),
                ]
            })
            .collect(),
    ))
}

#[test]
fn consensus_counts_sample_support_after_unioning_overlaps() {
    let first = peak_table(&[("chr1", 0, 10), ("chr1", 100, 110)]);
    let second = peak_table(&[("chr1", 5, 15), ("chr1", 200, 210)]);
    let Value::Table(result) = call_chipseq_builtin(
        "consensus_peaks",
        vec![Value::List(vec![first, second].into()), Value::Int(2)],
    )
    .unwrap() else {
        panic!("consensus did not return a table")
    };
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0][0], Value::Str("chr1".to_string()));
    assert_eq!(result.rows[0][1], Value::Int(0));
    assert_eq!(result.rows[0][2], Value::Int(15));
    assert_eq!(result.rows[0][3], Value::Int(2));
}
