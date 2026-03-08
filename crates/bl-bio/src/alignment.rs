use bio_core::alignment::{self, AlignMode, AlignParams};
use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};
use std::collections::HashMap;

pub fn alignment_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("align", Arity::Range(2, 3)),
        ("score_matrix", Arity::Exact(1)),
        ("edit_distance", Arity::Exact(2)),
        ("hamming_distance", Arity::Exact(2)),
    ]
}

pub fn is_alignment_builtin(name: &str) -> bool {
    matches!(
        name,
        "align" | "score_matrix" | "edit_distance" | "hamming_distance"
    )
}

pub fn call_alignment_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "align" => builtin_align(args),
        "score_matrix" => builtin_score_matrix(args),
        "edit_distance" => builtin_edit_distance(args),
        "hamming_distance" => builtin_hamming_distance(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown alignment builtin '{name}'"),
            None,
        )),
    }
}

fn get_seq_str(val: &Value, func: &str) -> Result<String> {
    match val {
        Value::Str(s) => Ok(s.clone()),
        Value::DNA(seq) | Value::RNA(seq) | Value::Protein(seq) => Ok(seq.data.clone()),
        other => Err(BioLangError::type_error(
            format!(
                "{func}() requires Str/DNA/RNA/Protein, got {}",
                other.type_of()
            ),
            None,
        )),
    }
}

fn builtin_align(args: Vec<Value>) -> Result<Value> {
    let seq1 = get_seq_str(&args[0], "align")?;
    let seq2 = get_seq_str(&args[1], "align")?;

    let mut params = AlignParams::default();

    // Parse optional options Record
    if args.len() > 2 {
        if let Value::Record(opts) = &args[2] {
            if let Some(Value::Str(t)) = opts.get("type") {
                params.mode = match t.as_str() {
                    "local" => AlignMode::Local,
                    "global" => AlignMode::Global,
                    _ => {
                        return Err(BioLangError::runtime(
                            ErrorKind::TypeError,
                            format!("align() unknown type '{t}', expected 'global' or 'local'"),
                            None,
                        ))
                    }
                };
            }
            if let Some(v) = opts.get("match") {
                params.match_score = val_to_i32(v, "match")?;
            }
            if let Some(v) = opts.get("mismatch") {
                params.mismatch_score = val_to_i32(v, "mismatch")?;
            }
            if let Some(v) = opts.get("gap_open") {
                params.gap_open = val_to_i32(v, "gap_open")?;
            }
            if let Some(v) = opts.get("gap_extend") {
                params.gap_extend = val_to_i32(v, "gap_extend")?;
            }
        }
    }

    let result = alignment::align(&seq1, &seq2, &params);

    let mut record = HashMap::new();
    record.insert("aligned1".to_string(), Value::Str(result.aligned1));
    record.insert("aligned2".to_string(), Value::Str(result.aligned2));
    record.insert("score".to_string(), Value::Int(result.score as i64));
    record.insert("identity".to_string(), Value::Float(result.identity));
    record.insert("gaps".to_string(), Value::Int(result.gaps as i64));
    record.insert("cigar".to_string(), Value::Str(result.cigar));

    Ok(Value::Record(record))
}

fn builtin_score_matrix(args: Vec<Value>) -> Result<Value> {
    let name = match &args[0] {
        Value::Str(s) => s.as_str(),
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "score_matrix() requires Str, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };

    let matrix = alignment::scoring_matrix(name).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "score_matrix() unknown matrix '{name}', expected 'blosum62', 'pam250', or 'blosum45'"
            ),
            None,
        )
    })?;

    // Convert to Matrix type (20x20)
    let data: Vec<f64> = matrix
        .iter()
        .flat_map(|row| row.iter().map(|&v| v as f64))
        .collect();
    let mut m = bl_core::matrix::Matrix::new(data, 20, 20)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;

    // Set row and column names to amino acid letters
    let aa_names: Vec<String> = alignment::AA_ORDER.iter().map(|&b| (b as char).to_string()).collect();
    m.row_names = Some(aa_names.clone());
    m.col_names = Some(aa_names);

    Ok(Value::Matrix(m))
}

fn builtin_edit_distance(args: Vec<Value>) -> Result<Value> {
    let seq1 = get_seq_str(&args[0], "edit_distance")?;
    let seq2 = get_seq_str(&args[1], "edit_distance")?;
    Ok(Value::Int(alignment::edit_distance(&seq1, &seq2) as i64))
}

fn builtin_hamming_distance(args: Vec<Value>) -> Result<Value> {
    let seq1 = get_seq_str(&args[0], "hamming_distance")?;
    let seq2 = get_seq_str(&args[1], "hamming_distance")?;
    let dist = alignment::hamming_distance(&seq1, &seq2)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(Value::Int(dist as i64))
}

fn val_to_i32(val: &Value, field: &str) -> Result<i32> {
    match val {
        Value::Int(n) => Ok(*n as i32),
        Value::Float(f) => Ok(*f as i32),
        _ => Err(BioLangError::type_error(
            format!("align() '{field}' must be a number"),
            None,
        )),
    }
}

