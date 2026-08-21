//! Tests for `gene_activity` / `peak_matrix` fragment counting.
//!
//! Expected counts here are computed by hand from small, explicitly-placed
//! fragments so the arithmetic can be checked by reading the test.

use std::collections::HashMap;
use std::io::Write;

use bl_core::sparse_matrix::SparseMatrix;
use bl_core::value::{Table, Value};
use bl_runtime::atac::call_atac_builtin;

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

fn genes_table(rows: Vec<(&str, &str, i64, i64, &str)>) -> Value {
    let cols = vec![
        "gene_name".to_string(),
        "chrom".to_string(),
        "start".to_string(),
        "end".to_string(),
        "strand".to_string(),
    ];
    let rows = rows
        .into_iter()
        .map(|(name, chrom, start, end, strand)| {
            vec![
                s(name),
                s(chrom),
                Value::Int(start),
                Value::Int(end),
                s(strand),
            ]
        })
        .collect();
    Value::Table(Table::new(cols, rows))
}

/// (chrom, start, end, barcode, count)
type Frag<'a> = (&'a str, i64, i64, &'a str, i64);

fn frags_table(rows: &[Frag]) -> Value {
    let cols = vec![
        "chrom".to_string(),
        "start".to_string(),
        "end".to_string(),
        "barcode".to_string(),
        "count".to_string(),
    ];
    let rows = rows
        .iter()
        .map(|(c, st, en, bc, n)| {
            vec![
                s(c),
                Value::Int(*st),
                Value::Int(*en),
                s(bc),
                Value::Int(*n),
            ]
        })
        .collect();
    Value::Table(Table::new(cols, rows))
}

fn frags_file(rows: &[Frag], gz: bool) -> tempfile::TempPath {
    let mut body = String::from("# fragments file\n");
    for (c, st, en, bc, n) in rows {
        body.push_str(&format!("{c}\t{st}\t{en}\t{bc}\t{n}\n"));
    }
    let suffix = if gz { ".tsv.gz" } else { ".tsv" };
    let mut f = tempfile::Builder::new()
        .suffix(suffix)
        .tempfile()
        .expect("tempfile");
    if gz {
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(body.as_bytes()).unwrap();
        f.write_all(&enc.finish().unwrap()).unwrap();
    } else {
        f.write_all(body.as_bytes()).unwrap();
    }
    f.flush().unwrap();
    f.into_temp_path()
}

struct Obj {
    genes: Vec<String>,
    barcodes: Vec<String>,
    matrix: Vec<Vec<f64>>,
    extra: HashMap<String, i64>,
}

fn unpack(v: Value) -> Obj {
    let Value::Record(rec) = v else {
        panic!("not a record")
    };
    let list = |k: &str| -> Vec<String> {
        match rec.get(k) {
            Some(Value::List(l)) => l
                .iter()
                .map(|x| match x {
                    Value::Str(s) => s.to_string(),
                    other => panic!("{k} holds {other:?}"),
                })
                .collect(),
            other => panic!("{k}: {other:?}"),
        }
    };
    let matrix = match rec.get("matrix") {
        Some(Value::List(rows)) => rows
            .iter()
            .map(|r| match r {
                Value::List(cs) => cs
                    .iter()
                    .map(|c| match c {
                        Value::Float(f) => *f,
                        Value::Int(i) => *i as f64,
                        other => panic!("cell {other:?}"),
                    })
                    .collect(),
                other => panic!("row {other:?}"),
            })
            .collect(),
        other => panic!("matrix: {other:?}"),
    };
    let mut extra = HashMap::new();
    for k in [
        "n_cells",
        "n_genes",
        "n_fragments",
        "n_fragments_in_features",
    ] {
        if let Some(Value::Int(i)) = rec.get(k) {
            extra.insert(k.to_string(), *i);
        }
    }
    Obj {
        genes: list("genes"),
        barcodes: list("barcodes"),
        matrix,
        extra,
    }
}

/// Genes chosen so the extension is strand-aware and one region clamps at 0.
fn demo_genes() -> Value {
    genes_table(vec![
        ("GENEA", "chr1", 1000, 2000, "+"), // upstream 500 -> [500, 2000)
        ("GENEB", "chr1", 5000, 6000, "-"), // upstream is 3' -> [5000, 6500)
        ("GENEC", "chr2", 100, 200, "+"),   // 100-500 < 0 -> clamps to [0, 200)
    ])
}

fn demo_frags() -> Vec<Frag<'static>> {
    vec![
        ("chr1", 600, 700, "AAA", 1),   // GENEA
        ("chr1", 1500, 1600, "AAA", 2), // GENEA, count 2
        ("chr1", 2500, 2600, "AAA", 1), // past GENEA's end -> nothing
        ("chr1", 6400, 6600, "BBB", 1), // GENEB (overlaps 6400..6500)
        ("chr2", 150, 250, "BBB", 3),   // GENEC (overlaps 150..200)
        ("chr3", 100, 200, "CCC", 1),   // chromosome not in the index
        ("chr1", 490, 500, "AAA", 1),   // abuts GENEA at 500, half-open -> no hit
    ]
}

#[test]
fn gene_activity_counts_match_hand_computation() {
    let out = call_atac_builtin(
        "gene_activity",
        vec![
            frags_table(&demo_frags()),
            demo_genes(),
            Value::Int(500),
            Value::Int(0),
        ],
    )
    .expect("gene_activity");
    let o = unpack(out);

    assert_eq!(o.genes, vec!["GENEA", "GENEB", "GENEC"]);
    // CCC's only fragment overlaps nothing, so it never becomes a cell
    assert_eq!(o.barcodes, vec!["AAA", "BBB"]);
    assert_eq!(o.matrix, vec![vec![3.0, 0.0, 0.0], vec![0.0, 1.0, 3.0]]);
    assert_eq!(o.extra["n_cells"], 2);
    assert_eq!(o.extra["n_genes"], 3);
    assert_eq!(o.extra["n_fragments"], 7);
    assert_eq!(o.extra["n_fragments_in_features"], 4);
}

#[test]
fn strand_direction_actually_matters() {
    // GENEB is on the minus strand, so extending 2 kb upstream must reach past
    // its END (6000), not before its START (5000).
    let genes = genes_table(vec![("GENEB", "chr1", 5000, 6000, "-")]);
    let frags = vec![
        ("chr1", 6500, 6600, "AAA", 1), // upstream of a minus-strand TSS
        ("chr1", 3500, 3600, "AAA", 1), // upstream only if strand were ignored
    ];
    let o = unpack(
        call_atac_builtin(
            "gene_activity",
            vec![frags_table(&frags), genes, Value::Int(2000), Value::Int(0)],
        )
        .unwrap(),
    );
    assert_eq!(
        o.matrix,
        vec![vec![1.0]],
        "minus-strand extension went the wrong way"
    );
}

#[test]
fn a_fragment_spanning_index_buckets_counts_once() {
    // BIN_SIZE is 16384; this gene spans several buckets and the fragment
    // straddles a bucket boundary, so a missing dedupe would double-count.
    let genes = genes_table(vec![("LONG", "chr4", 10_000, 60_000, "+")]);
    let frags = vec![("chr4", 16_000, 17_000, "AAA", 1)];
    let o = unpack(
        call_atac_builtin(
            "gene_activity",
            vec![frags_table(&frags), genes, Value::Int(0), Value::Int(0)],
        )
        .unwrap(),
    );
    assert_eq!(o.matrix, vec![vec![1.0]], "fragment counted more than once");
}

#[test]
fn rows_sharing_a_gene_name_collapse_into_one_column() {
    // Two annotation rows for one gene (e.g. separate transcripts) must give a
    // single column, and a fragment hitting both must not be counted twice.
    let genes = genes_table(vec![
        ("SAME", "chr1", 1000, 2000, "+"),
        ("SAME", "chr1", 1500, 2500, "+"),
    ]);
    let frags = vec![
        ("chr1", 1800, 1900, "AAA", 1), // inside both rows
        ("chr1", 2400, 2450, "AAA", 1), // inside the second row only
    ];
    let o = unpack(
        call_atac_builtin(
            "gene_activity",
            vec![frags_table(&frags), genes, Value::Int(0), Value::Int(0)],
        )
        .unwrap(),
    );
    assert_eq!(o.genes, vec!["SAME"]);
    // first fragment hits two regions sharing the column (2), second hits one
    assert_eq!(o.matrix, vec![vec![3.0]]);
}

#[test]
fn barcode_whitelist_sets_order_and_keeps_empty_cells() {
    let o = unpack(
        call_atac_builtin(
            "gene_activity",
            vec![
                frags_table(&demo_frags()),
                demo_genes(),
                Value::Int(500),
                Value::Int(0),
                Value::List(vec![s("BBB"), s("ZZZ"), s("AAA")].into()),
            ],
        )
        .unwrap(),
    );
    assert_eq!(o.barcodes, vec!["BBB", "ZZZ", "AAA"]);
    assert_eq!(
        o.matrix,
        vec![
            vec![0.0, 1.0, 3.0], // BBB
            vec![0.0, 0.0, 0.0], // ZZZ never observed, still present as a row
            vec![3.0, 0.0, 0.0], // AAA
        ]
    );
}

#[test]
fn reads_fragments_from_plain_and_gzipped_files() {
    let expect = vec![vec![3.0, 0.0, 0.0], vec![0.0, 1.0, 3.0]];
    for gz in [false, true] {
        let path = frags_file(&demo_frags(), gz);
        let o = unpack(
            call_atac_builtin(
                "gene_activity",
                vec![
                    s(path.to_str().unwrap()),
                    demo_genes(),
                    Value::Int(500),
                    Value::Int(0),
                ],
            )
            .unwrap_or_else(|e| panic!("gz={gz}: {e}")),
        );
        assert_eq!(o.matrix, expect, "gz={gz}");
        assert_eq!(
            o.extra["n_fragments"], 7,
            "gz={gz}: comment line miscounted"
        );
    }
}

#[test]
fn peak_matrix_names_and_counts_peaks() {
    let cols = vec!["chrom".to_string(), "start".to_string(), "end".to_string()];
    let peaks = Value::Table(Table::new(
        cols,
        vec![
            vec![s("chr1"), Value::Int(500), Value::Int(1500)],
            vec![s("chr1"), Value::Int(6000), Value::Int(7000)],
        ],
    ));
    let o =
        unpack(call_atac_builtin("peak_matrix", vec![frags_table(&demo_frags()), peaks]).unwrap());
    assert_eq!(o.genes, vec!["chr1:500-1500", "chr1:6000-7000"]);
    assert_eq!(o.barcodes, vec!["AAA", "BBB"]);
    // AAA: 600-700 in peak 1. BBB: 6400-6600 in peak 2 (count 1).
    assert_eq!(o.matrix, vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
}

#[test]
fn sparse_peak_matrix_keeps_zeros_implicit_and_sc_object_metadata() {
    let peaks = Value::Table(Table::new(
        vec!["chrom".to_string(), "start".to_string(), "end".to_string()],
        vec![
            vec![s("chr1"), Value::Int(500), Value::Int(1500)],
            vec![s("chr1"), Value::Int(6000), Value::Int(7000)],
        ],
    ));
    let Value::Record(object) = call_atac_builtin(
        "peak_matrix_sparse",
        vec![frags_table(&demo_frags()), peaks],
    )
    .unwrap() else {
        panic!("not an object")
    };
    let Some(Value::SparseMatrix(matrix)) = object.get("matrix") else {
        panic!("matrix is not sparse")
    };
    assert_eq!((matrix.nrow, matrix.ncol, matrix.nnz()), (2, 2, 2));
    assert_eq!(matrix.to_dense(), vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
    assert_eq!(
        matrix.row_names.as_ref().unwrap(),
        &vec!["AAA".to_string(), "BBB".to_string()]
    );
    assert_eq!(
        matrix.col_names.as_ref().unwrap(),
        &vec!["chr1:500-1500".to_string(), "chr1:6000-7000".to_string()]
    );
    assert!(matches!(object.get("is_sparse"), Some(Value::Bool(true))));
    assert!(matches!(object.get("obs"), Some(Value::Table(_))));
    assert!(matches!(object.get("var"), Some(Value::Table(_))));
    assert!(matches!(
        object.get("layers"),
        Some(Value::Record(layers)) if matches!(layers.get("counts"), Some(Value::SparseMatrix(_)))
    ));
}

#[test]
fn sparse_peak_matrix_crosses_dense_limit_without_allocating_zeroes() {
    // 10,001 x 10,000 = 100,010,000 logical entries, just beyond the dense
    // guard. With no fragments, CSR stores no numeric values at all.
    let peaks = Value::Table(Table::new(
        vec!["chrom".to_string(), "start".to_string(), "end".to_string()],
        (0..10_000)
            .map(|index| {
                vec![
                    s("chr1"),
                    Value::Int(index * 20),
                    Value::Int(index * 20 + 10),
                ]
            })
            .collect(),
    ));
    let barcodes = Value::List(
        (0..10_001)
            .map(|index| s(&format!("cell-{index}")))
            .collect::<Vec<_>>()
            .into(),
    );
    let Value::Record(object) = call_atac_builtin(
        "peak_matrix_sparse",
        vec![frags_table(&[]), peaks, barcodes],
    )
    .unwrap() else {
        panic!("not an object")
    };
    let Some(Value::SparseMatrix(matrix)) = object.get("matrix") else {
        panic!("matrix is not sparse")
    };
    assert_eq!(
        (matrix.nrow, matrix.ncol, matrix.nnz()),
        (10_001, 10_000, 0)
    );
}

#[test]
fn fragment_qc_matches_signac_nucleosome_boundaries_and_ratio() {
    let fragments = vec![
        ("chr1", 0, 146, "A", 9), // NFR; support count is ignored
        ("chr1", 0, 147, "A", 1), // mono lower boundary
        ("chr1", 0, 294, "A", 1), // mono upper boundary
        ("chr1", 0, 295, "A", 1), // neither
        ("chr1", 0, 100, "B", 1), // NFR
        ("chr1", 0, 300, "B", 1), // neither
    ];
    let Value::Table(metrics) = call_atac_builtin(
        "atac_fragment_qc",
        vec![
            frags_table(&fragments),
            Value::List(vec![s("A"), s("B"), s("C")].into()),
            Value::Nil,
        ],
    )
    .unwrap() else {
        panic!("fragment QC did not return a table")
    };
    assert_eq!(metrics.rows.len(), 3);
    assert_eq!(metrics.rows[0][1], Value::Int(4));
    assert_eq!(metrics.rows[0][2], Value::Int(1));
    assert_eq!(metrics.rows[0][3], Value::Int(2));
    assert_eq!(metrics.rows[0][4], Value::Float(2.0));
    assert_eq!(metrics.rows[1][4], Value::Float(0.0));
    assert_eq!(metrics.rows[1][5], Value::Float(0.5));
    assert_eq!(metrics.rows[2][4], Value::Nil);
}

#[test]
fn fragment_qc_stops_at_the_requested_input_line_cap() {
    let fragments = vec![
        ("chr1", 0, 100, "A", 1),
        ("chr1", 0, 200, "A", 1),
        ("chr1", 0, 200, "A", 1),
    ];
    let Value::Table(metrics) = call_atac_builtin(
        "atac_fragment_qc",
        vec![frags_table(&fragments), Value::Nil, Value::Int(2)],
    )
    .unwrap() else {
        panic!("fragment QC did not return a table")
    };
    assert_eq!(metrics.rows[0][1], Value::Int(2));
    assert_eq!(metrics.rows[0][4], Value::Float(1.0));
}

#[test]
fn tss_qc_matches_signac_slow_center_flanks_and_zero_flank_replacement() {
    let fragments = vec![
        ("chr1", 500, 1001, "A", 1), // recorded end is in the + strand center
        ("chr1", 0, 2000, "A", 1),   // recorded end is in the right flank
        ("chr1", 500, 1001, "B", 1), // center only; flank uses population mean
        // The second insertion site is the recorded end coordinate. This cut
        // lies on the first base of the downstream flank; using end - 1 would
        // incorrectly omit it.
        ("chr1", 102, 1903, "A", 1),
    ];
    let tss = Value::Table(Table::new(
        vec!["chrom".into(), "position".into()],
        vec![vec![s("chr1"), Value::Int(1000)]],
    ));
    let Value::Table(metrics) = call_atac_builtin(
        "atac_tss_qc",
        vec![
            frags_table(&fragments),
            tss,
            Value::List(vec![s("A"), s("B")].into()),
            Value::Int(1000),
        ],
    )
    .unwrap() else {
        panic!("TSS QC did not return a table")
    };
    assert_eq!(metrics.rows[0][1], Value::Int(1));
    assert_eq!(metrics.rows[0][2], Value::Int(2));
    let a = metrics.rows[0][3].as_float().unwrap();
    let b = metrics.rows[1][3].as_float().unwrap();
    assert!((a - (1.0 / 0.01 / 1001.0)).abs() < 1e-12);
    // Mean flank accessibility across A and B is 0.005; Signac substitutes
    // that population mean for B's zero denominator.
    assert!((b - (1.0 / 0.005 / 1001.0)).abs() < 1e-12);
}

#[test]
fn tss_qc_reverses_the_signac_slow_center_window_on_minus_strands() {
    let fragments = vec![("chr1", 503, 1503, "A", 1), ("chr1", 2, 1903, "A", 1)];
    let tss = Value::Table(Table::new(
        vec!["chrom".into(), "tss".into(), "strand".into()],
        vec![vec![s("chr1"), Value::Int(1000), s("-")]],
    ));
    let Value::Table(metrics) = call_atac_builtin(
        "atac_tss_qc",
        vec![
            frags_table(&fragments),
            tss,
            Value::List(vec![s("A")].into()),
            Value::Int(1000),
        ],
    )
    .unwrap() else {
        panic!("TSS QC did not return a table")
    };
    // 1503 is included only by the reversed minus-strand center window.
    assert_eq!(metrics.rows[0][1], Value::Int(2));
    assert_eq!(metrics.rows[0][2], Value::Int(2));
}

#[test]
fn tfidf_method_one_matches_hand_calculation_and_stays_sparse() {
    let counts =
        Value::SparseMatrix(SparseMatrix::from_dense(&[vec![2.0, 0.0], vec![1.0, 3.0]]).into());
    let Value::SparseMatrix(actual) =
        call_atac_builtin("atac_tfidf", vec![counts, Value::Float(10_000.0)]).unwrap()
    else {
        panic!("TF-IDF did not stay sparse")
    };
    let expected = [
        ((2.0 / 2.0) * (2.0 / 3.0) * 10_000.0f64).ln_1p(),
        ((1.0 / 4.0) * (2.0 / 3.0) * 10_000.0f64).ln_1p(),
        ((3.0 / 4.0) * (2.0 / 3.0) * 10_000.0f64).ln_1p(),
    ];
    assert_eq!(actual.indices, vec![0, 0, 1]);
    for (observed, expected) in actual.data.iter().zip(expected) {
        assert!(
            (observed - expected).abs() < 1e-12,
            "{observed} != {expected}"
        );
    }
}

#[test]
fn top_features_uses_total_counts_and_strict_cutoff() {
    // Feature 0 has total 11 but occurs in one cell; feature 1 has total 2 and
    // occurs in two cells. Signac's numeric cutoff is total counts, strictly >.
    let counts = Value::SparseMatrix(
        SparseMatrix::from_dense(&[vec![11.0, 0.0], vec![0.0, 1.0], vec![0.0, 1.0]]).into(),
    );
    let Value::List(indices) =
        call_atac_builtin("atac_top_features", vec![counts, Value::Int(2)]).unwrap()
    else {
        panic!("top features did not return a list")
    };
    assert_eq!(indices.as_ref(), &vec![Value::Int(0)]);
}

#[test]
fn peak_qc_computes_matrix_counts_and_whole_feature_blacklist_fraction() {
    let mut matrix = SparseMatrix::from_dense(&[vec![2.0, 3.0], vec![0.0, 4.0]]);
    matrix.row_names = Some(vec!["A".into(), "B".into()]);
    let peaks = Value::Table(Table::new(
        vec!["chrom".into(), "start".into(), "end".into()],
        vec![
            vec![s("chr1"), Value::Int(100), Value::Int(200)],
            vec![s("chr1"), Value::Int(300), Value::Int(400)],
        ],
    ));
    let blacklist = Value::Table(Table::new(
        vec!["chrom".into(), "start".into(), "end".into()],
        vec![vec![s("chr1"), Value::Int(150), Value::Int(151)]],
    ));
    let Value::Table(metrics) = call_atac_builtin(
        "atac_peak_qc",
        vec![Value::SparseMatrix(matrix.into()), peaks, blacklist],
    )
    .unwrap() else {
        panic!("peak QC did not return a table")
    };
    assert_eq!(metrics.rows[0][0], s("A"));
    assert_eq!(metrics.rows[0][1], Value::Float(5.0));
    assert_eq!(metrics.rows[0][2], Value::Int(2));
    assert_eq!(metrics.rows[0][3], Value::Float(2.0));
    assert_eq!(metrics.rows[0][4], Value::Float(0.4));
    assert_eq!(metrics.rows[1][4], Value::Float(0.0));
}

#[test]
fn frip_uses_cell_ranger_metadata_columns_and_returns_percent() {
    let Value::List(values) = call_atac_builtin(
        "atac_frip",
        vec![
            Value::List(vec![Value::Int(60), Value::Int(30), Value::Int(1)].into()),
            Value::List(vec![Value::Int(100), Value::Int(60), Value::Int(0)].into()),
        ],
    )
    .unwrap() else {
        panic!("FRiP did not return a list")
    };
    assert_eq!(
        values.as_ref(),
        &vec![Value::Float(60.0), Value::Float(50.0), Value::Nil]
    );
}

#[test]
fn ngs101_qc_uses_the_articles_inclusive_failure_boundaries() {
    let numbers = |values: &[f64]| {
        Value::List(
            values
                .iter()
                .copied()
                .map(Value::Float)
                .collect::<Vec<_>>()
                .into(),
        )
    };
    let Value::Record(result) = call_atac_builtin(
        "atac_ngs101_qc",
        vec![
            numbers(&[5000.0, 3000.0, 100000.0, 5000.0, 5000.0, 5000.0, 5000.0]),
            numbers(&[3.0, 3.0, 3.0, 2.0, 3.0, 3.0, 3.0]),
            numbers(&[20.0, 20.0, 20.0, 20.0, 15.0, 20.0, 20.0]),
            numbers(&[1.0, 1.0, 1.0, 1.0, 1.0, 4.0, 1.0]),
            numbers(&[0.01, 0.01, 0.01, 0.01, 0.01, 0.01, 0.05]),
        ],
    )
    .unwrap() else {
        panic!("NGS101 QC did not return a record")
    };
    assert_eq!(result.get("n_before"), Some(&Value::Int(7)));
    assert_eq!(result.get("n_after"), Some(&Value::Int(1)));
    for name in [
        "failed_low_depth",
        "failed_high_depth",
        "failed_low_tss",
        "failed_low_frip",
        "failed_high_nucleosome",
        "failed_high_blacklist",
    ] {
        assert_eq!(result.get(name), Some(&Value::Int(1)), "wrong {name}");
    }
}

#[test]
fn detected_feature_filter_counts_cells_not_total_fragments() {
    let matrix = Value::SparseMatrix(
        SparseMatrix::from_dense(&[vec![10.0, 1.0], vec![0.0, 1.0], vec![0.0, 0.0]]).into(),
    );
    let Value::List(features) =
        call_atac_builtin("atac_detected_features", vec![matrix, Value::Int(2)]).unwrap()
    else {
        panic!("detected feature filter did not return a list")
    };
    assert_eq!(features.as_ref(), &vec![Value::Int(1)]);
}

#[test]
fn depth_correlates_total_counts_with_embedding_components() {
    let counts = Value::SparseMatrix(
        SparseMatrix::from_dense(&[vec![2.0, 0.0], vec![1.0, 3.0], vec![0.0, 6.0]]).into(),
    );
    let embedding = Value::Matrix(
        bl_core::matrix::Matrix::new(vec![1.0, 3.0, 2.0, 1.0, 3.0, 2.0], 3, 2)
            .unwrap()
            .into(),
    );
    let Value::Record(result) =
        call_atac_builtin("atac_depth_cor", vec![counts, embedding, Value::Int(2)]).unwrap()
    else {
        panic!("depth correlation did not return a record")
    };
    let Some(Value::List(correlations)) = result.get("correlations") else {
        panic!("missing correlations")
    };
    let first = correlations[0].as_float().unwrap();
    assert!((first - 1.0).abs() < 1e-12);
}

#[test]
fn peak_filter_uses_strict_widths_standard_chromosomes_and_whole_peak_blacklist() {
    let peaks = Value::Table(Table::new(
        vec!["chrom".to_string(), "start".to_string(), "end".to_string()],
        vec![
            vec![s("chr1"), Value::Int(0), Value::Int(20)], // width == min
            vec![s("chr1"), Value::Int(100), Value::Int(121)], // keep
            vec![s("chr2"), Value::Int(200), Value::Int(250)], // blacklisted
            vec![s("chrUn"), Value::Int(0), Value::Int(100)], // non-standard
            vec![s("chr3"), Value::Int(0), Value::Int(10_000)], // width == max
        ],
    ));
    let blacklist = Value::Table(Table::new(
        vec!["chrom".to_string(), "start".to_string(), "end".to_string()],
        vec![vec![s("chr2"), Value::Int(225), Value::Int(226)]],
    ));
    let Value::Table(filtered) = call_atac_builtin(
        "atac_filter_peaks",
        vec![
            peaks,
            Value::Int(20),
            Value::Int(10_000),
            blacklist,
            Value::Bool(true),
        ],
    )
    .unwrap() else {
        panic!("filter did not return a table")
    };
    assert_eq!(filtered.rows.len(), 1);
    assert_eq!(filtered.rows[0][0], s("chr1"));
    assert_eq!(filtered.rows[0][1], Value::Int(100));
    assert_eq!(filtered.rows[0][2], Value::Int(121));
}

#[test]
fn batch_mixing_matches_the_articles_other_sample_fraction() {
    let embedding = Value::Matrix(
        bl_core::matrix::Matrix::new(vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0], 4, 2)
            .unwrap()
            .into(),
    );
    let batches = Value::List(vec![s("A"), s("A"), s("B"), s("B")].into());
    let Value::Record(result) =
        call_atac_builtin("atac_batch_mixing", vec![embedding, batches, Value::Int(3)]).unwrap()
    else {
        panic!("mixing score did not return a record")
    };
    let mean = result.get("mixing_score").unwrap().as_float().unwrap();
    let expected = result
        .get("expected_random_mixing")
        .unwrap()
        .as_float()
        .unwrap();
    assert!((mean - 0.5).abs() < 1e-12);
    assert!((expected - 0.5).abs() < 1e-12);
    assert_eq!(result.get("n_compared_neighbours"), Some(&Value::Int(2)));
}

#[test]
fn rejects_bad_inputs() {
    let bad_cols = Value::Table(Table::new(
        vec!["chromosome".to_string()],
        vec![vec![s("chr1")]],
    ));
    assert!(
        call_atac_builtin("gene_activity", vec![frags_table(&demo_frags()), bad_cols]).is_err()
    );

    assert!(call_atac_builtin("gene_activity", vec![Value::Int(3), demo_genes()]).is_err());

    let missing = call_atac_builtin(
        "gene_activity",
        vec![s("no/such/fragments.tsv"), demo_genes()],
    );
    assert!(missing.is_err());

    let sparse = Value::SparseMatrix(SparseMatrix::from_dense(&[vec![1.0]]).into());
    assert!(call_atac_builtin("atac_top_features", vec![sparse.clone(), s("twenty")]).is_err());
    assert!(call_atac_builtin(
        "atac_depth_cor",
        vec![
            sparse,
            Value::Matrix(
                bl_core::matrix::Matrix::new(vec![1.0], 1, 1)
                    .unwrap()
                    .into(),
            ),
            s("ten"),
        ],
    )
    .is_err());
}
