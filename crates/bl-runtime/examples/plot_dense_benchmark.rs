//! Reproducible dense-point rendering probe.
//!
//! Build this example in release mode, then run one size/mode per process. The
//! PowerShell harness in `packages/statistics/validation/plot_benchmark.ps1`
//! adds peak-working-set measurements and writes a machine-readable manifest.

use bl_core::value::Value;
use bl_runtime::bio_plots::call_bio_plots_builtin;
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

fn main() {
    let count: usize = argument("--size", "20000").parse().expect("valid --size");
    let mode = argument("--raster", "auto");
    let repeats: usize = argument("--repeats", "5").parse().expect("valid --repeats");
    let data = points(count);
    let options = Value::Record(
        HashMap::from([
            ("raster".into(), Value::Str(mode.clone())),
            ("width".into(), Value::Int(800)),
            ("height".into(), Value::Int(600)),
        ])
        .into(),
    );

    let mut elapsed_ms = Vec::with_capacity(repeats);
    let mut svg = String::new();
    for _ in 0..repeats {
        let started = Instant::now();
        svg = match call_bio_plots_builtin("umap_plot", vec![data.clone(), options.clone()]) {
            Ok(Value::Str(svg)) => svg,
            other => panic!("umap_plot failed: {other:?}"),
        };
        elapsed_ms.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    elapsed_ms.sort_by(f64::total_cmp);
    let median_ms = elapsed_ms[elapsed_ms.len() / 2];
    let svg_elements = svg.matches('<').count().saturating_sub(1);
    let point_circles = svg.matches("<circle").count();
    let raster_layers = svg.matches("<image").count();
    println!(
        "{}",
        serde_json::to_string(&json!({
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
