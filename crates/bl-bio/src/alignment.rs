use bio_core::alignment::{self, AlignMode, AlignParams};
use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};
use std::collections::HashMap;
use std::io::Write as IoWrite;

pub fn alignment_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("align", Arity::Range(2, 3)),
        ("score_matrix", Arity::Exact(1)),
        ("edit_distance", Arity::Exact(2)),
        ("hamming_distance", Arity::Exact(2)),
        ("msa", Arity::Range(1, 2)),
        ("distance_matrix", Arity::Range(1, 2)),
        ("conservation_scores", Arity::Exact(1)),
    ]
}

pub fn is_alignment_builtin(name: &str) -> bool {
    matches!(
        name,
        "align" | "score_matrix" | "edit_distance" | "hamming_distance"
            | "msa" | "distance_matrix" | "conservation_scores"
    )
}

pub fn call_alignment_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "align" => builtin_align(args),
        "score_matrix" => builtin_score_matrix(args),
        "edit_distance" => builtin_edit_distance(args),
        "hamming_distance" => builtin_hamming_distance(args),
        "msa" => builtin_msa(args),
        "distance_matrix" => builtin_distance_matrix(args),
        "conservation_scores" => builtin_conservation_scores(args),
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

// ── MSA ────────────────────────────────────────────────────────────

/// Extract (name, sequence) pairs from a List of Str/DNA/RNA/Protein values.
fn extract_sequences(list: &[Value], func: &str) -> Result<Vec<(String, String)>> {
    let mut seqs = Vec::with_capacity(list.len());
    for (i, val) in list.iter().enumerate() {
        let name = format!("seq{}", i + 1);
        let s = get_seq_str(val, func)?;
        seqs.push((name, s));
    }
    if seqs.len() < 2 {
        return Err(BioLangError::type_error(
            format!("{func}() requires at least 2 sequences, got {}", seqs.len()),
            None,
        ));
    }
    Ok(seqs)
}

/// Extract aligned sequences from either an MSA result Record or a List[Str].
fn extract_aligned_sequences(val: &Value, func: &str) -> Result<Vec<(String, String)>> {
    match val {
        Value::Record(rec) => {
            // MSA result record: { sequences: List[Str], names: List[Str], ... }
            let seqs = rec.get("sequences").ok_or_else(|| {
                BioLangError::type_error(
                    format!("{func}() Record must have 'sequences' field"),
                    None,
                )
            })?;
            let names = rec.get("names");
            if let Value::List(seq_list) = seqs {
                let name_list: Vec<String> = match names {
                    Some(Value::List(nl)) => nl
                        .iter()
                        .enumerate()
                        .map(|(i, v)| match v {
                            Value::Str(s) => s.clone(),
                            _ => format!("seq{}", i + 1),
                        })
                        .collect(),
                    _ => (0..seq_list.len())
                        .map(|i| format!("seq{}", i + 1))
                        .collect(),
                };
                let mut result = Vec::with_capacity(seq_list.len());
                for (i, v) in seq_list.iter().enumerate() {
                    let s = get_seq_str(v, func)?;
                    let name = name_list.get(i).cloned().unwrap_or_else(|| format!("seq{}", i + 1));
                    result.push((name, s));
                }
                if result.len() < 2 {
                    return Err(BioLangError::type_error(
                        format!("{func}() requires at least 2 sequences"),
                        None,
                    ));
                }
                Ok(result)
            } else {
                Err(BioLangError::type_error(
                    format!("{func}() Record 'sequences' must be a List"),
                    None,
                ))
            }
        }
        Value::List(list) => {
            let mut result = Vec::with_capacity(list.len());
            for (i, v) in list.iter().enumerate() {
                let s = get_seq_str(v, func)?;
                result.push((format!("seq{}", i + 1), s));
            }
            if result.len() < 2 {
                return Err(BioLangError::type_error(
                    format!("{func}() requires at least 2 sequences"),
                    None,
                ));
            }
            Ok(result)
        }
        _ => Err(BioLangError::type_error(
            format!("{func}() requires a List or MSA Record, got {}", val.type_of()),
            None,
        )),
    }
}

fn builtin_msa(args: Vec<Value>) -> Result<Value> {
    let list = match &args[0] {
        Value::List(l) => l,
        other => {
            return Err(BioLangError::type_error(
                format!("msa() requires a List of sequences, got {}", other.type_of()),
                None,
            ))
        }
    };

    let seqs = extract_sequences(list, "msa")?;

    // Parse optional opts
    let _opts: HashMap<String, Value> = if args.len() > 1 {
        if let Value::Record(o) = &args[1] {
            o.clone()
        } else {
            HashMap::new()
        }
    } else {
        HashMap::new()
    };

    // Try external tools first, then fall back to progressive alignment
    if let Ok(result) = try_msa_external(&seqs) {
        return Ok(result);
    }

    progressive_align(&seqs)
}

/// Try running MAFFT or MUSCLE for MSA.
fn try_msa_external(seqs: &[(String, String)]) -> std::result::Result<Value, String> {
    use std::process::Command;

    // Write sequences to a temp FASTA file
    let tmp_dir = std::env::temp_dir();
    let input_path = tmp_dir.join(format!("biolang_msa_{}.fasta", std::process::id()));

    {
        let mut f = std::fs::File::create(&input_path).map_err(|e| e.to_string())?;
        for (name, seq) in seqs {
            writeln!(f, ">{name}").map_err(|e| e.to_string())?;
            writeln!(f, "{seq}").map_err(|e| e.to_string())?;
        }
    }

    let input_str = input_path.to_string_lossy().to_string();

    // Try mafft
    let mafft_result = Command::new("mafft")
        .arg("--auto")
        .arg(&input_str)
        .output();

    if let Ok(output) = mafft_result {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let _ = std::fs::remove_file(&input_path);
            return parse_aligned_fasta(&stdout);
        }
    }

    // Try muscle
    let out_path = tmp_dir.join(format!("biolang_msa_{}_out.fasta", std::process::id()));
    let out_str = out_path.to_string_lossy().to_string();

    let muscle_result = Command::new("muscle")
        .args(["-align", &input_str, "-output", &out_str])
        .output();

    let _ = std::fs::remove_file(&input_path);

    if let Ok(output) = muscle_result {
        if output.status.success() {
            if let Ok(content) = std::fs::read_to_string(&out_path) {
                let _ = std::fs::remove_file(&out_path);
                return parse_aligned_fasta(&content);
            }
        }
    }
    let _ = std::fs::remove_file(&out_path);

    Err("no external MSA tool available".to_string())
}

/// Parse aligned FASTA output into an MSA result Record.
fn parse_aligned_fasta(fasta: &str) -> std::result::Result<Value, String> {
    let mut names = Vec::new();
    let mut sequences = Vec::new();
    let mut current_name = String::new();
    let mut current_seq = String::new();

    for line in fasta.lines() {
        let line = line.trim();
        if line.starts_with('>') {
            if !current_name.is_empty() {
                names.push(current_name.clone());
                sequences.push(current_seq.clone());
                current_seq.clear();
            }
            current_name = line.strip_prefix('>').unwrap_or(line).trim().to_string();
        } else if !line.is_empty() {
            current_seq.push_str(line);
        }
    }
    if !current_name.is_empty() {
        names.push(current_name);
        sequences.push(current_seq);
    }

    if sequences.len() < 2 {
        return Err("parsed fewer than 2 sequences from FASTA".to_string());
    }

    let length = sequences.first().map(|s| s.len()).unwrap_or(0);
    let n_seqs = sequences.len();

    let mut record = HashMap::new();
    record.insert(
        "sequences".to_string(),
        Value::List(sequences.into_iter().map(Value::Str).collect()),
    );
    record.insert(
        "names".to_string(),
        Value::List(names.into_iter().map(Value::Str).collect()),
    );
    record.insert("n_seqs".to_string(), Value::Int(n_seqs as i64));
    record.insert("length".to_string(), Value::Int(length as i64));

    Ok(Value::Record(record))
}

/// Progressive alignment: align each sequence to a growing consensus/profile.
fn progressive_align(seqs: &[(String, String)]) -> Result<Value> {
    let params = AlignParams::default();

    // Start with the first sequence as the initial "aligned" set
    let mut aligned: Vec<String> = vec![seqs[0].1.clone()];
    let mut names: Vec<String> = vec![seqs[0].0.clone()];

    for (name, seq) in &seqs[1..] {
        // Build consensus from current aligned sequences
        let consensus = build_consensus(&aligned);

        // Align the new sequence against the consensus
        let result = alignment::align(&consensus, seq, &params);

        // Use the alignment to insert gaps into all existing aligned sequences
        // and the new sequence
        let mut new_aligned = Vec::with_capacity(aligned.len() + 1);

        // Map gaps from the consensus alignment to existing sequences
        let consensus_aligned = result.aligned1.as_bytes();
        let new_seq_aligned = result.aligned2.clone();

        for existing in &aligned {
            let regapped = transfer_gaps(existing, consensus_aligned);
            new_aligned.push(regapped);
        }
        new_aligned.push(new_seq_aligned);

        aligned = new_aligned;
        names.push(name.clone());
    }

    // Ensure all sequences are the same length (pad with gaps if needed)
    let max_len = aligned.iter().map(|s| s.len()).max().unwrap_or(0);
    for seq in &mut aligned {
        while seq.len() < max_len {
            seq.push('-');
        }
    }

    let n_seqs = aligned.len();
    let length = max_len;

    let mut record = HashMap::new();
    record.insert(
        "sequences".to_string(),
        Value::List(aligned.into_iter().map(Value::Str).collect()),
    );
    record.insert(
        "names".to_string(),
        Value::List(names.into_iter().map(Value::Str).collect()),
    );
    record.insert("n_seqs".to_string(), Value::Int(n_seqs as i64));
    record.insert("length".to_string(), Value::Int(length as i64));

    Ok(Value::Record(record))
}

/// Build a simple consensus from aligned sequences (majority-rule).
fn build_consensus(aligned: &[String]) -> String {
    if aligned.is_empty() {
        return String::new();
    }
    let len = aligned[0].len();
    let mut consensus = String::with_capacity(len);

    for col in 0..len {
        let mut counts: HashMap<u8, usize> = HashMap::new();
        for seq in aligned {
            let b = seq.as_bytes().get(col).copied().unwrap_or(b'-');
            if b != b'-' {
                *counts.entry(b.to_ascii_uppercase()).or_insert(0) += 1;
            }
        }
        if counts.is_empty() {
            consensus.push('-');
        } else {
            let best = counts.into_iter().max_by_key(|&(_, c)| c).unwrap().0;
            consensus.push(best as char);
        }
    }

    consensus
}

/// Transfer gap pattern from a consensus alignment back to an original sequence.
/// `original` is the ungapped (or previously gapped) sequence.
/// `consensus_aligned` is the aligned consensus with gaps inserted.
/// We walk through the consensus alignment: where it has a gap, we insert a gap;
/// where it has a character, we consume the next character from `original`.
fn transfer_gaps(original: &str, consensus_aligned: &[u8]) -> String {
    let orig_bytes = original.as_bytes();
    let mut result = String::with_capacity(consensus_aligned.len());
    let mut orig_idx = 0;

    for &b in consensus_aligned {
        if b == b'-' {
            // Gap in consensus alignment => insert gap in this sequence too
            result.push('-');
        } else {
            // Consume next character from original
            if orig_idx < orig_bytes.len() {
                result.push(orig_bytes[orig_idx] as char);
                orig_idx += 1;
            } else {
                result.push('-');
            }
        }
    }

    result
}

// ── Distance Matrix ────────────────────────────────────────────────

fn builtin_distance_matrix(args: Vec<Value>) -> Result<Value> {
    let seqs = extract_aligned_sequences(&args[0], "distance_matrix")?;

    // Parse optional opts
    let model = if args.len() > 1 {
        if let Value::Record(opts) = &args[1] {
            match opts.get("model") {
                Some(Value::Str(m)) => m.clone(),
                _ => "p_distance".to_string(),
            }
        } else {
            "p_distance".to_string()
        }
    } else {
        "p_distance".to_string()
    };

    let n = seqs.len();
    let mut data = vec![0.0f64; n * n];

    for i in 0..n {
        for j in (i + 1)..n {
            let dist = pairwise_distance(&seqs[i].1, &seqs[j].1, &model)?;
            data[i * n + j] = dist;
            data[j * n + i] = dist;
        }
    }

    let names: Vec<String> = seqs.iter().map(|(name, _)| name.clone()).collect();
    let mut m = bl_core::matrix::Matrix::new(data, n, n)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    m.row_names = Some(names.clone());
    m.col_names = Some(names);

    Ok(Value::Matrix(m))
}

/// Compute pairwise distance between two aligned sequences.
fn pairwise_distance(seq1: &str, seq2: &str, model: &str) -> Result<f64> {
    let a = seq1.as_bytes();
    let b = seq2.as_bytes();
    let len = a.len().min(b.len());

    let mut diffs = 0usize;
    let mut compared = 0usize;

    for i in 0..len {
        // Skip positions where either has a gap
        if a[i] == b'-' || b[i] == b'-' {
            continue;
        }
        compared += 1;
        if !a[i].eq_ignore_ascii_case(&b[i]) {
            diffs += 1;
        }
    }

    if compared == 0 {
        return Ok(1.0); // no comparable positions
    }

    match model {
        "hamming" => Ok(diffs as f64),
        "p_distance" => Ok(diffs as f64 / compared as f64),
        "jc" => {
            let p = diffs as f64 / compared as f64;
            let inner = 1.0 - (4.0 / 3.0) * p;
            if inner <= 0.0 {
                // Saturation — return a large distance
                Ok(f64::INFINITY)
            } else {
                Ok(-0.75 * inner.ln())
            }
        }
        _ => Err(BioLangError::type_error(
            format!(
                "distance_matrix() unknown model '{model}', expected 'hamming', 'p_distance', or 'jc'"
            ),
            None,
        )),
    }
}

// ── Conservation Scores ────────────────────────────────────────────

fn builtin_conservation_scores(args: Vec<Value>) -> Result<Value> {
    let seqs = extract_aligned_sequences(&args[0], "conservation_scores")?;

    if seqs.is_empty() {
        return Ok(Value::List(vec![]));
    }

    let seq_strs: Vec<&[u8]> = seqs.iter().map(|(_, s)| s.as_bytes()).collect();
    let length = seq_strs.iter().map(|s| s.len()).max().unwrap_or(0);

    let mut scores = Vec::with_capacity(length);

    for col in 0..length {
        // Count character frequencies at this column (ignoring gaps)
        let mut counts: HashMap<u8, usize> = HashMap::new();
        let mut total = 0usize;

        for seq in &seq_strs {
            let b = seq.get(col).copied().unwrap_or(b'-');
            if b != b'-' {
                *counts.entry(b.to_ascii_uppercase()).or_insert(0) += 1;
                total += 1;
            }
        }

        if total == 0 {
            scores.push(Value::Float(0.0));
            continue;
        }

        let n_chars = counts.len();
        if n_chars <= 1 {
            // All the same character (or only one type) => perfectly conserved
            scores.push(Value::Float(1.0));
            continue;
        }

        // Shannon entropy: H = -sum(p * log2(p))
        let mut entropy = 0.0f64;
        for &count in counts.values() {
            let p = count as f64 / total as f64;
            if p > 0.0 {
                entropy -= p * p.log2();
            }
        }

        // Normalize: max possible entropy = log2(n_unique_chars)
        // But we want conservation relative to the alphabet size observed.
        // Use log2(min(total, 20)) as a reasonable maximum (20 for proteins, 4 for DNA).
        // A simpler approach: normalize by log2(n_chars) for the observed diversity.
        let max_entropy = (n_chars as f64).log2();
        let conservation = if max_entropy > 0.0 {
            1.0 - (entropy / max_entropy)
        } else {
            1.0
        };

        scores.push(Value::Float(conservation));
    }

    Ok(Value::List(scores))
}

