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

    /// Number of stored non-zero entries in each row.
    pub fn row_nnz(&self) -> Vec<usize> {
        self.indptr
            .windows(2)
            .map(|window| window[1] - window[0])
            .collect()
    }

    /// Number of stored non-zero entries in each column.
    pub fn col_nnz(&self) -> Vec<usize> {
        let mut counts = vec![0usize; self.ncol];
        for &column in &self.indices {
            if column < self.ncol {
                counts[column] += 1;
            }
        }
        counts
    }

    /// Return a matrix containing the selected rows in the requested order.
    pub fn subset_rows(&self, rows: &[usize]) -> Self {
        let mut indptr = Vec::with_capacity(rows.len() + 1);
        let mut indices = Vec::new();
        let mut data = Vec::new();
        indptr.push(0);

        for &row in rows {
            if row < self.nrow {
                let start = self.indptr[row];
                let end = self.indptr[row + 1];
                indices.extend_from_slice(&self.indices[start..end]);
                data.extend_from_slice(&self.data[start..end]);
            }
            indptr.push(indices.len());
        }

        Self {
            indptr,
            indices,
            data,
            nrow: rows.len(),
            ncol: self.ncol,
            row_names: self.row_names.as_ref().map(|names| {
                rows.iter()
                    .map(|&row| names.get(row).cloned().unwrap_or_default())
                    .collect()
            }),
            col_names: self.col_names.clone(),
        }
    }

    /// Return a matrix containing the selected columns in the requested order.
    ///
    /// Repeated source columns are not supported; the first requested occurrence
    /// is retained.
    pub fn subset_cols(&self, columns: &[usize]) -> Self {
        let mut remap = vec![None; self.ncol];
        for (new_column, &old_column) in columns.iter().enumerate() {
            if old_column < self.ncol && remap[old_column].is_none() {
                remap[old_column] = Some(new_column);
            }
        }

        let mut indptr = Vec::with_capacity(self.nrow + 1);
        let mut indices = Vec::new();
        let mut data = Vec::new();
        indptr.push(0);

        for row in 0..self.nrow {
            let start = self.indptr[row];
            let end = self.indptr[row + 1];
            let mut selected = Vec::new();
            for position in start..end {
                if let Some(new_column) = remap[self.indices[position]] {
                    selected.push((new_column, self.data[position]));
                }
            }
            selected.sort_by_key(|(column, _)| *column);
            for (column, value) in selected {
                indices.push(column);
                data.push(value);
            }
            indptr.push(indices.len());
        }

        Self {
            indptr,
            indices,
            data,
            nrow: self.nrow,
            ncol: columns.len(),
            row_names: self.row_names.clone(),
            col_names: self.col_names.as_ref().map(|names| {
                columns
                    .iter()
                    .map(|&column| names.get(column).cloned().unwrap_or_default())
                    .collect()
            }),
        }
    }

    /// Scale each row so its sum equals `target`, preserving zero rows.
    pub fn normalize_rows(&self, target: f64) -> Self {
        let row_sums = self.row_sums();
        let mut data = self.data.clone();
        for (row, total) in row_sums.into_iter().enumerate() {
            if total == 0.0 {
                continue;
            }
            let scale = target / total;
            for value in &mut data[self.indptr[row]..self.indptr[row + 1]] {
                *value *= scale;
            }
        }

        Self {
            indptr: self.indptr.clone(),
            indices: self.indices.clone(),
            data,
            nrow: self.nrow,
            ncol: self.ncol,
            row_names: self.row_names.clone(),
            col_names: self.col_names.clone(),
        }
    }

    /// Apply a function to stored values while retaining the sparse structure.
    pub fn map_nonzero(&self, transform: impl Fn(f64) -> f64) -> Self {
        Self {
            indptr: self.indptr.clone(),
            indices: self.indices.clone(),
            data: self.data.iter().copied().map(transform).collect(),
            nrow: self.nrow,
            ncol: self.ncol,
            row_names: self.row_names.clone(),
            col_names: self.col_names.clone(),
        }
    }

    /// Append rows from another CSR matrix with the same columns.
    pub fn append_rows(&self, other: &Self) -> Result<Self, String> {
        if self.ncol != other.ncol {
            return Err(format!(
                "cannot append sparse matrices with {} and {} columns",
                self.ncol, other.ncol
            ));
        }
        if let (Some(left), Some(right)) = (&self.col_names, &other.col_names) {
            if left != right {
                return Err("cannot append sparse matrices with different column names".into());
            }
        }

        let mut indptr = self.indptr.clone();
        let offset = self.data.len();
        indptr.extend(
            other
                .indptr
                .iter()
                .skip(1)
                .map(|position| offset + position),
        );
        let mut indices = self.indices.clone();
        indices.extend_from_slice(&other.indices);
        let mut data = self.data.clone();
        data.extend_from_slice(&other.data);
        let row_names = match (&self.row_names, &other.row_names) {
            (Some(left), Some(right)) => {
                let mut names = left.clone();
                names.extend(right.iter().cloned());
                Some(names)
            }
            _ => None,
        };

        Ok(Self {
            indptr,
            indices,
            data,
            nrow: self.nrow + other.nrow,
            ncol: self.ncol,
            row_names,
            col_names: self.col_names.clone().or_else(|| other.col_names.clone()),
        })
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
