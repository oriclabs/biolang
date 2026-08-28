use bl_core::matrix::Matrix;
use bl_core::value::{Table, Value};
use bl_runtime::singlecell::call_singlecell_builtin;
use std::collections::HashMap;

fn strings(values: &[&str]) -> Value {
    Value::List(
        values
            .iter()
            .map(|value| Value::Str((*value).to_string()))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn ints(values: &[i64]) -> Value {
    Value::List(
        values
            .iter()
            .copied()
            .map(Value::Int)
            .collect::<Vec<_>>()
            .into(),
    )
}

fn matrix(rows: usize, columns: usize) -> Value {
    Value::Matrix(
        Matrix::new(vec![1.0; rows * columns], rows, columns)
            .unwrap()
            .into(),
    )
}

fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect::<HashMap<_, _>>()
            .into(),
    )
}

fn base_object() -> Value {
    let counts = matrix(2, 3);
    record([
        ("matrix", counts.clone()),
        ("genes", strings(&["G0", "G1", "G2"])),
        ("barcodes", strings(&["C0", "C1"])),
        ("n_cells", Value::Int(2)),
        ("n_genes", Value::Int(3)),
        ("active_assay", Value::Str("RNA".into())),
        (
            "obs",
            Value::Table(Table::new(
                vec!["barcode".to_string()],
                vec![vec![Value::Str("C0".into())], vec![Value::Str("C1".into())]],
            )),
        ),
        (
            "var",
            Value::Table(Table::new(
                vec!["gene".to_string()],
                vec![
                    vec![Value::Str("G0".into())],
                    vec![Value::Str("G1".into())],
                    vec![Value::Str("G2".into())],
                ],
            )),
        ),
        ("layers", record([("counts", counts.clone())])),
        (
            "assays",
            record([(
                "RNA",
                record([
                    ("layers", record([("counts", counts)])),
                    ("variable_features", Value::List(Vec::new().into())),
                ]),
            )]),
        ),
        ("reductions", record([])),
        ("idents", Value::List(Vec::new().into())),
    ])
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    match value {
        Value::Record(fields) => fields.get(name).unwrap(),
        _ => panic!("expected Record"),
    }
}

fn issue_codes(value: &Value, field_name: &str) -> Vec<String> {
    let Value::List(issues) = field(value, field_name) else {
        panic!("expected issue List")
    };
    issues
        .iter()
        .map(|issue| match field(issue, "code") {
            Value::Str(code) => code.clone(),
            _ => panic!("expected issue code"),
        })
        .collect()
}

#[test]
fn canonical_object_with_unassigned_idents_is_valid() {
    let report = call_singlecell_builtin("sc_validate_object", vec![base_object()]).unwrap();
    assert_eq!(field(&report, "ok"), &Value::Bool(true));
    assert_eq!(field(&report, "error_count"), &Value::Int(0));
}

#[test]
fn cells_alias_is_accepted_and_disclosed() {
    let Value::Record(base) = base_object() else {
        unreachable!()
    };
    let mut fields = base.as_ref().clone();
    let barcodes = fields.remove("barcodes").unwrap();
    fields.insert("cells".to_string(), barcodes);
    let report =
        call_singlecell_builtin("sc_validate_object", vec![Value::Record(fields.into())]).unwrap();
    assert_eq!(field(&report, "ok"), &Value::Bool(true));
    assert!(issue_codes(&report, "warnings").contains(&"cell_axis_alias".to_string()));
}

#[test]
fn mismatched_feature_identity_is_caught_even_when_dimensions_fit() {
    let Value::Record(base) = base_object() else {
        unreachable!()
    };
    let mut fields = base.as_ref().clone();
    fields.insert("norm_matrix".to_string(), matrix(2, 2));
    fields.insert("hvg_matrix".to_string(), matrix(2, 2));
    fields.insert("hvg".to_string(), ints(&[2, 0]));
    fields.insert("hvg_genes".to_string(), strings(&["G1", "G0"]));
    let report =
        call_singlecell_builtin("sc_validate_object", vec![Value::Record(fields.into())]).unwrap();
    assert_eq!(field(&report, "ok"), &Value::Bool(false));
    assert!(issue_codes(&report, "errors").contains(&"feature_identity_mismatch".to_string()));
}

#[test]
fn mapped_feature_subset_and_assay_layers_are_valid() {
    let Value::Record(base) = base_object() else {
        unreachable!()
    };
    let mut fields = base.as_ref().clone();
    let residuals = matrix(2, 2);
    fields.insert("norm_matrix".to_string(), residuals.clone());
    fields.insert("hvg_matrix".to_string(), residuals.clone());
    fields.insert("hvg".to_string(), ints(&[2, 0]));
    fields.insert("hvg_genes".to_string(), strings(&["G2", "G0"]));
    fields.insert("hvg_ranked".to_string(), ints(&[2, 0, 1]));
    fields.insert("hvg_ranked_genes".to_string(), strings(&["G2", "G0", "G1"]));
    fields.insert("sct_theta".to_string(), ints(&[10, 20]));
    fields.insert("sct_intercept".to_string(), ints(&[-2, -3]));
    fields.insert("sct_residual_variance".to_string(), ints(&[4, 3]));
    fields.insert("active_assay".to_string(), Value::Str("SCT".into()));
    fields.insert(
        "assays".to_string(),
        record([(
            "SCT",
            record([
                ("layers", record([("data", residuals)])),
                ("variable_features", strings(&["G2", "G0"])),
            ]),
        )]),
    );
    let report =
        call_singlecell_builtin("sc_validate_object", vec![Value::Record(fields.into())]).unwrap();
    assert_eq!(field(&report, "ok"), &Value::Bool(true));
}

#[test]
fn selected_indices_without_names_are_rejected_as_an_unsafe_axis() {
    let Value::Record(base) = base_object() else {
        unreachable!()
    };
    let mut fields = base.as_ref().clone();
    fields.insert("hvg".to_string(), ints(&[1, 2]));
    fields.insert("hvg_matrix".to_string(), matrix(2, 2));
    let report =
        call_singlecell_builtin("sc_validate_object", vec![Value::Record(fields.into())]).unwrap();
    assert!(issue_codes(&report, "errors").contains(&"missing_feature_mapping".to_string()));
}

#[test]
fn strict_mode_turns_a_report_into_an_error() {
    let Value::Record(base) = base_object() else {
        unreachable!()
    };
    let mut fields = base.as_ref().clone();
    fields.insert("n_cells".to_string(), Value::Int(99));
    let error = call_singlecell_builtin(
        "sc_validate_object",
        vec![Value::Record(fields.into()), Value::Bool(true)],
    )
    .unwrap_err();
    assert!(error.message.contains("invalid single-cell object"));
    assert!(error.message.contains("cell axis"));
}

#[test]
fn graph_endpoints_and_active_assay_are_checked() {
    let Value::Record(base) = base_object() else {
        unreachable!()
    };
    let mut fields = base.as_ref().clone();
    fields.insert("active_assay".to_string(), Value::Str("missing".into()));
    fields.insert(
        "knn".to_string(),
        Value::List(
            vec![record([
                ("source", Value::Int(0)),
                ("target", Value::Int(9)),
                ("weight", Value::Float(0.5)),
            ])]
            .into(),
        ),
    );
    let report =
        call_singlecell_builtin("sc_validate_object", vec![Value::Record(fields.into())]).unwrap();
    let codes = issue_codes(&report, "errors");
    assert!(codes.contains(&"unknown_active_assay".to_string()));
    assert!(codes.contains(&"cell_index_out_of_bounds".to_string()));
}
