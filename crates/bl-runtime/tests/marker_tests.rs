//! find_all_markers and dot_plot, on data whose answer is known.
//!
//! These two are the step that turns numbered clusters into cell types, so the
//! failure that matters is not a crash - it is a table that looks entirely
//! reasonable and names the wrong genes. Every assertion here is therefore
//! about which genes come out and in what order, not about shapes and counts.

use bl_core::value::Value;
use bl_runtime::bio_plots::call_bio_plots_builtin;
use bl_runtime::singlecell::call_singlecell_builtin;
use std::collections::HashMap;

const PER_CLUSTER: usize = 40;
const CLUSTERS: usize = 3;
const N_CELLS: usize = PER_CLUSTER * CLUSTERS;

/// Gene 0/1/2 mark clusters 0/1/2. Gene 3 is housekeeping - everywhere, equally.
/// Genes 4 and 5 both belong to cluster 0 but differ in how: 4 blazes in a
/// quarter of its cells, 5 is modest across all of them. They exist to separate
/// the dot plot's two encodings, which a mean alone cannot.
fn counts() -> Value {
    let mut rows = Vec::with_capacity(N_CELLS);
    for cell in 0..N_CELLS {
        let cluster = cell / PER_CLUSTER;
        let within = cell % PER_CLUSTER;
        let mut row = vec![0.0_f64; 6];
        for marker in 0..CLUSTERS {
            if cluster == marker {
                // A little variation, so ranks are not all ties.
                row[marker] = 3.0 + (within % 5) as f64 * 0.05;
            }
        }
        row[3] = 1.5 + (within % 4) as f64 * 0.02;
        if cluster == 0 {
            if within < PER_CLUSTER / 4 {
                row[4] = 4.0;
            }
            row[5] = 0.45 + (within % 3) as f64 * 0.01;
        }
        rows.push(Value::List(
            row.into_iter().map(Value::Float).collect::<Vec<_>>().into(),
        ));
    }
    Value::List(rows.into())
}

fn clusters() -> Value {
    Value::List(
        (0..N_CELLS)
            .map(|cell| Value::Int((cell / PER_CLUSTER) as i64))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn gene_names() -> Value {
    Value::List(
        ["M0", "M1", "M2", "HOUSE", "RARE", "WEAK"]
            .iter()
            .map(|n| Value::Str(n.to_string()))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn options(pairs: Vec<(&str, Value)>) -> Value {
    let mut map = HashMap::new();
    map.insert("genes".to_string(), gene_names());
    for (key, value) in pairs {
        map.insert(key.to_string(), value);
    }
    Value::Record(map.into())
}

fn markers(opts: Vec<(&str, Value)>) -> Vec<HashMap<String, Value>> {
    match call_singlecell_builtin(
        "find_all_markers",
        vec![counts(), clusters(), options(opts)],
    ) {
        Ok(Value::List(rows)) => rows
            .iter()
            .filter_map(|row| match row {
                Value::Record(map) => Some(map.as_ref().clone()),
                _ => None,
            })
            .collect(),
        other => panic!("find_all_markers returned {other:?}"),
    }
}

fn text(row: &HashMap<String, Value>, key: &str) -> String {
    row.get(key).map(|v| format!("{v}")).unwrap_or_default()
}

fn number(row: &HashMap<String, Value>, key: &str) -> f64 {
    row.get(key).and_then(|v| v.as_float()).unwrap_or(f64::NAN)
}

#[test]
fn each_cluster_gets_its_own_marker_first() {
    // The claim the whole builtin exists to support: cluster 1 is whatever M1
    // says it is.
    let rows = markers(vec![("only_pos", Value::Bool(true))]);
    for cluster in 0..CLUSTERS {
        let top = rows
            .iter()
            .find(|row| text(row, "cluster") == cluster.to_string())
            .unwrap_or_else(|| panic!("no markers reported for cluster {cluster}"));
        assert_eq!(
            text(top, "gene"),
            format!("M{cluster}"),
            "cluster {cluster} was named by the wrong gene"
        );
    }
}

#[test]
fn a_gene_expressed_everywhere_is_never_a_marker() {
    // HOUSE is present in every cell of every cluster at the same level. A test
    // that reported it would be finding structure in a constant.
    let rows = markers(vec![]);
    assert!(
        !rows.iter().any(|row| text(row, "gene") == "HOUSE"),
        "the housekeeping gene was called a marker"
    );
}

#[test]
fn detection_rates_are_the_fractions_they_claim_to_be() {
    // RARE is in exactly a quarter of cluster 0 and nowhere else - a number that
    // can be checked by hand, unlike the p-value.
    let rows = markers(vec![("only_pos", Value::Bool(true))]);
    let rare = rows
        .iter()
        .find(|row| text(row, "gene") == "RARE")
        .expect("RARE was not reported");
    assert_eq!(text(rare, "cluster"), "0");
    assert!(
        (number(rare, "pct_1") - 0.25).abs() < 1e-9,
        "pct_1 was {}",
        number(rare, "pct_1")
    );
    assert!(
        number(rare, "pct_2").abs() < 1e-9,
        "RARE is absent outside cluster 0 but pct_2 was {}",
        number(rare, "pct_2")
    );
}

#[test]
fn enrichment_is_positive_for_the_cluster_that_has_the_gene() {
    let rows = markers(vec![]);
    for row in &rows {
        let gene = text(row, "gene");
        if let Some(rest) = gene.strip_prefix('M') {
            if rest.parse::<usize>().is_ok() {
                let own = text(row, "cluster") == rest;
                let fold = number(row, "avg_log2fc");
                assert_eq!(
                    fold > 0.0,
                    own,
                    "{gene} in cluster {} had log2FC {fold}",
                    text(row, "cluster")
                );
            }
        }
    }
}

#[test]
fn only_pos_drops_the_depleted_genes() {
    let all = markers(vec![]);
    let up = markers(vec![("only_pos", Value::Bool(true))]);
    assert!(
        all.iter().any(|row| number(row, "avg_log2fc") < 0.0),
        "the unfiltered table has no depleted genes to drop"
    );
    assert!(
        up.iter().all(|row| number(row, "avg_log2fc") > 0.0),
        "only_pos kept a depleted gene"
    );
    assert!(up.len() < all.len());
}

#[test]
fn correction_covers_the_whole_table_not_each_cluster() {
    // Adjusting within each cluster separately would understate the error rate
    // by however many clusters there are, and is an easy thing to get wrong.
    let rows = markers(vec![]);
    assert!(!rows.is_empty());
    for row in &rows {
        assert!(
            number(row, "p_adj") >= number(row, "p_value") - 1e-12,
            "an adjusted p-value came out below its raw one"
        );
        assert!(number(row, "p_adj") <= 1.0 + 1e-12);
    }
    // The direct statement of it: adjusting the whole column at once has to
    // reproduce the column. Correcting per cluster - a family of 5 here rather
    // than 15 - gives smaller values and fails this.
    //
    // Comparing against the p_adjust builtin rather than recomputing the
    // arithmetic here also means the two can never drift apart.
    let raw: Vec<Value> = rows
        .iter()
        .map(|row| Value::Float(number(row, "p_value")))
        .collect();
    let expected = match bl_runtime::stats::call_stats_builtin(
        "p_adjust",
        vec![Value::List(raw.into()), Value::Str("BH".to_string())],
    ) {
        Ok(Value::List(values)) => values
            .iter()
            .filter_map(|v| v.as_float())
            .collect::<Vec<f64>>(),
        other => panic!("p_adjust returned {other:?}"),
    };
    assert_eq!(expected.len(), rows.len());
    for (row, want) in rows.iter().zip(&expected) {
        assert!(
            (number(row, "p_adj") - want).abs() < 1e-12,
            "{} in cluster {}: p_adj {} but BH over the whole table gives {want}",
            text(row, "gene"),
            text(row, "cluster"),
            number(row, "p_adj")
        );
    }
}

#[test]
fn labels_that_do_not_match_the_cells_are_refused() {
    let short = Value::List(vec![Value::Int(0), Value::Int(1)].into());
    assert!(call_singlecell_builtin("find_all_markers", vec![counts(), short]).is_err());
}

// ── dot_plot ────────────────────────────────────────────────────────

fn dot_svg(opts: Vec<(&str, Value)>) -> String {
    match call_bio_plots_builtin("dot_plot", vec![counts(), clusters(), options(opts)]) {
        Ok(Value::Str(svg)) => svg,
        other => panic!("dot_plot returned {other:?}"),
    }
}

/// Every circle as (cx, cy, r, fill), excluding the legend's grey swatches.
fn dots(svg: &str) -> Vec<(f64, f64, f64, String)> {
    svg.split("<circle")
        .skip(1)
        .filter_map(|tag| {
            let tag = &tag[..tag.find("/>")?];
            let field = |name: &str| -> Option<f64> {
                let at = tag.find(&format!("{name}=\""))? + name.len() + 2;
                let rest = &tag[at..];
                rest[..rest.find('"')?].parse().ok()
            };
            let at = tag.find("fill=\"")? + 6;
            let rest = &tag[at..];
            let fill = rest[..rest.find('"')?].to_string();
            if fill == "#888888" {
                return None;
            }
            Some((field("cx")?, field("cy")?, field("r")?, fill))
        })
        .collect()
}

#[test]
fn size_and_colour_carry_different_facts() {
    // The reason a dot plot beats a heatmap of means. RARE is strong in a
    // quarter of cluster 0; WEAK is mild across all of it. A mean-only figure
    // would put them close together; here RARE must be the smaller dot.
    let svg = dot_svg(vec![(
        "features",
        Value::List(
            vec![
                Value::Str("RARE".to_string()),
                Value::Str("WEAK".to_string()),
            ]
            .into(),
        ),
    )]);
    let drawn = dots(&svg);
    // Row 0 is RARE, row 1 is WEAK; cluster 0 is the leftmost column.
    let rows: Vec<f64> = {
        let mut ys: Vec<f64> = drawn.iter().map(|d| d.1).collect();
        ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
        ys.dedup_by(|a, b| (*a - *b).abs() < 0.5);
        ys
    };
    assert_eq!(rows.len(), 2, "expected two gene rows");
    let leftmost = drawn.iter().map(|d| d.0).fold(f64::MAX, f64::min);
    let pick = |y: f64| {
        drawn
            .iter()
            .find(|d| (d.1 - y).abs() < 0.5 && (d.0 - leftmost).abs() < 0.5)
            .unwrap_or_else(|| panic!("no dot at row y={y}"))
    };
    let rare = pick(rows[0]);
    let weak = pick(rows[1]);
    assert!(
        rare.2 < weak.2,
        "RARE (25% of cells) drew radius {:.2}, WEAK (100%) drew {:.2}",
        rare.2,
        weak.2
    );
}

#[test]
fn a_marker_is_brightest_in_its_own_cluster() {
    let svg = dot_svg(vec![]);
    let drawn = dots(&svg);
    assert!(!drawn.is_empty());
    // M0's row: the dot over cluster 0 must be at the top of the colour scale,
    // which sequential_color renders distinctly from the low end.
    let top_row = drawn.iter().map(|d| d.1).fold(f64::MAX, f64::min);
    let mut in_row: Vec<&(f64, f64, f64, String)> = drawn
        .iter()
        .filter(|d| (d.1 - top_row).abs() < 0.5)
        .collect();
    in_row.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    assert_eq!(in_row.len(), 1, "M0 should only be detected in one cluster");
    let leftmost = drawn.iter().map(|d| d.0).fold(f64::MAX, f64::min);
    assert!(
        (in_row[0].0 - leftmost).abs() < 0.5,
        "M0's only dot is not over cluster 0"
    );
}

#[test]
fn features_choose_the_rows_and_their_order() {
    let svg = dot_svg(vec![(
        "features",
        Value::List(vec![Value::Str("M2".to_string()), Value::Str("M0".to_string())].into()),
    )]);
    let m2 = svg.find(">M2<").expect("M2 not labelled");
    let m0 = svg.find(">M0<").expect("M0 not labelled");
    assert!(m2 < m0, "the caller's feature order was not kept");
    assert!(!svg.contains(">M1<"), "an unrequested gene was drawn");
}

#[test]
fn dot_plot_refuses_labels_that_do_not_match() {
    let short = Value::List(vec![Value::Int(0)].into());
    assert!(call_bio_plots_builtin("dot_plot", vec![counts(), short]).is_err());
}
