use std::fmt;

/// Compressed Sparse Row (CSR) matrix.
///
/// Used for single-cell RNA-seq count matrices and other sparse biological data.
#[derive(Debug, Clone)]
pub struct SparseMatrix {
    /// Row pointers: indptr[i] is the index into indices/data where row i starts.
    /// Length = nrow + 1
    pub indptr: Vec<usize>,
    /// Column indices of non-zero entries.
    pub indices: Vec<usize>,
    /// Non-zero values.
    pub data: Vec<f64>,
    /// Number of rows.
    pub nrow: usize,
    /// Number of columns.
    pub ncol: usize,
    /// Optional row names (e.g., gene names).
    pub row_names: Option<Vec<String>>,
    /// Optional column names (e.g., cell barcodes).
    pub col_names: Option<Vec<String>>,
}

impl SparseMatrix {
    /// Create from triplets (row, col, val).
    pub fn from_triplets(
        rows: &[usize],
        cols: &[usize],
        vals: &[f64],
        nrow: usize,
        ncol: usize,
    ) -> Self {
        assert_eq!(rows.len(), cols.len());
        assert_eq!(rows.len(), vals.len());

        // Sort by row, then column
        let mut entries: Vec<(usize, usize, f64)> = rows
            .iter()
            .zip(cols.iter())
            .zip(vals.iter())
            .map(|((&r, &c), &v)| (r, c, v))
            .collect();
        entries.sort_by_key(|&(r, c, _)| (r, c));

        let nnz = entries.len();
        let mut indptr = vec![0usize; nrow + 1];
        let mut indices = Vec::with_capacity(nnz);
        let mut data = Vec::with_capacity(nnz);

        for &(r, c, v) in &entries {
            indptr[r + 1] += 1;
            indices.push(c);
            data.push(v);
        }

        // Cumulative sum for indptr
        for i in 1..=nrow {
            indptr[i] += indptr[i - 1];
        }

        SparseMatrix {
            indptr,
            indices,
            data,
            nrow,
            ncol,
            row_names: None,
            col_names: None,
        }
    }

    /// Create from a dense matrix (skip zeros).
    pub fn from_dense(dense: &[Vec<f64>]) -> Self {
        let nrow = dense.len();
        let ncol = if nrow > 0 { dense[0].len() } else { 0 };

        let mut rows = Vec::new();
        let mut cols = Vec::new();
        let mut vals = Vec::new();

        for (i, row) in dense.iter().enumerate() {
            for (j, &v) in row.iter().enumerate() {
                if v != 0.0 {
                    rows.push(i);
                    cols.push(j);
                    vals.push(v);
                }
            }
        }

        Self::from_triplets(&rows, &cols, &vals, nrow, ncol)
    }

    /// Get the value at (i, j). Returns 0.0 if not stored.
    pub fn get(&self, i: usize, j: usize) -> f64 {
        if i >= self.nrow {
            return 0.0;
        }
        let start = self.indptr[i];
        let end = self.indptr[i + 1];
        for idx in start..end {
            if self.indices[idx] == j {
                return self.data[idx];
            }
        }
        0.0
    }

    /// Number of non-zero entries.
    pub fn nnz(&self) -> usize {
        self.data.len()
    }

    /// Row sums.
    pub fn row_sums(&self) -> Vec<f64> {
        let mut sums = vec![0.0; self.nrow];
        for i in 0..self.nrow {
            let start = self.indptr[i];
            let end = self.indptr[i + 1];
            sums[i] = self.data[start..end].iter().sum();
        }
        sums
    }

    /// Column sums.
    pub fn col_sums(&self) -> Vec<f64> {
        let mut sums = vec![0.0; self.ncol];
        for window in self.indptr.windows(2) {
            let (start, end) = (window[0], window[1]);
            for pos in start..end {
                sums[self.indices[pos]] += self.data[pos];
            }
        }
        sums
    }

    /// Convert to dense representation.
    pub fn to_dense(&self) -> Vec<Vec<f64>> {
        let mut dense = vec![vec![0.0; self.ncol]; self.nrow];
        for (i, row) in dense.iter_mut().enumerate() {
            let start = self.indptr[i];
            let end = self.indptr[i + 1];
            for pos in start..end {
                row[self.indices[pos]] = self.data[pos];
            }
        }
        dense
    }

    /// Normalize: log1p(CPM) — counts per million with log1p transform.
    pub fn normalize_log1p_cpm(&self) -> SparseMatrix {
        let col_sums = self.col_sums();
        let mut new_data = self.data.clone();

        for i in 0..self.nrow {
            let start = self.indptr[i];
            let end = self.indptr[i + 1];
            for pos in start..end {
                let j = self.indices[pos];
                let total = col_sums[j];
                if total > 0.0 {
                    new_data[pos] = (self.data[pos] / total * 1e6 + 1.0).ln();
                }
            }
        }

        SparseMatrix {
            indptr: self.indptr.clone(),
            indices: self.indices.clone(),
            data: new_data,
            nrow: self.nrow,
            ncol: self.ncol,
            row_names: self.row_names.clone(),
            col_names: self.col_names.clone(),
        }
    }

    /// Normalize: scale each row to zero mean, unit variance.
    pub fn normalize_scale(&self) -> SparseMatrix {
        let mut new_data = self.data.clone();

        for i in 0..self.nrow {
            let start = self.indptr[i];
            let end = self.indptr[i + 1];
            let nnz_row = end - start;
            if nnz_row == 0 {
                continue;
            }

            let sum: f64 = self.data[start..end].iter().sum();
            // Mean includes zeros
            let mean = sum / self.ncol as f64;

            // Variance includes zeros
            let mut var: f64 = self.data[start..end]
                .iter()
                .map(|&v| (v - mean).powi(2))
                .sum();
            // Add contribution of zeros
            var += (self.ncol - nnz_row) as f64 * mean * mean;
            var /= self.ncol as f64;
            let std_dev = var.sqrt();

            if std_dev > 1e-10 {
                for pos in start..end {
                    new_data[pos] = (self.data[pos] - mean) / std_dev;
                }
            }
        }

        SparseMatrix {
            indptr: self.indptr.clone(),
            indices: self.indices.clone(),
            data: new_data,
            nrow: self.nrow,
            ncol: self.ncol,
            row_names: self.row_names.clone(),
            col_names: self.col_names.clone(),
        }
    }
}

impl fmt::Display for SparseMatrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SparseMatrix: {}x{} ({} non-zero, {:.1}% sparse)",
            self.nrow,
            self.ncol,
            self.nnz(),
            if self.nrow * self.ncol > 0 {
                (1.0 - self.nnz() as f64 / (self.nrow * self.ncol) as f64) * 100.0
            } else {
                100.0
            }
        )
    }
}

impl PartialEq for SparseMatrix {
    fn eq(&self, other: &Self) -> bool {
        self.nrow == other.nrow
            && self.ncol == other.ncol
            && self.indptr == other.indptr
            && self.indices == other.indices
            && self.data.len() == other.data.len()
            && self
                .data
                .iter()
                .zip(&other.data)
                .all(|(a, b)| (a - b).abs() < 1e-10)
    }
}
