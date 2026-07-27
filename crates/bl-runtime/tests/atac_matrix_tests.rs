//! Tests for `gene_activity` / `peak_matrix` fragment counting.
//!
//! Expected counts here are computed by hand from small, explicitly-placed
//! fragments so the arithmetic can be checked by reading the test.

use std::collections::HashMap;
use std::io::Write;

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
            vec![s(name), s(chrom), Value::Int(start), Value::Int(end), s(strand)]
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
            vec![s(c), Value::Int(*st), Value::Int(*en), s(bc), Value::Int(*n)]
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
        let mut enc =
            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
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
    for k in ["n_cells", "n_genes", "n_fragments", "n_fragments_in_features"] {
        if let Some(Value::Int(i)) = rec.get(k) {
            extra.insert(k.to_string(), *i);
        }
    }
    Obj { genes: list("genes"), barcodes: list("barcodes"), matrix, extra }
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
        vec![frags_table(&demo_frags()), demo_genes(), Value::Int(500), Value::Int(0)],
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
    assert_eq!(o.matrix, vec![vec![1.0]], "minus-strand extension went the wrong way");
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
        assert_eq!(o.extra["n_fragments"], 7, "gz={gz}: comment line miscounted");
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
    let o = unpack(
        call_atac_builtin("peak_matrix", vec![frags_table(&demo_frags()), peaks]).unwrap(),
    );
    assert_eq!(o.genes, vec!["chr1:500-1500", "chr1:6000-7000"]);
    assert_eq!(o.barcodes, vec!["AAA", "BBB"]);
    // AAA: 600-700 in peak 1. BBB: 6400-6600 in peak 2 (count 1).
    assert_eq!(o.matrix, vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
}

#[test]
fn rejects_bad_inputs() {
    let bad_cols = Value::Table(Table::new(
        vec!["chromosome".to_string()],
        vec![vec![s("chr1")]],
    ));
    assert!(call_atac_builtin(
        "gene_activity",
        vec![frags_table(&demo_frags()), bad_cols]
    )
    .is_err());

    assert!(call_atac_builtin("gene_activity", vec![Value::Int(3), demo_genes()]).is_err());

    let missing = call_atac_builtin(
        "gene_activity",
        vec![s("no/such/fragments.tsv"), demo_genes()],
    );
    assert!(missing.is_err());
}
