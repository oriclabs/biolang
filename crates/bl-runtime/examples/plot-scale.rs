//! What an embedding plot costs as the cell count grows.
//!
//! umap_plot emits one <circle> per cell. That is fine for the 2700 cells of
//! PBMC3k and the question is what happens at the sizes single-cell work
//! actually reaches now. Reports the generation time and the size of the
//! string, plus the DOM node count a browser would have to build from it.
//!
//!   cargo run --release --example plot-scale -p bl-runtime

use bl_core::value::Value;
use bl_runtime::bio_plots::call_bio_plots_builtin;
use std::collections::HashMap;
use std::time::Instant;

fn points(n: usize) -> Value {
    let mut rows = Vec::with_capacity(n);
    for i in 0..n {
        // Deterministic pseudo-scatter in a few blobs, so the shape is roughly
        // what a real embedding looks like.
        let cluster = i % 12;
        let jitter = ((i * 2_654_435_761) % 1000) as f64 / 1000.0;
        let mut record = HashMap::new();
        record.insert(
            "x".to_string(),
            Value::Float((cluster as f64).cos() * 10.0 + jitter * 2.0),
        );
        record.insert(
            "y".to_string(),
            Value::Float((cluster as f64).sin() * 10.0 + jitter * 2.0),
        );
        record.insert("cluster".to_string(), Value::Int(cluster as i64));
        rows.push(Value::Record(record.into()));
    }
    Value::List(rows.into())
}

fn main() {
    println!(
        "{:>10}  {:>10}  {:>12}  {:>10}",
        "cells", "build", "svg size", "elements"
    );
    for n in [2_700usize, 10_000, 50_000, 200_000, 1_000_000] {
        let data = points(n);
        let started = Instant::now();
        let svg = match call_bio_plots_builtin("umap_plot", vec![data]) {
            Ok(Value::Str(svg)) => svg,
            other => {
                println!("{n:>10}  failed: {other:?}");
                continue;
            }
        };
        let elapsed = started.elapsed();
        let elements = svg.matches("<circle").count() + svg.matches("<text").count();
        println!(
            "{:>10}  {:>8.0?}  {:>9.0} KB  {:>10}",
            n,
            elapsed,
            svg.len() as f64 / 1024.0,
            elements
        );
    }
}
