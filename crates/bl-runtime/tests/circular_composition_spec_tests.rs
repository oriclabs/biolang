use bl_core::value::{Table, Value};
use bl_runtime::bio_plots::call_bio_plots_builtin;
use bl_runtime::plot::call_plot_builtin;
use std::collections::HashMap;

fn record(entries: Vec<(&str, Value)>) -> Value {
    Value::Record(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect::<HashMap<_, _>>()
            .into(),
    )
}

fn options(entries: Vec<(&str, Value)>) -> Value {
    record(entries)
}

fn table(columns: &[&str], rows: Vec<Vec<Value>>) -> Value {
    Value::Table(Table::new(
        columns.iter().map(|column| column.to_string()).collect(),
        rows,
    ))
}

fn circular_input() -> Value {
    let segments = table(
        &["chrom", "end"],
        vec![
            vec![Value::Str("chr1".into()), Value::Float(100.0)],
            vec![Value::Str("chr2".into()), Value::Float(50.0)],
        ],
    );
    let links = table(
        &[
            "source_chr",
            "source_start",
            "source_end",
            "target_chr",
            "target_start",
            "target_end",
            "count",
        ],
        vec![vec![
            Value::Str("chr1".into()),
            Value::Float(10.0),
            Value::Float(20.0),
            Value::Str("chr2".into()),
            Value::Float(30.0),
            Value::Float(35.0),
            Value::Float(16.0),
        ]],
    );
    let coverage = table(
        &["chrom", "pos", "depth"],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Float(10.0),
                Value::Float(2.0),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Float(50.0),
                Value::Float(9.0),
            ],
            vec![
                Value::Str("chr2".into()),
                Value::Float(20.0),
                Value::Float(4.0),
            ],
        ],
    );
    let cnv = table(
        &["chrom", "start", "end", "log2ratio"],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Float(0.0),
                Value::Float(30.0),
                Value::Float(-0.6),
            ],
            vec![
                Value::Str("chr2".into()),
                Value::Float(5.0),
                Value::Float(40.0),
                Value::Float(0.8),
            ],
        ],
    );
    let tracks = Value::List(
        vec![
            record(vec![
                ("name", Value::Str("coverage".into())),
                ("type", Value::Str("line".into())),
                ("data", coverage),
            ]),
            record(vec![
                ("name", Value::Str("copy number".into())),
                ("type", Value::Str("cnv".into())),
                ("data", cnv),
            ]),
        ]
        .into(),
    );
    record(vec![
        ("segments", segments),
        ("links", links),
        ("tracks", tracks),
    ])
}

#[test]
fn circos_freezes_length_weighted_angles_tracks_ribbons_and_exact_replay() {
    let common = vec![
        ("theme", Value::Str("publication".into())),
        ("title", Value::Str("Genome overview".into())),
        ("gap_degrees", Value::Float(3.0)),
    ];
    let direct =
        call_bio_plots_builtin("circos", vec![circular_input(), options(common.clone())]).unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_bio_plots_builtin("circos", vec![circular_input(), options(spec_options)]).unwrap();
    let Value::Record(map) = &specification else {
        panic!("expected circos PlotSpec")
    };
    assert_eq!(map.get("plot").and_then(Value::as_str), Some("circos"));
    let Some(Value::Table(segments)) = map.get("segments") else {
        panic!("expected frozen chromosome geometry")
    };
    let first_sweep =
        segments.rows[0][7].as_float().unwrap() - segments.rows[0][6].as_float().unwrap();
    let second_sweep =
        segments.rows[1][7].as_float().unwrap() - segments.rows[1][6].as_float().unwrap();
    assert!((first_sweep / second_sweep - 2.0).abs() < 1e-10);
    let Some(Value::Table(tracks)) = map.get("tracks") else {
        panic!("expected frozen tracks")
    };
    assert_eq!(tracks.rows.len(), 5);
    let Some(Value::Table(links)) = map.get("links") else {
        panic!("expected frozen links")
    };
    assert_eq!(links.rows.len(), 1);
    assert!(links.rows[0][11].as_float().unwrap() > links.rows[0][10].as_float().unwrap());
    let replay = call_plot_builtin("render_plot", vec![specification]).unwrap();
    assert_eq!(direct, replay);
    let Value::Str(svg) = direct else {
        panic!("expected SVG")
    };
    assert!(svg.contains("data-circos-layer=\"ribbon\""));
    assert!(svg.contains("data-circos-layer=\"line-track\""));
    assert_eq!(svg.matches("data-circos-layer=\"line-track\"").count(), 2);
    assert!(svg.matches("<circle").count() >= 3);
}

#[test]
fn circos_rejects_unknown_chromosomes_and_tampered_angles() {
    let bad_links = record(vec![
        (
            "segments",
            table(
                &["chrom", "end"],
                vec![vec![Value::Str("chr1".into()), Value::Float(100.0)]],
            ),
        ),
        (
            "links",
            table(
                &["source_chr", "source_pos", "target_chr", "target_pos"],
                vec![vec![
                    Value::Str("chr1".into()),
                    Value::Float(10.0),
                    Value::Str("chr9".into()),
                    Value::Float(20.0),
                ]],
            ),
        ),
    ]);
    assert!(call_bio_plots_builtin("circos", vec![bad_links]).is_err());

    let mut spec = call_bio_plots_builtin(
        "circos",
        vec![
            circular_input(),
            options(vec![("format", Value::Str("spec".into()))]),
        ],
    )
    .unwrap();
    let Value::Record(map) = &mut spec else {
        unreachable!()
    };
    let owned = std::sync::Arc::make_mut(map);
    let Value::Table(segments) = owned.get_mut("segments").unwrap() else {
        unreachable!()
    };
    segments.rows[0][6] = Value::Float(99.0);
    assert!(call_plot_builtin("render_plot", vec![spec]).is_err());

    let mut radial_spec = call_bio_plots_builtin(
        "circos",
        vec![
            circular_input(),
            options(vec![("format", Value::Str("spec".into()))]),
        ],
    )
    .unwrap();
    let Value::Record(map) = &mut radial_spec else {
        unreachable!()
    };
    let owned = std::sync::Arc::make_mut(map);
    let Value::Table(tracks) = owned.get_mut("tracks").unwrap() else {
        unreachable!()
    };
    tracks.rows[0][11] = Value::Float(0.85);
    assert!(call_plot_builtin("render_plot", vec![radial_spec]).is_err());

    let mut ribbon_spec = call_bio_plots_builtin(
        "circos",
        vec![
            circular_input(),
            options(vec![("format", Value::Str("spec".into()))]),
        ],
    )
    .unwrap();
    let Value::Record(map) = &mut ribbon_spec else {
        unreachable!()
    };
    let owned = std::sync::Arc::make_mut(map);
    let Value::Table(links) = owned.get_mut("links").unwrap() else {
        unreachable!()
    };
    links.rows[0][11] = Value::Float(0.0);
    assert!(call_plot_builtin("render_plot", vec![ribbon_spec]).is_err());
}

#[test]
fn dense_circos_keeps_all_geometry_and_bounds_svg_nodes() {
    const ROWS: usize = 1_000;
    let segments = table(
        &["chrom", "end"],
        vec![
            vec![Value::Str("chr1".into()), Value::Float(10_000.0)],
            vec![Value::Str("chr2".into()), Value::Float(10_000.0)],
        ],
    );
    let links = table(
        &[
            "source_chr",
            "source_pos",
            "target_chr",
            "target_pos",
            "count",
        ],
        (0..ROWS)
            .map(|index| {
                vec![
                    Value::Str("chr1".into()),
                    Value::Float(index as f64 * 9.0),
                    Value::Str("chr2".into()),
                    Value::Float(9_500.0 - index as f64 * 9.0),
                    Value::Float((index % 20 + 1) as f64),
                ]
            })
            .collect(),
    );
    let input = record(vec![("segments", segments), ("links", links)]);
    let spec = call_bio_plots_builtin(
        "circos",
        vec![input, options(vec![("format", Value::Str("spec".into()))])],
    )
    .unwrap();
    let Value::Record(map) = &spec else {
        unreachable!()
    };
    let Some(Value::Table(links)) = map.get("links") else {
        unreachable!()
    };
    assert_eq!(links.rows.len(), ROWS);
    let Value::Str(svg) = call_plot_builtin("render_plot", vec![spec]).unwrap() else {
        unreachable!()
    };
    assert!(svg.matches("data-circos-layer=\"dense-links\"").count() <= 80);
    assert!(svg.len() < 250_000);
}

#[test]
fn plot_grid_freezes_equal_cells_panel_tags_shared_legend_and_exact_replay() {
    let Value::Str(first) = call_plot_builtin(
        "histogram",
        vec![Value::List(
            vec![Value::Float(1.0), Value::Float(2.0)].into(),
        )],
    )
    .unwrap() else {
        unreachable!()
    };
    let Value::Str(second) = call_plot_builtin(
        "histogram",
        vec![Value::List(
            vec![Value::Float(2.0), Value::Float(3.0)].into(),
        )],
    )
    .unwrap() else {
        unreachable!()
    };
    let legend = table(
        &["label", "color"],
        vec![vec![
            Value::Str("observed".into()),
            Value::Str("#4e79a7".into()),
        ]],
    );
    let plots = Value::List(vec![Value::Str(first), Value::Str(second)].into());
    let common = vec![
        ("columns", Value::Int(2)),
        ("title", Value::Str("Two panels".into())),
        ("shared_xlabel", Value::Str("measurement".into())),
        ("legend", legend),
    ];
    let direct =
        call_plot_builtin("plot_grid", vec![plots.clone(), options(common.clone())]).unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let spec = call_plot_builtin("plot_grid", vec![plots, options(spec_options)]).unwrap();
    let Value::Record(map) = &spec else {
        unreachable!()
    };
    let Some(Value::Table(panels)) = map.get("panels") else {
        unreachable!()
    };
    assert_eq!(panels.rows[0][3], Value::Str("A".into()));
    assert_eq!(panels.rows[1][3], Value::Str("B".into()));
    assert_eq!(panels.rows[0][6], panels.rows[1][6]);
    for row in &panels.rows {
        let child = row[10].as_str().unwrap();
        assert!(!child.contains("data-biolang-axis-title=\"x\""));
        assert!(child.contains("data-biolang-axis-title=\"y\""));
    }
    assert_eq!(
        direct,
        call_plot_builtin("render_plot", vec![spec]).unwrap()
    );
}

#[test]
fn plot_grid_rejects_active_svg_and_html_keeps_canvas_fallback() {
    let active = Value::List(
        vec![Value::Str(
            "<svg width=\"10\" height=\"10\"><script>alert(1)</script></svg>".into(),
        )]
        .into(),
    );
    assert!(call_plot_builtin("plot_grid", vec![active]).is_err());

    let safe = Value::List(
        vec![Value::Str(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"10\"><circle cx=\"5\" cy=\"5\" r=\"2\" /></svg>".into(),
        )]
        .into(),
    );
    let Value::Str(html) = call_plot_builtin(
        "plot_grid",
        vec![safe, options(vec![("format", Value::Str("html".into()))])],
    )
    .unwrap() else {
        unreachable!()
    };
    assert!(html.contains("<canvas"));
    assert!(html.contains("Use canvas"));
}
