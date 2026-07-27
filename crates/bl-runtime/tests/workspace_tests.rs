//! Round-trip tests for workspace save/restore.
//!
//! The point of this format over `json::value_to_json` is that types survive,
//! so most of these assert on the variant, not just the printed value.

use std::collections::HashMap;

use bl_core::matrix::Matrix;
use bl_core::value::{BioSequence, Table, Value};
use bl_runtime::workspace;

fn roundtrip(v: &Value) -> Value {
    let j = workspace::encode(v).expect("value should be encodable");
    workspace::decode(&j).expect("decode")
}

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

fn rec(pairs: Vec<(&str, Value)>) -> Value {
    let m: HashMap<String, Value> =
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    Value::Record(m.into())
}

#[test]
fn scalars_keep_their_types() {
    for v in [
        Value::Nil,
        Value::Bool(true),
        Value::Int(42),
        Value::Int(-7),
        Value::Float(1.5),
        s("hello"),
    ] {
        let got = roundtrip(&v);
        assert_eq!(
            std::mem::discriminant(&got),
            std::mem::discriminant(&v),
            "{v:?} changed variant"
        );
        assert_eq!(format!("{got}"), format!("{v}"));
    }
}

#[test]
fn an_integral_float_does_not_come_back_as_an_int() {
    // The trap this format exists to avoid: 2.0 must stay a Float.
    let got = roundtrip(&Value::Float(2.0));
    assert!(matches!(got, Value::Float(f) if f == 2.0), "got {got:?}");
}

#[test]
fn non_finite_floats_survive() {
    assert!(matches!(roundtrip(&Value::Float(f64::NAN)), Value::Float(f) if f.is_nan()));
    assert!(
        matches!(roundtrip(&Value::Float(f64::INFINITY)), Value::Float(f) if f == f64::INFINITY)
    );
    assert!(
        matches!(roundtrip(&Value::Float(f64::NEG_INFINITY)), Value::Float(f) if f == f64::NEG_INFINITY)
    );
}

#[test]
fn a_table_stays_a_table() {
    // json::value_to_json turns this into a List<Record>; here it must not.
    let t = Value::Table(Table::new(
        vec!["gene".into(), "padj".into()],
        vec![
            vec![s("CD3E"), Value::Float(0.001)],
            vec![s("MS4A1"), Value::Float(0.02)],
        ],
    ));
    let got = roundtrip(&t);
    let Value::Table(got) = got else {
        panic!("Table decoded as {got:?}");
    };
    assert_eq!(got.columns, vec!["gene", "padj"]);
    assert_eq!(got.rows.len(), 2);
    assert!(matches!(got.rows[0][1], Value::Float(f) if (f - 0.001).abs() < 1e-12));
}

#[test]
fn map_and_record_stay_distinct() {
    let m: HashMap<String, Value> = [("a".to_string(), Value::Int(1))].into_iter().collect();
    assert!(matches!(roundtrip(&Value::Map(m.clone().into())), Value::Map(_)));
    assert!(matches!(roundtrip(&Value::Record(m.into())), Value::Record(_)));
}

#[test]
fn a_record_key_named_like_the_tag_is_not_confused_for_one() {
    let v = rec(vec![("__t", s("table")), ("real", Value::Int(1))]);
    let Value::Record(got) = roundtrip(&v) else {
        panic!("not a record")
    };
    assert_eq!(got.get("__t").map(|x| format!("{x}")), Some("table".to_string()));
    assert!(matches!(got.get("real"), Some(Value::Int(1))));
}

#[test]
fn sequences_and_matrices_survive() {
    assert!(matches!(
        roundtrip(&Value::DNA(BioSequence { data: "ACGT".into() })),
        Value::DNA(sq) if sq.data == "ACGT"
    ));
    assert!(matches!(
        roundtrip(&Value::Protein(BioSequence { data: "MKV".into() })),
        Value::Protein(_)
    ));

    let mut m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    m.row_names = Some(vec!["r1".into(), "r2".into()]);
    let Value::Matrix(got) = roundtrip(&Value::Matrix(m)) else {
        panic!("not a matrix")
    };
    assert_eq!((got.nrow, got.ncol), (2, 2));
    assert_eq!(got.data, vec![1.0, 2.0, 3.0, 4.0]);
    assert_eq!(got.row_names, Some(vec!["r1".to_string(), "r2".to_string()]));
}

#[test]
fn nested_single_cell_shaped_object_survives() {
    let obj = rec(vec![
        (
            "matrix",
            Value::List(
                vec![
                    Value::List(vec![Value::Float(0.0), Value::Float(2.5)].into()),
                    Value::List(vec![Value::Float(1.0), Value::Float(0.0)].into()),
                ]
                .into(),
            ),
        ),
        ("genes", Value::List(vec![s("A"), s("B")].into())),
        ("n_cells", Value::Int(2)),
    ]);
    let got = roundtrip(&obj);
    let Value::Record(r) = &got else { panic!("not a record") };
    assert!(matches!(r.get("n_cells"), Some(Value::Int(2))));
    let Some(Value::List(rows)) = r.get("matrix") else {
        panic!("matrix missing")
    };
    assert_eq!(rows.len(), 2);
    let Value::List(row0) = &rows[0] else { panic!() };
    assert!(matches!(row0[1], Value::Float(f) if (f - 2.5).abs() < 1e-12));
}

#[test]
fn functions_are_reported_rather_than_silently_dropped() {
    // A closure cannot be represented; encode must decline it so the caller can
    // tell the user, instead of writing a garbage placeholder.
    let f = Value::NativeFunction {
        name: "len".to_string(),
        arity: bl_core::value::Arity::Exact(1),
    };
    assert!(workspace::encode(&f).is_none());
}

#[test]
fn save_and_load_a_file() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["ws.json", "ws.json.gz"] {
        let path = dir.path().join(name);
        let path = path.to_str().unwrap();

        let table = Value::Table(Table::new(
            vec!["g".into()],
            vec![vec![s("CD3E")]],
        ));
        let big = Value::List((0..500).map(|i| Value::Float(i as f64 / 3.0)).collect::<Vec<_>>().into());
        let fnval = Value::NativeFunction {
            name: "len".to_string(),
            arity: bl_core::value::Arity::Exact(1),
        };
        let vars = vec![("tbl", &table), ("big", &big), ("somefn", &fnval)];

        let report = workspace::save(path, vars).expect("save");
        assert_eq!(report.saved, vec!["big".to_string(), "tbl".to_string()]);
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].0, "somefn");
        assert!(report.bytes > 0);

        let loaded = workspace::load(path).expect("load");
        let map: HashMap<_, _> = loaded.into_iter().collect();
        assert!(matches!(map.get("tbl"), Some(Value::Table(_))));
        let Some(Value::List(l)) = map.get("big") else {
            panic!("big missing")
        };
        assert_eq!(l.len(), 500);
        assert!(matches!(l[3], Value::Float(f) if (f - 1.0).abs() < 1e-12));
        assert!(!map.contains_key("somefn"));
    }
}

#[test]
fn gzip_actually_shrinks_a_repetitive_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let big = Value::List((0..5000).map(|_| Value::Float(1.0)).collect::<Vec<_>>().into());

    let plain = dir.path().join("w.json");
    let gz = dir.path().join("w.json.gz");
    let a = workspace::save(plain.to_str().unwrap(), vec![("x", &big)]).unwrap();
    let b = workspace::save(gz.to_str().unwrap(), vec![("x", &big)]).unwrap();
    assert!(b.bytes < a.bytes / 2, "gzip did not compress: {} vs {}", b.bytes, a.bytes);
}

#[test]
fn rejects_files_that_are_not_workspaces() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("nope.json");
    std::fs::write(&p, br#"{"just":"json"}"#).unwrap();
    assert!(workspace::load(p.to_str().unwrap()).is_err());
    assert!(workspace::load("no/such/file.json").is_err());
}
