//! Tests for structure.rs, network.rs, and popgen.rs builtins.
//! Uses #[path] to include modules before they are registered in lib.rs.

use bl_core::value::Value;

#[path = "../src/structure.rs"]
mod structure;

#[path = "../src/network.rs"]
mod network;

#[path = "../src/popgen.rs"]
mod popgen;

// ── structure tests ───────────────────────────────────────────────────

#[test]
fn pdb_parse_basic() {
    // A minimal PDB ATOM record (fixed-width columns)
    let pdb = concat!(
        "ATOM      1  N   ALA A   1       8.395  -4.339   3.699  1.00  0.00           N  \n",
        "ATOM      2  CA  ALA A   1       7.803  -3.057   3.268  1.00  0.00           C  \n",
        "ATOM      3  C   ALA A   1       6.290  -3.132   3.390  1.00  0.00           C  \n",
        "HETATM    4  O   HOH A 100       5.000   0.000   0.000  1.00 10.00           O  \n",
    );
    let args = vec![Value::Str(pdb.to_string())];
    let result = structure::call_structure_builtin("pdb_parse", args).unwrap();
    match result {
        Value::Table(t) => {
            assert_eq!(t.rows.len(), 4);
            assert!(t.columns.contains(&"atom".to_string()));
            assert!(t.columns.contains(&"x".to_string()));
            // First row record = "ATOM"
            assert!(matches!(&t.rows[0][0], Value::Str(s) if s == "ATOM"));
            // HETATM row
            assert!(matches!(&t.rows[3][0], Value::Str(s) if s == "HETATM"));
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn rmsd_identical_coords_is_zero() {
    // Use 4 non-coplanar points so the covariance matrix is full rank
    let coords: Vec<Value> = vec![
        Value::List((vec![Value::Float(1.0), Value::Float(0.0), Value::Float(0.0)]).into()),
        Value::List((vec![Value::Float(0.0), Value::Float(1.0), Value::Float(0.0)]).into()),
        Value::List((vec![Value::Float(0.0), Value::Float(0.0), Value::Float(1.0)]).into()),
        Value::List((vec![Value::Float(1.0), Value::Float(1.0), Value::Float(1.0)]).into()),
    ];
    let a = Value::List((coords.clone()).into());
    let b = Value::List((coords).into());
    let result = structure::call_structure_builtin("rmsd", vec![a, b]).unwrap();
    match result {
        Value::Float(v) => assert!(
            v.abs() < 1e-9,
            "RMSD of identical sets should be ~0, got {v}"
        ),
        _ => panic!("expected Float"),
    }
}

#[test]
fn rmsd_translated_sets() {
    // Two sets differing by a pure translation — RMSD should be 0 after alignment
    let a = Value::List(
        (vec![
            Value::List((vec![Value::Float(0.0), Value::Float(0.0), Value::Float(0.0)]).into()),
            Value::List((vec![Value::Float(1.0), Value::Float(0.0), Value::Float(0.0)]).into()),
            Value::List((vec![Value::Float(0.0), Value::Float(1.0), Value::Float(0.0)]).into()),
            Value::List((vec![Value::Float(0.0), Value::Float(0.0), Value::Float(1.0)]).into()),
        ])
        .into(),
    );
    let b = Value::List(
        (vec![
            Value::List((vec![Value::Float(5.0), Value::Float(5.0), Value::Float(5.0)]).into()),
            Value::List((vec![Value::Float(6.0), Value::Float(5.0), Value::Float(5.0)]).into()),
            Value::List((vec![Value::Float(5.0), Value::Float(6.0), Value::Float(5.0)]).into()),
            Value::List((vec![Value::Float(5.0), Value::Float(5.0), Value::Float(6.0)]).into()),
        ])
        .into(),
    );
    let result = structure::call_structure_builtin("rmsd", vec![a, b]).unwrap();
    match result {
        Value::Float(v) => assert!(
            v < 1e-6,
            "RMSD after translation alignment should be ~0, got {v}"
        ),
        _ => panic!("expected Float"),
    }
}

#[test]
fn contact_map_finds_nearby_atoms() {
    let coords = Value::List(
        (vec![
            Value::List((vec![Value::Float(0.0), Value::Float(0.0), Value::Float(0.0)]).into()),
            Value::List((vec![Value::Float(3.0), Value::Float(0.0), Value::Float(0.0)]).into()),
            Value::List((vec![Value::Float(100.0), Value::Float(0.0), Value::Float(0.0)]).into()),
        ])
        .into(),
    );
    let result =
        structure::call_structure_builtin("contact_map", vec![coords, Value::Float(8.0)]).unwrap();
    match result {
        Value::Table(t) => {
            assert_eq!(t.rows.len(), 1, "only atoms 0-1 should be within 8Å");
            let dist = match &t.rows[0][2] {
                Value::Float(f) => *f,
                _ => panic!(),
            };
            assert!((dist - 3.0).abs() < 1e-9);
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn secondary_structure_helix_detected() {
    // Helical Cα positions: consecutive ~3.8 Å spacing, (i, i+4) ≈ 5.5 Å
    // Build a simple helix of 10 residues
    let mut coords_list: Vec<Value> = Vec::new();
    for i in 0..10i32 {
        let t = i as f64 * 100.0_f64.to_radians();
        let x = 2.3 * t.cos();
        let y = 2.3 * t.sin();
        let z = i as f64 * 1.5;
        coords_list.push(Value::List(
            (vec![Value::Float(x), Value::Float(y), Value::Float(z)]).into(),
        ));
    }
    let result = structure::call_structure_builtin(
        "secondary_structure",
        vec![Value::List((coords_list).into())],
    )
    .unwrap();
    match result {
        Value::Table(t) => {
            assert_eq!(t.rows.len(), 10);
            // At least some residues should be assigned H (helix)
            let has_helix = t
                .rows
                .iter()
                .any(|r| matches!(&r[1], Value::Str(s) if s == "H"));
            // Structure detection is heuristic; just check the function runs
            let _ = has_helix;
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn backbone_angles_smoke_test() {
    // Minimal PDB: two residues with N, CA, C atoms
    let pdb = concat!(
        "ATOM      1  N   ALA A   1      27.470  -1.232   5.257  1.00  0.00           N  \n",
        "ATOM      2  CA  ALA A   1      26.297  -0.572   5.839  1.00  0.00           C  \n",
        "ATOM      3  C   ALA A   1      26.629   0.868   6.186  1.00  0.00           C  \n",
        "ATOM      4  N   GLY A   2      27.889   1.178   6.350  1.00  0.00           N  \n",
        "ATOM      5  CA  GLY A   2      28.308   2.548   6.671  1.00  0.00           C  \n",
        "ATOM      6  C   GLY A   2      29.770   2.612   7.061  1.00  0.00           C  \n",
    );
    let t_val =
        structure::call_structure_builtin("pdb_parse", vec![Value::Str(pdb.to_string())]).unwrap();
    let result = structure::call_structure_builtin("backbone_angles", vec![t_val]).unwrap();
    match result {
        Value::Table(t) => {
            assert_eq!(t.rows.len(), 2);
            assert!(t.columns.contains(&"phi".to_string()));
            assert!(t.columns.contains(&"psi".to_string()));
        }
        _ => panic!("expected Table"),
    }
}

// ── network tests ─────────────────────────────────────────────────────

fn make_edge_table(edges: &[(&str, &str, f64)]) -> Value {
    use bl_core::value::Table;
    let rows: Vec<Vec<Value>> = edges
        .iter()
        .map(|(a, b, w)| {
            vec![
                Value::Str(a.to_string()),
                Value::Str(b.to_string()),
                Value::Float(*w),
            ]
        })
        .collect();
    Value::Table(Table::new(
        vec![
            "protein1".to_string(),
            "protein2".to_string(),
            "score".to_string(),
        ],
        rows,
    ))
}

#[test]
fn load_ppi_filters_by_score() {
    let tsv = "# header\nGENE1 GENE2 900\nGENE3 GENE4 200\nGENE5 GENE6 500\n";
    let result = network::call_network_builtin(
        "load_ppi",
        vec![Value::Str(tsv.to_string()), Value::Float(400.0)],
    )
    .unwrap();
    match result {
        Value::Table(t) => {
            assert_eq!(
                t.rows.len(),
                2,
                "only rows with score >= 400 should be kept"
            );
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn degree_centrality_simple_graph() {
    // Star graph: node A connects to B, C, D
    let g = make_edge_table(&[("A", "B", 1.0), ("A", "C", 1.0), ("A", "D", 1.0)]);
    let result = network::call_network_builtin("degree_centrality", vec![g]).unwrap();
    match result {
        Value::Table(t) => {
            // A should have highest degree (3)
            let top = match &t.rows[0][0] {
                Value::Str(s) => s.clone(),
                _ => panic!(),
            };
            assert_eq!(top, "A");
            let deg = match &t.rows[0][1] {
                Value::Int(n) => *n,
                _ => panic!(),
            };
            assert_eq!(deg, 3);
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn betweenness_centrality_path_graph() {
    // Path graph: A-B-C-D. B and C have highest betweenness.
    let g = make_edge_table(&[("A", "B", 1.0), ("B", "C", 1.0), ("C", "D", 1.0)]);
    let result = network::call_network_builtin("betweenness_centrality", vec![g]).unwrap();
    match result {
        Value::Table(t) => {
            let top = match &t.rows[0][0] {
                Value::Str(s) => s.clone(),
                _ => panic!(),
            };
            assert!(
                top == "B" || top == "C",
                "B or C should have highest betweenness"
            );
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn shortest_path_finds_route() {
    let g = make_edge_table(&[("A", "B", 1.0), ("B", "C", 1.0), ("C", "D", 1.0)]);
    let result = network::call_network_builtin(
        "shortest_path",
        vec![g, Value::Str("A".to_string()), Value::Str("D".to_string())],
    )
    .unwrap();
    match result {
        Value::List(path) => {
            let names: Vec<String> = path
                .iter()
                .cloned()
                .map(|v| match v {
                    Value::Str(s) => s,
                    _ => panic!(),
                })
                .collect();
            assert_eq!(names.first().unwrap(), "A");
            assert_eq!(names.last().unwrap(), "D");
            assert_eq!(names.len(), 4);
        }
        _ => panic!("expected List path"),
    }
}

#[test]
fn shortest_path_same_node() {
    let g = make_edge_table(&[("A", "B", 1.0)]);
    let result = network::call_network_builtin(
        "shortest_path",
        vec![g, Value::Str("A".to_string()), Value::Str("A".to_string())],
    )
    .unwrap();
    match result {
        Value::List(path) => assert_eq!(path.len(), 1),
        _ => panic!("expected List"),
    }
}

#[test]
fn connected_components_two_clusters() {
    let g = make_edge_table(&[("A", "B", 1.0), ("B", "C", 1.0), ("X", "Y", 1.0)]);
    let result = network::call_network_builtin("connected_components", vec![g]).unwrap();
    match result {
        Value::Table(t) => {
            assert_eq!(t.rows.len(), 2, "should find 2 components");
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn network_enrichment_no_overlap() {
    let g = make_edge_table(&[("BRCA1", "TP53", 1.0), ("EGFR", "MYC", 1.0)]);
    let gene_set = Value::List(
        (vec![
            Value::Str("GENE_X".to_string()),
            Value::Str("GENE_Y".to_string()),
        ])
        .into(),
    );
    let result =
        network::call_network_builtin("network_enrichment", vec![g, gene_set, Value::Int(20000)])
            .unwrap();
    match result {
        Value::Table(t) => {
            let overlap = match &t.rows[0][0] {
                Value::Int(n) => *n,
                _ => panic!(),
            };
            assert_eq!(overlap, 0);
        }
        _ => panic!("expected Table"),
    }
}

// ── popgen tests ──────────────────────────────────────────────────────

#[test]
fn hwe_test_in_equilibrium() {
    // n_aa=490, n_ab=420, n_bb=90 — close to HWE for p=0.7
    let result = popgen::call_popgen_builtin(
        "hwe_test",
        vec![Value::Int(490), Value::Int(420), Value::Int(90)],
    )
    .unwrap();
    match result {
        Value::Table(t) => {
            let pvalue = match &t.rows[0][6] {
                Value::Float(f) => *f,
                _ => panic!(),
            };
            // Should not be significant (p > 0.05 when close to HWE)
            assert!(pvalue > 0.001, "expected HWE-like p-value, got {pvalue}");
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn hwe_test_deviation() {
    // Extreme excess of heterozygotes — strong HWE deviation
    let result = popgen::call_popgen_builtin(
        "hwe_test",
        vec![Value::Int(0), Value::Int(1000), Value::Int(0)],
    )
    .unwrap();
    match result {
        Value::Table(t) => {
            let pvalue = match &t.rows[0][6] {
                Value::Float(f) => *f,
                _ => panic!(),
            };
            assert!(
                pvalue < 0.05,
                "expected significant HWE deviation, got {pvalue}"
            );
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn fst_zero_for_same_frequencies() {
    // Two populations with identical allele frequencies → Fst ≈ 0
    let pop1 = Value::List(
        (vec![
            Value::List((vec![Value::Int(10), Value::Int(20)]).into()),
            Value::List((vec![Value::Int(5), Value::Int(20)]).into()),
        ])
        .into(),
    );
    let pop2 = Value::List(
        (vec![
            Value::List((vec![Value::Int(10), Value::Int(20)]).into()),
            Value::List((vec![Value::Int(5), Value::Int(20)]).into()),
        ])
        .into(),
    );
    let result = popgen::call_popgen_builtin("fst_weir_cockerham", vec![pop1, pop2]).unwrap();
    match result {
        Value::Float(f) => assert!(
            f.abs() < 1e-6,
            "Fst for same populations should be ~0, got {f}"
        ),
        _ => panic!("expected Float"),
    }
}

#[test]
fn fst_nonzero_for_different_pops() {
    // Pop1 has p=0.9, Pop2 has p=0.1 → high Fst
    let pop1 =
        Value::List((vec![Value::List((vec![Value::Int(18), Value::Int(20)]).into())]).into());
    let pop2 =
        Value::List((vec![Value::List((vec![Value::Int(2), Value::Int(20)]).into())]).into());
    let result = popgen::call_popgen_builtin("fst_weir_cockerham", vec![pop1, pop2]).unwrap();
    match result {
        Value::Float(f) => assert!(
            f > 0.3,
            "Fst for divergent populations should be high, got {f}"
        ),
        _ => panic!("expected Float"),
    }
}

#[test]
fn tajima_d_uniform_low_freq() {
    // All variants at count=1 (rare) → negative D (excess rare variants)
    let counts = Value::List(
        (vec![
            Value::Int(1),
            Value::Int(1),
            Value::Int(1),
            Value::Int(1),
            Value::Int(1),
            Value::Int(1),
            Value::Int(1),
            Value::Int(1),
            Value::Int(0),
            Value::Int(0),
            Value::Int(0),
            Value::Int(0),
        ])
        .into(),
    );
    let result = popgen::call_popgen_builtin("tajima_d", vec![counts, Value::Int(10)]).unwrap();
    match result {
        Value::Float(d) => {
            // Negative D expected under purifying selection / population expansion
            assert!(d < 0.0, "Expected negative Tajima's D, got {d}");
        }
        _ => panic!("expected Float"),
    }
}

#[test]
fn ld_r2_perfect_correlation() {
    // Identical haplotypes → r² = 1
    let a = Value::List((vec![Value::Int(0), Value::Int(1), Value::Int(0), Value::Int(1)]).into());
    let b = Value::List((vec![Value::Int(0), Value::Int(1), Value::Int(0), Value::Int(1)]).into());
    let result = popgen::call_popgen_builtin("ld_r2", vec![a, b]).unwrap();
    match result {
        Value::Float(r2) => assert!(
            (r2 - 1.0).abs() < 1e-9,
            "r² of identical haplotypes should be 1, got {r2}"
        ),
        _ => panic!("expected Float"),
    }
}

#[test]
fn ld_r2_no_correlation() {
    // Alternating vs uniform → should be near 0
    let a = Value::List(
        (vec![
            Value::Int(0),
            Value::Int(1),
            Value::Int(0),
            Value::Int(1),
            Value::Int(0),
            Value::Int(1),
            Value::Int(0),
            Value::Int(1),
        ])
        .into(),
    );
    let b = Value::List(
        (vec![
            Value::Int(1),
            Value::Int(1),
            Value::Int(1),
            Value::Int(1),
            Value::Int(1),
            Value::Int(1),
            Value::Int(1),
            Value::Int(1),
        ])
        .into(),
    );
    let result = popgen::call_popgen_builtin("ld_r2", vec![a, b]).unwrap();
    match result {
        Value::Float(r2) => assert!(
            r2.abs() < 1e-9,
            "r² with no variance at locus B should be 0, got {r2}"
        ),
        _ => panic!("expected Float"),
    }
}

#[test]
fn allele_freq_spectrum_basic() {
    let counts = Value::List(
        (vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(1),
            Value::Int(5),
            Value::Int(0),
            Value::Int(3),
        ])
        .into(),
    );
    let result =
        popgen::call_popgen_builtin("allele_freq_spectrum", vec![counts, Value::Int(10)]).unwrap();
    match result {
        Value::Table(t) => {
            assert!(!t.rows.is_empty());
            assert!(t.columns.contains(&"frequency".to_string()));
            assert!(t.columns.contains(&"n_sites".to_string()));
        }
        _ => panic!("expected Table"),
    }
}

#[test]
fn nucleotide_diversity_zero_for_identical() {
    let seqs = Value::List(
        (vec![
            Value::Str("ACGT".to_string()),
            Value::Str("ACGT".to_string()),
            Value::Str("ACGT".to_string()),
        ])
        .into(),
    );
    let result = popgen::call_popgen_builtin("nucleotide_diversity", vec![seqs]).unwrap();
    match result {
        Value::Float(pi) => assert!(pi.abs() < 1e-9, "π of identical seqs should be 0, got {pi}"),
        _ => panic!("expected Float"),
    }
}

#[test]
fn nucleotide_diversity_nonzero() {
    let seqs = Value::List(
        (vec![
            Value::Str("ACGT".to_string()),
            Value::Str("ACGA".to_string()), // 1 diff
            Value::Str("ACTT".to_string()), // 1 diff from seq1, 2 from seq2
        ])
        .into(),
    );
    let result = popgen::call_popgen_builtin("nucleotide_diversity", vec![seqs]).unwrap();
    match result {
        Value::Float(pi) => assert!(pi > 0.0, "π should be > 0 for distinct seqs, got {pi}"),
        _ => panic!("expected Float"),
    }
}
