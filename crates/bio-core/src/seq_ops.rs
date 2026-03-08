//! Pure sequence algorithms operating on `&str`. No framework dependencies.

/// Detected molecule type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoleculeType {
    Dna,
    Rna,
    Ambiguous,
}

/// An open reading frame found in a nucleotide sequence.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Orf {
    pub start: usize,
    pub end: usize,
    pub frame: usize,
    pub protein: String,
}

// ── Validation ───────────────────────────────────────────────────────

/// Returns `true` if every character is a valid DNA base (A, T, G, C, N), case-insensitive.
pub fn is_valid_dna(seq: &str) -> bool {
    seq.bytes()
        .all(|b| matches!(b.to_ascii_uppercase(), b'A' | b'T' | b'G' | b'C' | b'N'))
}

/// Returns `true` if every character is a valid RNA base (A, U, G, C, N), case-insensitive.
pub fn is_valid_rna(seq: &str) -> bool {
    seq.bytes()
        .all(|b| matches!(b.to_ascii_uppercase(), b'A' | b'U' | b'G' | b'C' | b'N'))
}

/// Returns `true` if every character is a valid nucleotide (A, C, G, T, U, N, -), case-insensitive.
pub fn is_valid_nucleotide(seq: &str) -> bool {
    seq.bytes()
        .all(|b| matches!(b.to_ascii_uppercase(), b'A' | b'C' | b'G' | b'T' | b'U' | b'N' | b'-'))
}

/// Detect whether a sequence is DNA, RNA, or ambiguous.
pub fn detect_molecule(seq: &str) -> MoleculeType {
    let has_t = seq.bytes().any(|b| b == b'T' || b == b't');
    let has_u = seq.bytes().any(|b| b == b'U' || b == b'u');
    match (has_t, has_u) {
        (true, false) => MoleculeType::Dna,
        (false, true) => MoleculeType::Rna,
        _ => MoleculeType::Ambiguous,
    }
}

// ── Transcription ────────────────────────────────────────────────────

/// Transcribe DNA to RNA (T → U), preserving case.
pub fn transcribe(dna: &str) -> String {
    dna.bytes()
        .map(|b| match b {
            b'T' => b'U',
            b't' => b'u',
            _ => b,
        })
        .map(|b| b as char)
        .collect()
}

/// Back-transcribe RNA to DNA (U → T), preserving case.
pub fn back_transcribe(rna: &str) -> String {
    rna.bytes()
        .map(|b| match b {
            b'U' => b'T',
            b'u' => b't',
            _ => b,
        })
        .map(|b| b as char)
        .collect()
}

// ── Complement ───────────────────────────────────────────────────────

/// DNA complement: A↔T, G↔C. Non-ATGC characters pass through unchanged.
pub fn complement_dna(dna: &str) -> String {
    dna.bytes()
        .map(|b| match b {
            b'A' => b'T',
            b'T' => b'A',
            b'G' => b'C',
            b'C' => b'G',
            b'a' => b't',
            b't' => b'a',
            b'g' => b'c',
            b'c' => b'g',
            _ => b,
        })
        .map(|b| b as char)
        .collect()
}

/// RNA complement: A↔U, G↔C. Non-AUGC characters pass through unchanged.
pub fn complement_rna(rna: &str) -> String {
    rna.bytes()
        .map(|b| match b {
            b'A' => b'U',
            b'U' => b'A',
            b'G' => b'C',
            b'C' => b'G',
            b'a' => b'u',
            b'u' => b'a',
            b'g' => b'c',
            b'c' => b'g',
            _ => b,
        })
        .map(|b| b as char)
        .collect()
}

/// Complement that auto-detects DNA vs RNA. Falls back to DNA rules for ambiguous input.
pub fn complement(seq: &str) -> String {
    match detect_molecule(seq) {
        MoleculeType::Rna => complement_rna(seq),
        _ => complement_dna(seq),
    }
}

/// DNA reverse complement.
pub fn reverse_complement_dna(dna: &str) -> String {
    complement_dna(dna).chars().rev().collect()
}

/// RNA reverse complement.
pub fn reverse_complement_rna(rna: &str) -> String {
    complement_rna(rna).chars().rev().collect()
}

/// Reverse complement that auto-detects DNA vs RNA. Falls back to DNA rules for ambiguous input.
pub fn reverse_complement(seq: &str) -> String {
    match detect_molecule(seq) {
        MoleculeType::Rna => reverse_complement_rna(seq),
        _ => reverse_complement_dna(seq),
    }
}

// ── Metrics ──────────────────────────────────────────────────────────

/// GC content as a fraction in [0.0, 1.0].
pub fn gc_content(seq: &str) -> f64 {
    if seq.is_empty() {
        return 0.0;
    }
    let gc = seq
        .bytes()
        .filter(|b| matches!(b.to_ascii_uppercase(), b'G' | b'C'))
        .count();
    gc as f64 / seq.len() as f64
}

/// N content as a fraction in [0.0, 1.0].
pub fn n_content(seq: &str) -> f64 {
    if seq.is_empty() {
        return 0.0;
    }
    let n = seq
        .bytes()
        .filter(|b| b.eq_ignore_ascii_case(&b'N'))
        .count();
    n as f64 / seq.len() as f64
}

/// Effective length: count of characters excluding whitespace and gap characters ('-').
pub fn effective_length(seq: &str) -> usize {
    seq.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .count()
}

// ── Translation ──────────────────────────────────────────────────────

/// Translate a single codon to its amino acid. Accepts both DNA (ATG) and RNA (AUG) codons.
/// Returns 'X' for unknown/incomplete codons, '*' for stop codons.
pub fn codon_to_amino_acid(codon: &str) -> char {
    // Normalize to uppercase RNA for lookup
    let upper: String = codon
        .bytes()
        .map(|b| match b.to_ascii_uppercase() {
            b'T' => b'U',
            other => other,
        })
        .map(|b| b as char)
        .collect();

    match upper.as_str() {
        "UUU" | "UUC" => 'F',
        "UUA" | "UUG" | "CUU" | "CUC" | "CUA" | "CUG" => 'L',
        "AUU" | "AUC" | "AUA" => 'I',
        "AUG" => 'M',
        "GUU" | "GUC" | "GUA" | "GUG" => 'V',
        "UCU" | "UCC" | "UCA" | "UCG" | "AGU" | "AGC" => 'S',
        "CCU" | "CCC" | "CCA" | "CCG" => 'P',
        "ACU" | "ACC" | "ACA" | "ACG" => 'T',
        "GCU" | "GCC" | "GCA" | "GCG" => 'A',
        "UAU" | "UAC" => 'Y',
        "UAA" | "UAG" | "UGA" => '*',
        "CAU" | "CAC" => 'H',
        "CAA" | "CAG" => 'Q',
        "AAU" | "AAC" => 'N',
        "AAA" | "AAG" => 'K',
        "GAU" | "GAC" => 'D',
        "GAA" | "GAG" => 'E',
        "UGU" | "UGC" => 'C',
        "UGG" => 'W',
        "CGU" | "CGC" | "CGA" | "CGG" | "AGA" | "AGG" => 'R',
        "GGU" | "GGC" | "GGA" | "GGG" => 'G',
        _ => 'X',
    }
}

/// Full translation: translates the entire sequence, including '*' for stop codons.
/// Accepts both DNA and RNA input.
pub fn translate(seq: &str) -> String {
    let upper = seq.to_uppercase();
    (0..upper.len() / 3)
        .map(|i| &upper[i * 3..i * 3 + 3])
        .map(codon_to_amino_acid)
        .collect()
}

/// Translate until the first stop codon. Returns protein without the '*'.
/// Accepts both DNA and RNA input.
pub fn translate_to_stop(seq: &str) -> String {
    let upper = seq.to_uppercase();
    (0..upper.len() / 3)
        .map(|i| &upper[i * 3..i * 3 + 3])
        .map(codon_to_amino_acid)
        .take_while(|&aa| aa != '*')
        .collect()
}

// ── Search ───────────────────────────────────────────────────────────

/// Find all positions where `motif` occurs in `seq` (case-insensitive).
pub fn find_motif(seq: &str, motif: &str) -> Vec<usize> {
    let seq_upper = seq.to_uppercase();
    let motif_upper = motif.to_uppercase();
    let motif_len = motif_upper.len();
    if motif_len == 0 || motif_len > seq_upper.len() {
        return Vec::new();
    }
    (0..=seq_upper.len() - motif_len)
        .filter(|&i| seq_upper[i..i + motif_len] == motif_upper)
        .collect()
}

/// Extract all k-mers from the sequence. Returns slices of the original string.
pub fn kmers(seq: &str, k: usize) -> Vec<&str> {
    if k == 0 || k > seq.len() {
        return Vec::new();
    }
    (0..=seq.len() - k).map(|i| &seq[i..i + k]).collect()
}

// ── ORF finding ──────────────────────────────────────────────────────

/// Find ORFs in all 3 forward reading frames. `min_length` is in nucleotides.
/// Handles both DNA and RNA start/stop codons.
pub fn find_orfs(seq: &str, min_length: usize) -> Vec<Orf> {
    let mut orfs = Vec::new();
    for frame in 0..3 {
        orfs.extend(find_orfs_in_frame(seq, frame, min_length));
    }
    orfs
}

/// Find ORFs in a specific reading frame (0, 1, or 2). `min_length` is in nucleotides.
pub fn find_orfs_in_frame(seq: &str, frame: usize, min_length: usize) -> Vec<Orf> {
    let upper = seq.to_uppercase();
    let mut orfs = Vec::new();
    let mut i = frame;

    while i + 3 <= upper.len() {
        let codon = &upper[i..i + 3];
        // Start codons: ATG (DNA) or AUG (RNA)
        if codon == "ATG" || codon == "AUG" {
            let start = i;
            let mut j = i + 3;
            while j + 3 <= upper.len() {
                let c = &upper[j..j + 3];
                let is_stop = matches!(c, "TAA" | "TAG" | "TGA" | "UAA" | "UAG" | "UGA");
                if is_stop {
                    let orf_len = j + 3 - start;
                    if orf_len >= min_length {
                        let protein = translate_to_stop(&upper[start..j]);
                        orfs.push(Orf {
                            start,
                            end: j + 3,
                            frame,
                            protein,
                        });
                    }
                    i = j + 3;
                    break;
                }
                j += 3;
            }
            if j + 3 > upper.len() {
                break;
            }
        } else {
            i += 3;
        }
    }

    orfs
}
