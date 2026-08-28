//! Renderer-independent geometry for cytobands, copy-number segments and coverage tracks.

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
    call_plot_builtin("render_plot", vec![specification]).expect("render plot specification")
}

fn ideogram_input() -> Value {
    Value::Table(Table::new(
        vec![
            "chrom".into(),
            "start".into(),
            "end".into(),
            "band".into(),
            "stain".into(),
        ],
        vec![
            vec![
                Value::Str("chr2".into()),
                Value::Float(50.0),
                Value::Float(100.0),
                Value::Str("q11".into()),
                Value::Str("gpos75".into()),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Float(0.0),
                Value::Float(200.0),
                Value::Str("p11".into()),
                Value::Str("acen".into()),
            ],
            vec![
                Value::Str("chr2".into()),
                Value::Float(0.0),
                Value::Float(50.0),
                Value::Str("p11".into()),
                Value::Str("gneg".into()),
            ],
        ],
    ))
}

#[test]
fn ideogram_freezes_shared_scale_stain_classes_order_and_exact_replay() {
    let common = vec![("theme", Value::Str("publication".into()))];
    let direct =
        call_bio_plots_builtin("ideogram", vec![ideogram_input(), opts(common.clone())]).unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_bio_plots_builtin("ideogram", vec![ideogram_input(), opts(spec_options)]).unwrap();
    let map = record(&specification);
    assert!(matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "ideogram"));
    let Some(Value::Table(chromosomes)) = map.get("chromosomes") else {
        panic!("chromosomes should be a Table")
    };
    assert_eq!(chromosomes.rows[0][1], Value::Str("chr2".into()));
    assert_eq!(chromosomes.rows[0][3], Value::Float(100.0));
    assert_eq!(chromosomes.rows[1][1], Value::Str("chr1".into()));
    assert_eq!(chromosomes.rows[1][3], Value::Float(200.0));
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("ideogram geometry should be a Table")
    };
    assert_eq!(data.rows[0][0], Value::Int(2));
    assert_eq!(data.rows[0][6], Value::Str("p11".into()));
    assert_eq!(data.rows[2][8], Value::Str("acen".into()));
    assert_eq!(direct, render(specification));
}

fn cnv_input() -> Value {
    Value::Table(Table::new(
        vec![
            "chrom".into(),
            "start".into(),
            "end".into(),
            "log2ratio".into(),
        ],
        vec![
            vec![
                Value::Str("chr2".into()),
                Value::Float(100.0),
                Value::Float(200.0),
                Value::Float(0.7),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Float(0.0),
                Value::Float(50.0),
                Value::Float(-0.4),
            ],
            vec![
                Value::Str("chr2".into()),
                Value::Float(0.0),
                Value::Float(100.0),
                Value::Float(0.05),
            ],
        ],
    ))
}

#[test]
fn cnv_freezes_real_genomic_interval_bounds_states_and_exact_replay() {
    let common = vec![
        ("theme", Value::Str("publication".into())),
        ("loss_threshold", Value::Float(-0.25)),
        ("gain_threshold", Value::Float(0.25)),
    ];
    let direct =
        call_bio_plots_builtin("cnv_plot", vec![cnv_input(), opts(common.clone())]).unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_bio_plots_builtin("cnv_plot", vec![cnv_input(), opts(spec_options)]).unwrap();
    let map = record(&specification);
    let Some(Value::Table(chromosomes)) = map.get("chromosomes") else {
        panic!("chromosomes should be a Table")
    };
    assert_eq!(chromosomes.rows[1][2], Value::Float(204.0));
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("CNV geometry should be a Table")
    };
    assert_eq!(data.rows[0][0], Value::Int(2));
    assert_eq!(data.rows[0][6], Value::Float(0.0));
    assert_eq!(data.rows[0][7], Value::Float(100.0));
    assert_eq!(data.rows[0][10], Value::Str("neutral".into()));
    assert_eq!(data.rows[1][10], Value::Str("gain".into()));
    assert_eq!(data.rows[2][6], Value::Float(204.0));
    assert_eq!(data.rows[2][10], Value::Str("loss".into()));
    assert_eq!(direct, render(specification));
}

fn coverage_input() -> Value {
    Value::Table(Table::new(
        vec![
            "chrom".into(),
            "start".into(),
            "end".into(),
            "coverage".into(),
        ],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Float(20.0),
                Value::Float(40.0),
                Value::Float(8.0),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Float(0.0),
                Value::Float(20.0),
                Value::Float(3.0),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Float(40.0),
                Value::Float(80.0),
                Value::Float(5.0),
            ],
        ],
    ))
}

#[test]
fn coverage_track_clips_overlapping_intervals_instead_of_filtering_midpoints() {
    let common = vec![
        ("theme", Value::Str("publication".into())),
        ("region_start", Value::Float(10.0)),
        ("region_end", Value::Float(60.0)),
    ];
    let direct = call_bio_plots_builtin(
        "coverage_track",
        vec![coverage_input(), opts(common.clone())],
    )
    .unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_bio_plots_builtin("coverage_track", vec![coverage_input(), opts(spec_options)])
            .unwrap();
    let map = record(&specification);
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("coverage geometry should be a Table")
    };
    assert_eq!(data.num_rows(), 3);
    assert_eq!(data.rows[0][3], Value::Float(0.0));
    assert_eq!(data.rows[0][5], Value::Float(10.0));
    assert_eq!(data.rows[0][6], Value::Float(20.0));
    assert_eq!(data.rows[0][10], Value::Bool(true));
    assert_eq!(data.rows[2][5], Value::Float(40.0));
    assert_eq!(data.rows[2][6], Value::Float(60.0));
    assert_eq!(data.rows[2][10], Value::Bool(true));
    assert_eq!(direct, render(specification));
}

#[test]
fn coverage_requires_an_explicit_chromosome_for_multi_chromosome_data() {
    let input = Value::Table(Table::new(
        vec!["chrom".into(), "pos".into(), "value".into()],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Float(10.0),
                Value::Float(2.0),
            ],
            vec![
                Value::Str("chr2".into()),
                Value::Float(10.0),
                Value::Float(4.0),
            ],
        ],
    ));
    assert!(call_bio_plots_builtin("coverage_track", vec![input.clone()]).is_err());
    let specification = call_bio_plots_builtin(
        "coverage_track",
        vec![
            input,
            opts(vec![
                ("chromosome", Value::Str("chr2".into())),
                ("format", Value::Str("spec".into())),
            ]),
        ],
    )
    .unwrap();
    let Some(Value::Table(data)) = record(&specification).get("data") else {
        panic!("coverage geometry should be a Table")
    };
    assert_eq!(data.num_rows(), 1);
    assert_eq!(data.rows[0][2], Value::Str("chr2".into()));
    assert!(matches!(render(specification), Value::Str(_)));
}

#[test]
fn genomic_track_specs_reject_tampering_and_html_keeps_canvas_fallback() {
    let specifications = [
        call_bio_plots_builtin(
            "ideogram",
            vec![
                ideogram_input(),
                opts(vec![("format", Value::Str("spec".into()))]),
            ],
        )
        .unwrap(),
        call_bio_plots_builtin(
            "cnv_plot",
            vec![
                cnv_input(),
                opts(vec![("format", Value::Str("spec".into()))]),
            ],
        )
        .unwrap(),
        call_bio_plots_builtin(
            "coverage_track",
            vec![
                coverage_input(),
                opts(vec![("format", Value::Str("spec".into()))]),
            ],
        )
        .unwrap(),
    ];
    let Value::Str(html) = call_plot_builtin(
        "render_plot",
        vec![
            specifications[2].clone(),
            opts(vec![("format", Value::Str("html".into()))]),
        ],
    )
    .unwrap() else {
        panic!("expected standalone HTML")
    };
    assert!(html.contains("<svg"));
    assert!(html.contains("<canvas"));
    for specification in specifications {
        let mut broken = record(&specification).clone();
        broken.insert("data".into(), Value::Table(Table::empty()));
        assert!(call_plot_builtin("render_plot", vec![Value::Record(broken.into())]).is_err());
    }
}

#[test]
fn genomic_tracks_reject_invalid_intervals_thresholds_labels_and_regions() {
    let invalid_interval = Value::Table(Table::new(
        vec!["chrom".into(), "start".into(), "end".into(), "stain".into()],
        vec![vec![
            Value::Str("chr1".into()),
            Value::Float(20.0),
            Value::Float(10.0),
            Value::Str("gneg".into()),
        ]],
    ));
    assert!(call_bio_plots_builtin("ideogram", vec![invalid_interval]).is_err());

    let invalid_stain = Value::Table(Table::new(
        vec!["chrom".into(), "start".into(), "end".into(), "stain".into()],
        vec![vec![
            Value::Str("chr1".into()),
            Value::Float(0.0),
            Value::Float(10.0),
            Value::Int(25),
        ]],
    ));
    assert!(call_bio_plots_builtin("ideogram", vec![invalid_stain]).is_err());

    assert!(call_bio_plots_builtin(
        "cnv_plot",
        vec![
            cnv_input(),
            opts(vec![
                ("loss_threshold", Value::Float(0.5)),
                ("gain_threshold", Value::Float(0.2)),
            ]),
        ],
    )
    .is_err());
    assert!(call_bio_plots_builtin(
        "coverage_track",
        vec![
            coverage_input(),
            opts(vec![
                ("region_start", Value::Float(60.0)),
                ("region_end", Value::Float(10.0)),
            ]),
        ],
    )
    .is_err());
    assert!(call_bio_plots_builtin(
        "coverage_track",
        vec![
            coverage_input(),
            opts(vec![("color", Value::Str("red\" onclick=\"bad".into()))]),
        ],
    )
    .is_err());
}

#[test]
fn dense_genomic_tracks_keep_all_rows_with_bounded_svg_elements() {
    const ROWS: usize = 25_000;
    let ideogram = Value::Table(Table::new(
        vec!["chrom".into(), "start".into(), "end".into(), "stain".into()],
        (0..ROWS)
            .map(|index| {
                vec![
                    Value::Str(format!("chr{}", index % 4 + 1)),
                    Value::Float((index / 4) as f64),
                    Value::Float((index / 4 + 1) as f64),
                    Value::Str(if index % 2 == 0 { "gneg" } else { "gpos50" }.into()),
                ]
            })
            .collect(),
    ));
    let ideogram_spec = call_bio_plots_builtin(
        "ideogram",
        vec![ideogram, opts(vec![("format", Value::Str("spec".into()))])],
    )
    .unwrap();
    let Some(Value::Table(ideogram_rows)) = record(&ideogram_spec).get("data") else {
        panic!("ideogram geometry should be a Table")
    };
    assert_eq!(ideogram_rows.num_rows(), ROWS);
    let Value::Str(ideogram_svg) = render(ideogram_spec) else {
        panic!("expected ideogram SVG")
    };
    assert!(ideogram_svg.matches("<path").count() < 12);
    assert!(ideogram_svg.matches("<rect").count() < 12);

    let cnv = Value::Table(Table::new(
        vec![
            "chrom".into(),
            "start".into(),
            "end".into(),
            "log2ratio".into(),
        ],
        (0..ROWS)
            .map(|index| {
                vec![
                    Value::Str(format!("chr{}", index % 4 + 1)),
                    Value::Float((index / 4) as f64),
                    Value::Float((index / 4 + 1) as f64),
                    Value::Float((index % 7) as f64 / 5.0 - 0.6),
                ]
            })
            .collect(),
    ));
    let cnv_spec = call_bio_plots_builtin(
        "cnv_plot",
        vec![cnv, opts(vec![("format", Value::Str("spec".into()))])],
    )
    .unwrap();
    let Some(Value::Table(cnv_rows)) = record(&cnv_spec).get("data") else {
        panic!("CNV geometry should be a Table")
    };
    assert_eq!(cnv_rows.num_rows(), ROWS);
    let Value::Str(cnv_svg) = render(cnv_spec) else {
        panic!("expected CNV SVG")
    };
    assert!(cnv_svg.matches("<path").count() < 12);

    let coverage = Value::Table(Table::new(
        vec!["chrom".into(), "start".into(), "end".into(), "value".into()],
        (0..ROWS)
            .map(|index| {
                vec![
                    Value::Str("chr1".into()),
                    Value::Float(index as f64),
                    Value::Float((index + 1) as f64),
                    Value::Float((index % 101) as f64),
                ]
            })
            .collect(),
    ));
    let coverage_spec = call_bio_plots_builtin(
        "coverage_track",
        vec![coverage, opts(vec![("format", Value::Str("spec".into()))])],
    )
    .unwrap();
    let Some(Value::Table(coverage_rows)) = record(&coverage_spec).get("data") else {
        panic!("coverage geometry should be a Table")
    };
    assert_eq!(coverage_rows.num_rows(), ROWS);
    let Value::Str(coverage_svg) = render(coverage_spec) else {
        panic!("expected coverage SVG")
    };
    assert!(coverage_svg.matches("<path").count() < 12);
}
