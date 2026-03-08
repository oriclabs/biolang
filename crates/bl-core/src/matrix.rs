use std::fmt;

/// Row-major dense matrix of f64 values.
#[derive(Debug, Clone)]
pub struct Matrix {
    pub data: Vec<f64>,
    pub nrow: usize,
    pub ncol: usize,
    pub row_names: Option<Vec<String>>,
    pub col_names: Option<Vec<String>>,
}

impl Matrix {
    pub fn new(data: Vec<f64>, nrow: usize, ncol: usize) -> Result<Self, String> {
        if data.len() != nrow * ncol {
            return Err(format!(
                "data length {} != nrow({}) * ncol({})",
                data.len(),
                nrow,
                ncol
            ));
        }
        Ok(Self {
            data,
            nrow,
            ncol,
            row_names: None,
            col_names: None,
        })
    }

    pub fn zeros(nrow: usize, ncol: usize) -> Self {
        Self {
            data: vec![0.0; nrow * ncol],
            nrow,
            ncol,
            row_names: None,
            col_names: None,
        }
    }

    pub fn identity(n: usize) -> Self {
        let mut data = vec![0.0; n * n];
        for i in 0..n {
            data[i * n + i] = 1.0;
        }
        Self {
            data,
            nrow: n,
            ncol: n,
            row_names: None,
            col_names: None,
        }
    }

    pub fn get(&self, i: usize, j: usize) -> f64 {
        self.data[i * self.ncol + j]
    }

    pub fn set(&mut self, i: usize, j: usize, v: f64) {
        self.data[i * self.ncol + j] = v;
    }

    pub fn row(&self, i: usize) -> Vec<f64> {
        let start = i * self.ncol;
        self.data[start..start + self.ncol].to_vec()
    }

    pub fn col(&self, j: usize) -> Vec<f64> {
        (0..self.nrow).map(|i| self.data[i * self.ncol + j]).collect()
    }

    pub fn transpose(&self) -> Self {
        let mut data = vec![0.0; self.nrow * self.ncol];
        for i in 0..self.nrow {
            for j in 0..self.ncol {
                data[j * self.nrow + i] = self.data[i * self.ncol + j];
            }
        }
        Self {
            data,
            nrow: self.ncol,
            ncol: self.nrow,
            row_names: self.col_names.clone(),
            col_names: self.row_names.clone(),
        }
    }

    pub fn add(&self, other: &Self) -> Result<Self, String> {
        if self.nrow != other.nrow || self.ncol != other.ncol {
            return Err("dimension mismatch for add".into());
        }
        let data: Vec<f64> = self.data.iter().zip(&other.data).map(|(a, b)| a + b).collect();
        Ok(Self {
            data,
            nrow: self.nrow,
            ncol: self.ncol,
            row_names: self.row_names.clone(),
            col_names: self.col_names.clone(),
        })
    }

    pub fn sub(&self, other: &Self) -> Result<Self, String> {
        if self.nrow != other.nrow || self.ncol != other.ncol {
            return Err("dimension mismatch for sub".into());
        }
        let data: Vec<f64> = self.data.iter().zip(&other.data).map(|(a, b)| a - b).collect();
        Ok(Self {
            data,
            nrow: self.nrow,
            ncol: self.ncol,
            row_names: self.row_names.clone(),
            col_names: self.col_names.clone(),
        })
    }

    pub fn mul_elementwise(&self, other: &Self) -> Result<Self, String> {
        if self.nrow != other.nrow || self.ncol != other.ncol {
            return Err("dimension mismatch for mul".into());
        }
        let data: Vec<f64> = self.data.iter().zip(&other.data).map(|(a, b)| a * b).collect();
        Ok(Self {
            data,
            nrow: self.nrow,
            ncol: self.ncol,
            row_names: self.row_names.clone(),
            col_names: self.col_names.clone(),
        })
    }

    pub fn scale(&self, s: f64) -> Self {
        let data: Vec<f64> = self.data.iter().map(|v| v * s).collect();
        Self {
            data,
            nrow: self.nrow,
            ncol: self.ncol,
            row_names: self.row_names.clone(),
            col_names: self.col_names.clone(),
        }
    }

    /// Matrix multiplication (dot product).
    pub fn dot(&self, other: &Self) -> Result<Self, String> {
        if self.ncol != other.nrow {
            return Err(format!(
                "dot: incompatible dimensions {}x{} and {}x{}",
                self.nrow, self.ncol, other.nrow, other.ncol
            ));
        }
        let mut data = vec![0.0; self.nrow * other.ncol];
        for i in 0..self.nrow {
            for j in 0..other.ncol {
                let mut sum = 0.0;
                for k in 0..self.ncol {
                    sum += self.get(i, k) * other.get(k, j);
                }
                data[i * other.ncol + j] = sum;
            }
        }
        Ok(Self {
            data,
            nrow: self.nrow,
            ncol: other.ncol,
            row_names: self.row_names.clone(),
            col_names: other.col_names.clone(),
        })
    }

    pub fn row_sums(&self) -> Vec<f64> {
        (0..self.nrow)
            .map(|i| {
                let start = i * self.ncol;
                self.data[start..start + self.ncol].iter().sum()
            })
            .collect()
    }

    pub fn col_sums(&self) -> Vec<f64> {
        (0..self.ncol).map(|j| self.col(j).iter().sum()).collect()
    }

    pub fn row_means(&self) -> Vec<f64> {
        self.row_sums()
            .into_iter()
            .map(|s| s / self.ncol as f64)
            .collect()
    }

    pub fn col_means(&self) -> Vec<f64> {
        self.col_sums()
            .into_iter()
            .map(|s| s / self.nrow as f64)
            .collect()
    }

    /// Matrix trace (sum of diagonal elements). Requires square matrix.
    pub fn trace(&self) -> Result<f64, String> {
        if self.nrow != self.ncol {
            return Err(format!("trace requires square matrix, got {}x{}", self.nrow, self.ncol));
        }
        Ok((0..self.nrow).map(|i| self.get(i, i)).sum())
    }

    /// Frobenius norm (sqrt of sum of squared elements).
    pub fn norm(&self) -> f64 {
        self.data.iter().map(|v| v * v).sum::<f64>().sqrt()
    }

    /// Determinant via LU decomposition (partial pivoting). Requires square matrix.
    #[allow(clippy::needless_range_loop)]
    pub fn determinant(&self) -> Result<f64, String> {
        if self.nrow != self.ncol {
            return Err(format!("determinant requires square matrix, got {}x{}", self.nrow, self.ncol));
        }
        let n = self.nrow;
        if n == 0 {
            return Ok(1.0);
        }
        // Work on a copy
        let mut a: Vec<Vec<f64>> = (0..n).map(|i| self.row(i)).collect();
        let mut sign = 1.0;
        for col in 0..n {
            // Partial pivoting
            let mut max_row = col;
            let mut max_val = a[col][col].abs();
            for row in (col + 1)..n {
                let abs_val = a[row][col].abs();
                if abs_val > max_val {
                    max_val = abs_val;
                    max_row = row;
                }
            }
            if max_val < 1e-15 {
                return Ok(0.0);
            }
            if max_row != col {
                a.swap(col, max_row);
                sign = -sign;
            }
            let pivot = a[col][col];
            for row in (col + 1)..n {
                let factor = a[row][col] / pivot;
                let pivot_row = a[col].clone();
                for (j, pv) in pivot_row.iter().enumerate().skip(col) {
                    a[row][j] -= factor * pv;
                }
            }
        }
        let det: f64 = (0..n).map(|i| a[i][i]).product::<f64>() * sign;
        Ok(det)
    }

    /// Matrix inverse via Gauss-Jordan elimination. Requires square, non-singular matrix.
    #[allow(clippy::needless_range_loop)]
    pub fn inverse(&self) -> Result<Self, String> {
        if self.nrow != self.ncol {
            return Err(format!("inverse requires square matrix, got {}x{}", self.nrow, self.ncol));
        }
        let n = self.nrow;
        // Augmented matrix [A | I]
        let mut aug: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let mut row = self.row(i);
                let mut id = vec![0.0; n];
                id[i] = 1.0;
                row.extend(id);
                row
            })
            .collect();

        for col in 0..n {
            // Partial pivoting
            let mut max_row = col;
            let mut max_val = aug[col][col].abs();
            for row in (col + 1)..n {
                let abs_val = aug[row][col].abs();
                if abs_val > max_val {
                    max_val = abs_val;
                    max_row = row;
                }
            }
            if max_val < 1e-15 {
                return Err("matrix is singular".into());
            }
            if max_row != col {
                aug.swap(col, max_row);
            }
            let pivot = aug[col][col];
            for val in &mut aug[col] {
                *val /= pivot;
            }
            for row in 0..n {
                if row == col {
                    continue;
                }
                let factor = aug[row][col];
                let pivot_row = aug[col].clone();
                for (j, pv) in pivot_row.iter().enumerate() {
                    aug[row][j] -= factor * pv;
                }
            }
        }

        let data: Vec<f64> = aug.iter().flat_map(|row| row[n..].to_vec()).collect();
        Self::new(data, n, n)
    }

    /// Solve linear system Ax = b. A must be square, b is a column vector.
    #[allow(clippy::needless_range_loop)]
    pub fn solve(&self, b: &[f64]) -> Result<Vec<f64>, String> {
        if self.nrow != self.ncol {
            return Err(format!("solve requires square matrix, got {}x{}", self.nrow, self.ncol));
        }
        if b.len() != self.nrow {
            return Err(format!("solve: b length {} != matrix rows {}", b.len(), self.nrow));
        }
        let n = self.nrow;
        // Augmented [A | b]
        let mut aug: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let mut row = self.row(i);
                row.push(b[i]);
                row
            })
            .collect();

        for col in 0..n {
            let mut max_row = col;
            let mut max_val = aug[col][col].abs();
            for row in (col + 1)..n {
                let abs_val = aug[row][col].abs();
                if abs_val > max_val {
                    max_val = abs_val;
                    max_row = row;
                }
            }
            if max_val < 1e-15 {
                return Err("matrix is singular".into());
            }
            if max_row != col {
                aug.swap(col, max_row);
            }
            let pivot = aug[col][col];
            for val in &mut aug[col] {
                *val /= pivot;
            }
            for row in 0..n {
                if row == col {
                    continue;
                }
                let factor = aug[row][col];
                let pivot_row = aug[col].clone();
                for (j, pv) in pivot_row.iter().enumerate() {
                    aug[row][j] -= factor * pv;
                }
            }
        }

        Ok(aug.iter().map(|row| row[n]).collect())
    }

    /// Eigenvalues and eigenvectors via QR algorithm (for real symmetric matrices).
    /// Returns (eigenvalues, eigenvector_matrix) where columns are eigenvectors.
    #[allow(clippy::needless_range_loop)]
    pub fn eigen(&self) -> Result<(Vec<f64>, Self), String> {
        if self.nrow != self.ncol {
            return Err(format!("eigen requires square matrix, got {}x{}", self.nrow, self.ncol));
        }
        let n = self.nrow;
        if n == 0 {
            return Ok((vec![], Self::zeros(0, 0)));
        }

        // QR iteration with shifts
        let mut a: Vec<Vec<f64>> = (0..n).map(|i| self.row(i)).collect();
        // Accumulate eigenvectors
        let mut v: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let mut row = vec![0.0; n];
                row[i] = 1.0;
                row
            })
            .collect();

        let max_iter = 200 * n;
        for _ in 0..max_iter {
            // Wilkinson shift
            let shift = a[n - 1][n - 1];
            for i in 0..n {
                a[i][i] -= shift;
            }

            // QR decomposition via Gram-Schmidt
            let (q, r) = qr_decompose(&a, n);

            // A = R * Q + shift * I
            a = mat_mul_vv(&r, &q, n);
            for i in 0..n {
                a[i][i] += shift;
            }

            // V = V * Q
            v = mat_mul_vv(&v, &q, n);

            // Check convergence (off-diagonal elements)
            let mut off_diag = 0.0;
            for i in 0..n {
                for j in 0..n {
                    if i != j {
                        off_diag += a[i][j] * a[i][j];
                    }
                }
            }
            if off_diag.sqrt() < 1e-10 {
                break;
            }
        }

        let eigenvalues: Vec<f64> = (0..n).map(|i| a[i][i]).collect();
        let data: Vec<f64> = v.into_iter().flatten().collect();
        let eigenvectors = Self::new(data, n, n)?;
        Ok((eigenvalues, eigenvectors))
    }

    /// Singular Value Decomposition: A = U * S * V^T
    /// Returns (U, singular_values, Vt).
    pub fn svd(&self) -> Result<(Self, Vec<f64>, Self), String> {
        let m = self.nrow;
        let n = self.ncol;
        // Compute A^T * A
        let at = self.transpose();
        let ata = at.dot(self)?;

        // Eigendecompose A^T * A
        let (eigenvalues, v_mat) = ata.eigen()?;

        // Singular values = sqrt of eigenvalues (clamp negatives from numerical error)
        let singular_unsorted: Vec<f64> = eigenvalues.iter().map(|&e| if e > 0.0 { e.sqrt() } else { 0.0 }).collect();

        // Sort by descending singular value
        let mut indices: Vec<usize> = (0..n).collect();
        indices.sort_by(|&a, &b| singular_unsorted[b].partial_cmp(&singular_unsorted[a]).unwrap_or(std::cmp::Ordering::Equal));

        let singular: Vec<f64> = indices.iter().map(|&i| singular_unsorted[i]).collect();

        // Reorder V columns
        let mut v_data = vec![0.0; n * n];
        for (new_j, &old_j) in indices.iter().enumerate() {
            for i in 0..n {
                v_data[i * n + new_j] = v_mat.get(i, old_j);
            }
        }
        let v_sorted = Self::new(v_data, n, n)?;
        let vt = v_sorted.transpose();

        // U = A * V * S^{-1}
        let av = self.dot(&v_sorted)?;
        let mut u_data = vec![0.0; m * n];
        for i in 0..m {
            for j in 0..n {
                if singular[j] > 1e-14 {
                    u_data[i * n + j] = av.get(i, j) / singular[j];
                }
            }
        }
        let u = Self::new(u_data, m, n)?;

        Ok((u, singular, vt))
    }

    /// Matrix rank (number of singular values above threshold).
    pub fn rank(&self) -> Result<usize, String> {
        let (_, singular, _) = self.svd()?;
        let threshold = singular.first().copied().unwrap_or(0.0) * 1e-10;
        Ok(singular.iter().filter(|&&s| s > threshold).count())
    }
}

// Helper: QR decomposition via modified Gram-Schmidt
fn qr_decompose(a: &[Vec<f64>], n: usize) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let mut q = vec![vec![0.0; n]; n];
    let mut r = vec![vec![0.0; n]; n];

    // Get columns of A
    let cols: Vec<Vec<f64>> = (0..n)
        .map(|j| (0..n).map(|i| a[i][j]).collect())
        .collect();

    for j in 0..n {
        let mut u = cols[j].clone();
        for k in 0..j {
            let dot: f64 = (0..n).map(|i| cols[j][i] * q[i][k]).sum();
            r[k][j] = dot;
            for i in 0..n {
                u[i] -= dot * q[i][k];
            }
        }
        let norm: f64 = u.iter().map(|x| x * x).sum::<f64>().sqrt();
        r[j][j] = norm;
        if norm > 1e-15 {
            for i in 0..n {
                q[i][j] = u[i] / norm;
            }
        }
    }

    // Convert Q from column-storage to row-storage
    let q_rows: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|j| q[i][j]).collect())
        .collect();
    (q_rows, r)
}

// Helper: multiply two n×n matrices stored as Vec<Vec<f64>>
fn mat_mul_vv(a: &[Vec<f64>], b: &[Vec<f64>], n: usize) -> Vec<Vec<f64>> {
    let mut c = vec![vec![0.0; n]; n];
    for i in 0..n {
        for k in 0..n {
            let a_ik = a[i][k];
            for j in 0..n {
                c[i][j] += a_ik * b[k][j];
            }
        }
    }
    c
}

impl fmt::Display for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Matrix: {}x{}", self.nrow, self.ncol)?;
        let show = self.nrow.min(10);
        for i in 0..show {
            let row: Vec<String> = self.row(i).iter().map(|v| format!("{v:.4}")).collect();
            writeln!(f, " [{}]", row.join(", "))?;
        }
        if self.nrow > 10 {
            write!(f, " # {} more rows", self.nrow - 10)?;
        }
        Ok(())
    }
}

impl PartialEq for Matrix {
    fn eq(&self, other: &Self) -> bool {
        self.nrow == other.nrow
            && self.ncol == other.ncol
            && self.data.len() == other.data.len()
            && self
                .data
                .iter()
                .zip(&other.data)
                .all(|(a, b)| (a - b).abs() < 1e-10)
    }
}
