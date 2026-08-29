use bl_core::value::{Table, Value};
use bl_runtime::plot::call_plot_builtin;
use bl_runtime::table_ops::call_table_builtin;
use std::collections::HashMap;

fn contingency() -> Value {
    Value::Table(Table::new(
        vec!["group".into(), "yes".into(), "no".into()],
        vec![
            vec![Value::Str("A".into()), Value::Int(30), Value::Int(10)],
            vec![Value::Str("B".into()), Value::Int(10), Value::Int(50)],
        ],
    ))
}

fn options(items: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Record(
        items
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect::<HashMap<_, _>>()
            .into(),
    )
}

#[test]
fn cross_tab_can_pin_the_category_order_used_by_a_mosaic() {
    let observations = Value::Table(Table::new(
        vec!["Race".into(), "Insured".into()],
        vec![
            vec![Value::Str("White".into()), Value::Str("Yes".into())],
            vec![Value::Str("Asian".into()), Value::Str("No".into())],
            vec![Value::Str("Black".into()), Value::Str("Yes".into())],
            vec![Value::Str("Asian".into()), Value::Str("Yes".into())],
        ],
    ));
    let order = Value::List(
        ["Asian", "Black", "White"]
            .into_iter()
            .map(|label| Value::Str(label.into()))
            .collect::<Vec<_>>()
            .into(),
    );
    let table = call_table_builtin(
        "cross_tab",
        vec![
            observations,
            Value::Str("Race".into()),
            Value::Str("Insured".into()),
            options([("row_order", order)]),
        ],
    )
    .unwrap();
    let Value::Table(table) = table else {
        panic!("cross_tab must return a Table")
    };
    let labels = table
        .rows
        .iter()
        .map(|row| row[0].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(labels, ["Asian", "Black", "White"]);

    let Value::Record(specification) =
        call_plot_builtin("mosaic_data", vec![Value::Table(table)]).unwrap()
    else {
        panic!("mosaic_data must return a Record")
    };
    let Value::Table(cells) = &specification["data"] else {
        panic!("mosaic data")
    };
    assert_eq!(
        cells.rows[0][cells.col_index("row_label").unwrap()].as_str(),
        Some("Asian")
    );
}

fn record(value: &Value) -> &HashMap<String, Value> {
    let Value::Record(record) = value else {
        panic!("expected Record")
    };
    record
}

#[test]
fn mosaic_geometry_makes_rectangle_area_equal_observed_proportion() {
    let specification = call_plot_builtin(
        "mosaic_data",
        vec![contingency(), options([("shade", Value::Bool(true))])],
    )
    .unwrap();
    let map = record(&specification);
    assert!(matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "mosaic"));
    let Value::Table(cells) = &map["data"] else {
        panic!("mosaic data was not a Table")
    };
    assert_eq!(cells.num_rows(), 4);
    let column = |name: &str| cells.col_index(name).unwrap();
    for row in &cells.rows {
        let observed = row[column("observed")].as_float().unwrap();
        let area = (row[column("x1")].as_float().unwrap() - row[column("x0")].as_float().unwrap())
            * (row[column("y1")].as_float().unwrap() - row[column("y0")].as_float().unwrap());
        assert!((area - observed / 100.0).abs() < 1e-12);
    }
    assert_eq!(cells.rows[0][column("row_label")].as_str(), Some("A"));
    assert_eq!(cells.rows[0][column("column_label")].as_str(), Some("yes"));
    assert!((cells.rows[0][column("expected")].as_float().unwrap() - 16.0).abs() < 1e-12);
    assert!(
        (cells.rows[0][column("pearson_residual")]
            .as_float()
            .unwrap()
            - 3.5)
            .abs()
            < 1e-12
    );
}

#[test]
fn mosaic_spec_replays_exactly_and_has_browser_and_terminal_fallbacks() {
    let direct = call_plot_builtin("mosaic_plot", vec![contingency()]).unwrap();
    let specification = call_plot_builtin("mosaic_data", vec![contingency()]).unwrap();
    let replay = call_plot_builtin("render_plot", vec![specification.clone()]).unwrap();
    assert_eq!(direct, replay);
    let Value::Str(svg) = direct else {
        panic!("mosaic SVG was not a string")
    };
    assert!(svg.contains("<svg"));
    assert_eq!(svg.matches("data-biolang-mosaic-cell=\"true\"").count(), 4);
    assert!(svg.contains("Pearson residual"));

    let Value::Str(html) = call_plot_builtin(
        "render_plot",
        vec![
            specification.clone(),
            options([("format", Value::Str("canvas".into()))]),
        ],
    )
    .unwrap() else {
        panic!("mosaic HTML was not a string")
    };
    assert!(html.contains("<canvas"));

    let Value::Str(ascii) = call_plot_builtin(
        "render_plot",
        vec![
            specification,
            options([("format", Value::Str("ascii".into()))]),
        ],
    )
    .unwrap() else {
        panic!("mosaic terminal preview was not a string")
    };
    assert!(!ascii.trim().is_empty());
}

#[test]
fn mosaic_discloses_missing_cells_keeps_zero_cells_and_rejects_negative_counts() {
    let table = Value::Table(Table::new(
        vec!["group".into(), "yes".into(), "no".into()],
        vec![
            vec![Value::Str("A".into()), Value::Int(4), Value::Nil],
            vec![Value::Str("B".into()), Value::Int(0), Value::Int(6)],
        ],
    ));
    let specification = call_plot_builtin("mosaic_data", vec![table]).unwrap();
    let map = record(&specification);
    let Value::List(warnings) = &map["warnings"] else {
        panic!("warnings were not a List")
    };
    assert_eq!(warnings.len(), 1);
    let Value::Table(cells) = &map["data"] else {
        panic!("mosaic data was not a Table")
    };
    assert_eq!(cells.num_rows(), 4);

    let negative = Value::Table(Table::new(
        vec!["yes".into(), "no".into()],
        vec![vec![Value::Int(1), Value::Int(-1)]],
    ));
    let error = call_plot_builtin("mosaic_plot", vec![negative]).unwrap_err();
    assert!(error.message.contains("non-negative"));
}

#[test]
fn render_plot_names_unknown_plot_kinds() {
    let unknown = Value::Record(
        HashMap::from([
            ("schema".into(), Value::Str("biolang.plot.spec/v1".into())),
            ("kind".into(), Value::Str("future_plot".into())),
        ])
        .into(),
    );
    let error = call_plot_builtin("render_plot", vec![unknown]).unwrap_err();
    assert!(error.message.contains("unknown plot kind 'future_plot'"));
}
