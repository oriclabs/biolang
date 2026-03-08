use bl_bio::intervals::call_interval_builtin;
use bl_core::value::{Table, Value};

fn make_bed_table(intervals: &[(&str, i64, i64)]) -> Value {
    let columns = vec!["chrom".to_string(), "start".to_string(), "end".to_string()];
    let rows: Vec<Vec<Value>> = intervals
        .iter()
        .map(|(c, s, e)| vec![Value::Str(c.to_string()), Value::Int(*s), Value::Int(*e)])
        .collect();
    Value::Table(Table::new(columns, rows))
}

#[test]
fn test_intersect() {
    let a = make_bed_table(&[("chr1", 100, 200), ("chr1", 300, 400), ("chr2", 100, 200)]);
    let b = make_bed_table(&[("chr1", 150, 250), ("chr2", 500, 600)]);
    let result = call_interval_builtin("intersect", vec![a, b]).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 1);
        assert_eq!(t.rows[0][1], Value::Int(100));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_merge_intervals() {
    let t = make_bed_table(&[
        ("chr1", 100, 200),
        ("chr1", 150, 300),
        ("chr1", 500, 600),
        ("chr2", 100, 200),
    ]);
    let result = call_interval_builtin("merge_intervals", vec![t]).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 3);
        assert_eq!(t.rows[0][2], Value::Int(300));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_subtract() {
    let a = make_bed_table(&[("chr1", 100, 200), ("chr1", 300, 400)]);
    let b = make_bed_table(&[("chr1", 150, 250)]);
    let result = call_interval_builtin("subtract", vec![a, b]).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 1);
        assert_eq!(t.rows[0][1], Value::Int(300));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_closest() {
    let a = make_bed_table(&[("chr1", 100, 200)]);
    let b = make_bed_table(&[("chr1", 500, 600), ("chr1", 300, 400)]);
    let result = call_interval_builtin("closest", vec![a, b]).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 1);
        assert_eq!(t.rows[0][3], Value::Int(200));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_flank() {
    let t = make_bed_table(&[("chr1", 1000, 2000)]);
    let result = call_interval_builtin("flank", vec![t, Value::Int(500)]).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 2);
        assert_eq!(t.rows[0][1], Value::Int(500));
        assert_eq!(t.rows[0][2], Value::Int(1000));
        assert_eq!(t.rows[1][1], Value::Int(2000));
        assert_eq!(t.rows[1][2], Value::Int(2500));
    } else {
        panic!("expected Table");
    }
}
