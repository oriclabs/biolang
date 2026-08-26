//! Reproducible dense-point rendering probe.
//!
//! Build this example in release mode, then run one size/mode per process. The
//! PowerShell harness in `packages/statistics/validation/plot_benchmark.ps1`
//! adds peak-working-set measurements and writes a machine-readable manifest.

use bl_core::value::{Table, Value};
use bl_runtime::bio_plots::call_bio_plots_builtin;
use bl_runtime::plot::call_plot_builtin;
use serde_json::json;
use std::collections::HashMap;
use std::time::Instant;

fn argument(name: &str, default: &str) -> String {
    let arguments: Vec<String> = std::env::args().collect();
    arguments
        .windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
        .unwrap_or_else(|| default.to_string())
}

fn points(count: usize) -> Value {
    let mut rows = Vec::with_capacity(count);
    for index in 0..count {
        let angle = index as f64 * 0.017_453_292_519_943_295;
        let cluster = index % 12;
        let radius = 1.0 + (index % 97) as f64 / 97.0;
        let mut row = HashMap::new();
        row.insert(
            "x".to_string(),
            Value::Float(cluster as f64 * 3.0 + radius * angle.cos()),
        );
        row.insert(
            "y".to_string(),
            Value::Float((cluster % 4) as f64 * 3.0 + radius * angle.sin()),
        );
        row.insert("cluster".to_string(), Value::Int(cluster as i64));
        rows.push(Value::Record(row.into()));
    }
    Value::List(rows.into())
}

/// A table of the shape one plot reads, sized to `count`.
///
/// The columns carry the names each builtin expects and values in a plausible
/// range, cycled deterministically. Nothing here is a measurement of anything
/// biological -- it exists so the renderer has the stated number of points to
/// draw, and only bytes and element counts are read back.
fn table_for(plot: &str, count: usize) -> Value {
    let (columns, rows): (Vec<String>, Vec<Vec<Value>>) = match plot {
        "volcano" => (
            vec!["log2fc".into(), "pvalue".into()],
            (0..count)
                .map(|i| {
                    let fc = ((i % 801) as f64 - 400.0) / 80.0;
                    let p = 1.0 / (1.0 + (i % 9973) as f64);
                    vec![Value::Float(fc), Value::Float(p)]
                })
                .collect(),
        ),
        "ma_plot" => (
            vec!["baseMean".into(), "log2fc".into()],
            (0..count)
                .map(|i| {
                    let base = 1.0 + (i % 5003) as f64;
                    let m = ((i % 601) as f64 - 300.0) / 75.0;
                    vec![Value::Float(base), Value::Float(m)]
                })
                .collect(),
        ),
        // The bio_plots volcano reads the same two columns as the plot-module one.
        "volcano_plot" => {
            return table_for("volcano", count);
        }
        "rainfall" => (
            vec!["chrom".into(), "pos".into()],
            (0..count)
                .map(|i| {
                    vec![
                        Value::Str(format!("chr{}", (i % 22) + 1).into()),
                        Value::Int(((i % 250_000) * 1000) as i64),
                    ]
                })
                .collect(),
        ),
        "manhattan" => (
            vec!["chrom".into(), "pos".into(), "pvalue".into()],
            (0..count)
                .map(|i| {
                    let chrom = format!("chr{}", (i % 22) + 1);
                    let pos = ((i % 250_000) * 1000) as i64;
                    let p = 1.0 / (1.0 + (i % 9973) as f64);
                    vec![Value::Str(chrom.into()), Value::Int(pos), Value::Float(p)]
                })
                .collect(),
        ),
        "plot" => (
            vec!["x".into(), "y".into()],
            (0..count)
                .map(|i| {
                    vec![
                        Value::Float((i % 977) as f64 / 977.0),
                        Value::Float((i % 613) as f64 / 613.0),
                    ]
                })
                .collect(),
        ),
        // pca_plot reads samples as rows and every numeric column as a
        // feature. Eight features keeps the covariance step real without
        // letting it dominate the render being measured.
        "pca_plot" => (
            (0..8).map(|f| format!("feature{}", f + 1)).collect(),
            (0..count)
                .map(|i| {
                    (0..8)
                        .map(|f| {
                            let phase = (i % (211 + f * 13)) as f64 / 211.0;
                            Value::Float(phase + (i % 7) as f64 * 0.25)
                        })
                        .collect()
                })
                .collect(),
        ),
        "variable_feature_plot" => (
            vec!["gene".into(), "mean".into(), "dispersion".into()],
            (0..count)
                .map(|i| {
                    vec![
                        Value::Str(format!("gene{i}").into()),
                        Value::Float(0.01 + (i % 4001) as f64 / 400.0),
                        Value::Float(0.5 + (i % 997) as f64 / 300.0),
                    ]
                })
                .collect(),
        ),
        other => panic!("unknown --plot {other}"),
    };
    Value::Table(Table::new(columns, rows).into())
}

/// Options each plot needs beyond the shared raster and size settings.
fn extra_options(plot: &str) -> Vec<(String, Value)> {
    match plot {
        "plot" => vec![
            ("x".into(), Value::Str("x".into())),
            ("y".into(), Value::Str("y".into())),
            ("type".into(), Value::Str("scatter".into())),
        ],
        // Without an explicit `n` this plot highlights nothing -- a table
        // carrying no `variable` column is not assumed to be a selection. The
        // second raster layer would then be blank, and the probe would report
        // two layers while measuring one.
        "variable_feature_plot" => vec![("n".into(), Value::Int(2000))],
        _ => Vec::new(),
    }
}

fn main() {
    let count: usize = argument("--size", "20000").parse().expect("valid --size");
    let mode = argument("--raster", "auto");
    let repeats: usize = argument("--repeats", "5").parse().expect("valid --repeats");
    let plot = argument("--plot", "umap_plot");
    let data = match plot.as_str() {
        "umap_plot" => points(count),
        // qq_plot reads a bare numeric list rather than a table.
        "qq_plot" => Value::List(
            (0..count)
                .map(|i| Value::Float(1.0 / (1.0 + (i % 9973) as f64)))
                .collect::<Vec<_>>()
                .into(),
        ),
        _ => table_for(&plot, count),
    };
    let mut option_pairs: Vec<(String, Value)> = vec![
        ("raster".into(), Value::Str(mode.clone().into())),
        ("width".into(), Value::Int(800)),
        ("height".into(), Value::Int(600)),
    ];
    // Supersampling multiplier. Left off the option record entirely when not
    // given, so the run measures the builtin's own default rather than this
    // harness's idea of it.
    // Opt-in point thinning, for the plots that offer it.
    let thin = argument("--thin", "");
    if !thin.is_empty() {
        option_pairs.push(("thin".into(), Value::Str(thin.into())));
    }
    let scale = argument("--scale", "");
    if !scale.is_empty() {
        let scale: f64 = scale.parse().expect("valid --scale");
        option_pairs.push(("raster_scale".into(), Value::Float(scale)));
    }
    option_pairs.extend(extra_options(&plot));
    let options = Value::Record(HashMap::from_iter(option_pairs).into());

    let mut elapsed_ms = Vec::with_capacity(repeats);
    let mut svg = String::new();
    for _ in 0..repeats {
        let started = Instant::now();
        let arguments = vec![data.clone(), options.clone()];
        let produced = if matches!(
            plot.as_str(),
            "umap_plot"
                | "manhattan"
                | "qq_plot"
                | "rainfall"
                | "volcano_plot"
                | "pca_plot"
                | "variable_feature_plot"
        ) {
            call_bio_plots_builtin(&plot, arguments)
        } else {
            call_plot_builtin(&plot, arguments)
        };
        svg = match produced {
            Ok(Value::Str(svg)) => svg,
            other => panic!("{plot} failed: {other:?}"),
        };
        elapsed_ms.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    elapsed_ms.sort_by(f64::total_cmp);
    let median_ms = elapsed_ms[elapsed_ms.len() / 2];
    // `--out` writes the document itself, so a run can be inspected rather
    // than only counted -- decoding the raster layer, say, to check what the
    // encoder actually produced.
    let out = argument("--out", "");
    if !out.is_empty() {
        std::fs::write(&out, svg.as_bytes()).expect("write --out");
    }

    let svg_elements = svg.matches('<').count().saturating_sub(1);
    let point_circles = svg.matches("<circle").count();
    let raster_layers = svg.matches("<image").count();
    println!(
        "{}",
        serde_json::to_string(&json!({
            "plot": plot,
            "size": count,
            "raster": mode,
            "repeats": repeats,
            "elapsed_ms": elapsed_ms,
            "median_elapsed_ms": median_ms,
            "svg_bytes": svg.len(),
            "svg_elements": svg_elements,
            "point_circles": point_circles,
            "raster_layers": raster_layers
        }))
        .unwrap()
    );
}
