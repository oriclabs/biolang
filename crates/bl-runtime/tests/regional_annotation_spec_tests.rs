//! Renderer-independent contracts for regional annotation, mutation and splice plots.

use bl_core::value::{Table, Value};
use bl_runtime::bio_plots::call_bio_plots_builtin;
use bl_runtime::plot::call_plot_builtin;
use std::collections::HashMap;

fn opts(pairs: Vec<(&str, Value)>) -> Value {
    Value::Record(
        pairs
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect::<HashMap<_, _>>()
            .into(),
    )
}

fn record(value: &Value) -> &HashMap<String, Value> {
    let Value::Record(record) = value else {
        panic!("expected Record, got {}", value.type_of())
    };
    record
}

fn render(specification: Value) -> Value {
    call_plot_builtin("render_plot", vec![specification]).expect("render frozen plot")
}

fn genome_input() -> Value {
    Value::Table(Table::new(
        vec![
            "chrom".into(),
            "start".into(),
            "end".into(),
            "name".into(),
            "strand".into(),
        ],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Float(100.0),
                Value::Float(200.0),
                Value::Str("B".into()),
                Value::Str("+".into()),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Float(50.0),
                Value::Float(120.0),
                Value::Str("A".into()),
                Value::Str("-".into()),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Float(200.0),
                Value::Float(250.0),
                Value::Str("C".into()),
                Value::Str(".".into()),
            ],
        ],
    ))
}

#[test]
fn genome_track_freezes_clipping_order_lanes_and_exact_replay() {
    let common = vec![
        ("region_start", Value::Float(75.0)),
        ("region_end", Value::Float(225.0)),
        ("theme", Value::Str("publication".into())),
    ];
    let direct =
        call_plot_builtin("genome_track", vec![genome_input(), opts(common.clone())]).unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_plot_builtin("genome_track", vec![genome_input(), opts(spec_options)]).unwrap();
    let Some(Value::Table(data)) = record(&specification).get("data") else {
        panic!("genome geometry should be a Table")
    };
    assert_eq!(
        data.rows
            .iter()
            .map(|row| row[1].clone())
            .collect::<Vec<_>>(),
        vec![Value::Int(1), Value::Int(0), Value::Int(2)]
    );
    assert_eq!(data.rows[0][3], Value::Float(50.0));
    assert_eq!(data.rows[0][5], Value::Float(75.0));
    assert_eq!(data.rows[0][10], Value::Int(0));
    assert_eq!(data.rows[1][10], Value::Int(1));
    assert_eq!(data.rows[2][10], Value::Int(0));
    assert_eq!(data.rows[2][6], Value::Float(225.0));
    assert_eq!(direct, render(specification));
}

fn lollipop_input() -> Value {
    Value::Table(Table::new(
        vec!["position".into(), "count".into(), "label".into()],
        vec![
            vec![
                Value::Float(300.0),
                Value::Float(4.0),
                Value::Str("C".into()),
            ],
            vec![
                Value::Float(100.0),
                Value::Float(9.0),
                Value::Str("A".into()),
            ],
            vec![
                Value::Float(200.0),
                Value::Float(1.0),
                Value::Str("B".into()),
            ],
        ],
    ))
}

#[test]
fn lollipop_honours_sequence_length_and_freezes_sorted_marks() {
    let common = vec![
        ("length", Value::Float(500.0)),
        ("theme", Value::Str("publication".into())),
    ];
    let direct =
        call_bio_plots_builtin("lollipop", vec![lollipop_input(), opts(common.clone())]).unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_bio_plots_builtin("lollipop", vec![lollipop_input(), opts(spec_options)]).unwrap();
    let map = record(&specification);
    let Some(Value::Record(options)) = map.get("options") else {
        panic!("lollipop options should be a Record")
    };
    assert_eq!(options.get("domain_start"), Some(&Value::Float(0.0)));
    assert_eq!(options.get("domain_end"), Some(&Value::Float(500.0)));
    assert_eq!(options.get("y_max"), Some(&Value::Float(9.0)));
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("lollipop geometry should be a Table")
    };
    assert_eq!(
        data.rows
            .iter()
            .map(|row| row[2].clone())
            .collect::<Vec<_>>(),
        vec![
            Value::Float(100.0),
            Value::Float(200.0),
            Value::Float(300.0)
        ]
    );
    assert_eq!(
        data.rows
            .iter()
            .map(|row| row[1].clone())
            .collect::<Vec<_>>(),
        vec![Value::Int(1), Value::Int(2), Value::Int(0)]
    );
    assert_eq!(direct, render(specification));
}

fn sashimi_input() -> Value {
    let coverage = Value::Table(Table::new(
        vec!["pos".into(), "depth".into()],
        vec![
            vec![Value::Float(250.0), Value::Float(4.0)],
            vec![Value::Float(100.0), Value::Float(1.0)],
            vec![Value::Float(400.0), Value::Float(8.0)],
        ],
    ));
    let junctions = Value::Table(Table::new(
        vec!["chrom".into(), "start".into(), "end".into(), "count".into()],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Float(200.0),
                Value::Float(450.0),
                Value::Float(25.0),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Float(100.0),
                Value::Float(300.0),
                Value::Float(100.0),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Float(450.0),
                Value::Float(500.0),
                Value::Float(4.0),
            ],
        ],
    ));
    Value::Record(
        HashMap::from([
            ("coverage".into(), coverage),
            ("junctions".into(), junctions),
        ])
        .into(),
    )
}

#[test]
fn sashimi_freezes_coverage_arc_lanes_and_count_scaling() {
    let common = vec![("theme", Value::Str("publication".into()))];
    let direct =
        call_bio_plots_builtin("sashimi", vec![sashimi_input(), opts(common.clone())]).unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_bio_plots_builtin("sashimi", vec![sashimi_input(), opts(spec_options)]).unwrap();
    let map = record(&specification);
    let Some(Value::Table(coverage)) = map.get("coverage") else {
        panic!("coverage geometry should be a Table")
    };
    assert_eq!(
        coverage
            .rows
            .iter()
            .map(|row| row[3].clone())
            .collect::<Vec<_>>(),
        vec![
            Value::Float(100.0),
            Value::Float(250.0),
            Value::Float(400.0)
        ]
    );
    let Some(Value::Table(junctions)) = map.get("junctions") else {
        panic!("junction geometry should be a Table")
    };
    assert_eq!(
        junctions
            .rows
            .iter()
            .map(|row| row[1].clone())
            .collect::<Vec<_>>(),
        vec![Value::Int(1), Value::Int(0), Value::Int(2)]
    );
    assert_eq!(junctions.rows[0][8], Value::Int(0));
    assert_eq!(junctions.rows[1][8], Value::Int(1));
    assert_eq!(junctions.rows[2][8], Value::Int(0));
    assert!((junctions.rows[0][9].as_float().unwrap() - 1.0).abs() < 1e-12);
    assert!((junctions.rows[1][9].as_float().unwrap() - 0.675).abs() < 1e-12);
    assert_eq!(junctions.rows[0][10], Value::Float(4.0));
    assert_eq!(direct, render(specification));
}

#[test]
fn regional_specs_reject_tampering_and_offer_html_canvas_fallback() {
    let genome = call_plot_builtin(
        "genome_track",
        vec![
            genome_input(),
            opts(vec![("format", Value::Str("spec".into()))]),
        ],
    )
    .unwrap();
    let lollipop = call_bio_plots_builtin(
        "lollipop",
        vec![
            lollipop_input(),
            opts(vec![("format", Value::Str("spec".into()))]),
        ],
    )
    .unwrap();
    let sashimi = call_bio_plots_builtin(
        "sashimi",
        vec![
            sashimi_input(),
            opts(vec![("format", Value::Str("spec".into()))]),
        ],
    )
    .unwrap();
    let Value::Str(html) = call_plot_builtin(
        "render_plot",
        vec![
            sashimi.clone(),
            opts(vec![("format", Value::Str("html".into()))]),
        ],
    )
    .unwrap() else {
        panic!("expected standalone HTML")
    };
    assert!(html.contains("<svg"));
    assert!(html.contains("<canvas"));

    let mut broken_genome = record(&genome).clone();
    let Value::Table(mut rows) = broken_genome.get("data").unwrap().clone() else {
        unreachable!()
    };
    rows.rows[1][10] = Value::Int(0);
    broken_genome.insert("data".into(), Value::Table(rows));
    assert!(call_plot_builtin("render_plot", vec![Value::Record(broken_genome.into())]).is_err());

    let mut broken_lollipop = record(&lollipop).clone();
    let Value::Table(mut rows) = broken_lollipop.get("data").unwrap().clone() else {
        unreachable!()
    };
    rows.rows[0][2] = Value::Float(999.0);
    broken_lollipop.insert("data".into(), Value::Table(rows));
    assert!(call_plot_builtin("render_plot", vec![Value::Record(broken_lollipop.into())]).is_err());

    let mut broken_sashimi = record(&sashimi).clone();
    let Value::Table(mut rows) = broken_sashimi.get("junctions").unwrap().clone() else {
        unreachable!()
    };
    rows.rows[0][9] = Value::Float(0.5);
    broken_sashimi.insert("junctions".into(), Value::Table(rows));
    assert!(call_plot_builtin("render_plot", vec![Value::Record(broken_sashimi.into())]).is_err());
}

#[test]
fn dense_regional_plots_keep_rows_but_bound_svg_nodes() {
    const ROWS: usize = 2_000;
    let genome = Value::Table(Table::new(
        vec!["chrom".into(), "start".into(), "end".into()],
        (0..ROWS)
            .map(|index| {
                vec![
                    Value::Str("chr1".into()),
                    Value::Float(index as f64),
                    Value::Float(index as f64 + 0.8),
                ]
            })
            .collect(),
    ));
    let genome_spec = call_plot_builtin(
        "genome_track",
        vec![genome, opts(vec![("format", Value::Str("spec".into()))])],
    )
    .unwrap();
    let Some(Value::Table(rows)) = record(&genome_spec).get("data") else {
        unreachable!()
    };
    assert_eq!(rows.num_rows(), ROWS);
    let Value::Str(svg) = render(genome_spec) else {
        unreachable!()
    };
    assert!(svg.matches("<path").count() < 30);
    assert!(svg.matches("<rect").count() < 10);

    let lollipop = Value::Table(Table::new(
        vec!["position".into(), "count".into()],
        (0..ROWS)
            .map(|index| {
                vec![
                    Value::Float(index as f64),
                    Value::Float((index % 10 + 1) as f64),
                ]
            })
            .collect(),
    ));
    let lollipop_spec = call_bio_plots_builtin(
        "lollipop",
        vec![lollipop, opts(vec![("format", Value::Str("spec".into()))])],
    )
    .unwrap();
    let Value::Str(svg) = render(lollipop_spec) else {
        unreachable!()
    };
    assert!(svg.matches("<path").count() < 30);
    assert!(svg.matches("<circle").count() < 10);

    let junctions = Value::Table(Table::new(
        vec!["start".into(), "end".into(), "count".into()],
        (0..ROWS)
            .map(|index| {
                vec![
                    Value::Float(index as f64 * 2.0),
                    Value::Float(index as f64 * 2.0 + 1.0),
                    Value::Float((index % 16 + 1) as f64),
                ]
            })
            .collect(),
    ));
    let sashimi_spec = call_bio_plots_builtin(
        "sashimi",
        vec![junctions, opts(vec![("format", Value::Str("spec".into()))])],
    )
    .unwrap();
    let Some(Value::Table(rows)) = record(&sashimi_spec).get("junctions") else {
        unreachable!()
    };
    assert_eq!(rows.num_rows(), ROWS);
    let Value::Str(svg) = render(sashimi_spec) else {
        unreachable!()
    };
    assert!(svg.matches("<path").count() < 30);
}
