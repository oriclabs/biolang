//! variable_feature_plot draws the selection it claims to draw.
//!
//! The plot exists to answer one question - did feature selection pick genes
//! across the expression range, or only the rarest ones? That question is only
//! worth asking if the highlighted set is genuinely the set
//! `highly_variable_genes` returns, so the first test here compares them
//! directly. Both now go through one implementation; before that they were two
//! copies of the same ranking, free to drift apart.

use bl_runtime::bio_plots::call_bio_plots_builtin;
use bl_runtime::singlecell::call_singlecell_builtin;
use std::collections::{HashMap, HashSet};

/// Levels of the planted markers: on in half the cells, off in the other half,
/// so each has mean level/2 and dispersion equal to its mean. Chosen to span two
/// decades of expression, because a selection that respects the mean-variance
/// trend has to find all five and a mean-biased one cannot.
const MARKERS: [f64; 5] = [10.0, 40.0, 120.0, 300.0, 800.0];
/// Housekeeping genes at means log-spaced from 0.5 to 500, with Poisson-like
/// noise so their dispersion sits at 1 whatever their mean.
const N_FLAT: usize = 300;
/// Genes with a single count in a single cell. Their dispersion is also ~1, but
/// their variance/mean^2 is enormous - they are what a mean-blind rule picks,
/// and the reason this matrix has them.
const N_RARE: usize = 20;
const N_GENES: usize = MARKERS.len() + N_FLAT + N_RARE;

/// Deterministic noise in [-1.73, 1.73], so a gene's own values do not depend on
/// a seed or on iteration order.
fn jitter(cell: usize, gene: usize) -> f64 {
    let mixed = (cell as u64)
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add((gene as u64).wrapping_mul(1_442_695_040_888_963_407));
    let unit = ((mixed >> 40) & 0xFFFF) as f64 / 65535.0;
    (unit - 0.5) * 3.464
}

/// A cells x genes matrix with a known answer: only the five markers carry
/// structure.
fn matrix(n_cells: usize) -> Value {
    let mut rows = Vec::with_capacity(n_cells);
    for cell in 0..n_cells {
        let mut row = Vec::with_capacity(N_GENES);
        for level in MARKERS {
            row.push(Value::Float(if cell < n_cells / 2 { level } else { 0.0 }));
        }
        for gene in 0..N_FLAT {
            // 0.5 to 500, evenly in log space, so every bin the marker genes
            // land in is populated with ordinary genes to be judged against.
            let mean = 0.5 * 10.0_f64.powf(3.0 * gene as f64 / N_FLAT as f64);
            let value = mean + mean.sqrt() * jitter(cell, gene + MARKERS.len());
            row.push(Value::Float(value.max(0.0)));
        }
        for k in 0..N_RARE {
            row.push(Value::Float(if cell == k { 1.0 } else { 0.0 }));
        }
        rows.push(Value::List(row.into()));
    }
    Value::List(rows.into())
}

fn gene_names(n: usize) -> Value {
    Value::List(
        (0..n)
            .map(|i| Value::Str(format!("G{i}")))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn render(data: Value, opts: Vec<(&str, Value)>) -> String {
    let mut map = HashMap::new();
    for (key, value) in opts {
        map.insert(key.to_string(), value);
    }
    match call_bio_plots_builtin(
        "variable_feature_plot",
        vec![data, Value::Record(map.into())],
    ) {
        Ok(Value::Str(svg)) => svg,
        other => panic!("expected an SVG string, got {other:?}"),
    }
}

/// Every `<circle>` drawn in the highlight colour, as (cx, cy).
fn highlighted(svg: &str) -> Vec<(f64, f64)> {
    circles(svg)
        .into_iter()
        .filter(|(_, _, fill)| fill != "#bbbbbb")
        .map(|(x, y, _)| (x, y))
        .collect()
}

fn circles(svg: &str) -> Vec<(f64, f64, String)> {
    svg.split("<circle")
        .skip(1)
        .filter_map(|tag| {
            let end = tag.find("/>")?;
            let tag = &tag[..end];
            let field = |name: &str| -> Option<f64> {
                let at = tag.find(&format!("{name}=\""))? + name.len() + 2;
                let rest = &tag[at..];
                rest[..rest.find('"')?].parse::<f64>().ok()
            };
            let fill_at = tag.find("fill=\"")? + 6;
            let rest = &tag[fill_at..];
            let fill = rest[..rest.find('"')?].to_string();
            Some((field("cx")?, field("cy")?, fill))
        })
        .collect()
}

fn labels(svg: &str) -> Vec<String> {
    svg.split("</text>")
        .filter_map(|part| {
            let start = part.rfind('>')?;
            Some(part[start + 1..].to_string())
        })
        .filter(|s| s.starts_with('G') && s[1..].chars().all(|c| c.is_ascii_digit()))
        .collect()
}

use bl_core::value::Value;

#[test]
fn the_highlighted_genes_are_the_ones_highly_variable_genes_returns() {
    // The whole point of the figure. If these two ever disagree it is drawing a
    // selection nothing in the pipeline made.
    let data = matrix(120);
    let selected =
        match call_singlecell_builtin("highly_variable_genes", vec![data.clone(), Value::Int(8)]) {
            Ok(Value::List(items)) => items
                .iter()
                .filter_map(|v| match v {
                    Value::Int(i) => Some(format!("G{i}")),
                    _ => None,
                })
                .collect::<HashSet<String>>(),
            other => panic!("highly_variable_genes returned {other:?}"),
        };
    assert_eq!(selected.len(), 8);

    let svg = render(
        data,
        vec![
            ("genes", gene_names(N_GENES)),
            ("n", Value::Int(8)),
            ("label", Value::Int(60)),
        ],
    );
    assert_eq!(
        highlighted(&svg).len(),
        8,
        "highlighted a different number of genes than were selected"
    );
    let drawn: HashSet<String> = labels(&svg).into_iter().collect();
    assert_eq!(
        drawn, selected,
        "the plot highlighted a different set of genes than highly_variable_genes chose"
    );
}

#[test]
fn selection_spread_across_the_expression_range_is_visible() {
    // The diagnostic the figure exists for. The planted markers are bimodal at
    // means from 5 to 400, so a selection that respects the mean-variance trend
    // spreads them along the x-axis. Ranking by variance/mean^2 - the bug that
    // was live in this runtime - would instead pick the single-count genes, and
    // every highlighted point would sit in one narrow band at the left.
    let svg = render(
        matrix(120),
        vec![
            ("genes", gene_names(N_GENES)),
            ("n", Value::Int(MARKERS.len() as i64)),
        ],
    );
    let points = highlighted(&svg);
    assert_eq!(points.len(), MARKERS.len());

    let all: Vec<f64> = circles(&svg).iter().map(|(x, _, _)| *x).collect();
    let axis_left = all.iter().cloned().fold(f64::MAX, f64::min);
    let axis = all.iter().cloned().fold(f64::MIN, f64::max) - axis_left;
    let xs: Vec<f64> = points.iter().map(|(x, _)| *x).collect();
    let leftmost = xs.iter().cloned().fold(f64::MAX, f64::min);

    // The sharp claim, and the one with no threshold in it: every single-count
    // gene is drawn to the left of every selected gene. A mean-blind rule would
    // have selected those genes, putting them on the wrong side of this line.
    let to_the_left = all.iter().filter(|&&x| x < leftmost - 0.5).count();
    assert!(
        to_the_left >= N_RARE,
        "only {to_the_left} genes sit left of the selection - the rare genes were picked"
    );

    // And the selection is spread rather than banded. The markers span 1.9 of
    // the axis's 4.7 decades, so a third of the axis is the honest bar here;
    // the mean-biased alternative would land them all on one pixel column.
    let spread =
        xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min);
    assert!(
        spread > axis * 0.3,
        "selected genes span only {spread:.1} of a {axis:.1} axis - the selection is banded"
    );
}

#[test]
fn the_planted_markers_are_the_ones_picked() {
    // Independent of the plot: the first MARKERS.len() genes are the only ones
    // with real structure. Everything else is noise at dispersion 1.
    let svg = render(
        matrix(120),
        vec![
            ("genes", gene_names(N_GENES)),
            ("n", Value::Int(MARKERS.len() as i64)),
            ("label", Value::Int(MARKERS.len() as i64)),
        ],
    );
    let drawn: HashSet<String> = labels(&svg).into_iter().collect();
    let planted: HashSet<String> = (0..MARKERS.len()).map(|i| format!("G{i}")).collect();
    assert_eq!(
        drawn, planted,
        "feature selection missed the planted markers"
    );
}

#[test]
fn labels_are_capped_so_they_do_not_cover_the_cloud() {
    let svg = render(
        matrix(120),
        vec![
            ("genes", gene_names(N_GENES)),
            ("n", Value::Int(30)),
            ("label", Value::Int(4)),
        ],
    );
    assert_eq!(labels(&svg).len(), 4);
    assert_eq!(highlighted(&svg).len(), 30, "labels changed the selection");
}

/// A table of gene statistics with no selection information attached.
fn stats_table(n: usize) -> Value {
    let rows: Vec<Value> = (0..n)
        .map(|i| {
            let mut record = HashMap::new();
            record.insert("gene".to_string(), Value::Str(format!("g{i}")));
            record.insert("mean".to_string(), Value::Float(1.0 + i as f64));
            record.insert("dispersion".to_string(), Value::Float(n as f64 - i as f64));
            Value::Record(record.into())
        })
        .collect();
    Value::List(rows.into())
}

#[test]
fn a_table_with_no_selection_highlights_nothing() {
    // The default n is 2000, so taking "the top n" would have marked every row
    // of any smaller table - a figure asserting that every gene is variable.
    // Without a `variable` column, a highlight list or an explicit n, there is
    // no selection to draw.
    let svg = render(stats_table(12), vec![]);
    assert_eq!(
        highlighted(&svg).len(),
        0,
        "genes were highlighted by default"
    );
    assert!(svg.contains("0 variable of 12 genes"));

    // An explicit n does ask for one.
    let svg = render(stats_table(12), vec![("n", Value::Int(3))]);
    assert_eq!(highlighted(&svg).len(), 3);
}

#[test]
fn a_table_of_statistics_can_be_highlighted_explicitly() {
    // The path for a selection made by something other than
    // highly_variable_genes.
    let rows: Vec<Value> = ["A", "B", "C", "D"]
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let mut record = HashMap::new();
            record.insert("gene".to_string(), Value::Str(name.to_string()));
            record.insert("mean".to_string(), Value::Float(1.0 + i as f64));
            record.insert("dispersion".to_string(), Value::Float(4.0 - i as f64));
            Value::Record(record.into())
        })
        .collect();
    let svg = render(
        Value::List(rows.into()),
        vec![(
            "highlight",
            Value::List(vec![Value::Str("D".to_string())].into()),
        )],
    );
    assert_eq!(
        highlighted(&svg).len(),
        1,
        "explicit highlight list was ignored"
    );
    assert!(svg.contains(">D<"), "the highlighted gene was not labelled");
}

#[test]
fn the_axis_is_logarithmic_in_the_mean() {
    // On a linear axis a 1000-fold range collapses every gene onto the left
    // edge. Two genes a decade apart must sit the same distance apart as two
    // other genes a decade apart.
    let make = |means: [f64; 3]| {
        let rows: Vec<Value> = means
            .iter()
            .enumerate()
            .map(|(i, mean)| {
                let mut record = HashMap::new();
                record.insert("gene".to_string(), Value::Str(format!("g{i}")));
                record.insert("mean".to_string(), Value::Float(*mean));
                record.insert("dispersion".to_string(), Value::Float(1.0));
                Value::Record(record.into())
            })
            .collect();
        let svg = render(Value::List(rows.into()), vec![]);
        circles(&svg).iter().map(|(x, _, _)| *x).collect::<Vec<_>>()
    };
    let xs = make([1.0, 10.0, 100.0]);
    assert_eq!(xs.len(), 3);
    let first = xs[1] - xs[0];
    let second = xs[2] - xs[1];
    assert!(
        (first - second).abs() < 1.0,
        "decades are not evenly spaced: {first:.1} then {second:.1}"
    );
}

#[test]
fn the_normalised_view_plots_the_value_that_is_ranked() {
    let raw = render(
        matrix(120),
        vec![("genes", gene_names(N_GENES)), ("n", Value::Int(6))],
    );
    let normalised = render(
        matrix(120),
        vec![
            ("genes", gene_names(N_GENES)),
            ("n", Value::Int(6)),
            ("y", Value::Str("normalised".to_string())),
        ],
    );
    assert!(raw.contains("dispersion (variance / mean)"));
    assert!(normalised.contains("standardised dispersion"));
    assert_ne!(raw, normalised, "the y option changed nothing");
}

#[test]
fn variable_feature_plot_rejects_input_it_cannot_read() {
    assert!(call_bio_plots_builtin("variable_feature_plot", vec![Value::Int(1)]).is_err());
}
