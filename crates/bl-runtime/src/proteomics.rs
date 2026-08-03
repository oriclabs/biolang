//! Proteomics builtins: MaxQuant loading, log2 transform, quantile normalisation,
//! min-value imputation, per-protein t-test, and volcano-plot data preparation.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};

// ── Registry ──────────────────────────────────────────────────────────

pub fn proteomics_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("load_maxquant", Arity::Range(1, 2)),
        ("log2_transform", Arity::Exact(1)),
        ("quantile_normalize", Arity::Exact(1)),
        ("impute_minvalue", Arity::Range(1, 2)),
        ("protein_ttest", Arity::Exact(3)),
        ("linear_spectrum", Arity::Exact(1)),
        ("cyclic_spectrum", Arity::Exact(1)),
        ("spectrum_score", Arity::Exact(2)),
        ("spectrum_convolution", Arity::Exact(1)),
        ("amino_acid_masses", Arity::Exact(0)),
        ("peptide_mass", Arity::Exact(1)),
        ("volcano_data", Arity::Range(1, 3)),
    ]
}

pub fn is_proteomics_builtin(name: &str) -> bool {
    matches!(
        name,
        "load_maxquant"
            | "linear_spectrum"
            | "cyclic_spectrum"
            | "spectrum_score"
            | "spectrum_convolution"
            | "amino_acid_masses"
            | "peptide_mass"
            | "log2_transform"
            | "quantile_normalize"
            | "impute_minvalue"
            | "protein_ttest"
            | "volcano_data"
    )
}

pub fn call_proteomics_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "load_maxquant" => builtin_load_maxquant(args),
        "log2_transform" => builtin_log2_transform(args),
        "quantile_normalize" => builtin_quantile_normalize(args),
        "impute_minvalue" => builtin_impute_minvalue(args),
        "protein_ttest" => builtin_protein_ttest(args),
        "linear_spectrum" => builtin_linear_spectrum(args),
        "cyclic_spectrum" => builtin_cyclic_spectrum(args),
        "spectrum_score" => builtin_spectrum_score(args),
        "spectrum_convolution" => builtin_spectrum_convolution(args),
        "amino_acid_masses" => builtin_amino_acid_masses(args),
        "peptide_mass" => builtin_peptide_mass(args),
        "volcano_data" => builtin_volcano_data(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown proteomics builtin: {name}"),
            None,
        )),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

fn to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Float(f) => Some(*f),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

fn require_matrix(val: &Value, func: &str) -> Result<Vec<Vec<f64>>> {
    match val {
        Value::List(rows) => rows
            .iter()
            .map(|row| match row {
                Value::List(cells) => cells
                    .iter()
                    .map(|v| {
                        to_f64(v).ok_or_else(|| {
                            BioLangError::type_error(
                                format!("{func}() matrix must contain numbers"),
                                None,
                            )
                        })
                    })
                    .collect(),
                _ => Err(BioLangError::type_error(
                    format!("{func}() matrix rows must be Lists"),
                    None,
                )),
            })
            .collect(),
        Value::Table(t) => Ok(t
            .rows
            .iter()
            .map(|row| row.iter().map(|v| to_f64(v).unwrap_or(0.0)).collect())
            .collect()),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires List<List> or Table"),
            None,
        )),
    }
}

fn require_str_list(val: &Value, func: &str) -> Result<Vec<String>> {
    match val {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Str(s) => Ok(s.clone()),
                _ => Err(BioLangError::type_error(
                    format!("{func}() label list must contain strings"),
                    None,
                )),
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires a List of strings"),
            None,
        )),
    }
}

fn require_float(val: &Value, func: &str) -> Result<f64> {
    to_f64(val).ok_or_else(|| BioLangError::type_error(format!("{func}() requires a number"), None))
}

fn matrix_to_value(mat: Vec<Vec<f64>>) -> Value {
    Value::List(
        mat.into_iter()
            .map(|row| Value::List(row.into_iter().map(Value::Float).collect::<Vec<_>>().into()))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn vec_mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.iter().sum::<f64>() / v.len() as f64
}

fn vec_var(v: &[f64]) -> f64 {
    if v.len() < 2 {
        return 0.0;
    }
    let m = vec_mean(v);
    v.iter().map(|&x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64
}

// Welch t-test: returns (t_stat, p_value) using normal approximation for large n,
// otherwise simple two-sample Welch approximation.
fn welch_ttest(a: &[f64], b: &[f64]) -> (f64, f64) {
    if a.is_empty() || b.is_empty() {
        return (0.0, 1.0);
    }
    let na = a.len() as f64;
    let nb = b.len() as f64;
    let va = vec_var(a);
    let vb = vec_var(b);
    let se = (va / na + vb / nb).sqrt();
    if se == 0.0 {
        // No within-group variance — if means differ the groups are perfectly separated
        let diff = (vec_mean(a) - vec_mean(b)).abs();
        return if diff > 0.0 {
            (f64::INFINITY, 0.0)
        } else {
            (0.0, 1.0)
        };
    }
    let t = (vec_mean(a) - vec_mean(b)) / se;
    // Welch-Satterthwaite df
    let s2a = va / na;
    let s2b = vb / nb;
    let df = if s2a + s2b == 0.0 {
        1.0
    } else {
        (s2a + s2b).powi(2) / (s2a.powi(2) / (na - 1.0) + s2b.powi(2) / (nb - 1.0))
    };
    let p = p_from_t(t, df.max(1.0));
    (t, p)
}

fn p_from_t(t: f64, df: f64) -> f64 {
    if t.is_infinite() || t.is_nan() {
        return if t.abs() == f64::INFINITY { 0.0 } else { 1.0 };
    }
    let z = t.abs() * (1.0 - 1.0 / (4.0 * df));
    let p_one = standard_normal_tail(z);
    (2.0 * p_one).min(1.0)
}

fn standard_normal_tail(z: f64) -> f64 {
    if z <= 0.0 {
        return 0.5;
    }
    if z > 8.0 {
        return 0.0;
    }
    let t = 1.0 / (1.0 + 0.2316419 * z);
    let poly = t
        * (0.319381530
            + t * (-0.356563782 + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
    let pdf = (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt();
    pdf * poly
}

// ── load_maxquant(path, intensity_col_prefix="LFQ intensity ") ────────
//
// Parses a MaxQuant proteinGroups.txt (TSV). Expects columns: Protein IDs,
// Gene names, and any number of columns matching the prefix (default "LFQ intensity ").
// Returns a Table with columns: protein, gene, <sample1>, <sample2>, ...

fn builtin_load_maxquant(args: Vec<Value>) -> Result<Value> {
    let path = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => {
            return Err(BioLangError::type_error(
                "load_maxquant() requires a path string",
                None,
            ))
        }
    };
    let prefix = if args.len() > 1 {
        match &args[1] {
            Value::Str(s) => s.clone(),
            _ => "LFQ intensity ".to_string(),
        }
    } else {
        "LFQ intensity ".to_string()
    };

    let content = std::fs::read_to_string(&path).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("load_maxquant(): {e}"), None)
    })?;
    let mut lines = content.lines();

    let header = lines.next().ok_or_else(|| {
        BioLangError::runtime(ErrorKind::NameError, "load_maxquant(): empty file", None)
    })?;

    let cols: Vec<&str> = header.split('\t').collect();
    let protein_col = cols
        .iter()
        .position(|c| c.contains("Protein IDs"))
        .unwrap_or(0);
    let gene_col = cols
        .iter()
        .position(|c| c.contains("Gene names"))
        .unwrap_or(1);
    let sample_indices: Vec<usize> = cols
        .iter()
        .enumerate()
        .filter(|(_, c)| c.starts_with(prefix.as_str()))
        .map(|(i, _)| i)
        .collect();
    let sample_names: Vec<String> = sample_indices
        .iter()
        .map(|&i| cols[i].trim_start_matches(prefix.as_str()).to_string())
        .collect();

    let mut col_names = vec!["protein".to_string(), "gene".to_string()];
    col_names.extend(sample_names);

    let mut rows: Vec<Vec<Value>> = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.is_empty() {
            continue;
        }
        // Skip contaminants and reverse sequences (MaxQuant convention)
        let protein = fields.get(protein_col).copied().unwrap_or("");
        if protein.starts_with('+') || protein.is_empty() {
            continue;
        }
        let gene = fields.get(gene_col).copied().unwrap_or("").to_string();
        let mut row: Vec<Value> = vec![Value::Str(protein.to_string()), Value::Str(gene)];
        for &idx in &sample_indices {
            let val: f64 = fields.get(idx).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            row.push(Value::Float(val));
        }
        rows.push(row);
    }

    Ok(Value::Table(Table::new(col_names, rows)))
}

// ── log2_transform(matrix) ────────────────────────────────────────────

fn builtin_log2_transform(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "log2_transform")?;
    let result: Vec<Vec<f64>> = mat
        .into_iter()
        .map(|row| row.into_iter().map(|v| (v + 1.0).log2()).collect())
        .collect();
    Ok(matrix_to_value(result))
}

// ── quantile_normalize(matrix) ────────────────────────────────────────
//
// Standard quantile normalization: columns represent samples.
// Each row is a protein; each column a sample.
// Returns matrix with same dimensions but rank-normalized columns.

fn builtin_quantile_normalize(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "quantile_normalize")?;
    let n_rows = mat.len();
    if n_rows == 0 {
        return Ok(matrix_to_value(mat));
    }
    let n_cols = mat[0].len();
    if n_cols == 0 {
        return Ok(matrix_to_value(mat));
    }

    // Transpose: work column-by-column
    let mut columns: Vec<Vec<f64>> = (0..n_cols)
        .map(|c| (0..n_rows).map(|r| mat[r][c]).collect())
        .collect();

    // For each column: record sort order, compute row means of sorted values,
    // then replace with mean values at the matching quantile position.
    let mut sorted_indices: Vec<Vec<usize>> = columns
        .iter()
        .map(|col| {
            let mut idx: Vec<usize> = (0..col.len()).collect();
            idx.sort_by(|&a, &b| col[a].partial_cmp(&col[b]).unwrap());
            idx
        })
        .collect();

    // Row means of column-sorted values
    let row_means: Vec<f64> = (0..n_rows)
        .map(|rank| {
            let sum: f64 = (0..n_cols)
                .map(|c| columns[c][sorted_indices[c][rank]])
                .sum();
            sum / n_cols as f64
        })
        .collect();

    // Replace each column's values with row means
    for (c, idx) in sorted_indices.iter_mut().enumerate() {
        let mut norm = vec![0.0f64; n_rows];
        for (rank, &orig) in idx.iter().enumerate() {
            norm[orig] = row_means[rank];
        }
        columns[c] = norm;
    }

    // Transpose back
    let result: Vec<Vec<f64>> = (0..n_rows)
        .map(|r| (0..n_cols).map(|c| columns[c][r]).collect())
        .collect();

    Ok(matrix_to_value(result))
}

// ── impute_minvalue(matrix, fraction=0.5) ─────────────────────────────
//
// Replace zeros (missing intensities) with fraction * column_minimum.

fn builtin_impute_minvalue(args: Vec<Value>) -> Result<Value> {
    let mut mat = require_matrix(&args[0], "impute_minvalue")?;
    let fraction = if args.len() > 1 {
        require_float(&args[1], "impute_minvalue")?
    } else {
        0.5
    };

    let n_rows = mat.len();
    if n_rows == 0 {
        return Ok(matrix_to_value(mat));
    }
    let n_cols = mat[0].len();

    // Find per-column minimum (non-zero)
    let col_mins: Vec<f64> = (0..n_cols)
        .map(|c| {
            mat.iter()
                .map(|row| row[c])
                .filter(|&v| v > 0.0)
                .fold(f64::INFINITY, f64::min)
        })
        .collect();

    for row in mat.iter_mut() {
        for (c, val) in row.iter_mut().enumerate() {
            if *val == 0.0 {
                let floor = if col_mins[c].is_finite() {
                    col_mins[c] * fraction
                } else {
                    fraction
                };
                *val = floor;
            }
        }
    }

    Ok(matrix_to_value(mat))
}

// ── protein_ttest(matrix, group_a_indices, group_b_indices) ──────────
//
// Per-protein Welch t-test between two groups (column indices in Int list).
// Returns a Table: protein_idx, mean_a, mean_b, log2fc, t_stat, p_value.

fn builtin_protein_ttest(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "protein_ttest")?;
    let idx_a = require_index_list(&args[1], "protein_ttest")?;
    let idx_b = require_index_list(&args[2], "protein_ttest")?;

    let cols = vec![
        "protein_idx".to_string(),
        "mean_a".to_string(),
        "mean_b".to_string(),
        "log2fc".to_string(),
        "t_stat".to_string(),
        "p_value".to_string(),
    ];

    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(mat.len());
    for (i, row) in mat.iter().enumerate() {
        let a: Vec<f64> = idx_a.iter().filter_map(|&j| row.get(j).copied()).collect();
        let b: Vec<f64> = idx_b.iter().filter_map(|&j| row.get(j).copied()).collect();
        let mean_a = vec_mean(&a);
        let mean_b = vec_mean(&b);
        let log2fc = if mean_b > 0.0 {
            (mean_a / mean_b).log2()
        } else if mean_a > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };
        let (t, p) = welch_ttest(&a, &b);
        rows.push(vec![
            Value::Int(i as i64),
            Value::Float(mean_a),
            Value::Float(mean_b),
            Value::Float(log2fc),
            Value::Float(t),
            Value::Float(p),
        ]);
    }

    Ok(Value::Table(Table::new(cols, rows)))
}

fn require_index_list(val: &Value, func: &str) -> Result<Vec<usize>> {
    match val {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Int(n) if *n >= 0 => Ok(*n as usize),
                _ => Err(BioLangError::type_error(
                    format!("{func}() index list must contain non-negative Ints"),
                    None,
                )),
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires a List of Int indices"),
            None,
        )),
    }
}

// ── volcano_data(ttest_table, fc_threshold=1.0, p_threshold=0.05) ─────
//
// Annotates a protein_ttest Table with a "regulation" column:
// "up", "down", or "ns" (not significant).

fn builtin_volcano_data(args: Vec<Value>) -> Result<Value> {
    let ttest = match &args[0] {
        Value::Table(t) => t.clone(),
        _ => {
            return Err(BioLangError::type_error(
                "volcano_data() requires a Table from protein_ttest()",
                None,
            ))
        }
    };
    let fc_thresh = if args.len() > 1 {
        require_float(&args[1], "volcano_data")?
    } else {
        1.0
    };
    let p_thresh = if args.len() > 2 {
        require_float(&args[2], "volcano_data")?
    } else {
        0.05
    };

    let fc_col = ttest
        .columns
        .iter()
        .position(|c| c == "log2fc")
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::NameError,
                "volcano_data(): table missing 'log2fc' column",
                None,
            )
        })?;
    let p_col = ttest
        .columns
        .iter()
        .position(|c| c == "p_value")
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::NameError,
                "volcano_data(): table missing 'p_value' column",
                None,
            )
        })?;

    let mut new_cols = ttest.columns.clone();
    new_cols.push("regulation".to_string());

    let new_rows: Vec<Vec<Value>> = ttest
        .rows
        .into_iter()
        .map(|mut row| {
            let fc = to_f64(&row[fc_col]).unwrap_or(0.0);
            let p = to_f64(&row[p_col]).unwrap_or(1.0);
            let label = if p < p_thresh && fc > fc_thresh {
                "up"
            } else if p < p_thresh && fc < -fc_thresh {
                "down"
            } else {
                "ns"
            };
            row.push(Value::Str(label.to_string()));
            row
        })
        .collect();

    Ok(Value::Table(Table::new(new_cols, new_rows)))
}

// ── Theoretical spectra ──────────────────────────────────────────────
//
// A mass spectrometer breaks copies of a peptide at every position and weighs
// the pieces; sequencing runs that backwards. Peptides are lists of integer
// masses rather than letters, because leucine and isoleucine both weigh 113 and
// glutamine and lysine both weigh 128 — carrying letters would promise a
// resolution the data does not have.

use bl_core::bio_core::spectrum as core_spectrum;

fn read_masses(value: &Value, func: &str) -> Result<Vec<u32>> {
    match value {
        // A peptide written as letters is a convenience; the masses are what
        // the algorithms use.
        Value::Str(text) => core_spectrum::masses_of(text).ok_or_else(|| {
            BioLangError::type_error(
                format!("{func}(): '{text}' contains a residue with no known mass"),
                None,
            )
        }),
        Value::Protein(seq) => core_spectrum::masses_of(&seq.data).ok_or_else(|| {
            BioLangError::type_error(
                format!("{func}(): the peptide contains a residue with no known mass"),
                None,
            )
        }),
        Value::List(items) => items
            .iter()
            .map(|item| match item {
                Value::Int(n) if *n >= 0 => Ok(*n as u32),
                Value::Int(n) => Err(BioLangError::type_error(
                    format!("{func}(): a mass cannot be negative, got {n}"),
                    None,
                )),
                Value::Float(f) if *f >= 0.0 && f.fract() == 0.0 => Ok(*f as u32),
                Value::Float(f) => Err(BioLangError::type_error(
                    format!("{func}(): masses are integer daltons here, got {f} — round it first"),
                    None,
                )),
                other => Err(BioLangError::type_error(
                    format!("{func}(): masses must be numbers, got {}", other.type_of()),
                    None,
                )),
            })
            .collect(),
        other => Err(BioLangError::type_error(
            format!(
                "{func}() requires a peptide string or a list of masses, got {}",
                other.type_of()
            ),
            None,
        )),
    }
}

fn to_int_list(values: Vec<u32>) -> Value {
    Value::List(
        values
            .into_iter()
            .map(|v| Value::Int(i64::from(v)))
            .collect::<Vec<_>>()
            .into(),
    )
}

/// `linear_spectrum(peptide)` — every contiguous subpeptide's mass, sorted.
fn builtin_linear_spectrum(args: Vec<Value>) -> Result<Value> {
    let peptide = read_masses(&args[0], "linear_spectrum")?;
    Ok(to_int_list(core_spectrum::linear_spectrum(&peptide)))
}

/// `cyclic_spectrum(peptide)` — the same for a peptide joined end to end, which
/// additionally contains every piece that wraps past the end.
fn builtin_cyclic_spectrum(args: Vec<Value>) -> Result<Value> {
    let peptide = read_masses(&args[0], "cyclic_spectrum")?;
    Ok(to_int_list(core_spectrum::cyclic_spectrum(&peptide)))
}

/// `spectrum_score(theoretical, observed)` — how many masses the two share,
/// counting multiplicity.
///
/// A mass appearing twice in both counts twice. Set intersection would score a
/// peptide explaining a repeated mass once as well as one explaining it fully.
fn builtin_spectrum_score(args: Vec<Value>) -> Result<Value> {
    let theoretical = read_masses(&args[0], "spectrum_score")?;
    let observed = read_masses(&args[1], "spectrum_score")?;
    Ok(Value::Int(
        core_spectrum::score(&theoretical, &observed) as i64
    ))
}

/// `spectrum_convolution(spectrum)` — every positive pairwise difference,
/// sorted.
///
/// Differences between fragment masses are themselves residue masses, so the
/// commonest ones are what the peptide is most likely built from — which is how
/// an unknown peptide is sequenced without assuming the standard twenty.
fn builtin_spectrum_convolution(args: Vec<Value>) -> Result<Value> {
    let spectrum = read_masses(&args[0], "spectrum_convolution")?;
    Ok(to_int_list(core_spectrum::convolution(&spectrum)))
}

/// `amino_acid_masses()` — the 18 distinct integer residue masses.
fn builtin_amino_acid_masses(_args: Vec<Value>) -> Result<Value> {
    Ok(to_int_list(core_spectrum::distinct_masses()))
}

/// `peptide_mass(peptide)` — the total mass of a peptide.
fn builtin_peptide_mass(args: Vec<Value>) -> Result<Value> {
    let peptide = read_masses(&args[0], "peptide_mass")?;
    Ok(Value::Int(peptide.iter().map(|&m| i64::from(m)).sum()))
}

#[cfg(test)]
mod spectrum_tests {
    use super::*;

    fn ints(value: &Value) -> Vec<i64> {
        match value {
            Value::List(items) => items
                .iter()
                .map(|v| match v {
                    Value::Int(n) => *n,
                    other => panic!("expected an int, got {other:?}"),
                })
                .collect(),
            other => panic!("expected a list, got {other:?}"),
        }
    }

    #[test]
    fn a_peptide_may_be_written_as_letters_or_masses() {
        let from_letters =
            builtin_cyclic_spectrum(vec![Value::Str("LEQN".into())]).expect("letters");
        let from_masses = builtin_cyclic_spectrum(vec![Value::List(
            vec![
                Value::Int(113),
                Value::Int(129),
                Value::Int(128),
                Value::Int(114),
            ]
            .into(),
        )])
        .expect("masses");
        assert_eq!(from_letters, from_masses);
        assert_eq!(
            ints(&from_letters),
            vec![0, 113, 114, 128, 129, 227, 242, 242, 257, 355, 356, 370, 371, 484]
        );
    }

    #[test]
    fn an_unknown_residue_is_reported() {
        let error =
            builtin_cyclic_spectrum(vec![Value::Str("LEQZ".into())]).expect_err("Z has no mass");
        assert!(error.to_string().contains("no known mass"), "{error}");
    }

    #[test]
    fn a_fractional_mass_says_to_round_it() {
        // Instrument readings carry decimals; the search here is over integers,
        // and silently truncating would shift every subsequent total.
        let error = builtin_linear_spectrum(vec![Value::List(vec![Value::Float(113.5)].into())])
            .expect_err("not an integer");
        assert!(error.to_string().contains("round it first"), "{error}");
    }

    #[test]
    fn scoring_counts_multiplicity() {
        let peptide = Value::Str("LEQN".into());
        let theoretical = builtin_cyclic_spectrum(vec![peptide]).unwrap();
        let once = builtin_spectrum_score(vec![
            theoretical.clone(),
            Value::List(vec![Value::Int(242)].into()),
        ])
        .unwrap();
        let twice = builtin_spectrum_score(vec![
            theoretical,
            Value::List(vec![Value::Int(242), Value::Int(242)].into()),
        ])
        .unwrap();
        assert_eq!(once, Value::Int(1));
        assert_eq!(twice, Value::Int(2));
    }

    #[test]
    fn there_are_eighteen_masses_not_twenty() {
        let masses = ints(&builtin_amino_acid_masses(vec![]).unwrap());
        assert_eq!(masses.len(), 18, "I/L and K/Q collide");
        assert_eq!(
            builtin_peptide_mass(vec![Value::Str("LEQN".into())]).unwrap(),
            Value::Int(484)
        );
    }
}
