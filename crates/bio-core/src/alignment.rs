//! Pairwise sequence alignment algorithms: Needleman-Wunsch (global) and Smith-Waterman (local).

/// Alignment mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlignMode {
    Global,
    Local,
}

/// Parameters for pairwise alignment.
#[derive(Debug, Clone)]
pub struct AlignParams {
    pub match_score: i32,
    pub mismatch_score: i32,
    pub gap_open: i32,
    pub gap_extend: i32,
    pub mode: AlignMode,
}

impl Default for AlignParams {
    fn default() -> Self {
        Self {
            match_score: 2,
            mismatch_score: -1,
            gap_open: -5,
            gap_extend: -1,
            mode: AlignMode::Global,
        }
    }
}

/// Result of a pairwise alignment.
#[derive(Debug, Clone)]
pub struct AlignResult {
    pub aligned1: String,
    pub aligned2: String,
    pub score: i32,
    pub identity: f64,
    pub gaps: usize,
    pub cigar: String,
}

/// Perform pairwise alignment with affine gap penalties.
///
/// Uses three DP matrices (M, X, Y) for affine gaps.
/// Global = Needleman-Wunsch, Local = Smith-Waterman.
pub fn align(seq1: &str, seq2: &str, params: &AlignParams) -> AlignResult {
    let s1: Vec<u8> = seq1.as_bytes().to_vec();
    let s2: Vec<u8> = seq2.as_bytes().to_vec();
    let n = s1.len();
    let m = s2.len();

    let neg_inf = i32::MIN / 2;

    // M[i][j] = best score ending with s1[i-1] aligned to s2[j-1]
    // X[i][j] = best score ending with gap in seq2 (consuming s1)
    // Y[i][j] = best score ending with gap in seq1 (consuming s2)
    let mut mat_m = vec![vec![neg_inf; m + 1]; n + 1];
    let mut mat_x = vec![vec![neg_inf; m + 1]; n + 1];
    let mut mat_y = vec![vec![neg_inf; m + 1]; n + 1];

    let is_local = params.mode == AlignMode::Local;

    // Initialization
    mat_m[0][0] = 0;
    for i in 1..=n {
        if !is_local {
            mat_x[i][0] = params.gap_open + params.gap_extend * i as i32;
            mat_m[i][0] = mat_x[i][0];
        } else {
            mat_m[i][0] = 0;
        }
    }
    for j in 1..=m {
        if !is_local {
            mat_y[0][j] = params.gap_open + params.gap_extend * j as i32;
            mat_m[0][j] = mat_y[0][j];
        } else {
            mat_m[0][j] = 0;
        }
    }

    let mut max_score = 0;
    let mut max_i = 0;
    let mut max_j = 0;

    // Fill
    for i in 1..=n {
        for j in 1..=m {
            let s = if s1[i - 1].eq_ignore_ascii_case(&s2[j - 1]) {
                params.match_score
            } else {
                params.mismatch_score
            };

            mat_x[i][j] = (mat_m[i - 1][j] + params.gap_open + params.gap_extend)
                .max(mat_x[i - 1][j] + params.gap_extend);
            mat_y[i][j] = (mat_m[i][j - 1] + params.gap_open + params.gap_extend)
                .max(mat_y[i][j - 1] + params.gap_extend);

            mat_m[i][j] = (mat_m[i - 1][j - 1] + s)
                .max(mat_x[i][j])
                .max(mat_y[i][j]);

            if is_local {
                mat_m[i][j] = mat_m[i][j].max(0);
                mat_x[i][j] = mat_x[i][j].max(0);
                mat_y[i][j] = mat_y[i][j].max(0);
                if mat_m[i][j] > max_score {
                    max_score = mat_m[i][j];
                    max_i = i;
                    max_j = j;
                }
            }
        }
    }

    // Traceback
    let (mut i, mut j, score) = if is_local {
        (max_i, max_j, max_score)
    } else {
        (n, m, mat_m[n][m])
    };

    let mut aln1 = Vec::new();
    let mut aln2 = Vec::new();

    while i > 0 || j > 0 {
        if is_local && mat_m[i][j] == 0 {
            break;
        }

        if i > 0 && j > 0 {
            let s = if s1[i - 1].eq_ignore_ascii_case(&s2[j - 1]) {
                params.match_score
            } else {
                params.mismatch_score
            };
            if mat_m[i][j] == mat_m[i - 1][j - 1] + s {
                aln1.push(s1[i - 1]);
                aln2.push(s2[j - 1]);
                i -= 1;
                j -= 1;
                continue;
            }
        }
        if i > 0 && mat_m[i][j] == mat_x[i][j] {
            aln1.push(s1[i - 1]);
            aln2.push(b'-');
            i -= 1;
        } else if j > 0 {
            aln1.push(b'-');
            aln2.push(s2[j - 1]);
            j -= 1;
        } else {
            break;
        }
    }

    aln1.reverse();
    aln2.reverse();

    // Compute stats
    let mut matches = 0usize;
    let mut gaps = 0usize;
    let total = aln1.len();
    for k in 0..total {
        if aln1[k] == b'-' || aln2[k] == b'-' {
            gaps += 1;
        } else if aln1[k].eq_ignore_ascii_case(&aln2[k]) {
            matches += 1;
        }
    }
    let identity = if total > 0 {
        matches as f64 / total as f64
    } else {
        0.0
    };

    // Build CIGAR
    let cigar = build_cigar(&aln1, &aln2);

    AlignResult {
        aligned1: String::from_utf8_lossy(&aln1).to_string(),
        aligned2: String::from_utf8_lossy(&aln2).to_string(),
        score,
        identity,
        gaps,
        cigar,
    }
}

fn build_cigar(aln1: &[u8], aln2: &[u8]) -> String {
    if aln1.is_empty() {
        return String::new();
    }
    let mut cigar = String::new();
    let mut count = 1u32;
    let mut prev = cigar_op(aln1[0], aln2[0]);
    for k in 1..aln1.len() {
        let op = cigar_op(aln1[k], aln2[k]);
        if op == prev {
            count += 1;
        } else {
            cigar.push_str(&format!("{count}{prev}"));
            prev = op;
            count = 1;
        }
    }
    cigar.push_str(&format!("{count}{prev}"));
    cigar
}

fn cigar_op(a: u8, b: u8) -> char {
    if a == b'-' {
        'I'
    } else if b == b'-' {
        'D'
    } else if a.eq_ignore_ascii_case(&b) {
        'M'
    } else {
        'X'
    }
}

/// Levenshtein edit distance. O(min(n,m)) space.
pub fn edit_distance(seq1: &str, seq2: &str) -> usize {
    let a = seq1.as_bytes();
    let b = seq2.as_bytes();
    let (short, long) = if a.len() <= b.len() {
        (a, b)
    } else {
        (b, a)
    };

    let mut prev: Vec<usize> = (0..=short.len()).collect();
    let mut curr = vec![0; short.len() + 1];

    for i in 1..=long.len() {
        curr[0] = i;
        for j in 1..=short.len() {
            let cost = if long[i - 1].eq_ignore_ascii_case(&short[j - 1]) {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[short.len()]
}

/// Hamming distance. Requires equal-length sequences.
pub fn hamming_distance(seq1: &str, seq2: &str) -> Result<usize, String> {
    if seq1.len() != seq2.len() {
        return Err(format!(
            "hamming_distance requires equal-length sequences ({} vs {})",
            seq1.len(),
            seq2.len()
        ));
    }
    Ok(seq1
        .bytes()
        .zip(seq2.bytes())
        .filter(|(a, b)| !a.eq_ignore_ascii_case(b))
        .count())
}

// ── Scoring Matrices ────────────────────────────────────────────

/// Amino acid order for scoring matrices (alphabetical standard 20).
pub const AA_ORDER: &[u8; 20] = b"ACDEFGHIKLMNPQRSTVWY";

/// Map amino acid to index in scoring matrix.
pub fn aa_index(c: char) -> Option<usize> {
    AA_ORDER.iter().position(|&a| a == c.to_ascii_uppercase() as u8)
}

/// BLOSUM62 scoring matrix (20x20, AA_ORDER).
#[rustfmt::skip]
pub const BLOSUM62: [[i32; 20]; 20] = [
//   A   C   D   E   F   G   H   I   K   L   M   N   P   Q   R   S   T   V   W   Y
    [4, 0,-2,-1,-2, 0,-2,-1,-1,-1,-1,-2,-1,-1,-1, 1, 0, 0,-3,-2], // A
    [0, 9,-3,-4,-2,-3,-3,-1,-3,-1,-1,-3,-3,-3,-3,-1,-1,-1,-2,-2], // C
    [-2,-3, 6, 2,-3,-1,-1,-3,-1,-4,-3, 1,-1, 0,-2, 0,-1,-3,-4,-3], // D
    [-1,-4, 2, 5,-3,-2, 0,-3, 1,-3,-2, 0,-1, 2, 0, 0,-1,-2,-3,-2], // E
    [-2,-2,-3,-3, 6,-3,-1, 0,-3, 0, 0,-3,-4,-3,-3,-2,-2,-1, 1, 3], // F
    [0,-3,-1,-2,-3, 6,-2,-4,-2,-4,-3, 0,-2,-2,-2, 0,-2,-3,-2,-3], // G
    [-2,-3,-1, 0,-1,-2, 8,-3,-1,-3,-2, 1,-2, 0, 0,-1,-2,-3,-2, 2], // H
    [-1,-1,-3,-3, 0,-4,-3, 4,-3, 2, 1,-3,-3,-3,-3,-2,-1, 3,-3,-1], // I
    [-1,-3,-1, 1,-3,-2,-1,-3, 5,-2,-1, 0,-1, 1, 2, 0,-1,-2,-3,-2], // K
    [-1,-1,-4,-3, 0,-4,-3, 2,-2, 4, 2,-3,-3,-2,-2,-2,-1, 1,-2,-1], // L
    [-1,-1,-3,-2, 0,-3,-2, 1,-1, 2, 5,-2,-2, 0,-1,-1,-1, 1,-1,-1], // M
    [-2,-3, 1, 0,-3, 0, 1,-3, 0,-3,-2, 6,-2, 0, 0, 1, 0,-3,-4,-2], // N
    [-1,-3,-1,-1,-4,-2,-2,-3,-1,-3,-2,-2, 7,-1,-2,-1,-1,-2,-4,-3], // P
    [-1,-3, 0, 2,-3,-2, 0,-3, 1,-2, 0, 0,-1, 5, 1, 0,-1,-2,-2,-1], // Q
    [-1,-3,-2, 0,-3,-2, 0,-3, 2,-2,-1, 0,-2, 1, 5,-1,-1,-3,-3,-2], // R
    [1,-1, 0, 0,-2, 0,-1,-2, 0,-2,-1, 1,-1, 0,-1, 4, 1,-2,-3,-2], // S
    [0,-1,-1,-1,-2,-2,-2,-1,-1,-1,-1, 0,-1,-1,-1, 1, 5, 0,-2,-2], // T
    [0,-1,-3,-2,-1,-3,-3, 3,-2, 1, 1,-3,-2,-2,-3,-2, 0, 4,-3,-1], // V
    [-3,-2,-4,-3, 1,-2,-2,-3,-3,-2,-1,-4,-4,-2,-3,-3,-2,-3,11, 2], // W
    [-2,-2,-3,-2, 3,-3, 2,-1,-2,-1,-1,-2,-3,-1,-2,-2,-2,-1, 2, 7], // Y
];

/// PAM250 scoring matrix (20x20, AA_ORDER).
#[rustfmt::skip]
pub const PAM250: [[i32; 20]; 20] = [
//   A   C   D   E   F   G   H   I   K   L   M   N   P   Q   R   S   T   V   W   Y
    [2,-2, 0, 0,-4, 1,-1,-1,-1,-2,-1, 0, 1, 0,-2, 1, 1, 0,-6,-3], // A
    [-2,12,-5,-5,-4,-3,-3,-2,-5,-6,-5,-4,-3,-5,-4, 0,-2,-2,-8, 0], // C
    [0,-5, 4, 3,-6, 1, 1,-2, 0,-4,-3, 2,-1, 2,-1, 0, 0,-2,-7,-4], // D
    [0,-5, 3, 4,-5, 0, 1,-2, 0,-3,-2, 1,-1, 2,-1, 0, 0,-2,-7,-4], // E
    [-4,-4,-6,-5, 9,-5,-2, 1,-5, 2, 0,-4,-5,-5,-4,-3,-3,-1, 0, 7], // F
    [1,-3, 1, 0,-5, 5,-2,-3,-2,-4,-3, 0,-1,-1,-3, 1, 0,-1,-7,-5], // G
    [-1,-3, 1, 1,-2,-2, 6,-2, 0,-2,-2, 2, 0, 3, 2,-1,-1,-2,-3, 0], // H
    [-1,-2,-2,-2, 1,-3,-2, 5,-2, 2, 2,-2,-2,-2,-2,-1, 0, 4,-5,-1], // I
    [-1,-5, 0, 0,-5,-2, 0,-2, 5,-3, 0, 1,-1, 1, 3, 0, 0,-2,-3,-4], // K
    [-2,-6,-4,-3, 2,-4,-2, 2,-3, 6, 4,-3,-3,-2,-3,-3,-2, 2,-2,-1], // L
    [-1,-5,-3,-2, 0,-3,-2, 2, 0, 4, 6,-2,-2,-1, 0,-2,-1, 2,-4,-2], // M
    [0,-4, 2, 1,-4, 0, 2,-2, 1,-3,-2, 2,-1, 1, 0, 1, 0,-2,-4,-2], // N
    [1,-3,-1,-1,-5,-1, 0,-2,-1,-3,-2,-1, 6, 0, 0, 1, 0,-1,-6,-5], // P
    [0,-5, 2, 2,-5,-1, 3,-2, 1,-2,-1, 1, 0, 4, 1,-1,-1,-2,-5,-4], // Q
    [-2,-4,-1,-1,-4,-3, 2,-2, 3,-3, 0, 0, 0, 1, 6, 0,-1,-2, 2,-4], // R
    [1, 0, 0, 0,-3, 1,-1,-1, 0,-3,-2, 1, 1,-1, 0, 2, 1,-1,-2,-3], // S
    [1,-2, 0, 0,-3, 0,-1, 0, 0,-2,-1, 0, 0,-1,-1, 1, 3, 0,-5,-3], // T
    [0,-2,-2,-2,-1,-1,-2, 4,-2, 2, 2,-2,-1,-2,-2,-1, 0, 4,-6,-2], // V
    [-6,-8,-7,-7, 0,-7,-3,-5,-3,-2,-4,-4,-6,-5, 2,-2,-5,-6,17, 0], // W
    [-3, 0,-4,-4, 7,-5, 0,-1,-4,-1,-2,-2,-5,-4,-4,-3,-3,-2, 0,10], // Y
];

/// BLOSUM45 scoring matrix (20x20, AA_ORDER).
#[rustfmt::skip]
pub const BLOSUM45: [[i32; 20]; 20] = [
//   A   C   D   E   F   G   H   I   K   L   M   N   P   Q   R   S   T   V   W   Y
    [5,-1,-2,-1,-2, 0,-2,-1,-1,-1,-1,-1,-1,-1,-2, 1, 0, 0,-2,-2], // A
    [-1,12,-3,-3,-2,-3,-3,-3,-3,-2,-2,-2,-4,-3,-3,-1,-1,-1,-5,-3], // C
    [-2,-3, 7, 2,-4,-1, 0,-4, 0,-3,-3, 2,-1, 0,-1, 0,-1,-3,-4,-2], // D
    [-1,-3, 2, 6,-3,-2, 0,-3, 1,-2,-2, 0, 0, 2, 0, 0,-1,-3,-3,-2], // E
    [-2,-2,-4,-3, 8,-3,-2, 0,-3, 1, 0,-2,-3,-4,-2,-2,-1, 0, 1, 3], // F
    [0,-3,-1,-2,-3, 7,-2,-4,-2,-3,-2, 0,-2,-2,-2, 0,-2,-3,-2,-3], // G
    [-2,-3, 0, 0,-2,-2,10,-3, 0,-2, 0, 1,-2, 1, 0,-1,-2,-3,-3, 2], // H
    [-1,-3,-4,-3, 0,-4,-3, 5,-3, 2, 2,-2,-2,-2,-3,-2,-1, 3,-2, 0], // I
    [-1,-3, 0, 1,-3,-2, 0,-3, 5,-3,-1, 0,-1, 1, 3,-1,-1,-2,-2,-1], // K
    [-1,-2,-3,-2, 1,-3,-2, 2,-3, 5, 2,-3,-3,-2,-2,-3,-1, 1,-2, 0], // L
    [-1,-2,-3,-2, 0,-2, 0, 2,-1, 2, 6,-2,-2, 0,-1,-2,-1, 1,-2, 0], // M
    [-1,-2, 2, 0,-2, 0, 1,-2, 0,-3,-2, 6,-2, 0, 0, 1, 0,-3,-4,-2], // N
    [-1,-4,-1, 0,-3,-2,-2,-2,-1,-3,-2,-2, 9,-1,-2,-1,-1,-3,-3,-3], // P
    [-1,-3, 0, 2,-4,-2, 1,-2, 1,-2, 0, 0,-1, 6, 1, 0,-1,-3,-2,-1], // Q
    [-2,-3,-1, 0,-2,-2, 0,-3, 3,-2,-1, 0,-2, 1, 7,-1,-1,-2,-2,-1], // R
    [1,-1, 0, 0,-2, 0,-1,-2, 0,-3,-2, 1,-1, 0,-1, 4, 2,-1,-4,-2], // S
    [0,-1,-1,-1,-1,-2,-2,-1,-1,-1,-1, 0,-1,-1,-1, 2, 5, 0,-3,-1], // T
    [0,-1,-3,-3, 0,-3,-3, 3,-2, 1, 1,-3,-3,-3,-2,-1, 0, 5,-3,-1], // V
    [-2,-5,-4,-3, 1,-2,-3,-2,-2,-2,-2,-4,-3,-2,-2,-4,-3,-3,15, 3], // W
    [-2,-3,-2,-2, 3,-3, 2, 0,-1, 0, 0,-2,-3,-1,-1,-2,-1,-1, 3, 8], // Y
];

/// Get scoring matrix by name.
pub fn scoring_matrix(name: &str) -> Option<&'static [[i32; 20]; 20]> {
    match name.to_lowercase().as_str() {
        "blosum62" => Some(&BLOSUM62),
        "pam250" => Some(&PAM250),
        "blosum45" => Some(&BLOSUM45),
        _ => None,
    }
}
