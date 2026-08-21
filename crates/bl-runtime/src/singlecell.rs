//! Single-cell RNA-seq (Section 6) and CNV / tumour-purity (Section 7) builtins.
//!
//! Functions: normalize_total, log1p_transform, highly_variable_genes, cell_qc,
//! gene_qc, knn_graph, doublet_score, cnv_segment, loh_detect, tumor_purity,
//! vaf_to_ccf, mutational_signature,
//! read_10x, cell_cycle_score, module_score, sc_sctransform, sc_integrate,
//! diffusion_pseudotime, lr_score, lr_aggregate,
//! spatial_neighbors, spatial_moransi, reference_classify, pseudobulk_aggregate,
//! wnn_graph, velocity_estimate.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::sparse_matrix::SparseMatrix;
use bl_core::value::{Arity, Table, Value};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::io::{BufRead, BufReader};
#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Write};
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::process::{Command, Stdio};

// â”€â”€ Registry â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub fn singlecell_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        // Section 6: Single-cell QC / normalisation
        ("normalize_total", Arity::Range(1, 2)),
        ("log1p_transform", Arity::Exact(1)),
        ("highly_variable_genes", Arity::Range(1, 3)),
        ("find_all_markers", Arity::Range(2, 3)),
        ("sc_find_all_markers", Arity::Range(2, 3)),
        ("harmony_integrate", Arity::Range(2, 3)),
        ("cca", Arity::Range(2, 3)),
        ("sc_anchor_candidates", Arity::Range(2, 3)),
        ("sc_find_anchors", Arity::Range(2, 3)),
        ("sc_integrate_anchors", Arity::Range(3, 4)),
        ("cell_qc", Arity::Range(1, 3)),
        ("gene_qc", Arity::Range(1, 2)),
        ("knn_graph", Arity::Range(1, 2)),
        ("snn_graph", Arity::Range(1, 3)),
        ("sc_umap", Arity::Range(1, 2)),
        ("louvain_graph", Arity::Range(3, 4)),
        ("leiden_cluster", Arity::Range(2, 3)),
        ("louvain_cluster", Arity::Range(2, 3)),
        ("leiden_graph", Arity::Exact(3)),
        ("select_rows", Arity::Exact(2)),
        ("select_cols", Arity::Exact(2)),
        ("matrix_at", Arity::Exact(3)),
        ("sc_subset_cells", Arity::Exact(2)),
        ("sc_subset_genes", Arity::Exact(2)),
        ("sc_merge_objects", Arity::Exact(4)),
        ("sc_pca", Arity::Range(1, 4)),
        ("sc_scale", Arity::Range(1, 2)),
        ("doublet_score", Arity::Range(1, 2)),
        // Section 6 extensions: Seurat-compatible single-cell ops
        ("read_10x", Arity::Range(1, 2)),
        ("read_10x_sparse", Arity::Range(1, 2)),
        ("cell_cycle_score", Arity::Exact(3)),
        ("module_score", Arity::Exact(2)),
        ("sc_sctransform", Arity::Range(1, 3)),
        ("sc_integrate", Arity::Exact(2)),
        ("diffusion_pseudotime", Arity::Exact(3)),
        // Cell-cell communication
        ("lr_score", Arity::Exact(3)),
        ("lr_aggregate", Arity::Exact(2)),
        // Spatial transcriptomics
        ("spatial_neighbors", Arity::Range(1, 2)),
        ("spatial_moransi", Arity::Exact(2)),
        // Cell type annotation
        ("reference_classify", Arity::Exact(3)),
        // Pseudobulk aggregation
        ("pseudobulk_aggregate", Arity::Exact(3)),
        // Multimodal integration
        ("wnn_graph", Arity::Exact(3)),
        // RNA velocity
        ("velocity_estimate", Arity::Exact(2)),
        // Section 7: CNV / tumour purity
        ("cnv_segment", Arity::Range(1, 2)),
        ("loh_detect", Arity::Exact(1)),
        ("tumor_purity", Arity::Exact(1)),
        ("vaf_to_ccf", Arity::Range(2, 4)),
        ("mutational_signature", Arity::Exact(1)),
    ]
}

pub fn is_singlecell_builtin(name: &str) -> bool {
    matches!(
        name,
        "normalize_total"
            | "log1p_transform"
            | "highly_variable_genes"
            | "find_all_markers"
            | "sc_find_all_markers"
            | "harmony_integrate"
            | "cca"
            | "sc_anchor_candidates"
            | "sc_find_anchors"
            | "sc_integrate_anchors"
            | "cell_qc"
            | "gene_qc"
            | "knn_graph"
            | "snn_graph"
            | "sc_umap"
            | "louvain_graph"
            | "leiden_cluster"
            | "louvain_cluster"
            | "leiden_graph"
            | "select_rows"
            | "select_cols"
            | "matrix_at"
            | "sc_subset_cells"
            | "sc_subset_genes"
            | "sc_merge_objects"
            | "sc_pca"
            | "sc_scale"
            | "doublet_score"
            | "read_10x"
            | "read_10x_sparse"
            | "cell_cycle_score"
            | "module_score"
            | "sc_sctransform"
            | "sc_integrate"
            | "diffusion_pseudotime"
            | "lr_score"
            | "lr_aggregate"
            | "spatial_neighbors"
            | "spatial_moransi"
            | "reference_classify"
            | "pseudobulk_aggregate"
            | "wnn_graph"
            | "velocity_estimate"
            | "cnv_segment"
            | "loh_detect"
            | "tumor_purity"
            | "vaf_to_ccf"
            | "mutational_signature"
    )
}

pub fn call_singlecell_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "normalize_total" => builtin_normalize_total(args),
        "log1p_transform" => builtin_log1p_transform(args),
        "highly_variable_genes" => builtin_highly_variable_genes(args),
        "find_all_markers" => builtin_find_all_markers(args),
        "sc_find_all_markers" => builtin_find_all_markers(args),
        "harmony_integrate" => builtin_harmony_integrate(args),
        "cca" => builtin_cca(args),
        "sc_anchor_candidates" => builtin_sc_anchor_candidates(args),
        "sc_find_anchors" => builtin_sc_find_anchors(args),
        "sc_integrate_anchors" => builtin_sc_integrate_anchors(args),
        "cell_qc" => builtin_cell_qc(args),
        "gene_qc" => builtin_gene_qc(args),
        "knn_graph" => builtin_knn_graph(args),
        "snn_graph" => builtin_snn_graph(args),
        "sc_umap" => builtin_sc_umap(args),
        "louvain_graph" => builtin_louvain_graph(args),
        "leiden_cluster" => builtin_leiden_cluster(args),
        "louvain_cluster" => builtin_louvain_cluster(args),
        "leiden_graph" => builtin_leiden_graph(args),
        "select_rows" => builtin_select_rows(args),
        "select_cols" => builtin_select_cols(args),
        "matrix_at" => builtin_matrix_at(args),
        "sc_subset_cells" => builtin_sc_subset_cells(args),
        "sc_subset_genes" => builtin_sc_subset_genes(args),
        "sc_merge_objects" => builtin_sc_merge_objects(args),
        "sc_pca" => builtin_sc_pca(args),
        "sc_scale" => builtin_sc_scale(args),
        "doublet_score" => builtin_doublet_score(args),
        "read_10x" => builtin_read_10x(args),
        "read_10x_sparse" => builtin_read_10x_sparse(args),
        "cell_cycle_score" => builtin_cell_cycle_score(args),
        "module_score" => builtin_module_score(args),
        "sc_sctransform" => builtin_sc_sctransform(args),
        "sc_integrate" => builtin_sc_integrate(args),
        "diffusion_pseudotime" => builtin_diffusion_pseudotime(args),
        "lr_score" => builtin_lr_score(args),
        "lr_aggregate" => builtin_lr_aggregate(args),
        "spatial_neighbors" => builtin_spatial_neighbors(args),
        "spatial_moransi" => builtin_spatial_moransi(args),
        "reference_classify" => builtin_reference_classify(args),
        "pseudobulk_aggregate" => builtin_pseudobulk_aggregate(args),
        "wnn_graph" => builtin_wnn_graph(args),
        "velocity_estimate" => builtin_velocity_estimate(args),
        "cnv_segment" => builtin_cnv_segment(args),
        "loh_detect" => builtin_loh_detect(args),
        "tumor_purity" => builtin_tumor_purity(args),
        "vaf_to_ccf" => builtin_vaf_to_ccf(args),
        "mutational_signature" => builtin_mutational_signature(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown singlecell builtin: {name}"),
            None,
        )),
    }
}

// â”€â”€ Helper functions â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

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
        // `matrix(...)` is the idiomatic way to write a dense matrix literal, so
        // every builtin that takes one should accept it rather than only the
        // List<List> spelling.
        Value::Matrix(m) => Ok((0..m.nrow)
            .map(|i| m.data[i * m.ncol..(i + 1) * m.ncol].to_vec())
            .collect()),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires List<List>, Matrix, or Table"),
            None,
        )),
    }
}

/// Like `require_matrix`, but also densifies a CSR input for algorithms whose
/// mathematical working representation is dense.
fn require_dense_matrix(val: &Value, func: &str) -> Result<Vec<Vec<f64>>> {
    match val {
        Value::SparseMatrix(matrix) => Ok(matrix.to_dense()),
        other => require_matrix(other, func),
    }
}

fn require_int(val: &Value, func: &str) -> Result<i64> {
    match val {
        Value::Int(n) => Ok(*n),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires Int"),
            None,
        )),
    }
}

fn matrix_to_value(mat: Vec<Vec<f64>>) -> Value {
    Value::List(
        mat.into_iter()
            .map(|row| Value::List(row.into_iter().map(Value::Float).collect::<Vec<_>>().into()))
            .collect::<Vec<_>>()
            .into(),
    )
}

/// Store a native numeric result without boxing every number as a language
/// value. This matters for ATAC LSI loadings, where hundreds of thousands of
/// peaks times 50 components are otherwise represented by millions of heap-
/// heavy `Value::Float` entries.
fn matrix_to_compact_value(mat: Vec<Vec<f64>>, func: &str) -> Result<Value> {
    let nrow = mat.len();
    let ncol = mat.first().map(Vec::len).unwrap_or(0);
    if mat.iter().any(|row| row.len() != ncol) {
        return Err(BioLangError::type_error(
            format!("{func}(): internal matrix rows have unequal length"),
            None,
        ));
    }
    let mut data = Vec::with_capacity(nrow.saturating_mul(ncol));
    for row in mat {
        data.extend(row);
    }
    let matrix = bl_core::matrix::Matrix::new(data, nrow, ncol)
        .map_err(|error| BioLangError::type_error(format!("{func}(): {error}"), None))?;
    Ok(Value::Matrix(matrix.into()))
}

// â”€â”€ select_cols(matrix, indices) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Subset a matrix to the given column indices, in Rust. Replaces the
// interpreted `mat |> map(|row| idx |> map(|j| row[j]))` double-loop that
// dominates HVG selection on real-sized data (hundreds of thousands of
// interpreted closure calls).
enum SingleCellMatrix<'a> {
    Dense(Vec<Vec<f64>>),
    /// Dense rows borrowed by an internal caller. This avoids converting a
    /// native analysis matrix to millions of boxed `Value::Float` objects just
    /// to feed it back into another Rust builtin.
    BorrowedDense(&'a [Vec<f64>]),
    /// Two compact runtime matrices exposed as one logical matrix. Integration
    /// PCA needs the merged reference/query population, but not an additional
    /// physical copy of all residuals.
    JoinedFlat(&'a bl_core::matrix::Matrix, &'a bl_core::matrix::Matrix),
    /// A `Value::Matrix` read where it already lies. `Value::Matrix` is
    /// row-major behind an `Arc`, which is exactly the layout every method here
    /// wants, so copying it into a vector-per-row costs a duplicate of the whole
    /// matrix plus an allocation per cell and gives nothing back. On the
    /// SCTransform residuals feeding PCA that duplicate is hundreds of
    /// megabytes, and the copy's rows are scattered where the original's are
    /// contiguous.
    Flat(&'a bl_core::matrix::Matrix),
    Sparse(&'a SparseMatrix),
}

impl SingleCellMatrix<'_> {
    /// Row `index` of a dense input, whichever way it is stored.
    #[inline]
    fn dense_row(&self, index: usize) -> &[f64] {
        match self {
            Self::Dense(matrix) => &matrix[index],
            Self::BorrowedDense(matrix) => &matrix[index],
            Self::JoinedFlat(left, right) => {
                if index < left.nrow {
                    &left.data[index * left.ncol..(index + 1) * left.ncol]
                } else {
                    let index = index - left.nrow;
                    &right.data[index * right.ncol..(index + 1) * right.ncol]
                }
            }
            Self::Flat(matrix) => &matrix.data[index * matrix.ncol..(index + 1) * matrix.ncol],
            Self::Sparse(_) => &[],
        }
    }

    fn dimensions(&self) -> (usize, usize) {
        match self {
            Self::Dense(matrix) => (
                matrix.len(),
                matrix.first().map(|row| row.len()).unwrap_or(0),
            ),
            Self::BorrowedDense(matrix) => (
                matrix.len(),
                matrix.first().map(|row| row.len()).unwrap_or(0),
            ),
            Self::JoinedFlat(left, right) => (left.nrow + right.nrow, left.ncol),
            Self::Flat(matrix) => (matrix.nrow, matrix.ncol),
            Self::Sparse(matrix) => (matrix.nrow, matrix.ncol),
        }
    }

    fn column_moments(&self) -> (Vec<f64>, Vec<f64>) {
        let (n_rows, n_columns) = self.dimensions();
        let mut sums = vec![0.0; n_columns];
        let mut sums_squared = vec![0.0; n_columns];
        match self {
            Self::Dense(_) | Self::BorrowedDense(_) | Self::JoinedFlat(_, _) | Self::Flat(_) => {
                for index in 0..n_rows {
                    for (column, value) in self.dense_row(index).iter().copied().enumerate() {
                        sums[column] += value;
                        sums_squared[column] += value * value;
                    }
                }
            }
            Self::Sparse(matrix) => {
                sums = matrix.col_sums();
                for (&column, &value) in matrix.indices.iter().zip(&matrix.data) {
                    sums_squared[column] += value * value;
                }
            }
        }
        if n_rows == 0 {
            return (sums, sums_squared);
        }
        (sums, sums_squared)
    }

    fn value_at(&self, row: usize, column: usize) -> f64 {
        match self {
            Self::Sparse(matrix) => matrix.get(row, column),
            _ => self.dense_row(row)[column],
        }
    }

    fn multiply_centered(&self, means: &[f64], vector: &[f64]) -> Vec<f64> {
        let (n_rows, _) = self.dimensions();
        let center: f64 = means
            .iter()
            .zip(vector)
            .map(|(mean, weight)| mean * weight)
            .sum();
        match self {
            Self::Dense(_) | Self::BorrowedDense(_) | Self::JoinedFlat(_, _) | Self::Flat(_) => (0
                ..n_rows)
                .map(|index| {
                    self.dense_row(index)
                        .iter()
                        .zip(vector)
                        .map(|(value, weight)| value * weight)
                        .sum::<f64>()
                        - center
                })
                .collect(),
            Self::Sparse(matrix) => (0..n_rows)
                .map(|row| {
                    let value = (matrix.indptr[row]..matrix.indptr[row + 1])
                        .map(|position| matrix.data[position] * vector[matrix.indices[position]])
                        .sum::<f64>();
                    value - center
                })
                .collect(),
        }
    }

    fn transpose_multiply_centered(&self, means: &[f64], vector: &[f64]) -> Vec<f64> {
        let (_, n_columns) = self.dimensions();
        let mut result = vec![0.0; n_columns];
        match self {
            Self::Dense(_) | Self::BorrowedDense(_) | Self::JoinedFlat(_, _) | Self::Flat(_) => {
                for (index, weight) in vector.iter().enumerate() {
                    for (column, value) in self.dense_row(index).iter().copied().enumerate() {
                        result[column] += value * weight;
                    }
                }
            }
            Self::Sparse(matrix) => {
                for (row, weight) in vector.iter().copied().enumerate() {
                    for position in matrix.indptr[row]..matrix.indptr[row + 1] {
                        result[matrix.indices[position]] += matrix.data[position] * weight;
                    }
                }
            }
        }
        let vector_sum: f64 = vector.iter().sum();
        for (value, mean) in result.iter_mut().zip(means) {
            *value -= mean * vector_sum;
        }
        result
    }

    fn multiply_centered_rows(&self, means: &[f64], vector: &[f64], rows: &[usize]) -> Vec<f64> {
        let center: f64 = means
            .iter()
            .zip(vector)
            .map(|(mean, weight)| mean * weight)
            .sum();
        match self {
            Self::Dense(_) | Self::BorrowedDense(_) | Self::JoinedFlat(_, _) | Self::Flat(_) => {
                rows.iter()
                    .map(|&row| {
                        self.dense_row(row)
                            .iter()
                            .zip(vector)
                            .map(|(value, weight)| value * weight)
                            .sum::<f64>()
                            - center
                    })
                    .collect()
            }
            Self::Sparse(matrix) => rows
                .iter()
                .map(|&row| {
                    let value = (matrix.indptr[row]..matrix.indptr[row + 1])
                        .map(|position| matrix.data[position] * vector[matrix.indices[position]])
                        .sum::<f64>();
                    value - center
                })
                .collect(),
        }
    }

    /// `(X - 1 mean^T) B` for a whole block at once.
    ///
    /// `block` is gene-major, `n_genes * width`, so the innermost loop walks a
    /// gene's `width` coefficients contiguously. Applying the covariance one
    /// vector at a time — which is what the per-vector methods above force —
    /// costs a separate pass over the entire matrix per vector: fifty passes
    /// over a 711 MB residual matrix per sweep, where one pass would do. The
    /// arithmetic is identical; only the traffic changes, and on this shape the
    /// traffic is the whole cost.
    fn multiply_centered_block(&self, means: &[f64], block: &[f64], width: usize) -> Vec<f64> {
        let (n_rows, n_columns) = self.dimensions();
        let mut center = vec![0.0f64; width];
        for gene in 0..n_columns {
            let mean = means[gene];
            if mean == 0.0 {
                continue;
            }
            let row = &block[gene * width..(gene + 1) * width];
            for (accumulator, weight) in center.iter_mut().zip(row) {
                *accumulator += mean * weight;
            }
        }

        let mut scores = vec![0.0f64; n_rows * width];
        // One cell per output row, so the split is disjoint and the sums inside
        // a row are unaffected by how many threads run.
        par_rows_mut(&mut scores, width, |first_cell, out| {
            for (offset, target) in out.chunks_mut(width).enumerate() {
                let cell = first_cell + offset;
                for (value, shift) in target.iter_mut().zip(&center) {
                    *value = -*shift;
                }
                match self {
                    Self::Sparse(matrix) => {
                        for position in matrix.indptr[cell]..matrix.indptr[cell + 1] {
                            let gene = matrix.indices[position];
                            let count = matrix.data[position];
                            let row = &block[gene * width..(gene + 1) * width];
                            for (value, weight) in target.iter_mut().zip(row) {
                                *value += count * weight;
                            }
                        }
                    }
                    _ => {
                        for (gene, &count) in self.dense_row(cell).iter().enumerate() {
                            if count == 0.0 {
                                continue;
                            }
                            let row = &block[gene * width..(gene + 1) * width];
                            for (value, weight) in target.iter_mut().zip(row) {
                                *value += count * weight;
                            }
                        }
                    }
                }
            }
        });
        scores
    }

    /// `(X - 1 mean^T)^T S`, the other half of one covariance application.
    ///
    /// This one accumulates across cells, so it cannot simply be split by row.
    /// The cell range is divided into a fixed number of chunks — fixed as a
    /// function of the data's shape and nothing else — each chunk summed into
    /// its own slab and the slabs reduced in chunk order. Two machines with
    /// different core counts therefore add the same numbers in the same order.
    fn transpose_multiply_centered_block(
        &self,
        means: &[f64],
        scores: &[f64],
        width: usize,
    ) -> Vec<f64> {
        let (n_rows, n_columns) = self.dimensions();
        let slab = n_columns * width;
        let chunks = if slab > 4_000_000 { 4 } else { 16 }.min(n_rows.max(1));
        let span = n_rows.div_ceil(chunks).max(1);

        let mut partials = vec![0.0f64; chunks * slab];
        par_rows_mut(&mut partials, slab, |first_chunk, block| {
            for (offset, target) in block.chunks_mut(slab).enumerate() {
                let start = (first_chunk + offset) * span;
                let end = (start + span).min(n_rows);
                for cell in start..end {
                    let weights = &scores[cell * width..(cell + 1) * width];
                    match self {
                        Self::Sparse(matrix) => {
                            for position in matrix.indptr[cell]..matrix.indptr[cell + 1] {
                                let gene = matrix.indices[position];
                                let count = matrix.data[position];
                                let row = &mut target[gene * width..(gene + 1) * width];
                                for (value, weight) in row.iter_mut().zip(weights) {
                                    *value += count * weight;
                                }
                            }
                        }
                        _ => {
                            for (gene, &count) in self.dense_row(cell).iter().enumerate() {
                                if count == 0.0 {
                                    continue;
                                }
                                let row = &mut target[gene * width..(gene + 1) * width];
                                for (value, weight) in row.iter_mut().zip(weights) {
                                    *value += count * weight;
                                }
                            }
                        }
                    }
                }
            }
        });

        let mut result = vec![0.0f64; slab];
        for chunk in 0..chunks {
            let source = &partials[chunk * slab..(chunk + 1) * slab];
            for (value, partial) in result.iter_mut().zip(source) {
                *value += *partial;
            }
        }
        drop(partials);

        let mut score_sums = vec![0.0f64; width];
        for cell in 0..n_rows {
            let weights = &scores[cell * width..(cell + 1) * width];
            for (accumulator, weight) in score_sums.iter_mut().zip(weights) {
                *accumulator += *weight;
            }
        }
        for gene in 0..n_columns {
            let mean = means[gene];
            if mean == 0.0 {
                continue;
            }
            let row = &mut result[gene * width..(gene + 1) * width];
            for (value, total) in row.iter_mut().zip(&score_sums) {
                *value -= mean * *total;
            }
        }
        result
    }

    fn transpose_multiply_centered_rows(
        &self,
        means: &[f64],
        vector: &[f64],
        rows: &[usize],
    ) -> Vec<f64> {
        let (_, n_columns) = self.dimensions();
        let mut result = vec![0.0; n_columns];
        match self {
            Self::Dense(_) | Self::BorrowedDense(_) | Self::JoinedFlat(_, _) | Self::Flat(_) => {
                for (&row, &weight) in rows.iter().zip(vector) {
                    for (column, value) in self.dense_row(row).iter().copied().enumerate() {
                        result[column] += value * weight;
                    }
                }
            }
            Self::Sparse(matrix) => {
                for (&row, &weight) in rows.iter().zip(vector) {
                    for position in matrix.indptr[row]..matrix.indptr[row + 1] {
                        result[matrix.indices[position]] += matrix.data[position] * weight;
                    }
                }
            }
        }
        let vector_sum: f64 = vector.iter().sum();
        for (value, mean) in result.iter_mut().zip(means) {
            *value -= mean * vector_sum;
        }
        result
    }
}

fn singlecell_matrix<'a>(value: &'a Value, func: &str) -> Result<SingleCellMatrix<'a>> {
    if let Value::SparseMatrix(matrix) = value {
        return Ok(SingleCellMatrix::Sparse(matrix));
    }
    if let Value::Matrix(matrix) = value {
        return Ok(SingleCellMatrix::Flat(matrix));
    }
    let matrix = require_matrix(value, func)?;
    let n_columns = matrix.first().map(|row| row.len()).unwrap_or(0);
    if matrix.iter().any(|row| row.len() != n_columns) {
        return Err(BioLangError::type_error(
            format!("{func}() requires a rectangular matrix"),
            None,
        ));
    }
    Ok(SingleCellMatrix::Dense(matrix))
}

fn orthogonalize(vector: &mut [f64], basis: &[Vec<f64>]) {
    for component in basis {
        let projection: f64 = vector
            .iter()
            .zip(component)
            .map(|(value, loading)| value * loading)
            .sum();
        for (value, loading) in vector.iter_mut().zip(component) {
            *value -= projection * loading;
        }
    }
}

/// Orthonormalise a block in place by modified Gram-Schmidt, twice.
///
/// Twice is not belt-and-braces. Classical Gram-Schmidt loses orthogonality in
/// proportion to the condition number, and a single modified pass still drifts
/// once the vectors are nearly dependent â€” which is precisely the state of a
/// subspace iterate as it converges. Two passes restores orthogonality to
/// machine precision ("twice is enough", Kahan). The previous PCA orthogonalised
/// once and returned components whose explained variance was not monotone.
///
/// Columns that collapse to nothing are dropped, so the block can shrink.
fn orthonormalise_block(block: &mut Vec<Vec<f64>>) {
    let mut kept: Vec<Vec<f64>> = Vec::with_capacity(block.len());
    for mut vector in block.drain(..) {
        for _ in 0..2 {
            for basis in &kept {
                let projection: f64 = vector
                    .iter()
                    .zip(basis)
                    .map(|(value, other)| value * other)
                    .sum();
                for (value, other) in vector.iter_mut().zip(basis) {
                    *value -= projection * other;
                }
            }
        }
        let norm = vector.iter().map(|v| v * v).sum::<f64>().sqrt();
        if norm > 1e-10 {
            for value in &mut vector {
                *value /= norm;
            }
            kept.push(vector);
        }
    }
    *block = kept;
}

/// Eigendecomposition of a small symmetric matrix by cyclic Jacobi rotations.
///
/// Returns `(eigenvectors_as_rows, eigenvalues)` sorted by descending
/// eigenvalue. Jacobi is slower than the tridiagonal-QL route for large
/// matrices and unconditionally accurate for small ones; the input here is
/// `k x k` for `k` around fifty, so accuracy is the only axis that matters.
fn jacobi_eigen_symmetric(input: &[Vec<f64>]) -> (Vec<Vec<f64>>, Vec<f64>) {
    let n = input.len();
    let mut a: Vec<Vec<f64>> = input.to_vec();
    // Vectors accumulate as rows, so v[i] is the eigenvector for value[i].
    let mut v: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
        .collect();

    for _ in 0..100 {
        let off_diagonal: f64 = (0..n)
            .flat_map(|i| ((i + 1)..n).map(move |j| (i, j)))
            .map(|(i, j)| a[i][j] * a[i][j])
            .sum();
        if off_diagonal.sqrt() < 1e-14 {
            break;
        }
        for p in 0..n {
            for q in (p + 1)..n {
                if a[p][q].abs() < 1e-18 {
                    continue;
                }
                let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..n {
                    let (akp, akq) = (a[k][p], a[k][q]);
                    a[k][p] = c * akp - s * akq;
                    a[k][q] = s * akp + c * akq;
                }
                for k in 0..n {
                    let (apk, aqk) = (a[p][k], a[q][k]);
                    a[p][k] = c * apk - s * aqk;
                    a[q][k] = s * apk + c * aqk;
                }
                for k in 0..n {
                    let (vpk, vqk) = (v[p][k], v[q][k]);
                    v[p][k] = c * vpk - s * vqk;
                    v[q][k] = s * vpk + c * vqk;
                }
            }
        }
    }

    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| a[j][j].total_cmp(&a[i][i]));
    let values = order.iter().map(|&i| a[i][i]).collect();
    let vectors = order.iter().map(|&i| v[i].clone()).collect();
    (vectors, values)
}

/// Column-center, sample-standardize, and clip a matrix without boxing every
/// element through interpreted nested maps.
fn builtin_sc_scale(args: Vec<Value>) -> Result<Value> {
    let matrix = singlecell_matrix(&args[0], "sc_scale")?;
    // Omitted means the historical BioLang default of 10. Explicit nil means
    // no clipping, matching Signac RunSVD(scale.max = NULL).
    let clip = match args.get(1) {
        None => Some(10.0),
        Some(Value::Nil) => None,
        Some(value) => Some(
            to_f64(value)
                .ok_or_else(|| {
                    BioLangError::type_error("sc_scale() clip must be a Number or nil", None)
                })?
                .abs(),
        ),
    };
    let (rows, columns) = matrix.dimensions();
    if rows == 0 || columns == 0 {
        return Ok(Value::Matrix(
            bl_core::matrix::Matrix::new(Vec::new(), rows, columns)
                .map_err(|error| BioLangError::type_error(format!("sc_scale(): {error}"), None))?
                .into(),
        ));
    }
    let (sums, sums_squared) = matrix.column_moments();
    let means: Vec<f64> = sums.iter().map(|sum| sum / rows as f64).collect();
    let divisor = rows.saturating_sub(1).max(1) as f64;
    let deviations: Vec<f64> = (0..columns)
        .map(|column| {
            ((sums_squared[column] - rows as f64 * means[column] * means[column]) / divisor)
                .max(0.0)
                .sqrt()
                .max(1e-6)
        })
        .collect();
    let mut scaled = Vec::with_capacity(rows * columns);
    for row in 0..rows {
        for column in 0..columns {
            let value = (matrix.value_at(row, column) - means[column]) / deviations[column];
            scaled.push(match clip {
                Some(limit) => value.clamp(-limit, limit),
                None => value,
            });
        }
    }
    let output = bl_core::matrix::Matrix::new(scaled, rows, columns)
        .map_err(|error| BioLangError::type_error(format!("sc_scale(): {error}"), None))?;
    Ok(Value::Matrix(output.into()))
}

#[cfg(not(target_arch = "wasm32"))]
fn external_provider_program(opts: &HashMap<String, Value>) -> Option<String> {
    let requested = match opts.get("external_provider") {
        Some(Value::Bool(true)) => Some("auto"),
        Some(Value::Str(value)) => Some(value.as_str()),
        _ => None,
    }?;
    if requested.eq_ignore_ascii_case("auto")
        || requested.eq_ignore_ascii_case("external")
        || requested.eq_ignore_ascii_case("seurat")
    {
        Some(
            std::env::var("BIOLANG_SEURAT_PROVIDER")
                .unwrap_or_else(|_| "bl-seurat-provider".to_string()),
        )
    } else {
        Some(requested.to_string())
    }
}

#[cfg(target_arch = "wasm32")]
fn external_provider_program(opts: &HashMap<String, Value>) -> Option<String> {
    match opts.get("external_provider") {
        Some(Value::Bool(true)) | Some(Value::Str(_)) => Some("unavailable-in-wasm".to_string()),
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_blmat_values(
    path: &Path,
    rows: usize,
    columns: usize,
    mut value_at: impl FnMut(usize, usize) -> f64,
) -> Result<()> {
    let file = std::fs::File::create(path).map_err(|error| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("cannot create provider matrix {}: {error}", path.display()),
            None,
        )
    })?;
    let mut writer = std::io::BufWriter::new(file);
    writer
        .write_all(b"BLMATF64")
        .map_err(|error| BioLangError::runtime(ErrorKind::IOError, error.to_string(), None))?;
    writer
        .write_all(&(rows as u64).to_le_bytes())
        .and_then(|_| writer.write_all(&(columns as u64).to_le_bytes()))
        .map_err(|error| BioLangError::runtime(ErrorKind::IOError, error.to_string(), None))?;
    for row in 0..rows {
        for column in 0..columns {
            writer
                .write_all(&value_at(row, column).to_le_bytes())
                .map_err(|error| {
                    BioLangError::runtime(ErrorKind::IOError, error.to_string(), None)
                })?;
        }
    }
    writer
        .flush()
        .map_err(|error| BioLangError::runtime(ErrorKind::IOError, error.to_string(), None))
}

#[cfg(not(target_arch = "wasm32"))]
fn read_blmat_values(path: &Path, context: &str) -> Result<Vec<Vec<f64>>> {
    let mut file = std::fs::File::open(path).map_err(|error| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("{context}: cannot open {}: {error}", path.display()),
            None,
        )
    })?;
    let mut header = [0_u8; 24];
    file.read_exact(&mut header).map_err(|error| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("{context}: invalid matrix header: {error}"),
            None,
        )
    })?;
    if &header[..8] != b"BLMATF64" {
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("{context}: invalid BLMATF64 magic"),
            None,
        ));
    }
    let rows =
        usize::try_from(u64::from_le_bytes(header[8..16].try_into().unwrap())).map_err(|_| {
            BioLangError::type_error(format!("{context}: row count is too large"), None)
        })?;
    let columns =
        usize::try_from(u64::from_le_bytes(header[16..24].try_into().unwrap())).map_err(|_| {
            BioLangError::type_error(format!("{context}: column count is too large"), None)
        })?;
    let values = rows.checked_mul(columns).ok_or_else(|| {
        BioLangError::type_error(format!("{context}: matrix dimensions overflow"), None)
    })?;
    let mut bytes = vec![0_u8; values.saturating_mul(8)];
    file.read_exact(&mut bytes).map_err(|error| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("{context}: truncated matrix payload: {error}"),
            None,
        )
    })?;
    let flat: Vec<f64> = bytes
        .chunks_exact(8)
        .map(|chunk| f64::from_le_bytes(chunk.try_into().unwrap()))
        .collect();
    if flat.iter().any(|value| !value.is_finite()) {
        return Err(BioLangError::type_error(
            format!("{context}: provider returned non-finite values"),
            None,
        ));
    }
    Ok(flat
        .chunks(columns.max(1))
        .take(rows)
        .map(<[f64]>::to_vec)
        .collect())
}

#[cfg(not(target_arch = "wasm32"))]
fn read_provider_manifest(path: &Path, context: &str) -> Result<HashMap<String, Value>> {
    let contents = std::fs::read_to_string(path).map_err(|error| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!(
                "{context}: cannot read provider manifest {}: {error}",
                path.display()
            ),
            None,
        )
    })?;
    let mut lines = contents.lines();
    let names = lines.next().unwrap_or_default().split(',');
    let values = lines.next().unwrap_or_default().split(',');
    let fields: HashMap<String, Value> = names
        .zip(values)
        .map(|(name, value)| (name.to_string(), Value::Str(value.to_string())))
        .collect();
    if fields.is_empty() {
        return Err(BioLangError::type_error(
            format!("{context}: provider manifest is empty"),
            None,
        ));
    }
    Ok(fields)
}

#[cfg(not(target_arch = "wasm32"))]
fn run_external_provider(program: &str, arguments: &[String], context: &str) -> Result<()> {
    let operation = arguments.first().map(String::as_str).unwrap_or("unknown");
    eprintln!(
        "BioLang single-cell backend: external provider '{program}' ({operation}, requested by {context})"
    );
    let output = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!(
                    "{context}: cannot start external provider '{program}': {error}. Install bl-seurat-provider or set BIOLANG_SEURAT_PROVIDER"
                ),
                None,
            )
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!(
                "{context}: external provider '{program}' failed with {}: {}",
                output
                    .status
                    .code()
                    .map_or_else(|| "no exit code".to_string(), |code| code.to_string()),
                stderr.trim()
            ),
            None,
        ));
    }
    Ok(())
}

struct ExternalCcaResult {
    left_embedding: Vec<Vec<f64>>,
    right_embedding: Vec<Vec<f64>>,
    filter_features: Vec<usize>,
    weight_reduction: Vec<Vec<f64>>,
    manifest: HashMap<String, Value>,
    program: String,
}

#[cfg(not(target_arch = "wasm32"))]
fn external_cca(
    program: String,
    left: &[Vec<f64>],
    right: &[Vec<f64>],
    dimensions: usize,
    seed: u64,
    max_features: usize,
) -> Result<ExternalCcaResult> {
    let directory = tempfile::Builder::new()
        .prefix("biolang-seurat-cca-")
        .tempdir()
        .map_err(|error| BioLangError::runtime(ErrorKind::IOError, error.to_string(), None))?;
    let left_path = directory.path().join("left.f64");
    let right_path = directory.path().join("right.f64");
    let output_path = directory.path().join("output");
    std::fs::create_dir(&output_path)
        .map_err(|error| BioLangError::runtime(ErrorKind::IOError, error.to_string(), None))?;
    let columns = left.first().map(Vec::len).unwrap_or(0);
    write_blmat_values(&left_path, left.len(), columns, |row, column| {
        left[row][column]
    })?;
    write_blmat_values(&right_path, right.len(), columns, |row, column| {
        right[row][column]
    })?;
    run_external_provider(
        &program,
        &[
            "cca".to_string(),
            left_path.to_string_lossy().into_owned(),
            right_path.to_string_lossy().into_owned(),
            output_path.to_string_lossy().into_owned(),
            dimensions.to_string(),
            seed.to_string(),
            max_features.to_string(),
        ],
        "sc_find_anchors()",
    )?;
    let left_embedding =
        read_blmat_values(&output_path.join("left-embedding.f64"), "sc_find_anchors()")?;
    let right_embedding = read_blmat_values(
        &output_path.join("right-embedding.f64"),
        "sc_find_anchors()",
    )?;
    let weight_reduction = read_blmat_values(
        &output_path.join("weight-reduction.f64"),
        "sc_find_anchors()",
    )?;
    let manifest = read_provider_manifest(&output_path.join("manifest.csv"), "sc_find_anchors()")?;
    let filter_file = std::fs::File::open(output_path.join("filter-features.csv"))
        .map_err(|error| BioLangError::runtime(ErrorKind::IOError, error.to_string(), None))?;
    let filter_features = BufReader::new(filter_file)
        .lines()
        .skip(1)
        .map(|line| {
            line.map_err(|error| {
                BioLangError::runtime(ErrorKind::IOError, error.to_string(), None)
            })?
            .trim()
            .parse::<usize>()
            .map_err(|error| {
                BioLangError::type_error(format!("invalid provider filter index: {error}"), None)
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if left_embedding.len() != left.len()
        || right_embedding.len() != right.len()
        || weight_reduction.len() != right.len()
        || left_embedding.first().map(Vec::len).unwrap_or(0) != dimensions
        || right_embedding.first().map(Vec::len).unwrap_or(0) != dimensions
        || weight_reduction.first().map(Vec::len).unwrap_or(0) != dimensions
        || filter_features.iter().any(|index| *index >= columns)
    {
        return Err(BioLangError::type_error(
            "sc_find_anchors(): external provider returned incompatible dimensions",
            None,
        ));
    }
    Ok(ExternalCcaResult {
        left_embedding,
        right_embedding,
        filter_features,
        weight_reduction,
        manifest,
        program,
    })
}

#[cfg(target_arch = "wasm32")]
fn external_cca(
    _program: String,
    _left: &[Vec<f64>],
    _right: &[Vec<f64>],
    _dimensions: usize,
    _seed: u64,
    _max_features: usize,
) -> Result<ExternalCcaResult> {
    Err(BioLangError::runtime(
        ErrorKind::IOError,
        "sc_find_anchors(): external providers are unavailable in WebAssembly; use the native CLI"
            .to_string(),
        None,
    ))
}

fn builtin_sc_pca(args: Vec<Value>) -> Result<Value> {
    let matrix = singlecell_matrix(&args[0], "sc_pca")?;
    let requested = if args.len() > 1 {
        let value = require_int(&args[1], "sc_pca")?;
        if value < 1 {
            return Err(BioLangError::type_error(
                "sc_pca() n_components must be at least 1",
                None,
            ));
        }
        value as usize
    } else {
        50
    };
    let center = match args.get(2) {
        Some(Value::Bool(value)) => *value,
        Some(other) => {
            return Err(BioLangError::type_error(
                format!("sc_pca() center must be Bool, got {}", other.type_of()),
                None,
            ))
        }
        None => true,
    };
    let opts = match args.get(3) {
        Some(Value::Record(fields)) => fields.as_ref().clone(),
        _ => HashMap::new(),
    };
    if matches!(opts.get("solver"), Some(Value::Str(name)) if name.eq_ignore_ascii_case("external"))
    {
        let program = external_provider_program(&opts).unwrap_or_else(|| {
            std::env::var("BIOLANG_SEURAT_PROVIDER")
                .unwrap_or_else(|_| "bl-seurat-provider".to_string())
        });
        return builtin_sc_pca_matrix_external(&matrix, requested, center, &program);
    }
    if matches!(opts.get("solver"), Some(Value::Str(name)) if name.eq_ignore_ascii_case("lanczos"))
    {
        return builtin_sc_pca_matrix_lanczos(&matrix, requested, center, &opts);
    }
    builtin_sc_pca_matrix(&matrix, requested, center)
}

#[cfg(not(target_arch = "wasm32"))]
fn builtin_sc_pca_matrix_external(
    matrix: &SingleCellMatrix<'_>,
    requested: usize,
    center: bool,
    program: &str,
) -> Result<Value> {
    let (n_cells, n_genes) = matrix.dimensions();
    // IRLBA requires the requested rank to be strictly smaller than both
    // matrix dimensions. Real single-cell runs request 50 of thousands of
    // features; the extra bound mainly makes tiny provider smoke tests valid.
    let n_components = requested
        .min(n_genes.saturating_sub(1))
        .min(n_cells.saturating_sub(1));
    if n_components == 0 {
        return Err(BioLangError::type_error(
            "sc_pca(): external PCA requires at least two cells and one feature",
            None,
        ));
    }
    let directory = tempfile::Builder::new()
        .prefix("biolang-seurat-pca-")
        .tempdir()
        .map_err(|error| BioLangError::runtime(ErrorKind::IOError, error.to_string(), None))?;
    let input_path = directory.path().join("input.f64");
    let output_path = directory.path().join("output");
    std::fs::create_dir(&output_path)
        .map_err(|error| BioLangError::runtime(ErrorKind::IOError, error.to_string(), None))?;
    write_blmat_values(&input_path, n_cells, n_genes, |row, column| {
        matrix.value_at(row, column)
    })?;
    run_external_provider(
        program,
        &[
            "pca".to_string(),
            input_path.to_string_lossy().into_owned(),
            output_path.to_string_lossy().into_owned(),
            n_components.to_string(),
            "42".to_string(),
            center.to_string(),
        ],
        "sc_pca()",
    )?;
    let scores = read_blmat_values(&output_path.join("scores.f64"), "sc_pca()")?;
    let loadings = read_blmat_values(&output_path.join("loadings.f64"), "sc_pca()")?;
    let manifest = read_provider_manifest(&output_path.join("manifest.csv"), "sc_pca()")?;
    if scores.len() != n_cells
        || loadings.len() != n_genes
        || scores.first().map(Vec::len).unwrap_or(0) != n_components
        || loadings.first().map(Vec::len).unwrap_or(0) != n_components
    {
        return Err(BioLangError::type_error(
            "sc_pca(): external provider returned incompatible dimensions",
            None,
        ));
    }
    let (sums, sums_squared) = matrix.column_moments();
    let observed_means: Vec<f64> = sums.iter().map(|sum| sum / n_cells as f64).collect();
    let means = if center {
        observed_means.clone()
    } else {
        vec![0.0; n_genes]
    };
    let divisor = n_cells.saturating_sub(1).max(1) as f64;
    let total_variance = sums_squared
        .iter()
        .zip(&observed_means)
        .map(|(sum_squared, mean)| {
            ((sum_squared - n_cells as f64 * mean * mean) / divisor).max(0.0)
        })
        .sum::<f64>();
    let explained_variance: Vec<f64> = (0..n_components)
        .map(|component| {
            scores
                .iter()
                .map(|row| row[component] * row[component])
                .sum::<f64>()
                / divisor
        })
        .collect();
    let explained_variance_ratio = explained_variance
        .iter()
        .map(|variance| {
            Value::Float(if total_variance > 0.0 {
                variance / total_variance
            } else {
                0.0
            })
        })
        .collect::<Vec<_>>();
    Ok(Value::Record(
        HashMap::from([
            ("scores".to_string(), matrix_to_value(scores)),
            ("loadings".to_string(), matrix_to_value(loadings)),
            (
                "explained_variance".to_string(),
                Value::List(
                    explained_variance
                        .into_iter()
                        .map(Value::Float)
                        .collect::<Vec<_>>()
                        .into(),
                ),
            ),
            (
                "explained_variance_ratio".to_string(),
                Value::List(explained_variance_ratio.into()),
            ),
            (
                "mean".to_string(),
                Value::List(
                    means
                        .into_iter()
                        .map(Value::Float)
                        .collect::<Vec<_>>()
                        .into(),
                ),
            ),
            ("n_components".to_string(), Value::Int(n_components as i64)),
            ("sweeps".to_string(), Value::Int(0)),
            ("converged".to_string(), Value::Bool(true)),
            (
                "compute_method".to_string(),
                Value::Str(format!("external_process:{program}")),
            ),
            (
                "external_provider_manifest".to_string(),
                Value::Record(manifest.into()),
            ),
        ])
        .into(),
    ))
}

#[cfg(target_arch = "wasm32")]
fn builtin_sc_pca_matrix_external(
    _matrix: &SingleCellMatrix<'_>,
    _requested: usize,
    _center: bool,
    _program: &str,
) -> Result<Value> {
    Err(BioLangError::runtime(
        ErrorKind::IOError,
        "sc_pca(): external providers are unavailable in WebAssembly; use the native CLI"
            .to_string(),
        None,
    ))
}

/// Direct-matrix truncated SVD compatibility path. This is opt-in because the
/// ordinary block PCA is faster and needs no random start; the restarted
/// Lanczos path exists for workflows that must reproduce an IRLBA-style
/// stopping point rather than the more fully converged principal subspace.
fn builtin_sc_pca_matrix_lanczos(
    matrix: &SingleCellMatrix<'_>,
    requested: usize,
    center: bool,
    opts: &HashMap<String, Value>,
) -> Result<Value> {
    let (n_cells, n_genes) = matrix.dimensions();
    let n_components = requested.min(n_genes).min(n_cells.saturating_sub(1));
    let (sums, sums_squared) = matrix.column_moments();
    let observed_means: Vec<f64> = if n_cells == 0 {
        vec![0.0; n_genes]
    } else {
        sums.iter().map(|sum| sum / n_cells as f64).collect()
    };
    let means = if center {
        observed_means.clone()
    } else {
        vec![0.0; n_genes]
    };
    let total_variance = if n_cells > 1 {
        sums_squared
            .iter()
            .zip(&observed_means)
            .map(|(sum_squared, mean)| {
                ((sum_squared - n_cells as f64 * mean * mean) / (n_cells - 1) as f64).max(0.0)
            })
            .sum::<f64>()
    } else {
        0.0
    };
    let initial: Option<Vec<f64>> = match opts.get("initial") {
        Some(Value::List(values)) => Some(
            values
                .iter()
                .map(|value| {
                    value.as_float().ok_or_else(|| {
                        BioLangError::type_error("sc_pca() initial must be a List<Number>", None)
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        ),
        Some(_) => {
            return Err(BioLangError::type_error(
                "sc_pca() initial must be a List<Number>",
                None,
            ))
        }
        None => None,
    };
    let work_extra = record_number(opts, "work_extra", 7.0).clamp(2.0, 256.0) as usize;
    let tolerance = record_number(opts, "tolerance", 1e-5).clamp(1e-12, 1.0);
    let max_iterations =
        record_number(opts, "max_iterations", 1000.0).clamp(1.0, 10_000.0) as usize;
    let seed = record_number(opts, "seed", 42.0).max(1.0) as u64;

    let forward = |basis: &[Vec<f64>]| {
        let width = basis.len();
        let mut packed = vec![0.0; n_genes * width];
        for (component, vector) in basis.iter().enumerate() {
            for (gene, value) in vector.iter().copied().enumerate() {
                packed[gene * width + component] = value;
            }
        }
        let applied = matrix.multiply_centered_block(&means, &packed, width);
        let columns = (0..width)
            .map(|component| {
                (0..n_cells)
                    .map(|cell| applied[cell * width + component])
                    .collect()
            })
            .collect();
        (columns, false)
    };
    let reverse = |basis: &[Vec<f64>]| {
        let width = basis.len();
        let mut packed = vec![0.0; n_cells * width];
        for (component, vector) in basis.iter().enumerate() {
            for (cell, value) in vector.iter().copied().enumerate() {
                packed[cell * width + component] = value;
            }
        }
        let applied = matrix.transpose_multiply_centered_block(&means, &packed, width);
        let columns = (0..width)
            .map(|component| {
                (0..n_genes)
                    .map(|gene| applied[gene * width + component])
                    .collect()
            })
            .collect();
        (columns, false)
    };
    let (left, loadings, singular, _, iterations, converged) = restarted_lanczos_svd_with(
        n_cells,
        n_genes,
        n_genes,
        n_components,
        work_extra,
        tolerance,
        max_iterations,
        seed,
        initial.as_deref(),
        forward,
        reverse,
    )?;
    let scores: Vec<Vec<f64>> = left
        .into_iter()
        .map(|row| {
            row.into_iter()
                .zip(&singular)
                .map(|(value, scale)| value * scale)
                .collect()
        })
        .collect();
    let explained_variance: Vec<f64> = singular
        .iter()
        .map(|value| value * value / n_cells.saturating_sub(1).max(1) as f64)
        .collect();
    let explained_variance_ratio = explained_variance
        .iter()
        .map(|variance| {
            Value::Float(if total_variance > 0.0 {
                variance / total_variance
            } else {
                0.0
            })
        })
        .collect::<Vec<_>>();
    let compact = matches!(opts.get("compact"), Some(Value::Bool(true)));
    let scores_value = if compact {
        matrix_to_compact_value(scores, "sc_pca")?
    } else {
        matrix_to_value(scores)
    };
    let loadings_value = if compact {
        matrix_to_compact_value(loadings, "sc_pca")?
    } else {
        matrix_to_value(loadings)
    };
    Ok(Value::Record(
        HashMap::from([
            ("scores".to_string(), scores_value),
            ("loadings".to_string(), loadings_value),
            (
                "explained_variance".to_string(),
                Value::List(
                    explained_variance
                        .into_iter()
                        .map(Value::Float)
                        .collect::<Vec<_>>()
                        .into(),
                ),
            ),
            (
                "singular_values".to_string(),
                Value::List(
                    singular
                        .iter()
                        .copied()
                        .map(Value::Float)
                        .collect::<Vec<_>>()
                        .into(),
                ),
            ),
            (
                "explained_variance_ratio".to_string(),
                Value::List(explained_variance_ratio.into()),
            ),
            (
                "mean".to_string(),
                Value::List(
                    means
                        .into_iter()
                        .map(Value::Float)
                        .collect::<Vec<_>>()
                        .into(),
                ),
            ),
            ("n_components".to_string(), Value::Int(n_components as i64)),
            ("sweeps".to_string(), Value::Int(iterations as i64)),
            ("converged".to_string(), Value::Bool(converged)),
            (
                "compute_method".to_string(),
                Value::Str("direct_matrix_restarted_lanczos_cpu".to_string()),
            ),
        ])
        .into(),
    ))
}

/// PCA implementation shared by the public builtin and native single-cell
/// stages. Keeping the matrix in `SingleCellMatrix` form is important for
/// large integration objects: a language `List<List<Float>>` representation
/// costs several times the raw f64 storage and used to dominate peak memory.
fn builtin_sc_pca_matrix(
    matrix: &SingleCellMatrix<'_>,
    requested: usize,
    center: bool,
) -> Result<Value> {
    let (n_cells, n_genes) = matrix.dimensions();
    let n_components = requested.min(n_genes).min(n_cells.saturating_sub(1));
    let (sums, sums_squared) = matrix.column_moments();
    let observed_means: Vec<f64> = if n_cells == 0 {
        vec![0.0; n_genes]
    } else {
        sums.iter().map(|sum| sum / n_cells as f64).collect()
    };
    let means = if center {
        observed_means.clone()
    } else {
        vec![0.0; n_genes]
    };
    let total_variance = if n_cells > 1 {
        sums_squared
            .iter()
            .zip(&observed_means)
            .map(|(sum_squared, mean)| {
                ((sum_squared - n_cells as f64 * mean * mean) / (n_cells - 1) as f64).max(0.0)
            })
            .sum::<f64>()
    } else {
        0.0
    };

    // Subspace iteration with a Rayleigh-Ritz projection, replacing deflated
    // power iteration.
    //
    // The old routine solved for one component at a time: power-iterate, keep
    // it, orthogonalise the next against it. That fails in two ways here. Power
    // iteration separates two eigenvectors at a rate set by the ratio of their
    // eigenvalues, and past the first handful of PCs a single-cell spectrum is
    // nearly flat - PC9 and PC10 differ by a few percent, so forty iterations
    // moved almost nothing. And deflation compounds: whatever error survives in
    // one component is injected into every component after it. Measured on the
    // course data, 8 of 40 components came back with explained variance *higher*
    // than their predecessor, which is not a rounding artefact but a statement
    // that they were not principal components at all.
    //
    // Iterating the whole block at once fixes both. The rate that governs a
    // block is lambda_{k+p} / lambda_k rather than the gap between neighbours,
    // so oversampling by `p` extra vectors buys convergence that no amount of
    // per-vector iteration can. The closing Rayleigh-Ritz step diagonalises the
    // problem *within* the converged subspace, which is what makes the result
    // ordered and mutually orthogonal by construction rather than by hope.
    // A ceiling, not a schedule: the loop below stops when the returned
    // components stop moving. Keep the default ceiling bounded, however.
    // Near-degenerate values at the requested-component boundary can make the
    // full returned span rotate for hundreds of sweeps even after the leading
    // components used downstream have settled. On the 14,847 x 3,000 HBC
    // residual matrix, 30 versus 132 converged sweeps gives >= 0.9999999
    // same-index correlation through PC40; the default of 50 keeps additional
    // margin for the tail while preventing a pathological 300-sweep run.
    // Users who need a more precise tail can raise BIOLANG_PCA_MAX_SWEEPS.
    let max_sweeps: usize = std::env::var("BIOLANG_PCA_MAX_SWEEPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50);
    const OVERSAMPLE: usize = 10;

    let block_width = (n_components + OVERSAMPLE).min(n_genes).min(n_cells);
    // A deterministic start, so two runs of the same script agree. The spiral
    // is arbitrary; all it has to do is avoid being orthogonal to the leading
    // eigenvectors, which a low-discrepancy pattern does more reliably than a
    // pseudo-random one.
    let mut block: Vec<Vec<f64>> = (0..block_width)
        .map(|column| {
            (0..n_genes)
                .map(|gene| (((gene + 1) * (column + 1)) as f64 * 1.618_033_988_75).sin())
                .collect()
        })
        .collect();
    orthonormalise_block(&mut block);

    // Every cell, every sweep.
    //
    // This previously fitted the loadings on 5,000 evenly spaced cells and
    // stopped after six sweeps, on the reasoning that a subset is
    // representative. Two things were wrong with that. Six sweeps of subspace
    // iteration does not converge a single-cell spectrum, which is nearly flat
    // past the leading handful — measured on the course data, PCs 14, 17, 26,
    // 28 and 30 came back with *more* variance than the component before them,
    // which a principal component cannot do and which is the same symptom the
    // comment above records as already fixed. And a 17% subsample estimates a
    // different subspace, not a noisier version of the same one: against an
    // independent Seurat run the leading principal angles agreed to cos 0.99
    // while the last few collapsed to 0.57, 0.40, 0.27, 0.04. Those unconverged
    // trailing components go straight into the 40-PC neighbour graph.
    //
    // Doing it properly is also cheaper here, because the loop below now
    // applies the covariance to the whole block in one pass rather than
    // re-reading the matrix once per vector.
    let apply_covariance_block = |columns: &[Vec<f64>]| -> Vec<Vec<f64>> {
        let width = columns.len();
        let mut packed = vec![0.0f64; n_genes * width];
        for (column, values) in columns.iter().enumerate() {
            for (gene, &value) in values.iter().enumerate() {
                packed[gene * width + column] = value;
            }
        }
        let scores = matrix.multiply_centered_block(&means, &packed, width);
        drop(packed);
        let applied = matrix.transpose_multiply_centered_block(&means, &scores, width);
        (0..width)
            .map(|column| {
                (0..n_genes)
                    .map(|gene| applied[gene * width + column])
                    .collect()
            })
            .collect()
    };

    // Iterate until the subspace that will actually be *returned* stops moving.
    //
    // Two earlier criteria both failed to fire, so every run spent the full
    // ceiling of 300 sweeps. Watching Rayleigh quotients fails because on this
    // spectrum they never settle: measured on the course data the largest
    // relative quotient change falls to 2e-3 by sweep 33 and then climbs back to
    // 4e-3, as near-degenerate Ritz values trade places indefinitely. Watching
    // the whole block's span fails for a different reason: the block carries ten
    // oversampling vectors whose only job is to accelerate the ones in front of
    // them, and they keep swinging long after the leading forty have stopped.
    //
    // So do the Rayleigh-Ritz step every sweep — it costs a `width` x `width`
    // eigenproblem, nothing against a sweep of many billion operations — and
    // compare the top `n_components` Ritz vectors with the previous sweep's.
    // Both sets are orthonormal, so ||V_old^T v_new|| is the length of the new
    // vector's projection into the old span and sqrt(1 - that^2) is how far it
    // has stepped outside; the worst vector gives the largest principal angle
    // between the two subspaces. That is precisely "has the answer settled", and
    // it is blind to rotation within the subspace, which is the thing that never
    // settles and never mattered.
    //
    // Directly measured against a 300-sweep run on the course data: the span is
    // already identical at sweep 30 (minimum principal-angle cosine 1.0000) and
    // agrees to 0.9985 by sweep 15.
    let trace_sweeps = std::env::var_os("BIOLANG_PCA_TRACE_SHIFT").is_some();
    let mut previous_ritz: Vec<Vec<f64>> = Vec::new();
    let mut converged = false;
    let mut sweeps_used = 0usize;
    for _ in 0..max_sweeps {
        let applied = apply_covariance_block(&block);

        let span = block.len();
        let mut small = vec![vec![0.0f64; span]; span];
        for row in 0..span {
            for column in row..span {
                let entry: f64 = block[row]
                    .iter()
                    .zip(&applied[column])
                    .map(|(a, b)| a * b)
                    .sum();
                small[row][column] = entry;
                small[column][row] = entry;
            }
        }
        let (rotations, _) = jacobi_eigen_symmetric(&small);
        let ritz: Vec<Vec<f64>> = rotations
            .iter()
            .take(n_components)
            .map(|rotation| {
                let mut vector = vec![0.0f64; n_genes];
                for (weight, basis) in rotation.iter().zip(&block) {
                    for (value, component) in vector.iter_mut().zip(basis) {
                        *value += weight * component;
                    }
                }
                vector
            })
            .collect();

        let drift = if previous_ritz.len() == ritz.len() && !ritz.is_empty() {
            ritz.iter()
                .map(|fresh| {
                    let projected: f64 = previous_ritz
                        .iter()
                        .map(|old| {
                            let dot: f64 = old.iter().zip(fresh).map(|(a, b)| a * b).sum();
                            dot * dot
                        })
                        .sum();
                    (1.0 - projected.min(1.0)).max(0.0).sqrt()
                })
                .fold(0.0, f64::max)
        } else {
            f64::INFINITY
        };
        previous_ritz = ritz;

        let mut next = applied;
        orthonormalise_block(&mut next);
        if next.is_empty() {
            break;
        }
        block = next;
        sweeps_used += 1;
        if trace_sweeps {
            eprintln!("    sweep {sweeps_used}: returned-subspace drift {drift:.3e}");
        }
        if drift < 1e-6 {
            converged = true;
            break;
        }
    }
    if std::env::var_os("BIOLANG_PCA_TRACE").is_some() {
        eprintln!(
            "  sc_pca: {sweeps_used} sweeps, converged={converged}, {n_cells} cells x {n_genes} genes"
        );
    }

    // Rayleigh-Ritz: project the covariance onto the converged subspace and
    // diagonalise the small dense problem exactly.
    let width = block.len();
    let projected: Vec<Vec<f64>> = apply_covariance_block(&block);
    let mut small = vec![vec![0.0f64; width]; width];
    for i in 0..width {
        for j in i..width {
            let entry: f64 = block[i].iter().zip(&projected[j]).map(|(a, b)| a * b).sum();
            small[i][j] = entry;
            small[j][i] = entry;
        }
    }
    let (rotations, _) = jacobi_eigen_symmetric(&small);

    let mut components: Vec<Vec<f64>> = Vec::with_capacity(n_components);
    let mut score_columns: Vec<Vec<f64>> = Vec::with_capacity(n_components);
    let mut explained_variance = Vec::with_capacity(n_components);
    for rotation in rotations.iter().take(n_components) {
        let mut loading = vec![0.0f64; n_genes];
        for (weight, basis) in rotation.iter().zip(&block) {
            for (value, component) in loading.iter_mut().zip(basis) {
                *value += weight * component;
            }
        }
        let norm = loading.iter().map(|v| v * v).sum::<f64>().sqrt();
        if norm <= 1e-12 {
            break;
        }
        for value in &mut loading {
            *value /= norm;
        }
        // Sign is arbitrary in any eigendecomposition, so fix it by a rule
        // rather than leaving it to the arithmetic: the heaviest loading is
        // positive. Without this, an unrelated change upstream can flip a
        // component and make two identical analyses look different.
        let pivot = loading
            .iter()
            .copied()
            .fold(0.0f64, |acc, v| if v.abs() > acc.abs() { v } else { acc });
        if pivot < 0.0 {
            for value in &mut loading {
                *value = -*value;
            }
        }

        let scores = matrix.multiply_centered(&means, &loading);
        let variance = if n_cells > 1 {
            scores.iter().map(|value| value * value).sum::<f64>() / (n_cells - 1) as f64
        } else {
            0.0
        };
        if variance <= 1e-12 {
            break;
        }
        components.push(loading);
        score_columns.push(scores);
        explained_variance.push(variance);
    }
    let _ = converged;

    let actual_components = components.len();
    let scores: Vec<Vec<f64>> = (0..n_cells)
        .map(|cell| {
            score_columns
                .iter()
                .map(|component| component[cell])
                .collect()
        })
        .collect();
    let loadings: Vec<Vec<f64>> = (0..n_genes)
        .map(|gene| components.iter().map(|component| component[gene]).collect())
        .collect();
    let explained_variance_ratio: Vec<Value> = explained_variance
        .iter()
        .map(|variance| {
            Value::Float(if total_variance > 0.0 {
                variance / total_variance
            } else {
                0.0
            })
        })
        .collect();

    let mut result = HashMap::new();
    result.insert("scores".to_string(), matrix_to_value(scores));
    result.insert("loadings".to_string(), matrix_to_value(loadings));
    result.insert(
        "explained_variance".to_string(),
        Value::List(
            explained_variance
                .into_iter()
                .map(Value::Float)
                .collect::<Vec<_>>()
                .into(),
        ),
    );
    result.insert(
        "explained_variance_ratio".to_string(),
        Value::List(explained_variance_ratio.into()),
    );
    result.insert(
        "mean".to_string(),
        Value::List(
            means
                .into_iter()
                .map(Value::Float)
                .collect::<Vec<_>>()
                .into(),
        ),
    );
    result.insert(
        "n_components".to_string(),
        Value::Int(actual_components as i64),
    );
    result.insert("sweeps".to_string(), Value::Int(sweeps_used as i64));
    result.insert("converged".to_string(), Value::Bool(converged));
    Ok(Value::Record(result.into()))
}

fn require_indices(val: &Value, func: &str) -> Result<Vec<usize>> {
    match val {
        Value::List(items) => items
            .iter()
            .map(|value| match value {
                Value::Int(index) if *index >= 0 => Ok(*index as usize),
                Value::Float(index) if *index >= 0.0 && index.fract() == 0.0 => Ok(*index as usize),
                other => Err(BioLangError::type_error(
                    format!(
                        "{func}() indices must be non-negative Int values, got {}",
                        other.type_of()
                    ),
                    None,
                )),
            })
            .collect(),
        other => Err(BioLangError::type_error(
            format!(
                "{func}() requires a List of indices, got {}",
                other.type_of()
            ),
            None,
        )),
    }
}

fn builtin_select_rows(args: Vec<Value>) -> Result<Value> {
    let indices = require_indices(&args[1], "select_rows")?;
    match &args[0] {
        Value::Table(table) => {
            let mut rows = Vec::with_capacity(indices.len());
            for &index in &indices {
                let row = table.rows.get(index).ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::IndexOutOfBounds,
                        format!(
                            "select_rows() row index {index} is outside a table with {} rows",
                            table.rows.len()
                        ),
                        None,
                    )
                })?;
                rows.push(row.clone());
            }
            Ok(Value::Table(Table::new(table.columns.clone(), rows)))
        }
        Value::SparseMatrix(matrix) => {
            if let Some(index) = indices.iter().find(|&&index| index >= matrix.nrow) {
                return Err(BioLangError::runtime(
                    ErrorKind::IndexOutOfBounds,
                    format!(
                        "select_rows() row index {index} is outside a matrix with {} rows",
                        matrix.nrow
                    ),
                    None,
                ));
            }
            Ok(Value::SparseMatrix(std::sync::Arc::new(
                matrix.subset_rows(&indices),
            )))
        }
        _ => {
            let matrix = require_matrix(&args[0], "select_rows")?;
            let mut selected = Vec::with_capacity(indices.len());
            for index in indices {
                let row = matrix.get(index).ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::IndexOutOfBounds,
                        format!(
                            "select_rows() row index {index} is outside a matrix with {} rows",
                            matrix.len()
                        ),
                        None,
                    )
                })?;
                selected.push(row.clone());
            }
            Ok(matrix_to_value(selected))
        }
    }
}

fn builtin_matrix_at(args: Vec<Value>) -> Result<Value> {
    let row = require_int(&args[1], "matrix_at")?;
    let column = require_int(&args[2], "matrix_at")?;
    if row < 0 || column < 0 {
        return Err(BioLangError::runtime(
            ErrorKind::IndexOutOfBounds,
            "matrix_at() indices must be non-negative",
            None,
        ));
    }
    let row = row as usize;
    let column = column as usize;
    match &args[0] {
        Value::SparseMatrix(matrix) if row < matrix.nrow && column < matrix.ncol => {
            Ok(Value::Float(matrix.get(row, column)))
        }
        Value::Matrix(matrix) if row < matrix.nrow && column < matrix.ncol => {
            Ok(Value::Float(matrix.get(row, column)))
        }
        Value::List(rows) => rows
            .get(row)
            .and_then(|value| match value {
                Value::List(values) => values.get(column),
                _ => None,
            })
            .cloned()
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::IndexOutOfBounds,
                    format!("matrix_at() index ({row}, {column}) is outside the matrix"),
                    None,
                )
            }),
        Value::Table(table) => table
            .rows
            .get(row)
            .and_then(|values| values.get(column))
            .cloned()
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::IndexOutOfBounds,
                    format!("matrix_at() index ({row}, {column}) is outside the table"),
                    None,
                )
            }),
        other => Err(BioLangError::type_error(
            format!(
                "matrix_at() requires a matrix-like value, got {}",
                other.type_of()
            ),
            None,
        )),
    }
}

fn builtin_select_cols(args: Vec<Value>) -> Result<Value> {
    let indices = require_indices(&args[1], "select_cols")?;
    if let Value::SparseMatrix(matrix) = &args[0] {
        if let Some(index) = indices.iter().find(|&&index| index >= matrix.ncol) {
            return Err(BioLangError::runtime(
                ErrorKind::IndexOutOfBounds,
                format!(
                    "select_cols() column index {index} is outside a matrix with {} columns",
                    matrix.ncol
                ),
                None,
            ));
        }
        return Ok(Value::SparseMatrix(std::sync::Arc::new(
            matrix.subset_cols(&indices),
        )));
    }

    // Preserve the compact representation of a native dense Matrix. Going
    // through `require_matrix` and `matrix_to_value` used to copy the source
    // into `Vec<Vec<f64>>` and then box every selected number as a `Value`.
    // Selecting 3,000 SCT features for the HBC object therefore turned a
    // roughly 711 MB numeric matrix into several gigabytes of interpreter
    // values before anchor finding had even started.
    if let Value::Matrix(matrix) = &args[0] {
        if let Some(index) = indices.iter().find(|&&index| index >= matrix.ncol) {
            return Err(BioLangError::runtime(
                ErrorKind::IndexOutOfBounds,
                format!(
                    "select_cols() column index {index} is outside a matrix with {} columns",
                    matrix.ncol
                ),
                None,
            ));
        }
        let mut data = Vec::with_capacity(matrix.nrow.saturating_mul(indices.len()));
        for row in 0..matrix.nrow {
            data.extend(
                indices
                    .iter()
                    .map(|&column| matrix.data[row * matrix.ncol + column]),
            );
        }
        let selected = bl_core::matrix::Matrix::new(data, matrix.nrow, indices.len())
            .map_err(|error| BioLangError::type_error(format!("select_cols(): {error}"), None))?;
        return Ok(Value::Matrix(selected.into()));
    }

    let mat = require_matrix(&args[0], "select_cols")?;
    let ncol = mat.first().map(|r| r.len()).unwrap_or(0);
    if let Some(index) = indices.iter().find(|&&index| index >= ncol) {
        return Err(BioLangError::runtime(
            ErrorKind::IndexOutOfBounds,
            format!("select_cols() column index {index} is outside a matrix with {ncol} columns"),
            None,
        ));
    }
    let out: Vec<Vec<f64>> = mat
        .iter()
        .map(|row| indices.iter().map(|&j| row[j]).collect())
        .collect();
    Ok(matrix_to_value(out))
}

// â”€â”€ Section 6: Single-cell QC / normalisation â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

// â”€â”€ normalize_total(matrix, target=10000) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn subset_list(value: &Value, indices: &[usize], func: &str) -> Result<Value> {
    let values = match value {
        Value::List(values) => values,
        other => {
            return Err(BioLangError::type_error(
                format!("{func}() expected List, got {}", other.type_of()),
                None,
            ))
        }
    };
    let mut selected = Vec::with_capacity(indices.len());
    for &index in indices {
        selected.push(
            values
                .get(index)
                .ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::IndexOutOfBounds,
                        format!(
                            "{func}() index {index} is outside a list with {} values",
                            values.len()
                        ),
                        None,
                    )
                })?
                .clone(),
        );
    }
    Ok(Value::List(selected.into()))
}

fn builtin_sc_subset_cells(args: Vec<Value>) -> Result<Value> {
    let object = match &args[0] {
        Value::Record(object) => object,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "sc_subset_cells() requires a single-cell Record, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    let indices = require_indices(&args[1], "sc_subset_cells")?;
    let index_value = Value::List(
        indices
            .iter()
            .map(|index| Value::Int(*index as i64))
            .collect::<Vec<_>>()
            .into(),
    );
    let mut result = object.as_ref().clone();

    for field in [
        "matrix",
        "norm_matrix",
        "hvg_matrix",
        "scaled_matrix",
        "integrated_matrix",
        "pca_scores",
        "umap",
    ] {
        if let Some(value) = object.get(field) {
            result.insert(
                field.to_string(),
                builtin_select_rows(vec![value.clone(), index_value.clone()])?,
            );
        }
    }
    for field in [
        "barcodes",
        "clusters",
        "pseudotime",
        "doublet_scores",
        "is_doublet",
        "batch_ids",
        "cell_cycle_info",
        "module_scores",
    ] {
        if let Some(value) = object.get(field) {
            result.insert(
                field.to_string(),
                subset_list(value, &indices, "sc_subset_cells")?,
            );
        }
    }
    if let Some(value) = object.get("obs") {
        result.insert(
            "obs".to_string(),
            builtin_select_rows(vec![value.clone(), index_value.clone()])?,
        );
    }
    if let Some(Value::Record(layers)) = object.get("layers") {
        let mut selected_layers = layers.as_ref().clone();
        for (name, value) in layers.iter() {
            selected_layers.insert(
                name.clone(),
                builtin_select_rows(vec![value.clone(), index_value.clone()])?,
            );
        }
        result.insert("layers".to_string(), Value::Record(selected_layers.into()));
    }
    if let Some(Value::Record(assays)) = object.get("assays") {
        let mut selected_assays = assays.as_ref().clone();
        for (assay_name, assay_value) in assays.iter() {
            if let Value::Record(assay) = assay_value {
                let mut selected_assay = assay.as_ref().clone();
                if let Some(Value::Record(layers)) = assay.get("layers") {
                    let mut selected_layers = layers.as_ref().clone();
                    for (layer_name, value) in layers.iter() {
                        selected_layers.insert(
                            layer_name.clone(),
                            builtin_select_rows(vec![value.clone(), index_value.clone()])?,
                        );
                    }
                    selected_assay
                        .insert("layers".to_string(), Value::Record(selected_layers.into()));
                }
                selected_assays.insert(assay_name.clone(), Value::Record(selected_assay.into()));
            }
        }
        result.insert("assays".to_string(), Value::Record(selected_assays.into()));
    }
    if let Some(Value::Record(reductions)) = object.get("reductions") {
        let mut selected_reductions = reductions.as_ref().clone();
        for (name, reduction_value) in reductions.iter() {
            if let Value::Record(reduction) = reduction_value {
                let mut selected = reduction.as_ref().clone();
                if let Some(embedding) = reduction.get("embeddings") {
                    selected.insert(
                        "embeddings".to_string(),
                        builtin_select_rows(vec![embedding.clone(), index_value.clone()])?,
                    );
                }
                selected_reductions.insert(name.clone(), Value::Record(selected.into()));
            }
        }
        result.insert(
            "reductions".to_string(),
            Value::Record(selected_reductions.into()),
        );
    }
    if let Some(value) = object.get("idents") {
        if !matches!(value, Value::List(values) if values.is_empty()) {
            result.insert(
                "idents".to_string(),
                subset_list(value, &indices, "sc_subset_cells")?,
            );
        }
    }
    result.remove("knn");
    result.insert("graphs".to_string(), Value::Record(HashMap::new().into()));
    result.remove("cell_qc_table");
    result.insert("n_cells".to_string(), Value::Int(indices.len() as i64));
    Ok(Value::Record(result.into()))
}

fn builtin_sc_subset_genes(args: Vec<Value>) -> Result<Value> {
    let object = match &args[0] {
        Value::Record(object) => object,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "sc_subset_genes() requires a single-cell Record, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    let indices = require_indices(&args[1], "sc_subset_genes")?;
    let index_value = Value::List(
        indices
            .iter()
            .map(|index| Value::Int(*index as i64))
            .collect::<Vec<_>>()
            .into(),
    );
    let mut result = object.as_ref().clone();

    for field in ["matrix", "norm_matrix"] {
        if let Some(value) = object.get(field) {
            result.insert(
                field.to_string(),
                builtin_select_cols(vec![value.clone(), index_value.clone()])?,
            );
        }
    }
    if let Some(value) = object.get("genes") {
        result.insert(
            "genes".to_string(),
            subset_list(value, &indices, "sc_subset_genes")?,
        );
    }
    if let Some(value) = object.get("var") {
        result.insert(
            "var".to_string(),
            builtin_select_rows(vec![value.clone(), index_value.clone()])?,
        );
    }
    if let Some(Value::Record(layers)) = object.get("layers") {
        let mut selected_layers = layers.as_ref().clone();
        for (name, value) in layers.iter() {
            selected_layers.insert(
                name.clone(),
                builtin_select_cols(vec![value.clone(), index_value.clone()])?,
            );
        }
        result.insert("layers".to_string(), Value::Record(selected_layers.into()));
    }
    for field in [
        "hvg",
        "hvg_matrix",
        "hvg_genes",
        "scaled_matrix",
        "integrated_matrix",
        "integrated_features",
        "integrated_embedding",
        "integration_method",
        "pca",
        "pca_scores",
        "pca_loadings",
        "knn",
        "clusters",
        "cluster_inertia",
        "umap",
        "pseudotime",
        "gene_qc_table",
        "cell_qc_table",
    ] {
        result.remove(field);
    }
    let layers = result.get("layers").cloned().unwrap_or_else(|| {
        Value::Record(
            HashMap::from([(
                "counts".to_string(),
                result.get("matrix").cloned().unwrap_or(Value::Nil),
            )])
            .into(),
        )
    });
    let rna = HashMap::from([
        ("layers".to_string(), layers),
        (
            "variable_features".to_string(),
            Value::List(Vec::new().into()),
        ),
    ]);
    result.insert(
        "assays".to_string(),
        Value::Record(HashMap::from([("RNA".to_string(), Value::Record(rna.into()))]).into()),
    );
    result.insert("active_assay".to_string(), Value::Str("RNA".to_string()));
    result.insert(
        "reductions".to_string(),
        Value::Record(HashMap::new().into()),
    );
    result.insert("graphs".to_string(), Value::Record(HashMap::new().into()));
    result.insert("idents".to_string(), Value::List(Vec::new().into()));
    result.insert("n_genes".to_string(), Value::Int(indices.len() as i64));
    Ok(Value::Record(result.into()))
}

fn append_matrix_values(left: &Value, right: &Value, func: &str) -> Result<Value> {
    match (left, right) {
        (Value::SparseMatrix(left), Value::SparseMatrix(right)) => left
            .append_rows(right)
            .map(|m| Value::SparseMatrix(std::sync::Arc::new(m)))
            .map_err(|message| BioLangError::type_error(format!("{func}(): {message}"), None)),
        (Value::List(left), Value::List(right)) => {
            let mut values = left.as_ref().clone();
            values.extend(right.iter().cloned());
            Ok(Value::List(values.into()))
        }
        (Value::Table(left), Value::Table(right)) if left.columns == right.columns => {
            let mut rows = left.rows.clone();
            rows.extend(right.rows.iter().cloned());
            Ok(Value::Table(Table::new(left.columns.clone(), rows)))
        }
        _ => Err(BioLangError::type_error(
            format!(
                "{func}() cannot append {} and {}",
                left.type_of(),
                right.type_of()
            ),
            None,
        )),
    }
}

fn builtin_sc_merge_objects(args: Vec<Value>) -> Result<Value> {
    let left = match &args[0] {
        Value::Record(object) => object,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "sc_merge_objects() expected Record, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    let right = match &args[1] {
        Value::Record(object) => object,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "sc_merge_objects() expected Record, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    if left.get("genes") != right.get("genes") {
        return Err(BioLangError::type_error(
            "sc_merge_objects() requires identical genes in identical order",
            None,
        ));
    }
    let left_cells = left
        .get("n_cells")
        .and_then(|value| match value {
            Value::Int(count) if *count >= 0 => Some(*count as usize),
            _ => None,
        })
        .ok_or_else(|| {
            BioLangError::type_error("sc_merge_objects() left object has invalid n_cells", None)
        })?;
    let right_cells = right
        .get("n_cells")
        .and_then(|value| match value {
            Value::Int(count) if *count >= 0 => Some(*count as usize),
            _ => None,
        })
        .ok_or_else(|| {
            BioLangError::type_error("sc_merge_objects() right object has invalid n_cells", None)
        })?;

    let mut result = left.as_ref().clone();
    let left_matrix = left.get("matrix").ok_or_else(|| {
        BioLangError::type_error("sc_merge_objects() left object has no matrix", None)
    })?;
    let right_matrix = right.get("matrix").ok_or_else(|| {
        BioLangError::type_error("sc_merge_objects() right object has no matrix", None)
    })?;
    result.insert(
        "matrix".to_string(),
        append_matrix_values(left_matrix, right_matrix, "sc_merge_objects")?,
    );
    if let (Some(left_barcodes), Some(right_barcodes)) =
        (left.get("barcodes"), right.get("barcodes"))
    {
        result.insert(
            "barcodes".to_string(),
            append_matrix_values(left_barcodes, right_barcodes, "sc_merge_objects")?,
        );
    }
    if let (Some(left_obs), Some(right_obs)) = (left.get("obs"), right.get("obs")) {
        result.insert(
            "obs".to_string(),
            append_matrix_values(left_obs, right_obs, "sc_merge_objects")?,
        );
    }
    if let (Some(Value::Record(left_layers)), Some(Value::Record(right_layers))) =
        (left.get("layers"), right.get("layers"))
    {
        let mut layers = HashMap::new();
        for (name, left_layer) in left_layers.iter() {
            let right_layer = right_layers.get(name).ok_or_else(|| {
                BioLangError::type_error(
                    format!("sc_merge_objects() right object is missing layer '{name}'"),
                    None,
                )
            })?;
            layers.insert(
                name.clone(),
                append_matrix_values(left_layer, right_layer, "sc_merge_objects")?,
            );
        }
        result.insert("layers".to_string(), Value::Record(layers.into()));
    }
    let labels_for = |object: &HashMap<String, Value>,
                      cells: usize,
                      supplied: &Value,
                      side: &str|
     -> Result<Vec<Value>> {
        if !matches!(supplied, Value::Nil) {
            return Ok((0..cells).map(|_| supplied.clone()).collect());
        }
        match object.get("batch_ids") {
            Some(Value::List(existing)) if existing.len() == cells => {
                Ok(existing.iter().cloned().collect())
            }
            Some(Value::List(existing)) => Err(BioLangError::type_error(
                format!(
                    "sc_merge_objects() {side} object has {} batch_ids for {cells} cells",
                    existing.len()
                ),
                None,
            )),
            _ => Err(BioLangError::type_error(
                format!(
                    "sc_merge_objects() nil {side} batch label requires an existing batch_ids list"
                ),
                None,
            )),
        }
    };
    let mut batch_ids = labels_for(left, left_cells, &args[2], "left")?;
    batch_ids.extend(labels_for(right, right_cells, &args[3], "right")?);
    result.insert("batch_ids".to_string(), Value::List(batch_ids.into()));
    result.insert(
        "n_cells".to_string(),
        Value::Int((left_cells + right_cells) as i64),
    );
    for field in [
        "norm_matrix",
        "hvg",
        "hvg_matrix",
        "hvg_genes",
        "pca",
        "pca_scores",
        "pca_loadings",
        "integrated_embedding",
        "knn",
        "clusters",
        "cluster_inertia",
        "umap",
        "pseudotime",
        "gene_qc_table",
        "cell_qc_table",
    ] {
        result.remove(field);
    }
    let layers = result.get("layers").cloned().unwrap_or_else(|| {
        Value::Record(
            HashMap::from([(
                "counts".to_string(),
                result.get("matrix").cloned().unwrap_or(Value::Nil),
            )])
            .into(),
        )
    });
    let rna = HashMap::from([
        ("layers".to_string(), layers),
        (
            "variable_features".to_string(),
            Value::List(Vec::new().into()),
        ),
    ]);
    result.insert(
        "assays".to_string(),
        Value::Record(HashMap::from([("RNA".to_string(), Value::Record(rna.into()))]).into()),
    );
    result.insert("active_assay".to_string(), Value::Str("RNA".to_string()));
    result.insert(
        "reductions".to_string(),
        Value::Record(HashMap::new().into()),
    );
    result.insert("graphs".to_string(), Value::Record(HashMap::new().into()));
    result.insert("idents".to_string(), Value::List(Vec::new().into()));
    Ok(Value::Record(result.into()))
}

fn builtin_normalize_total(args: Vec<Value>) -> Result<Value> {
    let target = if args.len() > 1 {
        match &args[1] {
            Value::Float(f) => *f,
            Value::Int(n) => *n as f64,
            _ => {
                return Err(BioLangError::type_error(
                    "normalize_total() target must be a number",
                    None,
                ))
            }
        }
    } else {
        10_000.0
    };

    if !target.is_finite() || target < 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "normalize_total() target must be a finite non-negative number",
            None,
        ));
    }

    if let Value::SparseMatrix(matrix) = &args[0] {
        return Ok(Value::SparseMatrix(std::sync::Arc::new(
            matrix.normalize_rows(target),
        )));
    }

    let mat = require_matrix(&args[0], "normalize_total")?;
    let normalized: Vec<Vec<f64>> = mat
        .into_iter()
        .map(|row| {
            let total: f64 = row.iter().sum();
            if total == 0.0 {
                row
            } else {
                row.into_iter().map(|v| v / total * target).collect()
            }
        })
        .collect();

    Ok(matrix_to_value(normalized))
}

// â”€â”€ log1p_transform(matrix) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn builtin_log1p_transform(args: Vec<Value>) -> Result<Value> {
    if let Value::SparseMatrix(matrix) = &args[0] {
        if matrix.data.iter().any(|value| *value < -1.0) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "log1p_transform() values must be greater than or equal to -1",
                None,
            ));
        }
        return Ok(Value::SparseMatrix(std::sync::Arc::new(
            matrix.map_nonzero(|value| (value + 1.0).ln()),
        )));
    }

    let mat = require_matrix(&args[0], "log1p_transform")?;
    let transformed: Vec<Vec<f64>> = mat
        .into_iter()
        .map(|row| row.into_iter().map(|v| (v + 1.0).ln()).collect())
        .collect();
    Ok(matrix_to_value(transformed))
}

// â”€â”€ highly_variable_genes(matrix, n=2000) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Per-gene mean and bin-normalised dispersion, and the ranking built from them.
///
/// Shared with `variable_feature_plot`, which draws these statistics and marks
/// the genes this selection keeps. Computing them twice would let the figure
/// drift away from the selection it claims to illustrate - a plot that
/// highlights a different set than the pipeline used is worse than no plot.
pub(crate) struct HvgStats {
    /// Mean expression of every gene, including the never-observed ones.
    pub means: Vec<f64>,
    /// Genes with a non-zero mean, in gene order; the other vectors are indexed
    /// by position within this list.
    pub expressed: Vec<usize>,
    /// Raw dispersion (variance / mean) for each expressed gene.
    pub dispersions: Vec<f64>,
    /// Dispersion standardised within its mean-expression bin: the ranking key.
    pub normalised: Vec<f64>,
}

impl HvgStats {
    /// The `n` most variable genes, as gene indices, most variable first.
    pub fn select(&self, n: usize) -> Vec<usize> {
        let mut ranked: Vec<(usize, f64)> = self
            .expressed
            .iter()
            .enumerate()
            .map(|(position, &gene)| (gene, self.normalised[position]))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
            .into_iter()
            .take(n.min(self.means.len()))
            .map(|(gene, _)| gene)
            .collect()
    }
}

/// Variance-stabilising HVG selection, the `vst` method of Stuart et al. 2019.
///
/// Implemented from the published description, not from Seurat's source:
///
///   1. per-gene mean and variance of the raw counts,
///   2. a local regression of log10(variance) on log10(mean),
///   3. an expected standard deviation per gene, sqrt(10^fitted),
///   4. counts standardised by that expectation and clipped at sqrt(n_cells),
///   5. genes ranked by the variance of those standardised values.
///
/// Step 3 is the point of the method. Ranking on raw variance returns the
/// highest-expressed genes, which you already knew about; dividing by what a
/// gene of that expression level is *expected* to vary by leaves only the
/// genes that vary more than their abundance explains. The clip stops a
/// handful of extreme cells carrying a gene into the list.
///
/// The regression is a tricube-weighted local quadratic â€” the standard loess
/// construction â€” with span 0.3, Seurat's default. R's `loess` interpolates
/// over a kd-tree for speed rather than fitting at every point; this fits
/// directly, so the two agree in method and can differ in the last digits.
fn vst_standardised_variance(
    means: &[f64],
    variances: &[f64],
    columns: &[Vec<f64>],
    n_cells: usize,
) -> Vec<f64> {
    const SPAN: f64 = 0.3;
    let n_genes = means.len();

    // Fit only where both are positive; log of zero is not a data point.
    let fit_points: Vec<(f64, f64)> = (0..n_genes)
        .filter(|&g| means[g] > 0.0 && variances[g] > 0.0)
        .map(|g| (means[g].log10(), variances[g].log10()))
        .collect();
    if fit_points.len() < 3 {
        return variances.to_vec();
    }

    let clip = (n_cells as f64).sqrt();
    let mut out = vec![0.0; n_genes];
    for gene in 0..n_genes {
        if means[gene] <= 0.0 || variances[gene] <= 0.0 {
            continue;
        }
        let x = means[gene].log10();
        let fitted = loess_at(&fit_points, x, SPAN);
        let expected_sd = (10.0_f64.powf(fitted)).sqrt();
        if !expected_sd.is_finite() || expected_sd <= 0.0 {
            continue;
        }
        // Variance of the clipped standardised counts. Cells not stored in a
        // sparse column are zeros, and a zero standardises to -mean/sd, so the
        // absent entries contribute and cannot be skipped.
        let mu = means[gene];
        let stored = &columns[gene];
        let mut total = 0.0;
        let mut total_sq = 0.0;
        for &value in stored {
            let z = ((value - mu) / expected_sd).clamp(-clip, clip);
            total += z;
            total_sq += z * z;
        }
        let zero_z = ((0.0 - mu) / expected_sd).clamp(-clip, clip);
        let n_zero = n_cells.saturating_sub(stored.len()) as f64;
        total += zero_z * n_zero;
        total_sq += zero_z * zero_z * n_zero;

        let n = n_cells as f64;
        let mean_z = total / n;
        out[gene] = (total_sq / n - mean_z * mean_z).max(0.0);
    }
    out
}

/// One tricube-weighted local quadratic fit evaluated at `x`.
fn loess_at(points: &[(f64, f64)], x: f64, span: f64) -> f64 {
    let n = points.len();
    let window = ((span * n as f64).ceil() as usize).clamp(3, n);

    // The `window` nearest points by distance in x set the bandwidth.
    let mut distances: Vec<(f64, usize)> = points
        .iter()
        .enumerate()
        .map(|(i, (px, _))| ((px - x).abs(), i))
        .collect();
    distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let bandwidth = distances[window - 1].0.max(1e-12);

    // Weighted least squares on [1, dx, dx^2] via normal equations.
    let mut ata = [[0.0f64; 3]; 3];
    let mut atb = [0.0f64; 3];
    for &(distance, index) in distances.iter().take(window) {
        let u = (distance / bandwidth).min(1.0);
        let w = (1.0 - u * u * u).powi(3);
        if w <= 0.0 {
            continue;
        }
        let (px, py) = points[index];
        let dx = px - x;
        let basis = [1.0, dx, dx * dx];
        for r in 0..3 {
            for c in 0..3 {
                ata[r][c] += w * basis[r] * basis[c];
            }
            atb[r] += w * basis[r] * py;
        }
    }
    // Solve; fall back to the weighted mean if the system is degenerate.
    solve3(&mut ata, &mut atb).unwrap_or_else(|| {
        let (mut sw, mut sy) = (0.0, 0.0);
        for &(_, index) in distances.iter().take(window) {
            sw += 1.0;
            sy += points[index].1;
        }
        if sw > 0.0 {
            sy / sw
        } else {
            0.0
        }
    })
}

/// Gauss-Jordan on a 3x3 system, returning the intercept (the fit at dx = 0).
fn solve3(a: &mut [[f64; 3]; 3], b: &mut [f64; 3]) -> Option<f64> {
    for col in 0..3 {
        let pivot = (col..3).max_by(|&r1, &r2| {
            a[r1][col]
                .abs()
                .partial_cmp(&a[r2][col].abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;
        if a[pivot][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        let d = a[col][col];
        for c in col..3 {
            a[col][c] /= d;
        }
        b[col] /= d;
        for r in 0..3 {
            if r != col && a[r][col] != 0.0 {
                let factor = a[r][col];
                for c in col..3 {
                    a[r][c] -= factor * a[col][c];
                }
                b[r] -= factor * b[col];
            }
        }
    }
    b[0].is_finite().then(|| b[0])
}

fn builtin_highly_variable_genes(args: Vec<Value>) -> Result<Value> {
    let n_hvg = if args.len() > 1 {
        let value = require_int(&args[1], "highly_variable_genes")?;
        if value < 0 {
            return Err(BioLangError::type_error(
                "highly_variable_genes() n must be non-negative",
                None,
            ));
        }
        value as usize
    } else {
        2000
    };

    // method: "dispersion" (default, scanpy's seurat flavour) or "vst"
    // (Stuart et al. 2019, Seurat's default since v3). They rank genes
    // differently and neither is wrong; "vst" is what you want when comparing
    // against a Seurat analysis.
    let method = if args.len() > 2 {
        match &args[2] {
            Value::Str(s) => s.clone(),
            other => {
                return Err(BioLangError::type_error(
                    format!(
                        "highly_variable_genes() method must be Str, got {}",
                        other.type_of()
                    ),
                    None,
                ))
            }
        }
    } else {
        "dispersion".to_string()
    };

    if method == "vst" {
        let (n_cells, _n_genes, columns) = expression_columns(&args[0], "highly_variable_genes")?;
        let means: Vec<f64> = columns
            .iter()
            .map(|c| c.iter().sum::<f64>() / n_cells as f64)
            .collect();
        let variances: Vec<f64> = columns
            .iter()
            .zip(&means)
            .map(|(c, &mu)| {
                let stored: f64 = c.iter().map(|v| (v - mu) * (v - mu)).sum();
                let zeros = (n_cells - c.len()) as f64;
                (stored + zeros * mu * mu) / n_cells as f64
            })
            .collect();
        let scores = vst_standardised_variance(&means, &variances, &columns, n_cells);
        let mut ranked: Vec<(usize, f64)> = scores.iter().copied().enumerate().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let indices: Vec<Value> = ranked
            .into_iter()
            .take(n_hvg.min(scores.len()))
            .map(|(gene, _)| Value::Int(gene as i64))
            .collect();
        return Ok(Value::List((indices).into()));
    }

    let stats = hvg_statistics(&args[0], "highly_variable_genes")?;
    let indices: Vec<Value> = stats
        .select(n_hvg)
        .into_iter()
        .map(|gene| Value::Int(gene as i64))
        .collect();
    Ok(Value::List((indices).into()))
}

/// Compute [`HvgStats`] from a dense or sparse cells x genes matrix.
pub(crate) fn hvg_statistics(value: &Value, who: &str) -> Result<HvgStats> {
    let (n_cells, n_genes, sums, sums_squared) = match value {
        Value::SparseMatrix(matrix) => {
            let mut sums_squared = vec![0.0; matrix.ncol];
            for (&column, &value) in matrix.indices.iter().zip(&matrix.data) {
                sums_squared[column] += value * value;
            }
            (matrix.nrow, matrix.ncol, matrix.col_sums(), sums_squared)
        }
        _ => {
            let matrix = require_matrix(value, who)?;
            let n_genes = matrix.first().map(|row| row.len()).unwrap_or(0);
            let mut sums = vec![0.0; n_genes];
            let mut sums_squared = vec![0.0; n_genes];
            for row in &matrix {
                for (column, value) in row.iter().copied().enumerate() {
                    sums[column] += value;
                    sums_squared[column] += value * value;
                }
            }
            (matrix.len(), n_genes, sums, sums_squared)
        }
    };

    if n_cells == 0 || n_genes == 0 {
        return Ok(HvgStats {
            means: vec![0.0; n_genes],
            expressed: Vec::new(),
            dispersions: Vec::new(),
            normalised: Vec::new(),
        });
    }
    let n_cells_float = n_cells as f64;
    let means: Vec<f64> = sums.iter().map(|sum| sum / n_cells_float).collect();
    let variances: Vec<f64> = sums_squared
        .iter()
        .zip(&means)
        .map(|(sum_squared, mean)| (sum_squared / n_cells_float - mean * mean).max(0.0))
        .collect();

    // Rank genes by dispersion *relative to other genes of similar expression*.
    //
    // The previous version ranked cv2 = variance / mean^2, which is maximised by
    // genes with a near-zero mean: a transcript seen in two cells out of 2700
    // has an enormous cv2 by chance alone. On PBMC3k that selected MICALCL,
    // GJC3, AARD and a run of RP11- lncRNAs, while every canonical marker -
    // LYZ, MS4A1, GNLY, PPBP, CD14, NKG7, CD8A - was absent from the top 2000.
    // Clustering then ran on noise and split 2700 cells into 17 groups, where
    // the sample has nine known populations.
    //
    // Counts have a mean-variance relationship, so dispersion has to be judged
    // against that trend rather than in absolute terms. Bin the genes by mean
    // expression and standardise dispersion within each bin, which is what
    // Scanpy's `seurat` flavour does; Seurat's `vst` fits a smooth curve and
    // takes residuals, arriving at the same place by a different route.
    const N_BINS: usize = 20;

    // Genes never observed carry no information and would otherwise dominate
    // the lowest bin.
    let expressed: Vec<usize> = (0..n_genes).filter(|&j| means[j] > 0.0).collect();
    if expressed.is_empty() {
        return Ok(HvgStats {
            means,
            expressed,
            dispersions: Vec::new(),
            normalised: Vec::new(),
        });
    }

    // dispersion = variance / mean. Unlike cv2 this is flat in the mean for
    // Poisson noise, which is the baseline the binning then removes.
    let dispersions: Vec<f64> = expressed
        .iter()
        .map(|&j| variances[j] / means[j].max(1e-12))
        .collect();

    // Equal-count bins over mean expression, so every bin has enough genes to
    // give a meaningful spread.
    let mut by_mean: Vec<usize> = (0..expressed.len()).collect();
    by_mean.sort_by(|&a, &b| {
        means[expressed[a]]
            .partial_cmp(&means[expressed[b]])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut normalised = vec![0.0_f64; expressed.len()];
    let bin_size = by_mean.len().div_ceil(N_BINS).max(1);
    for chunk in by_mean.chunks(bin_size) {
        let count = chunk.len() as f64;
        let mean_dispersion: f64 = chunk.iter().map(|&i| dispersions[i]).sum::<f64>() / count;
        let variance_dispersion: f64 = chunk
            .iter()
            .map(|&i| (dispersions[i] - mean_dispersion).powi(2))
            .sum::<f64>()
            / count;
        let sd = variance_dispersion.sqrt();
        for &i in chunk {
            // A bin whose genes all share a dispersion has nothing to rank;
            // leaving them at zero keeps them out of the selection rather than
            // dividing by zero.
            normalised[i] = if sd > 1e-12 {
                (dispersions[i] - mean_dispersion) / sd
            } else {
                0.0
            };
        }
    }

    Ok(HvgStats {
        means,
        expressed,
        dispersions,
        normalised,
    })
}

// â”€â”€ cca(matrix1, matrix2, opts?) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Canonical correlation analysis: the shared axes of two datasets.
///
/// PCA on one dataset finds the directions it varies in. CCA takes two and
/// finds the directions along which they vary *together* - which is why Seurat
/// integration starts here. Sources of variation present in only one dataset,
/// which is what a batch effect is, score poorly by construction; shared
/// biology scores well.
///
/// Both matrices are cells x genes over the same genes, in the same order. The
/// cross-product is decomposed, and its singular vectors are the two datasets'
/// coordinates on a common set of axes: `u` for the first, `v` for the second.
/// Rows are L2-normalised, as Seurat normalises them, so the result is
/// comparable across datasets of different depth.
///
/// A caution worth stating before it is discovered on real data: the
/// cross-product is cells x cells, so this is quadratic in cell count. Two
/// samples of a few thousand cells is comfortable; two atlases of a hundred
/// thousand each is not, and `harmony_integrate` is the tool for that scale.
fn count_sketch_rows(matrix: &[Vec<f64>], width: usize) -> Vec<Vec<f64>> {
    matrix
        .iter()
        .map(|row| {
            let mut sketch = vec![0.0; width];
            for (feature, &value) in row.iter().enumerate() {
                let hash = (feature as u64)
                    .wrapping_mul(0x9e3779b97f4a7c15)
                    .rotate_left(17);
                let bucket = (hash as usize) % width;
                let sign = if hash & (1_u64 << 63) == 0 { 1.0 } else { -1.0 };
                sketch[bucket] += sign * value;
            }
            sketch
        })
        .collect()
}

fn cross_apply(left: &[Vec<f64>], right: &[Vec<f64>], vector: &[f64]) -> Vec<f64> {
    let width = left.first().map(Vec::len).unwrap_or(0);
    let mut feature = vec![0.0; width];
    for (row, &weight) in right.iter().zip(vector) {
        for (slot, &value) in feature.iter_mut().zip(row) {
            *slot += value * weight;
        }
    }
    left.iter()
        .map(|row| row.iter().zip(&feature).map(|(a, b)| a * b).sum())
        .collect()
}

fn cross_apply_block_cpu(
    left: &[Vec<f64>],
    right: &[Vec<f64>],
    basis: &[Vec<f64>],
) -> Vec<Vec<f64>> {
    basis
        .iter()
        .map(|vector| cross_apply(left, right, vector))
        .collect()
}

fn cross_apply_block(
    left: &[Vec<f64>],
    right: &[Vec<f64>],
    basis: &[Vec<f64>],
) -> (Vec<Vec<f64>>, bool) {
    match crate::gpu::cross_apply_block(left, right, basis) {
        Ok(Some(result)) => (result, true),
        // A runtime driver failure must not make analysis unavailable. The
        // f64 implementation is also the reproducibility reference path.
        Ok(None) | Err(_) => (cross_apply_block_cpu(left, right, basis), false),
    }
}

/// Truncated SVD of X Y' without constructing the quadratic cell matrix.
fn scalable_cross_svd(
    first: &[Vec<f64>],
    second: &[Vec<f64>],
    requested: usize,
    sweeps: usize,
    oversample: usize,
) -> Result<(Vec<Vec<f64>>, Vec<Vec<f64>>, Vec<f64>, bool, bool)> {
    let source_width = first.first().map(Vec::len).unwrap_or(0);
    // At the normal 2,000-3,000 integration-feature scale, work directly on
    // the input. This remains matrix-free (the quadratic cells x cells cross
    // product is never allocated) and avoids corrupting the projected gene
    // loadings used by Seurat's TopDimFeatures filter. Wider custom analyses
    // retain a bounded CountSketch subspace.
    let sketch_width = source_width.min(3_000).max(1);
    let left_storage =
        (sketch_width < source_width).then(|| count_sketch_rows(first, sketch_width));
    let right_storage =
        (sketch_width < source_width).then(|| count_sketch_rows(second, sketch_width));
    let left = left_storage.as_deref().unwrap_or(first);
    let right = right_storage.as_deref().unwrap_or(second);
    // The HBC spectrum is nearly degenerate around the trailing requested
    // components. Eight oversampling vectors converged the leading axes but
    // left the weakest CCA directions a few degrees from IRLBA, which was
    // enough to perturb anchor identities. A 32-vector guard subspace keeps
    // those neighbouring singular directions during iteration without ever
    // materialising the quadratic cell cross-product.
    let block_width = (requested + oversample)
        .min(first.len())
        .min(second.len())
        .max(1);
    let mut right_basis: Vec<Vec<f64>> = (0..block_width)
        .map(|component| {
            (0..right.len())
                .map(|row| {
                    let phase = (row + 1) as f64 * (component + 1) as f64;
                    (phase * 0.618_033_988_749_894_9).sin()
                        + (phase * 0.414_213_562_373_095_0).cos()
                })
                .collect()
        })
        .collect();
    orthonormalise_block(&mut right_basis);
    let mut left_basis = Vec::new();
    let mut used_gpu = false;
    // Extra block-power passes matter mainly for the trailing CCA components.
    // Twelve passes plus the wider guard subspace converge the full requested
    // space while retaining the same O(cells * (features + block_width))
    // memory bound.
    for _ in 0..sweeps {
        let (next_left, accelerated) = cross_apply_block(&left, &right, &right_basis);
        left_basis = next_left;
        used_gpu |= accelerated;
        orthonormalise_block(&mut left_basis);
        let (next_right, accelerated) = cross_apply_block(&right, &left, &left_basis);
        right_basis = next_right;
        used_gpu |= accelerated;
        orthonormalise_block(&mut right_basis);
    }
    let (next_left, accelerated) = cross_apply_block(&left, &right, &right_basis);
    left_basis = next_left;
    used_gpu |= accelerated;
    orthonormalise_block(&mut left_basis);
    let rank = left_basis.len().min(right_basis.len());
    left_basis.truncate(rank);
    right_basis.truncate(rank);

    let (applied_right, accelerated) = cross_apply_block(&left, &right, &right_basis);
    used_gpu |= accelerated;
    let small: Vec<f64> = left_basis
        .iter()
        .flat_map(|left_vector| {
            applied_right.iter().map(move |right_vector| {
                left_vector
                    .iter()
                    .zip(right_vector)
                    .map(|(a, b)| a * b)
                    .sum::<f64>()
            })
        })
        .collect();
    let small = bl_core::matrix::Matrix::new(small, rank, rank).map_err(|error| {
        BioLangError::runtime(ErrorKind::TypeError, format!("cca(): {error}"), None)
    })?;
    let (u_small, singular, vt_small) = small.svd().map_err(|error| {
        BioLangError::runtime(ErrorKind::TypeError, format!("cca(): {error}"), None)
    })?;
    let k = requested.min(rank).min(singular.len()).max(1);
    let left_embedding: Vec<Vec<f64>> = (0..first.len())
        .map(|cell| {
            (0..k)
                .map(|component| {
                    (0..rank)
                        .map(|basis| left_basis[basis][cell] * u_small.get(basis, component))
                        .sum()
                })
                .collect()
        })
        .collect();
    let right_embedding: Vec<Vec<f64>> = (0..second.len())
        .map(|cell| {
            (0..k)
                .map(|component| {
                    (0..rank)
                        .map(|basis| right_basis[basis][cell] * vt_small.get(component, basis))
                        .sum()
                })
                .collect()
        })
        .collect();
    Ok((
        left_embedding,
        right_embedding,
        singular.into_iter().take(k).collect(),
        used_gpu,
        sketch_width < source_width,
    ))
}

/// Accurate SVD for the small projected matrices used by iterative solvers.
/// Vectors are component-major: `left[c][row]` and `right[c][column]`.
fn projected_svd(input: &[Vec<f64>]) -> (Vec<Vec<f64>>, Vec<f64>, Vec<Vec<f64>>) {
    let rows = input.len();
    let columns = input.first().map(Vec::len).unwrap_or(0);
    let mut gram = vec![vec![0.0; columns]; columns];
    for i in 0..columns {
        for j in i..columns {
            let value: f64 = input.iter().map(|row| row[i] * row[j]).sum();
            gram[i][j] = value;
            gram[j][i] = value;
        }
    }
    let (right, eigenvalues) = jacobi_eigen_symmetric(&gram);
    let singular: Vec<f64> = eigenvalues
        .into_iter()
        .map(|value| value.max(0.0).sqrt())
        .collect();
    let left: Vec<Vec<f64>> = right
        .iter()
        .zip(&singular)
        .map(|(vector, &sigma)| {
            if sigma <= 1e-14 {
                return vec![0.0; rows];
            }
            input
                .iter()
                .map(|row| row.iter().zip(vector).map(|(a, b)| a * b).sum::<f64>() / sigma)
                .collect()
        })
        .collect();
    (left, singular, right)
}

/// Matrix-free augmented Lanczos bidiagonalization derived from Algorithm 3.1
/// of Baglama and Reichel (2005). This is an independent implementation from
/// the published equations: build a partial Golub-Kahan bidiagonalization,
/// retain the requested Ritz vectors, augment them with the final residual,
/// and expand the working space again. No implementation from `irlba` is used.
#[allow(dead_code)]
fn augmented_lanczos_cross_svd(
    first: &[Vec<f64>],
    second: &[Vec<f64>],
    requested: usize,
    work_extra: usize,
    tolerance: f64,
    max_iterations: usize,
    seed: u64,
    supplied_initial: Option<&[f64]>,
) -> Result<(Vec<Vec<f64>>, Vec<Vec<f64>>, Vec<f64>, bool, usize)> {
    fn dot_product(left: &[f64], right: &[f64]) -> f64 {
        left.iter().zip(right).map(|(a, b)| a * b).sum()
    }

    fn vector_norm(vector: &[f64]) -> f64 {
        dot_product(vector, vector).sqrt()
    }

    fn subtract_scaled(target: &mut [f64], source: &[f64], scale: f64) {
        for (value, basis) in target.iter_mut().zip(source) {
            *value -= scale * basis;
        }
    }

    fn normalise(vector: &mut [f64]) -> f64 {
        let norm = vector_norm(vector);
        if norm > 1e-14 && norm.is_finite() {
            for value in vector {
                *value /= norm;
            }
        }
        norm
    }

    fn combine_basis(basis: &[Vec<f64>], coefficients: &[f64]) -> Vec<f64> {
        let mut result = vec![0.0; basis.first().map(Vec::len).unwrap_or(0)];
        for (vector, coefficient) in basis.iter().zip(coefficients) {
            for (value, basis_value) in result.iter_mut().zip(vector) {
                *value += coefficient * basis_value;
            }
        }
        result
    }

    let left_rows = first.len();
    let right_rows = second.len();
    let available = left_rows
        .min(right_rows)
        .min(first.first().map(Vec::len).unwrap_or(0));
    let wanted = requested.min(available).max(1);
    let work = (wanted + work_extra.max(2)).min(available);
    if work <= wanted {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "cca(): Lanczos work space must exceed the requested rank".to_string(),
            None,
        ));
    }

    // A deterministic, non-zero start keeps BioLang runs reproducible. The
    // state transition is local to this clean-room solver; it does not attempt
    // to reproduce any package's random-number implementation.
    if let Some(initial) = supplied_initial {
        if initial.len() != right_rows || initial.iter().any(|value| !value.is_finite()) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!(
                    "cca(): supplied Lanczos start has {} values; expected {right_rows} finite values",
                    initial.len()
                ),
                None,
            ));
        }
    }
    let mut state = seed.max(1);
    let mut initial: Vec<f64> = supplied_initial.map(<[f64]>::to_vec).unwrap_or_else(|| {
        (0..right_rows)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                ((state >> 11) as f64 / ((1_u64 << 53) as f64)) * 2.0 - 1.0
            })
            .collect()
    });
    if normalise(&mut initial) <= 1e-14 {
        initial.fill(1.0 / (right_rows as f64).sqrt());
    }

    let mut right_basis = vec![initial];
    let mut left_basis: Vec<Vec<f64>> = Vec::with_capacity(work);
    let mut small = vec![0.0; work * work];
    let mut residual = Vec::new();
    let mut residual_norm = 0.0;
    let mut previous_singular: Option<Vec<f64>> = None;
    let mut used_gpu = false;

    for iteration in 1..=max_iterations.max(1) {
        while left_basis.len() < work {
            let column = left_basis.len();
            let (mut next_left_block, accelerated) =
                cross_apply_block(first, second, &[right_basis[column].clone()]);
            used_gpu |= accelerated;
            let mut next_left = next_left_block.pop().unwrap_or_default();

            // Subtract coefficients already supplied by the augmented restart,
            // then perform two full reorthogonalization passes. Recording the
            // small corrections keeps A*P = Q*B true to working precision.
            for (row, basis) in left_basis.iter().enumerate() {
                subtract_scaled(&mut next_left, basis, small[row * work + column]);
            }
            for _ in 0..2 {
                for (row, basis) in left_basis.iter().enumerate() {
                    let coefficient = dot_product(basis, &next_left);
                    small[row * work + column] += coefficient;
                    subtract_scaled(&mut next_left, basis, coefficient);
                }
            }
            let alpha = normalise(&mut next_left);
            if alpha <= 1e-14 || !alpha.is_finite() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "cca(): Lanczos bidiagonalization broke down".to_string(),
                    None,
                ));
            }
            small[column * work + column] = alpha;
            left_basis.push(next_left);

            let (mut next_right_block, accelerated) =
                cross_apply_block(second, first, &[left_basis[column].clone()]);
            used_gpu |= accelerated;
            residual = next_right_block.pop().unwrap_or_default();
            for (basis_column, basis) in right_basis.iter().enumerate() {
                subtract_scaled(&mut residual, basis, small[column * work + basis_column]);
            }
            for _ in 0..2 {
                for (basis_column, basis) in right_basis.iter().enumerate() {
                    let coefficient = dot_product(basis, &residual);
                    small[column * work + basis_column] += coefficient;
                    subtract_scaled(&mut residual, basis, coefficient);
                }
            }
            residual_norm = vector_norm(&residual);
            if column + 1 < work {
                if residual_norm <= 1e-14 || !residual_norm.is_finite() {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "cca(): Lanczos residual vanished before the working space was complete"
                            .to_string(),
                        None,
                    ));
                }
                let mut next_right = residual.clone();
                for value in &mut next_right {
                    *value /= residual_norm;
                }
                small[column * work + column + 1] = residual_norm;
                right_basis.push(next_right);
            }
        }

        let projected: Vec<Vec<f64>> = small.chunks(work).map(<[f64]>::to_vec).collect();
        let (u_small, singular, v_small) = projected_svd(&projected);
        let current: Vec<f64> = singular.iter().take(wanted).copied().collect();
        let largest = current
            .first()
            .copied()
            .unwrap_or(1.0)
            .max(f64::MIN_POSITIVE);
        let max_residual = (0..wanted)
            .map(|component| residual_norm * u_small[component][work - 1].abs())
            .fold(0.0_f64, f64::max);
        let singular_change = previous_singular
            .as_ref()
            .map(|previous| {
                current
                    .iter()
                    .zip(previous)
                    .map(|(now, before)| (now - before).abs() / now.abs().max(f64::MIN_POSITIVE))
                    .fold(0.0_f64, f64::max)
            })
            .unwrap_or(f64::INFINITY);
        let invariant_subspace = residual_norm <= f64::EPSILON.sqrt() * largest;
        let converged = max_residual <= tolerance * largest
            && (singular_change <= tolerance || invariant_subspace);

        if converged || iteration == max_iterations.max(1) {
            let left_vectors: Vec<Vec<f64>> = (0..wanted)
                .map(|component| {
                    combine_basis(
                        &left_basis,
                        &(0..work)
                            .map(|row| u_small[component][row])
                            .collect::<Vec<_>>(),
                    )
                })
                .collect();
            let right_vectors: Vec<Vec<f64>> = (0..wanted)
                .map(|component| {
                    combine_basis(
                        &right_basis,
                        &(0..work)
                            .map(|row| v_small[component][row])
                            .collect::<Vec<_>>(),
                    )
                })
                .collect();
            let left_embedding: Vec<Vec<f64>> = (0..left_rows)
                .map(|row| left_vectors.iter().map(|vector| vector[row]).collect())
                .collect();
            let right_embedding: Vec<Vec<f64>> = (0..right_rows)
                .map(|row| right_vectors.iter().map(|vector| vector[row]).collect())
                .collect();
            return Ok((
                left_embedding,
                right_embedding,
                current,
                used_gpu,
                iteration,
            ));
        }

        // Ritz augmentation (paper section 3.1): keep the wanted Ritz pairs,
        // append the normalized final residual as the next right vector, and
        // preserve its coupling rho_j = beta_m * U[m,j].
        let retained_left: Vec<Vec<f64>> = (0..wanted)
            .map(|component| {
                combine_basis(
                    &left_basis,
                    &(0..work)
                        .map(|row| u_small[component][row])
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        let mut retained_right: Vec<Vec<f64>> = (0..wanted)
            .map(|component| {
                combine_basis(
                    &right_basis,
                    &(0..work)
                        .map(|row| v_small[component][row])
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        if residual_norm <= 1e-14 || !residual_norm.is_finite() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "cca(): Lanczos restart residual vanished before convergence".to_string(),
                None,
            ));
        }
        for value in &mut residual {
            *value /= residual_norm;
        }
        retained_right.push(residual.clone());
        left_basis = retained_left;
        right_basis = retained_right;
        small.fill(0.0);
        for component in 0..wanted {
            small[component * work + component] = current[component];
            small[component * work + wanted] = residual_norm * u_small[component][work - 1];
        }
        previous_singular = Some(current);
    }
    unreachable!()
}

/// Matrix-free augmented restarted Lanczos bidiagonalization following the
/// recurrence and three-vector augmentation described by Baglama and Reichel.
/// Unlike the older experimental solver above, this keeps the residual Ritz
/// couplings unchanged across a restart and performs the single full
/// reorthogonalization prescribed by the recurrence.  Those details matter on
/// the nearly degenerate tail of real single-cell CCA spectra.
fn restarted_lanczos_svd_with<Forward, Reverse>(
    left_rows: usize,
    right_rows: usize,
    rank_limit: usize,
    requested: usize,
    work_extra: usize,
    tolerance: f64,
    max_iterations: usize,
    seed: u64,
    supplied_initial: Option<&[f64]>,
    mut forward: Forward,
    mut reverse: Reverse,
) -> Result<(Vec<Vec<f64>>, Vec<Vec<f64>>, Vec<f64>, bool, usize, bool)>
where
    Forward: FnMut(&[Vec<f64>]) -> (Vec<Vec<f64>>, bool),
    Reverse: FnMut(&[Vec<f64>]) -> (Vec<Vec<f64>>, bool),
{
    fn dot(left: &[f64], right: &[f64]) -> f64 {
        left.iter().zip(right).map(|(a, b)| a * b).sum()
    }
    fn norm(vector: &[f64]) -> f64 {
        dot(vector, vector).sqrt()
    }
    fn normalise(vector: &mut [f64]) -> f64 {
        let length = norm(vector);
        if length > 1e-14 && length.is_finite() {
            for value in vector {
                *value /= length;
            }
        }
        length
    }
    fn orthogonalise_once(vector: &mut [f64], basis: &[Vec<f64>]) {
        if basis.is_empty() {
            return;
        }
        let coefficients: Vec<f64> = basis.iter().map(|column| dot(column, vector)).collect();
        for (column, coefficient) in basis.iter().zip(coefficients) {
            for (value, basis_value) in vector.iter_mut().zip(column) {
                *value -= coefficient * basis_value;
            }
        }
    }
    fn combine(basis: &[Vec<f64>], coefficients: &[f64]) -> Vec<f64> {
        let mut output = vec![0.0; basis.first().map(Vec::len).unwrap_or(0)];
        for (column, coefficient) in basis.iter().zip(coefficients) {
            for (value, basis_value) in output.iter_mut().zip(column) {
                *value += coefficient * basis_value;
            }
        }
        output
    }

    let available = left_rows.min(right_rows).min(rank_limit);
    let wanted = requested.min(available).max(1);
    let work = (wanted + work_extra.max(2)).min(available);
    if work <= wanted {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "restarted Lanczos work space must exceed the requested rank".to_string(),
            None,
        ));
    }
    if let Some(initial) = supplied_initial {
        if initial.len() != right_rows || initial.iter().any(|value| !value.is_finite()) {
            return Err(BioLangError::type_error(
                format!(
                    "supplied Lanczos start has {} values; expected {right_rows} finite values",
                    initial.len()
                ),
                None,
            ));
        }
    }
    let mut state = seed.max(1);
    let mut start: Vec<f64> = supplied_initial.map(<[f64]>::to_vec).unwrap_or_else(|| {
        (0..right_rows)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                ((state >> 11) as f64 / ((1_u64 << 53) as f64)) * 2.0 - 1.0
            })
            .collect()
    });
    if normalise(&mut start) <= 1e-14 {
        start.fill(1.0 / (right_rows as f64).sqrt());
    }

    let mut right_basis = vec![start];
    let mut left_basis: Vec<Vec<f64>> = Vec::with_capacity(work);
    let mut bidiagonal = vec![0.0; work * work];
    let mut previous_singular: Option<Vec<f64>> = None;
    let mut retained = wanted;
    let mut spectral_max = 1.0_f64;
    let mut used_gpu = false;

    for iteration in 1..=max_iterations.max(1) {
        let first_new = if iteration == 1 { 0 } else { retained };
        let (mut left_block, accelerated) = forward(&[right_basis[first_new].clone()]);
        used_gpu |= accelerated;
        let mut next_left = left_block.pop().unwrap_or_default();
        if iteration != 1 {
            orthogonalise_once(&mut next_left, &left_basis);
        }
        let mut diagonal = normalise(&mut next_left);
        if diagonal <= 1e-14 || !diagonal.is_finite() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "restarted Lanczos left basis reached an invariant subspace".to_string(),
                None,
            ));
        }
        left_basis.push(next_left);

        let mut residual = Vec::new();
        let mut residual_norm = 0.0;
        for column in first_new..work {
            let (mut right_block, accelerated) = reverse(&[left_basis[column].clone()]);
            used_gpu |= accelerated;
            residual = right_block.pop().unwrap_or_default();
            for (value, basis_value) in residual.iter_mut().zip(&right_basis[column]) {
                *value -= diagonal * basis_value;
            }
            orthogonalise_once(&mut residual, &right_basis[..=column]);
            residual_norm = norm(&residual);
            bidiagonal[column * work + column] = diagonal;

            if column + 1 < work {
                if residual_norm <= 1e-14 || !residual_norm.is_finite() {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "restarted Lanczos right basis reached an invariant subspace".to_string(),
                        None,
                    ));
                }
                let mut next_right = residual.clone();
                for value in &mut next_right {
                    *value /= residual_norm;
                }
                bidiagonal[column * work + column + 1] = residual_norm;
                right_basis.push(next_right);

                let (mut following_left, accelerated) = forward(&[right_basis[column + 1].clone()]);
                used_gpu |= accelerated;
                let mut following_left = following_left.pop().unwrap_or_default();
                for (value, previous) in following_left.iter_mut().zip(&left_basis[column]) {
                    *value -= residual_norm * previous;
                }
                orthogonalise_once(&mut following_left, &left_basis);
                diagonal = normalise(&mut following_left);
                if diagonal <= 1e-14 || !diagonal.is_finite() {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "restarted Lanczos left basis reached an invariant subspace".to_string(),
                        None,
                    ));
                }
                left_basis.push(following_left);
            }
        }

        let projected: Vec<Vec<f64>> = bidiagonal.chunks(work).map(<[f64]>::to_vec).collect();
        let (left_small, singular, right_small) = projected_svd(&projected);
        let current: Vec<f64> = singular.iter().take(wanted).copied().collect();
        spectral_max = spectral_max.max(current[0]);
        let residuals: Vec<f64> = (0..wanted)
            .map(|component| residual_norm * left_small[component][work - 1].abs())
            .collect();
        let stable: Vec<bool> = match &previous_singular {
            Some(previous) => current
                .iter()
                .zip(previous)
                .map(|(now, before)| {
                    (now - before).abs() / now.abs().max(f64::MIN_POSITIVE) < tolerance
                })
                .collect(),
            None => vec![false; wanted],
        };
        let converged_count = residuals
            .iter()
            .zip(&stable)
            .filter(|(residual, stable)| **residual < tolerance * spectral_max && **stable)
            .count();
        let converged = converged_count >= wanted;
        if std::env::var_os("BIOLANG_CCA_TRACE").is_some() {
            let worst_residual = residuals.iter().copied().fold(0.0_f64, f64::max);
            let stable_count = stable.iter().filter(|value| **value).count();
            eprintln!(
                "  restarted lanczos: iteration={iteration} retained={retained} residual_converged={} stable={stable_count}/{wanted} both={converged_count}/{wanted} worst_residual={worst_residual:.3e}",
                residuals
                    .iter()
                    .filter(|value| **value < tolerance * spectral_max)
                    .count()
            );
        }

        if converged || iteration == max_iterations.max(1) {
            let left_vectors: Vec<Vec<f64>> = (0..wanted)
                .map(|component| combine(&left_basis, &left_small[component]))
                .collect();
            let right_vectors: Vec<Vec<f64>> = (0..wanted)
                .map(|component| combine(&right_basis, &right_small[component]))
                .collect();
            let left_embedding = (0..left_rows)
                .map(|row| left_vectors.iter().map(|vector| vector[row]).collect())
                .collect();
            let right_embedding = (0..right_rows)
                .map(|row| right_vectors.iter().map(|vector| vector[row]).collect())
                .collect();
            return Ok((
                left_embedding,
                right_embedding,
                current,
                used_gpu,
                iteration,
                converged,
            ));
        }

        retained = retained
            .max(
                wanted
                    + residuals
                        .iter()
                        .take(wanted)
                        .filter(|value| **value < tolerance * spectral_max)
                        .count()
                        .min(3),
            )
            .min(work - 1);
        let new_left: Vec<Vec<f64>> = (0..retained)
            .map(|component| combine(&left_basis, &left_small[component]))
            .collect();
        let mut new_right: Vec<Vec<f64>> = (0..retained)
            .map(|component| combine(&right_basis, &right_small[component]))
            .collect();
        if residual_norm <= 1e-14 || !residual_norm.is_finite() {
            // A vanished terminal residual means the Krylov space is invariant:
            // the projected decomposition is already an exact decomposition of
            // the represented matrix, even though the change-based stopping
            // test needs a second iteration to call it stable.
            let left_vectors: Vec<Vec<f64>> = (0..wanted)
                .map(|component| combine(&left_basis, &left_small[component]))
                .collect();
            let right_vectors: Vec<Vec<f64>> = (0..wanted)
                .map(|component| combine(&right_basis, &right_small[component]))
                .collect();
            let left_embedding = (0..left_rows)
                .map(|row| left_vectors.iter().map(|vector| vector[row]).collect())
                .collect();
            let right_embedding = (0..right_rows)
                .map(|row| right_vectors.iter().map(|vector| vector[row]).collect())
                .collect();
            return Ok((
                left_embedding,
                right_embedding,
                current,
                used_gpu,
                iteration,
                true,
            ));
        }
        for value in &mut residual {
            *value /= residual_norm;
        }
        new_right.push(residual);
        left_basis = new_left;
        right_basis = new_right;
        bidiagonal.fill(0.0);
        for component in 0..retained {
            bidiagonal[component * work + component] = singular[component];
            bidiagonal[component * work + retained] =
                residual_norm * left_small[component][work - 1];
        }
        previous_singular = Some(current);
    }
    unreachable!()
}

fn restarted_lanczos_cross_svd(
    first: &[Vec<f64>],
    second: &[Vec<f64>],
    requested: usize,
    work_extra: usize,
    tolerance: f64,
    max_iterations: usize,
    seed: u64,
    supplied_initial: Option<&[f64]>,
) -> Result<(Vec<Vec<f64>>, Vec<Vec<f64>>, Vec<f64>, bool, usize, bool)> {
    restarted_lanczos_svd_with(
        first.len(),
        second.len(),
        first.first().map(Vec::len).unwrap_or(0),
        requested,
        work_extra,
        tolerance,
        max_iterations,
        seed,
        supplied_initial,
        |basis| cross_apply_block(first, second, basis),
        |basis| cross_apply_block(second, first, basis),
    )
}

type CcaParts = (
    Vec<Vec<f64>>,
    Vec<Vec<f64>>,
    Vec<Vec<f64>>,
    Vec<Vec<f64>>,
    Vec<f64>,
    String,
);

fn cca_dense(
    first: &[Vec<f64>],
    second: &[Vec<f64>],
    requested: usize,
    sweeps: usize,
    oversample: usize,
    solver: &str,
    lanczos_work_extra: usize,
    lanczos_tolerance: f64,
    lanczos_max_iterations: usize,
    lanczos_seed: u64,
    lanczos_initial: Option<&[f64]>,
) -> Result<CcaParts> {
    let genes = first.first().map(|row| row.len()).unwrap_or(0);
    let genes_second = second.first().map(|row| row.len()).unwrap_or(0);
    if genes == 0 || genes_second == 0 || first.is_empty() || second.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "cca() needs two non-empty cells x genes matrices".to_string(),
            None,
        ));
    }
    if genes != genes_second {
        // Silently intersecting would be worse: the caller would get a result
        // computed over whichever genes happened to line up by position.
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "cca() requires the same genes in both matrices, in the same order: \
                 {genes} columns and {genes_second}"
            ),
            None,
        ));
    }

    if solver == "lanczos" {
        let (left_raw, right_raw, singular, used_gpu, iterations, converged) =
            restarted_lanczos_cross_svd(
                first,
                second,
                requested,
                lanczos_work_extra,
                lanczos_tolerance,
                lanczos_max_iterations,
                lanczos_seed,
                lanczos_initial,
            )?;
        let left = l2_normalise_rows(&left_raw);
        let right = l2_normalise_rows(&right_raw);
        return Ok((
            left,
            right,
            left_raw,
            right_raw,
            singular,
            format!(
                "matrix_free_augmented_lanczos_{}_iter_{iterations}_{}",
                if used_gpu { "gpu" } else { "cpu" },
                if converged {
                    "converged"
                } else {
                    "iteration_limit"
                }
            ),
        ));
    }

    if first.len().saturating_mul(second.len()) > 4_000_000 {
        let (left_raw, right_raw, singular, used_gpu, used_sketch) =
            scalable_cross_svd(first, second, requested, sweeps, oversample)?;
        let left = l2_normalise_rows(&left_raw);
        let right = l2_normalise_rows(&right_raw);
        return Ok((
            left,
            right,
            left_raw,
            right_raw,
            singular,
            if used_sketch && used_gpu {
                "countsketch_subspace_gpu".to_string()
            } else if used_sketch {
                "countsketch_subspace_cpu".to_string()
            } else if used_gpu {
                "matrix_free_subspace_gpu".to_string()
            } else {
                "matrix_free_subspace_cpu".to_string()
            },
        ));
    }

    // cells1 x cells2, one entry per pair of cells: how alike they are across
    // the shared genes.
    let mut cross = vec![0.0_f64; first.len() * second.len()];
    for (i, row) in first.iter().enumerate() {
        for (j, other) in second.iter().enumerate() {
            cross[i * second.len() + j] = row
                .iter()
                .zip(other.iter())
                .map(|(a, b)| a * b)
                .sum::<f64>();
        }
    }
    let cross = bl_core::matrix::Matrix::new(cross, first.len(), second.len())
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, format!("cca(): {e}"), None))?;

    let (u, d, vt) = cross
        .svd()
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, format!("cca(): {e}"), None))?;

    let k = requested.min(d.len()).min(u.ncol).min(vt.nrow).max(1);

    // u is cells1 x r; vt is r x cells2, so the second dataset's coordinates are
    // its rows read down the columns.
    let left: Vec<Vec<f64>> = (0..u.nrow)
        .map(|i| (0..k).map(|c| u.get(i, c)).collect())
        .collect();
    let right: Vec<Vec<f64>> = (0..vt.ncol)
        .map(|j| (0..k).map(|c| vt.get(c, j)).collect())
        .collect();

    let singular = d.iter().take(k).copied().collect();
    Ok((
        l2_normalise_rows(&left),
        l2_normalise_rows(&right),
        left,
        right,
        singular,
        "exact_cross_svd_cpu".to_string(),
    ))
}

fn builtin_cca(args: Vec<Value>) -> Result<Value> {
    let opts: HashMap<String, Value> = match args.get(2) {
        Some(Value::Record(map)) => map.as_ref().clone(),
        _ => HashMap::new(),
    };
    let requested = opts
        .get("k")
        .and_then(|v| v.as_float())
        .map(|v| v as usize)
        .unwrap_or(20);
    let sweeps = record_number(&opts, "sweeps", 12.0).clamp(1.0, 100.0) as usize;
    let oversample = record_number(&opts, "oversample", 32.0).clamp(0.0, 256.0) as usize;
    let solver = match opts.get("solver") {
        Some(Value::Str(value)) if value.eq_ignore_ascii_case("lanczos") => "lanczos",
        _ => "subspace",
    };
    let lanczos_work_extra = record_number(&opts, "work_extra", 7.0).clamp(2.0, 256.0) as usize;
    let lanczos_tolerance = record_number(&opts, "tolerance", 1e-5).clamp(1e-12, 1.0);
    let lanczos_max_iterations =
        record_number(&opts, "max_iterations", 1000.0).clamp(1.0, 10_000.0) as usize;
    let lanczos_seed = record_number(&opts, "seed", 42.0).max(1.0) as u64;
    let lanczos_initial: Option<Vec<f64>> = match opts.get("initial") {
        Some(Value::List(values)) => Some(
            values
                .iter()
                .map(|value| {
                    value.as_float().ok_or_else(|| {
                        BioLangError::type_error("cca() initial must be a List<Number>", None)
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        ),
        Some(_) => {
            return Err(BioLangError::type_error(
                "cca() initial must be a List<Number>",
                None,
            ))
        }
        None => None,
    };
    let first = require_matrix(&args[0], "cca")?;
    let second = require_matrix(&args[1], "cca")?;
    let (left, right, left_raw, right_raw, singular, method) = cca_dense(
        &first,
        &second,
        requested,
        sweeps,
        oversample,
        solver,
        lanczos_work_extra,
        lanczos_tolerance,
        lanczos_max_iterations,
        lanczos_seed,
        lanczos_initial.as_deref(),
    )?;
    Ok(Value::Record(
        HashMap::from([
            ("u".to_string(), matrix_to_value(left)),
            ("v".to_string(), matrix_to_value(right)),
            ("u_raw".to_string(), matrix_to_value(left_raw)),
            ("v_raw".to_string(), matrix_to_value(right_raw)),
            (
                "d".to_string(),
                Value::List(
                    singular
                        .into_iter()
                        .map(Value::Float)
                        .collect::<Vec<_>>()
                        .into(),
                ),
            ),
            ("method".to_string(), Value::Str(method)),
        ])
        .into(),
    ))
}

// â”€â”€ harmony_integrate(embedding, batches, opts?) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Remove batch effects from an embedding, the way Harmony does.
///
/// The problem this solves is the one that makes multi-sample single-cell work
/// hard: cluster two donors together without correction and you get two of
/// every cell type, one per donor, and a UMAP that shows the donor rather than
/// the biology. `sc_integrate` subtracts a per-batch mean, which only helps
/// when the batch effect is the same everywhere - and it is not, because a
/// batch shifts monocytes and T cells by different amounts.
///
/// Harmony (Korsunsky et al. 2019) alternates two steps on the PCA embedding:
///
/// 1. Soft-assign cells to clusters, penalising clusters that are dominated by
///    one batch. That diversity penalty is the whole idea - it pushes the
///    clustering towards groups that *should* contain every batch.
/// 2. Within each cluster, regress the batch out and subtract it, weighted by
///    how strongly each cell belongs to that cluster.
///
/// Because the correction is per cluster, each cell type gets its own batch
/// shift, which is what a single global mean cannot express.
///
/// The danger is the opposite failure: correcting so hard that genuinely
/// different cell types are merged into one. `theta` controls that - higher
/// mixes more aggressively - and the tests hold both ends, checking that
/// batches mix *and* that distinct populations stay apart.
fn record_matrix(record: &Value, field: &str, func: &str) -> Result<Vec<Vec<f64>>> {
    match record {
        Value::Record(fields) => fields
            .get(field)
            .ok_or_else(|| BioLangError::type_error(format!("{func}() missing '{field}'"), None))
            .and_then(|value| require_matrix(value, func)),
        other => Err(BioLangError::type_error(
            format!("{func}() expected Record, got {}", other.type_of()),
            None,
        )),
    }
}

fn record_number(record: &HashMap<String, Value>, field: &str, fallback: f64) -> f64 {
    record
        .get(field)
        .and_then(Value::as_float)
        .unwrap_or(fallback)
}

fn squared_euclidean(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| (a - b) * (a - b)).sum()
}

enum CrossRpTree {
    Leaf(Vec<usize>),
    Split {
        first: usize,
        second: usize,
        threshold: f64,
        fallback_dimension: usize,
        usable: bool,
        left: Box<CrossRpTree>,
        right: Box<CrossRpTree>,
    },
}

fn cross_rp_projection(
    reference: &[Vec<f64>],
    row: &[f64],
    first: usize,
    second: usize,
    fallback_dimension: usize,
    usable: bool,
) -> f64 {
    if usable {
        row.iter()
            .zip(reference[first].iter().zip(&reference[second]))
            .map(|(value, (a, b))| value * (a - b))
            .sum()
    } else {
        row.get(fallback_dimension).copied().unwrap_or(0.0)
    }
}

fn build_cross_rp_tree(
    reference: &[Vec<f64>],
    indices: Vec<usize>,
    leaf_size: usize,
    state: &mut u64,
) -> CrossRpTree {
    if indices.len() <= leaf_size {
        return CrossRpTree::Leaf(indices);
    }
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let first_position = (*state as usize) % indices.len();
    let first = indices[first_position];
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let mut second_position = (*state as usize) % indices.len();
    if second_position == first_position {
        second_position = (second_position + 1) % indices.len();
    }
    let second = indices[second_position];
    let dimensions = reference.first().map(Vec::len).unwrap_or(0);
    let fallback_dimension = if dimensions == 0 {
        0
    } else {
        ((*state >> 32) as usize) % dimensions
    };
    let usable = reference[first]
        .iter()
        .zip(&reference[second])
        .any(|(a, b)| (a - b).abs() > 1e-12);
    let mut projected: Vec<(usize, f64)> = indices
        .into_iter()
        .map(|index| {
            (
                index,
                cross_rp_projection(
                    reference,
                    &reference[index],
                    first,
                    second,
                    fallback_dimension,
                    usable,
                ),
            )
        })
        .collect();
    projected.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let right_projected = projected.split_off(projected.len() / 2);
    let threshold = (projected.last().map(|value| value.1).unwrap_or(0.0)
        + right_projected.first().map(|value| value.1).unwrap_or(0.0))
        * 0.5;
    let left_indices = projected.into_iter().map(|value| value.0).collect();
    let right_indices = right_projected.into_iter().map(|value| value.0).collect();
    CrossRpTree::Split {
        first,
        second,
        threshold,
        fallback_dimension,
        usable,
        left: Box::new(build_cross_rp_tree(
            reference,
            left_indices,
            leaf_size,
            state,
        )),
        right: Box::new(build_cross_rp_tree(
            reference,
            right_indices,
            leaf_size,
            state,
        )),
    }
}

fn query_cross_rp_tree<'a>(
    tree: &'a CrossRpTree,
    reference: &[Vec<f64>],
    query: &[f64],
) -> &'a [usize] {
    match tree {
        CrossRpTree::Leaf(indices) => indices,
        CrossRpTree::Split {
            first,
            second,
            threshold,
            fallback_dimension,
            usable,
            left,
            right,
        } => {
            let projection = cross_rp_projection(
                reference,
                query,
                *first,
                *second,
                *fallback_dimension,
                *usable,
            );
            query_cross_rp_tree(
                if projection <= *threshold {
                    left
                } else {
                    right
                },
                reference,
                query,
            )
        }
    }
}

fn approximate_cross_neighbour_rows(
    query: &[Vec<f64>],
    reference: &[Vec<f64>],
    wanted: usize,
) -> Vec<Vec<(usize, f64)>> {
    // Seurat's Annoy backend defaults to 50 trees. Keep the same independent
    // tree count here; leaves are bounded, and candidates are materialised for
    // one query at a time rather than for the whole merged dataset.
    let leaf_size = if wanted <= 64 { 64 } else { 256 };
    let tree_count = 50;
    let trees: Vec<CrossRpTree> = (0..tree_count)
        .map(|tree| {
            let mut state =
                0x9e3779b97f4a7c15_u64 ^ (tree as u64 + 1).wrapping_mul(0x517cc1b727220a95);
            build_cross_rp_tree(
                reference,
                (0..reference.len()).collect(),
                leaf_size,
                &mut state,
            )
        })
        .collect();
    query
        .iter()
        .map(|row| {
            let mut candidates = Vec::with_capacity(leaf_size * tree_count);
            for tree in &trees {
                candidates.extend_from_slice(query_cross_rp_tree(tree, reference, row));
            }
            candidates.sort_unstable();
            candidates.dedup();
            let mut distances: Vec<(usize, f64)> = candidates
                .into_iter()
                .map(|index| (index, squared_euclidean(row, &reference[index]).sqrt()))
                .collect();
            distances.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            distances.truncate(wanted);
            distances
        })
        .collect()
}

/// Match Seurat's `Standardize`: center and sample-standardize every cell
/// across integration features before taking the cross-product CCA.
fn standardize_cells(rows: &[Vec<f64>]) -> Vec<Vec<f64>> {
    rows.iter()
        .map(|row| {
            if row.len() < 2 {
                return vec![0.0; row.len()];
            }
            let mean = row.iter().sum::<f64>() / row.len() as f64;
            let deviation = (row
                .iter()
                .map(|value| (value - mean) * (value - mean))
                .sum::<f64>()
                / row.len().saturating_sub(1) as f64)
                .sqrt();
            if !deviation.is_finite() || deviation <= 1e-15 {
                vec![0.0; row.len()]
            } else {
                row.iter().map(|value| (value - mean) / deviation).collect()
            }
        })
        .collect()
}

/// Cross-dataset kNN. Small fixtures use an exact search. Large inputs reuse
/// the deterministic/GPU neighbour backend with an expanded candidate set,
/// analogous to Seurat's approximate Annoy search but without linking Annoy.
fn cross_neighbour_rows(
    query: &[Vec<f64>],
    reference: &[Vec<f64>],
    k: usize,
) -> Vec<Vec<(usize, f64)>> {
    let wanted = k.min(reference.len());
    if wanted == 0 {
        return vec![Vec::new(); query.len()];
    }
    if query.len().saturating_mul(reference.len()) <= 4_000_000 {
        return query
            .iter()
            .map(|row| {
                let mut distances: Vec<(usize, f64)> = reference
                    .iter()
                    .enumerate()
                    .map(|(index, other)| (index, squared_euclidean(row, other).sqrt()))
                    .collect();
                distances.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
                distances.truncate(wanted);
                distances
            })
            .collect();
    }

    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(neighbours) = bl_seurat_compat::annoy_euclidean(reference, query, wanted, 50) {
        return neighbours;
    }

    let mut combined = reference.to_vec();
    let query_offset = combined.len();
    combined.extend(query.iter().cloned());
    // Small anchor/scoring searches can use the GPU's bounded top-k kernel and
    // cheaply over-fetch. For Seurat's k.filter=200, an 8x mixed search asked
    // the CPU fallback to retain 1,600 neighbours even though only 200
    // cross-dataset hits survive. Three times k is a conservative allowance
    // for the approximately half same-dataset candidates in a balanced merge
    // without materialising an oversized result.
    let expansion = if wanted <= 64 { 8 } else { 3 };
    let search_k = wanted
        .saturating_mul(expansion)
        .max(64)
        .min(combined.len().saturating_sub(1));
    if let Ok(Some(neighbours)) = crate::gpu::nearest_rows(&combined, search_k, "euclidean") {
        return neighbours
            .into_iter()
            .skip(query_offset)
            .map(|row| {
                row.into_iter()
                    .filter(|(index, _)| *index < query_offset)
                    .take(wanted)
                    .collect()
            })
            .collect();
    }
    approximate_cross_neighbour_rows(query, reference, wanted)
}

fn projected_feature_loadings(
    left: &[Vec<f64>],
    right: &[Vec<f64>],
    left_embedding: &[Vec<f64>],
    right_embedding: &[Vec<f64>],
) -> Vec<Vec<f64>> {
    let features = left.first().map(Vec::len).unwrap_or(0);
    let dimensions = left_embedding.first().map(Vec::len).unwrap_or(0);
    let mut loadings = vec![vec![0.0; dimensions]; features];
    for (matrix, embedding) in [(left, left_embedding), (right, right_embedding)] {
        for (cell, row) in matrix.iter().enumerate() {
            for (feature, value) in row.iter().copied().enumerate() {
                for dimension in 0..dimensions {
                    loadings[feature][dimension] += value * embedding[cell][dimension];
                }
            }
        }
    }
    loadings
}

fn balanced_top_features(loadings: &[Vec<f64>], dimension: usize, number: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..loadings.len()).collect();
    order.sort_by(|&a, &b| {
        loadings[b][dimension]
            .total_cmp(&loadings[a][dimension])
            .then(a.cmp(&b))
    });
    if number <= 1 {
        return order.into_iter().take(number).collect();
    }
    let half = ((number as f64) / 2.0).round() as usize;
    let mut selected: Vec<usize> = order.iter().copied().take(half).collect();
    selected.extend(order.iter().rev().copied().take(half));
    selected.sort_unstable();
    selected.dedup();
    selected
}

/// Seurat's TopDimFeatures grows a balanced positive/negative set per CCA
/// dimension until the union approaches `max.features`.
fn top_dim_features(loadings: &[Vec<f64>], requested_max: usize) -> Vec<usize> {
    let dimensions = loadings.first().map(Vec::len).unwrap_or(0);
    if dimensions == 0 || loadings.is_empty() {
        return Vec::new();
    }
    let limit = requested_max.max(dimensions.saturating_mul(2));
    let mut per_dimension = 1;
    for number in 1..=100.min(loadings.len()) {
        let mut union = HashSet::new();
        for dimension in 0..dimensions {
            union.extend(balanced_top_features(loadings, dimension, number));
        }
        if union.len() < limit {
            per_dimension = number;
        } else {
            break;
        }
    }
    let mut selected = Vec::new();
    let mut seen = HashSet::new();
    for dimension in 0..dimensions {
        for feature in balanced_top_features(loadings, dimension, per_dimension) {
            if seen.insert(feature) {
                selected.push(feature);
            }
        }
    }
    selected
}

fn pca_projection_parts(
    matrix: &[Vec<f64>],
    dimensions: usize,
    func: &str,
) -> Result<(Vec<Vec<f64>>, Vec<Vec<f64>>, Vec<f64>)> {
    let matrix = SingleCellMatrix::BorrowedDense(matrix);
    pca_projection_parts_from_matrix(&matrix, dimensions, func)
}

fn pca_projection_parts_from_matrix(
    matrix: &SingleCellMatrix<'_>,
    dimensions: usize,
    func: &str,
) -> Result<(Vec<Vec<f64>>, Vec<Vec<f64>>, Vec<f64>)> {
    let pca = builtin_sc_pca_matrix(matrix, dimensions, true)?;
    let scores = record_matrix(&pca, "scores", func)?;
    let loadings = record_matrix(&pca, "loadings", func)?;
    let means = match &pca {
        Value::Record(fields) => match fields.get("mean") {
            Some(Value::List(values)) => values
                .iter()
                .map(|value| {
                    value.as_float().ok_or_else(|| {
                        BioLangError::type_error(format!("{func}() invalid PCA mean"), None)
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            _ => {
                return Err(BioLangError::type_error(
                    format!("{func}() missing PCA mean"),
                    None,
                ))
            }
        },
        _ => unreachable!(),
    };
    Ok((scores, loadings, means))
}

fn project_pca(matrix: &[Vec<f64>], loadings: &[Vec<f64>], means: &[f64]) -> Vec<Vec<f64>> {
    let dimensions = loadings.first().map(Vec::len).unwrap_or(0);
    matrix
        .iter()
        .map(|row| {
            (0..dimensions)
                .map(|component| {
                    row.iter()
                        .zip(means)
                        .zip(loadings)
                        .map(|((value, mean), gene_loadings)| {
                            (value - mean) * gene_loadings[component]
                        })
                        .sum()
                })
                .collect()
        })
        .collect()
}

/// Find mutual cross-dataset neighbours in embeddings supplied by the caller.
///
/// This deliberately stops before feature filtering and anchor scoring.  It is
/// useful both for custom integration pipelines and for testing the neighbour
/// backend independently of CCA/PCA numerical differences.
fn builtin_sc_anchor_candidates(args: Vec<Value>) -> Result<Value> {
    let left = require_dense_matrix(&args[0], "sc_anchor_candidates")?;
    let right = require_dense_matrix(&args[1], "sc_anchor_candidates")?;
    let opts = match args.get(2) {
        Some(Value::Record(fields)) => fields.as_ref().clone(),
        _ => HashMap::new(),
    };
    if left.is_empty() || right.is_empty() {
        return Err(BioLangError::type_error(
            "sc_anchor_candidates() requires two non-empty embedding matrices",
            None,
        ));
    }
    let dimensions = left[0].len();
    if dimensions == 0
        || right[0].len() != dimensions
        || left.iter().any(|row| row.len() != dimensions)
        || right.iter().any(|row| row.len() != dimensions)
    {
        return Err(BioLangError::type_error(
            "sc_anchor_candidates() requires rectangular matrices with identical columns",
            None,
        ));
    }
    let k_anchor = record_number(&opts, "k_anchor", 5.0).max(1.0) as usize;
    let k_neighbours = record_number(&opts, "k_neighbours", 30.0).max(k_anchor as f64) as usize;
    let left_to_right = cross_neighbour_rows(&left, &right, k_neighbours);
    let right_to_left = cross_neighbour_rows(&right, &left, k_neighbours);
    let mut candidates = Vec::new();
    for (left_index, neighbours) in left_to_right.iter().enumerate() {
        for &(right_index, _) in neighbours.iter().take(k_anchor) {
            if right_to_left[right_index]
                .iter()
                .take(k_anchor)
                .any(|&(candidate, _)| candidate == left_index)
            {
                candidates.push(Value::Record(
                    HashMap::from([
                        ("left".to_string(), Value::Int(left_index as i64)),
                        ("right".to_string(), Value::Int(right_index as i64)),
                    ])
                    .into(),
                ));
            }
        }
    }
    Ok(Value::Record(
        HashMap::from([
            (
                "candidate_anchors".to_string(),
                Value::List(candidates.into()),
            ),
            ("left_count".to_string(), Value::Int(left.len() as i64)),
            ("right_count".to_string(), Value::Int(right.len() as i64)),
            ("dims".to_string(), Value::Int(dimensions as i64)),
            ("k_anchor".to_string(), Value::Int(k_anchor as i64)),
            ("k_neighbours".to_string(), Value::Int(k_neighbours as i64)),
            (
                "compute_method".to_string(),
                Value::Str("annoy_euclidean_50_trees".to_string()),
            ),
        ])
        .into(),
    ))
}

/// Seurat 5.5.1-compatible integration anchors: shared CCA or reciprocal-PCA
/// space, mutual cross-dataset neighbours, high-dimensional filtering, then
/// four-neighbour shared-neighbour scoring.
///
/// Ported and adapted from Seurat's MIT-licensed `R/integration.R` and
/// `src/integration.cpp`; see `packages/singlecell/SEURAT_MIT_NOTICE.md`.
fn builtin_sc_find_anchors(args: Vec<Value>) -> Result<Value> {
    let left = require_dense_matrix(&args[0], "sc_find_anchors")?;
    let right = require_dense_matrix(&args[1], "sc_find_anchors")?;
    let opts = match args.get(2) {
        Some(Value::Record(fields)) => fields.as_ref().clone(),
        _ => HashMap::new(),
    };
    if left.is_empty() || right.is_empty() {
        return Err(BioLangError::type_error(
            "sc_find_anchors() requires two non-empty matrices",
            None,
        ));
    }
    let genes = left[0].len();
    if genes == 0
        || right[0].len() != genes
        || left.iter().any(|row| row.len() != genes)
        || right.iter().any(|row| row.len() != genes)
    {
        return Err(BioLangError::type_error(
            "sc_find_anchors() requires rectangular matrices with identical feature columns",
            None,
        ));
    }
    let requested_dims = record_number(&opts, "dims", 30.0).max(1.0) as usize;
    let k_anchor = record_number(&opts, "k_anchor", 5.0).max(1.0) as usize;
    let k_filter = record_number(&opts, "k_filter", 200.0).max(0.0) as usize;
    let k_score = record_number(&opts, "k_score", 30.0).max(1.0) as usize;
    let max_features = record_number(&opts, "max_features", 200.0).max(1.0) as usize;
    let cca_sweeps = record_number(&opts, "cca_sweeps", 12.0).clamp(1.0, 100.0) as usize;
    let cca_oversample = record_number(&opts, "cca_oversample", 32.0).clamp(0.0, 256.0) as usize;
    let cca_solver = match opts.get("cca_solver") {
        Some(Value::Str(value)) if value.eq_ignore_ascii_case("lanczos") => "lanczos",
        _ => "subspace",
    };
    let cca_work_extra = record_number(&opts, "cca_work_extra", 7.0).clamp(2.0, 256.0) as usize;
    let cca_tolerance = record_number(&opts, "cca_tolerance", 1e-5).clamp(1e-12, 1.0);
    let cca_max_iterations =
        record_number(&opts, "cca_max_iterations", 1000.0).clamp(1.0, 10_000.0) as usize;
    let cca_seed = record_number(&opts, "cca_seed", 42.0).max(1.0) as u64;
    let cca_standardize = !matches!(opts.get("cca_standardize"), Some(Value::Bool(false)));
    let trace_neighbours = matches!(opts.get("trace_neighbours"), Some(Value::Bool(true)));
    let requested_external_provider = external_provider_program(&opts);
    let cca_initial: Option<Vec<f64>> = match opts.get("cca_initial") {
        Some(Value::List(values)) => Some(
            values
                .iter()
                .map(|value| {
                    value.as_float().ok_or_else(|| {
                        BioLangError::type_error(
                            "sc_find_anchors() cca_initial must be a List<Number>",
                            None,
                        )
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        ),
        Some(_) => {
            return Err(BioLangError::type_error(
                "sc_find_anchors() cca_initial must be a List<Number>",
                None,
            ))
        }
        None => None,
    };
    let mut supplied_filter_features: Option<Vec<usize>> = match opts.get("filter_features") {
        Some(Value::List(values)) => Some(
            values
                .iter()
                .map(|value| match value {
                    Value::Int(index) if *index >= 0 && (*index as usize) < genes => {
                        Ok(*index as usize)
                    }
                    _ => Err(BioLangError::type_error(
                        "sc_find_anchors() filter_features must contain valid non-negative feature indices",
                        None,
                    )),
                })
                .collect::<Result<Vec<_>>>()?,
        ),
        Some(_) => {
            return Err(BioLangError::type_error(
                "sc_find_anchors() filter_features must be a List<Int>",
                None,
            ))
        }
        None => None,
    };
    let reduction = match opts.get("reduction") {
        Some(Value::Str(name)) => name.to_ascii_lowercase(),
        _ => "cca".to_string(),
    };

    let mut provider_weight_reduction: Option<Vec<Vec<f64>>> = None;
    let mut provider_manifest: Option<HashMap<String, Value>> = None;
    let mut provider_program_used: Option<String> = None;

    let (left_embedding, right_embedding, left_projection, right_projection, compute_method) =
        match reduction.as_str() {
            "cca" => {
                let mut supplied_embeddings =
                    match (opts.get("left_embedding"), opts.get("right_embedding")) {
                        (Some(left_value), Some(right_value)) => Some((
                            require_dense_matrix(left_value, "sc_find_anchors")?,
                            require_dense_matrix(right_value, "sc_find_anchors")?,
                        )),
                        (None, None) => None,
                        _ => return Err(BioLangError::type_error(
                            "sc_find_anchors() requires both left_embedding and right_embedding",
                            None,
                        )),
                    };
                if supplied_embeddings.is_none() {
                    if let Some(program) = requested_external_provider.clone() {
                        let result = external_cca(
                            program,
                            &left,
                            &right,
                            requested_dims,
                            cca_seed,
                            max_features,
                        )?;
                        supplied_filter_features = Some(result.filter_features);
                        provider_weight_reduction = Some(result.weight_reduction);
                        provider_manifest = Some(result.manifest);
                        provider_program_used = Some(result.program);
                        supplied_embeddings = Some((result.left_embedding, result.right_embedding));
                    }
                }
                if let Some((left_embedding, right_embedding)) = supplied_embeddings {
                    let dimensions = left_embedding.first().map(Vec::len).unwrap_or(0);
                    if left_embedding.len() != left.len()
                        || right_embedding.len() != right.len()
                        || dimensions == 0
                        || right_embedding.first().map(Vec::len).unwrap_or(0) != dimensions
                        || left_embedding.iter().any(|row| row.len() != dimensions)
                        || right_embedding.iter().any(|row| row.len() != dimensions)
                    {
                        return Err(BioLangError::type_error(
                            "sc_find_anchors() supplied embeddings must have one rectangular row per input cell and identical dimensions",
                            None,
                        ));
                    }
                    (
                        left_embedding,
                        right_embedding,
                        Vec::new(),
                        Vec::new(),
                        if provider_program_used.is_some() {
                            "external_process_cca_embedding".to_string()
                        } else {
                            "supplied_cca_embedding".to_string()
                        },
                    )
                } else {
                    let left_standardized = if cca_standardize {
                        standardize_cells(&left)
                    } else {
                        left.clone()
                    };
                    let right_standardized = if cca_standardize {
                        standardize_cells(&right)
                    } else {
                        right.clone()
                    };
                    let (
                        left_embedding,
                        right_embedding,
                        left_projection,
                        right_projection,
                        _,
                        method,
                    ) = cca_dense(
                        &left_standardized,
                        &right_standardized,
                        requested_dims,
                        cca_sweeps,
                        cca_oversample,
                        cca_solver,
                        cca_work_extra,
                        cca_tolerance,
                        cca_max_iterations,
                        cca_seed,
                        cca_initial.as_deref(),
                    )?;
                    (
                        left_embedding,
                        right_embedding,
                        left_projection,
                        right_projection,
                        method,
                    )
                }
            }
            "rpca" => {
                let (left_scores, left_loadings, left_means) =
                    pca_projection_parts(&left, requested_dims, "sc_find_anchors")?;
                let (right_scores, right_loadings, right_means) =
                    pca_projection_parts(&right, requested_dims, "sc_find_anchors")?;
                let right_in_left =
                    l2_normalise_rows(&project_pca(&right, &left_loadings, &left_means));
                let left_in_right =
                    l2_normalise_rows(&project_pca(&left, &right_loadings, &right_means));
                let left_unit = l2_normalise_rows(&left_scores);
                let right_unit = l2_normalise_rows(&right_scores);
                let dimensions = left_unit
                    .first()
                    .map(Vec::len)
                    .unwrap_or(0)
                    .min(right_unit.first().map(Vec::len).unwrap_or(0));
                let left_common: Vec<Vec<f64>> = left_unit
                    .iter()
                    .zip(&left_in_right)
                    .map(|(own, reciprocal)| {
                        own.iter()
                            .take(dimensions)
                            .chain(reciprocal.iter().take(dimensions))
                            .copied()
                            .collect()
                    })
                    .collect();
                let right_common: Vec<Vec<f64>> = right_in_left
                    .iter()
                    .zip(&right_unit)
                    .map(|(reciprocal, own)| {
                        reciprocal
                            .iter()
                            .take(dimensions)
                            .chain(own.iter().take(dimensions))
                            .copied()
                            .collect()
                    })
                    .collect();
                (
                    left_common.clone(),
                    right_common.clone(),
                    left_common,
                    right_common,
                    "reciprocal_pca_cpu".to_string(),
                )
            }
            other => {
                return Err(BioLangError::type_error(
                    format!("sc_find_anchors() reduction must be 'cca' or 'rpca', got '{other}'"),
                    None,
                ))
            }
        };

    let neighbour_k = k_anchor.max(k_score);
    let left_to_right = cross_neighbour_rows(&left_embedding, &right_embedding, neighbour_k);
    let right_to_left = cross_neighbour_rows(&right_embedding, &left_embedding, neighbour_k);
    // Seurat asks its self-search for k+1 neighbours, so the first k scoring
    // entries include the cell itself and k-1 other cells. Ask the shared
    // helper for k non-self rows: its Annoy path requests k+1 before removing
    // self, preserving Seurat's n.trees * (k+1) default search budget.
    let left_within: Vec<Vec<usize>> = neighbour_rows_metric(
        &left_embedding,
        k_score.min(left_embedding.len().saturating_sub(1)),
        "euclidean",
    )
    .into_iter()
    .enumerate()
    .map(|(cell, row)| {
        std::iter::once(cell)
            .chain(row.into_iter().map(|(index, _)| index))
            .take(k_score)
            .collect()
    })
    .collect();
    let right_within: Vec<Vec<usize>> = neighbour_rows_metric(
        &right_embedding,
        k_score.min(right_embedding.len().saturating_sub(1)),
        "euclidean",
    )
    .into_iter()
    .enumerate()
    .map(|(cell, row)| {
        std::iter::once(cell)
            .chain(row.into_iter().map(|(index, _)| index))
            .take(k_score)
            .collect()
    })
    .collect();

    let mut anchor_pairs = Vec::new();
    for (left_index, candidates) in left_to_right.iter().enumerate() {
        for &(right_index, _) in candidates.iter().take(k_anchor) {
            if !right_to_left[right_index]
                .iter()
                .take(k_anchor)
                .any(|&(candidate, _)| candidate == left_index)
            {
                continue;
            }
            anchor_pairs.push((left_index, right_index));
        }
    }
    if anchor_pairs.is_empty() {
        return Err(BioLangError::runtime(ErrorKind::TypeError,
            "sc_find_anchors() found no mutual nearest neighbours; increase k_anchor or check that the datasets share biology".to_string(), None));
    }

    let effective_filter = if reduction == "cca" { k_filter } else { 0 };
    let anchors_before_filter = anchor_pairs.len();
    let candidate_pairs = anchor_pairs.clone();
    let mut selected_filter_features = Vec::new();
    if effective_filter > 0
        && left.len().min(right.len()) >= effective_filter
        && (supplied_filter_features.is_some() || !left_projection.is_empty())
    {
        let filter_features = if let Some(features) = &supplied_filter_features {
            features.clone()
        } else {
            let loadings =
                projected_feature_loadings(&left, &right, &left_projection, &right_projection);
            top_dim_features(&loadings, max_features)
        };
        selected_filter_features = filter_features.clone();
        if !filter_features.is_empty() {
            let left_filter = l2_normalise_rows(
                &left
                    .iter()
                    .map(|row| {
                        filter_features
                            .iter()
                            .map(|&feature| row[feature])
                            .collect()
                    })
                    .collect::<Vec<Vec<f64>>>(),
            );
            let right_filter = l2_normalise_rows(
                &right
                    .iter()
                    .map(|row| {
                        filter_features
                            .iter()
                            .map(|&feature| row[feature])
                            .collect()
                    })
                    .collect::<Vec<Vec<f64>>>(),
            );
            let filter_neighbours =
                cross_neighbour_rows(&left_filter, &right_filter, effective_filter);
            anchor_pairs.retain(|&(left_index, right_index)| {
                filter_neighbours[left_index]
                    .iter()
                    .any(|&(candidate, _)| candidate == right_index)
            });
        }
    }
    if anchor_pairs.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "sc_find_anchors() retained no anchors after high-dimensional filtering".to_string(),
            None,
        ));
    }

    let offset = left_embedding.len();
    let left_neighbour_sets: Vec<HashSet<usize>> = (0..left_embedding.len())
        .map(|cell| {
            left_within[cell]
                .iter()
                .copied()
                .chain(
                    left_to_right[cell]
                        .iter()
                        .take(k_score)
                        .map(|(index, _)| offset + *index),
                )
                .collect()
        })
        .collect();
    let right_neighbour_sets: Vec<HashSet<usize>> = (0..right_embedding.len())
        .map(|cell| {
            right_to_left[cell]
                .iter()
                .take(k_score)
                .map(|(index, _)| *index)
                .chain(right_within[cell].iter().map(|index| offset + *index))
                .collect()
        })
        .collect();
    let anchor_raw: Vec<(usize, usize, f64)> = anchor_pairs
        .into_iter()
        .map(|(left_index, right_index)| {
            let shared = left_neighbour_sets[left_index]
                .intersection(&right_neighbour_sets[right_index])
                .count();
            (left_index, right_index, shared as f64)
        })
        .collect();
    let mut raw_scores: Vec<f64> = anchor_raw.iter().map(|anchor| anchor.2).collect();
    raw_scores.sort_by(f64::total_cmp);
    let quantile = |probability: f64| {
        let position = probability * raw_scores.len().saturating_sub(1) as f64;
        let low = position.floor() as usize;
        let high = position.ceil() as usize;
        let fraction = position - low as f64;
        raw_scores[low] * (1.0 - fraction) + raw_scores[high] * fraction
    };
    let low_score = quantile(0.01);
    let high_score = quantile(0.90);
    let span = (high_score - low_score).max(1e-12);
    let anchors: Vec<Value> = anchor_raw
        .into_iter()
        .map(|(left_index, right_index, raw_score)| {
            let score = ((raw_score - low_score) / span).clamp(0.0, 1.0);
            Value::Record(
                HashMap::from([
                    ("left".to_string(), Value::Int(left_index as i64)),
                    ("right".to_string(), Value::Int(right_index as i64)),
                    ("score".to_string(), Value::Float(score)),
                    ("raw_score".to_string(), Value::Float(raw_score)),
                ])
                .into(),
            )
        })
        .collect();
    let candidate_anchors: Vec<Value> = candidate_pairs
        .into_iter()
        .map(|(left_index, right_index)| {
            Value::Record(
                HashMap::from([
                    ("left".to_string(), Value::Int(left_index as i64)),
                    ("right".to_string(), Value::Int(right_index as i64)),
                ])
                .into(),
            )
        })
        .collect();
    let dimensions = left_embedding.first().map(Vec::len).unwrap_or(0);
    let mut output = HashMap::from([
        ("anchors".to_string(), Value::List(anchors.into())),
        (
            "candidate_anchors".to_string(),
            Value::List(candidate_anchors.into()),
        ),
        (
            "filter_features".to_string(),
            Value::List(
                selected_filter_features
                    .into_iter()
                    .map(|feature| Value::Int(feature as i64))
                    .collect::<Vec<_>>()
                    .into(),
            ),
        ),
        (
            "left_embedding".to_string(),
            matrix_to_value(left_embedding),
        ),
        (
            "right_embedding".to_string(),
            matrix_to_value(right_embedding),
        ),
        ("reduction".to_string(), Value::Str(reduction)),
        ("compute_method".to_string(), Value::Str(compute_method)),
        ("dims".to_string(), Value::Int(dimensions as i64)),
        ("k_anchor".to_string(), Value::Int(k_anchor as i64)),
        ("k_filter".to_string(), Value::Int(effective_filter as i64)),
        ("k_score".to_string(), Value::Int(k_score as i64)),
        ("max_features".to_string(), Value::Int(max_features as i64)),
        ("cca_sweeps".to_string(), Value::Int(cca_sweeps as i64)),
        (
            "cca_oversample".to_string(),
            Value::Int(cca_oversample as i64),
        ),
        ("cca_solver".to_string(), Value::Str(cca_solver.to_string())),
        (
            "cca_work_extra".to_string(),
            Value::Int(cca_work_extra as i64),
        ),
        ("cca_tolerance".to_string(), Value::Float(cca_tolerance)),
        (
            "cca_max_iterations".to_string(),
            Value::Int(cca_max_iterations as i64),
        ),
        ("cca_seed".to_string(), Value::Int(cca_seed as i64)),
        ("cca_standardize".to_string(), Value::Bool(cca_standardize)),
        (
            "anchors_before_filter".to_string(),
            Value::Int(anchors_before_filter as i64),
        ),
    ]);
    if let Some(weight_reduction) = provider_weight_reduction {
        output.insert(
            "weight_reduction".to_string(),
            matrix_to_value(weight_reduction),
        );
    }
    if let Some(manifest) = provider_manifest {
        output.insert(
            "external_provider_manifest".to_string(),
            Value::Record(manifest.into()),
        );
    }
    if let Some(program) = provider_program_used {
        output.insert("external_provider".to_string(), Value::Str(program));
    }
    if trace_neighbours {
        let neighbour_matrix = |rows: &[Vec<(usize, f64)>]| {
            matrix_to_value(
                rows.iter()
                    .map(|row| {
                        row.iter()
                            .take(k_score)
                            .map(|(index, _)| *index as f64)
                            .collect()
                    })
                    .collect(),
            )
        };
        let index_matrix = |rows: &[Vec<usize>]| {
            matrix_to_value(
                rows.iter()
                    .map(|row| {
                        row.iter()
                            .take(k_score)
                            .map(|index| *index as f64)
                            .collect()
                    })
                    .collect(),
            )
        };
        output.insert(
            "left_to_right_neighbours".to_string(),
            neighbour_matrix(&left_to_right),
        );
        output.insert(
            "right_to_left_neighbours".to_string(),
            neighbour_matrix(&right_to_left),
        );
        output.insert(
            "left_within_neighbours".to_string(),
            index_matrix(&left_within),
        );
        output.insert(
            "right_within_neighbours".to_string(),
            index_matrix(&right_within),
        );
    }
    Ok(Value::Record(output.into()))
}

/// Locally weighted anchor correction. The kernel and integration-vector
/// direction follow Seurat 5.5.1 FindWeightsC/IntegrateDataC. Returns reference
/// rows followed by corrected query rows in the same order as
/// `sc_merge_objects`.
fn builtin_sc_integrate_anchors(args: Vec<Value>) -> Result<Value> {
    let left = singlecell_matrix(&args[0], "sc_integrate_anchors")?;
    let right = singlecell_matrix(&args[1], "sc_integrate_anchors")?;
    let anchor_set = match &args[2] {
        Value::Record(fields) => fields,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "sc_integrate_anchors() expected AnchorSet Record, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    let opts = match args.get(3) {
        Some(Value::Record(fields)) => fields.as_ref().clone(),
        _ => HashMap::new(),
    };
    let return_details = matches!(opts.get("return_details"), Some(Value::Bool(true)));
    let diagnostic_weight_cells: Vec<usize> = match opts.get("diagnostic_weight_cells") {
        Some(Value::List(values)) => values
            .iter()
            .map(|value| match value {
                Value::Int(index) if *index >= 0 => Ok(*index as usize),
                _ => Err(BioLangError::type_error(
                    "sc_integrate_anchors() diagnostic_weight_cells must contain non-negative integers",
                    None,
                )),
            })
            .collect::<Result<Vec<_>>>()?,
        Some(_) => {
            return Err(BioLangError::type_error(
                "sc_integrate_anchors() diagnostic_weight_cells must be a List<Int>",
                None,
            ))
        }
        None => Vec::new(),
    };
    let (left_cells, features) = left.dimensions();
    let (right_cells, right_features) = right.dimensions();
    if left_cells == 0 || right_cells == 0 || features == 0 || right_features != features {
        return Err(BioLangError::type_error(
            "sc_integrate_anchors() requires non-empty matrices with identical feature columns",
            None,
        ));
    }
    let anchor_values = match anchor_set.get("anchors") {
        Some(Value::List(values)) => values,
        _ => {
            return Err(BioLangError::type_error(
                "sc_integrate_anchors() AnchorSet has no anchors",
                None,
            ))
        }
    };
    let mut anchors = Vec::with_capacity(anchor_values.len());
    for value in anchor_values.iter() {
        let fields = match value {
            Value::Record(fields) => fields,
            _ => {
                return Err(BioLangError::type_error(
                    "sc_integrate_anchors() invalid anchor entry",
                    None,
                ))
            }
        };
        let index = |name: &str| -> Result<usize> {
            match fields.get(name) {
                Some(Value::Int(value)) if *value >= 0 => Ok(*value as usize),
                _ => Err(BioLangError::type_error(
                    format!("sc_integrate_anchors() anchor has invalid {name} index"),
                    None,
                )),
            }
        };
        let left_index = index("left")?;
        let right_index = index("right")?;
        if left_index >= left_cells || right_index >= right_cells {
            return Err(BioLangError::type_error(
                "sc_integrate_anchors() anchor index is outside its matrix",
                None,
            ));
        }
        let score = fields.get("score").and_then(Value::as_float).unwrap_or(1.0);
        anchors.push((left_index, right_index, score.max(0.0)));
    }
    if anchors.is_empty() {
        return Err(BioLangError::type_error(
            "sc_integrate_anchors() requires at least one anchor",
            None,
        ));
    }
    let k_weight = record_number(&opts, "k_weight", 100.0).max(1.0) as usize;
    let sd_weight = record_number(&opts, "sd_weight", 1.0).max(1e-8);
    let right_embedding = if let Some(reduction) = opts.get("weight_reduction") {
        let reduction = require_matrix(reduction, "sc_integrate_anchors")?;
        if reduction.len() != right_cells {
            return Err(BioLangError::type_error(
                "sc_integrate_anchors() weight_reduction must have one row per query cell",
                None,
            ));
        }
        reduction
    } else {
        // Seurat 5.5.1 RunIntegration defaults to a fresh PCA over the merged,
        // feature-centered SCT residuals when no weight.reduction is supplied.
        // `pca_projection_parts` performs that centering internally. Reusing
        // the CCA embedding here was numerically convenient but changed which
        // anchors were local to each query cell and propagated into every
        // integrated PC, graph edge, cluster, and marker table.
        let dimensions = anchor_set
            .get("dims")
            .and_then(Value::as_float)
            .map(|value| value.max(1.0) as usize)
            .unwrap_or(30);
        let scores = if let (SingleCellMatrix::Flat(left), SingleCellMatrix::Flat(right)) =
            (&left, &right)
        {
            let joined = SingleCellMatrix::JoinedFlat(left, right);
            pca_projection_parts_from_matrix(
                &joined,
                dimensions,
                "sc_integrate_anchors weight reduction",
            )?
            .0
        } else {
            // Compatibility path for interpreted nested lists and sparse test
            // inputs. Package SCT data arrives as compact `Matrix`, so the HBC
            // path above does not allocate this merged representation.
            let merged: Vec<Vec<f64>> = (0..left_cells + right_cells)
                .map(|cell| {
                    let (matrix, row) = if cell < left_cells {
                        (&left, cell)
                    } else {
                        (&right, cell - left_cells)
                    };
                    (0..features)
                        .map(|feature| matrix.value_at(row, feature))
                        .collect()
                })
                .collect();
            pca_projection_parts(&merged, dimensions, "sc_integrate_anchors weight reduction")?.0
        };
        scores.into_iter().skip(left_cells).collect()
    };
    let mut anchors_by_query_cell = vec![Vec::new(); right_cells];
    for (anchor_index, &(_, query_cell, _)) in anchors.iter().enumerate() {
        anchors_by_query_cell[query_cell].push(anchor_index);
    }
    let mut seen_anchor_cells = HashSet::new();
    let unique_anchor_cells: Vec<usize> = anchors
        .iter()
        .filter_map(|anchor| seen_anchor_cells.insert(anchor.1).then_some(anchor.1))
        .collect();
    let effective_k = k_weight
        .min(anchors.len())
        .min(unique_anchor_cells.len())
        .max(1);
    let anchor_cell_embedding: Vec<Vec<f64>> = unique_anchor_cells
        .iter()
        .map(|&cell| right_embedding[cell].clone())
        .collect();
    let query_neighbours =
        cross_neighbour_rows(&right_embedding, &anchor_cell_embedding, effective_k);
    let mut corrected = Vec::with_capacity(right_cells.saturating_mul(features));
    for cell in 0..right_cells {
        for feature in 0..features {
            corrected.push(right.value_at(cell, feature));
        }
    }
    // Every corrected query row is independent. Parallelising by complete rows
    // preserves the exact per-feature summation order while avoiding a serial
    // 14k cells x 3k features x 100 anchors kernel on HBC-scale integrations.
    par_rows_mut(&mut corrected, features, |first_query, corrected_rows| {
        for (local_query, corrected_row) in corrected_rows.chunks_mut(features).enumerate() {
            let query_index = first_query + local_query;
            let neighbours = &query_neighbours[query_index];
            if neighbours.is_empty() {
                continue;
            }
            let weights = integration_anchor_weights(
                neighbours,
                &unique_anchor_cells,
                &anchors_by_query_cell,
                &anchors,
                effective_k,
                sd_weight,
            );
            if weights.is_empty() {
                continue;
            }
            for (feature, value) in corrected_row.iter_mut().enumerate() {
                let adjustment: f64 = weights
                    .iter()
                    .map(|&(anchor_index, weight)| {
                        let (left_index, right_index, _) = anchors[anchor_index];
                        weight
                            * (left.value_at(left_index, feature)
                                - right.value_at(right_index, feature))
                    })
                    .sum::<f64>();
                *value += adjustment;
            }
        }
    });
    // The integrated assay is dense but it does not need interpreter boxing.
    // Keep it in the native flat Matrix representation so the following PCA
    // reads one compact allocation instead of retaining tens of millions of
    // `Value::Float` elements alongside another numeric copy.
    let rows = left_cells + right_cells;
    let mut data = Vec::with_capacity(rows.saturating_mul(features));
    for cell in 0..left_cells {
        for feature in 0..features {
            data.push(left.value_at(cell, feature));
        }
    }
    data.extend(corrected);
    let matrix = bl_core::matrix::Matrix::new(data, rows, features).map_err(|error| {
        BioLangError::type_error(format!("sc_integrate_anchors(): {error}"), None)
    })?;
    let matrix = Value::Matrix(matrix.into());
    if return_details {
        let mut diagnostic_weights = Vec::new();
        for &query_index in &diagnostic_weight_cells {
            if query_index >= right_cells {
                return Err(BioLangError::type_error(
                    "sc_integrate_anchors() diagnostic_weight_cells index is outside the query matrix",
                    None,
                ));
            }
            for (anchor_index, weight) in integration_anchor_weights(
                &query_neighbours[query_index],
                &unique_anchor_cells,
                &anchors_by_query_cell,
                &anchors,
                effective_k,
                sd_weight,
            ) {
                diagnostic_weights.push(Value::Record(
                    HashMap::from([
                        ("query_cell".to_string(), Value::Int(query_index as i64)),
                        ("anchor_index".to_string(), Value::Int(anchor_index as i64)),
                        ("weight".to_string(), Value::Float(weight)),
                    ])
                    .into(),
                ));
            }
        }
        Ok(Value::Record(
            HashMap::from([
                ("integrated_matrix".to_string(), matrix),
                (
                    "query_weight_embedding".to_string(),
                    matrix_to_value(right_embedding),
                ),
                ("effective_k".to_string(), Value::Int(effective_k as i64)),
                (
                    "diagnostic_weights".to_string(),
                    Value::List(diagnostic_weights.into()),
                ),
            ])
            .into(),
        ))
    } else {
        Ok(matrix)
    }
}

fn integration_anchor_weights(
    neighbours: &[(usize, f64)],
    unique_anchor_cells: &[usize],
    anchors_by_query_cell: &[Vec<usize>],
    anchors: &[(usize, usize, f64)],
    effective_k: usize,
    sd_weight: f64,
) -> Vec<(usize, f64)> {
    let distance_scale = neighbours
        .last()
        .map(|(_, distance)| *distance)
        .unwrap_or(1.0);
    let mut weights = Vec::with_capacity(effective_k);
    for &(anchor_cell_position, distance) in neighbours {
        let anchor_cell = unique_anchor_cells[anchor_cell_position];
        let similarity = if distance_scale <= 1e-15 {
            1.0
        } else {
            (1.0 - distance / distance_scale).max(0.0)
        };
        for &anchor_index in &anchors_by_query_cell[anchor_cell] {
            if weights.len() >= effective_k {
                break;
            }
            let score = anchors[anchor_index].2;
            // Seurat 5.5.1 FindWeightsC:
            // 1 - exp(-distance_similarity * anchor_score / (2 / sd)^2)
            let weight = 1.0 - (-similarity * score / (2.0 / sd_weight).powi(2)).exp();
            weights.push((anchor_index, weight));
        }
        if weights.len() >= effective_k {
            break;
        }
    }
    let total_weight: f64 = weights.iter().map(|(_, weight)| *weight).sum();
    if total_weight <= 1e-15 {
        Vec::new()
    } else {
        weights
            .into_iter()
            .map(|(anchor_index, weight)| (anchor_index, weight / total_weight))
            .collect()
    }
}

fn builtin_harmony_integrate(args: Vec<Value>) -> Result<Value> {
    let opts: HashMap<String, Value> = match args.get(2) {
        Some(Value::Record(map)) => map.as_ref().clone(),
        _ => HashMap::new(),
    };
    let number = |key: &str, fallback: f64| -> f64 {
        opts.get(key).and_then(|v| v.as_float()).unwrap_or(fallback)
    };
    let theta = number("theta", 2.0);
    let sigma = number("sigma", 0.1).max(1e-6);
    let ridge = number("lambda", 1.0);
    let max_iter = number("max_iter", 10.0).max(1.0) as usize;
    let compact = matches!(opts.get("compact"), Some(Value::Bool(true)));

    let embedding = require_matrix(&args[0], "harmony_integrate")?;
    let n_cells = embedding.len();
    let n_dims = embedding.first().map(|row| row.len()).unwrap_or(0);
    if n_cells == 0 || n_dims == 0 {
        return if compact {
            matrix_to_compact_value(embedding, "harmony_integrate")
        } else {
            Ok(matrix_to_value(embedding))
        };
    }

    let labels: Vec<String> = match &args[1] {
        Value::List(items) => items.iter().map(|v| format!("{v}")).collect(),
        _ => {
            return Err(BioLangError::type_error(
                "harmony_integrate() requires a List of batch labels, one per cell",
                None,
            ))
        }
    };
    if labels.len() != n_cells {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "harmony_integrate(): {} batch labels for {n_cells} cells",
                labels.len()
            ),
            None,
        ));
    }

    let mut batch_order: Vec<String> = Vec::new();
    let batch_of: Vec<usize> = labels
        .iter()
        .map(|label| {
            batch_order
                .iter()
                .position(|seen| seen == label)
                .unwrap_or_else(|| {
                    batch_order.push(label.clone());
                    batch_order.len() - 1
                })
        })
        .collect();
    let n_batches = batch_order.len();
    if n_batches < 2 {
        // One batch is nothing to correct, and the regression would be singular.
        return if compact {
            matrix_to_compact_value(embedding, "harmony_integrate")
        } else {
            Ok(matrix_to_value(embedding))
        };
    }
    let batch_sizes: Vec<f64> = (0..n_batches)
        .map(|b| batch_of.iter().filter(|&&x| x == b).count() as f64)
        .collect();

    // Harmony's own default: enough clusters to separate cell types, capped so
    // the per-cluster regressions stay cheap.
    let n_clusters = opts
        .get("n_clusters")
        .and_then(|v| v.as_float())
        .map(|v| v as usize)
        .unwrap_or_else(|| (n_cells / 30).clamp(1, 100))
        .clamp(1, n_cells);

    // Flatten. Every inner loop below is a length-`n_dims` dot product, and a
    // separate allocation per cell means a pointer chase before each one and
    // 30,000 allocations per iteration for the three working copies. One array
    // with a fixed stride makes those loops contiguous and the copies memcpys.
    let mut corrected: Vec<f64> = embedding.iter().flatten().copied().collect();
    drop(embedding);
    let mut unit = vec![0.0f64; n_cells * n_dims];
    let mut snapshot = vec![0.0f64; n_cells * n_dims];
    let mut shift = vec![0.0f64; n_cells * n_dims];

    for _ in 0..max_iter {
        // Cosine geometry, so the clustering follows direction rather than
        // magnitude - the same reason Harmony L2-normalises here.
        l2_normalise_into(&corrected, n_dims, &mut unit);
        let centroids = kmeans_cosine(&unit, n_dims, n_clusters);
        let n_clusters_found = centroids.len() / n_dims.max(1);
        let assignments = soft_assign(
            &unit,
            n_dims,
            &centroids,
            sigma,
            theta,
            &batch_of,
            &batch_sizes,
        );

        // Every cluster regresses against the same embedding, and the shifts are
        // summed and applied once. Correcting in place instead would have each
        // cluster fitting data the previous ones had already moved - the
        // clusters overlap, so those corrections compound rather than combine.
        snapshot.copy_from_slice(&corrected);
        shift.fill(0.0);
        correct_all_clusters(
            &snapshot,
            n_dims,
            &assignments,
            n_clusters_found,
            &batch_of,
            n_batches,
            ridge,
            &mut shift,
        );
        for (value, delta) in corrected.iter_mut().zip(&shift) {
            *value -= delta;
        }
    }

    if compact {
        let matrix = bl_core::matrix::Matrix::new(corrected, n_cells, n_dims).map_err(|error| {
            BioLangError::type_error(format!("harmony_integrate(): {error}"), None)
        })?;
        Ok(Value::Matrix(matrix.into()))
    } else {
        Ok(flat_matrix_to_value(&corrected, n_dims))
    }
}

/// A row-major flat matrix as the nested-list `Value` the language expects.
fn flat_matrix_to_value(data: &[f64], n_dims: usize) -> Value {
    if n_dims == 0 {
        return Value::List(Vec::new().into());
    }
    Value::List(
        data.chunks(n_dims)
            .map(|row| {
                Value::List(
                    row.iter()
                        .copied()
                        .map(Value::Float)
                        .collect::<Vec<_>>()
                        .into(),
                )
            })
            .collect::<Vec<_>>()
            .into(),
    )
}

fn l2_normalise_rows(rows: &[Vec<f64>]) -> Vec<Vec<f64>> {
    rows.iter()
        .map(|row| {
            let norm = row.iter().map(|v| v * v).sum::<f64>().sqrt();
            if norm > 1e-12 {
                row.iter().map(|v| v / norm).collect()
            } else {
                row.clone()
            }
        })
        .collect()
}

fn l2_normalise_into(rows: &[f64], n_dims: usize, out: &mut [f64]) {
    for (row, target) in rows.chunks(n_dims).zip(out.chunks_mut(n_dims)) {
        let norm = row.iter().map(|v| v * v).sum::<f64>().sqrt();
        if norm > 1e-12 {
            for (value, source) in target.iter_mut().zip(row) {
                *value = source / norm;
            }
        } else {
            target.copy_from_slice(row);
        }
    }
}

/// Split `output` into per-thread runs of whole rows and fill them in parallel.
///
/// `body` receives the index of the first row in its slice and the slice
/// itself. Nothing is summed across slices, so the answer does not depend on how
/// many threads ran — anything that *does* accumulate stays on a serial path, so
/// that two machines with different core counts still produce the same
/// embedding.
fn par_rows_mut<T, F>(output: &mut [T], stride: usize, body: F)
where
    T: Send,
    F: Fn(usize, &mut [T]) + Sync,
{
    let rows = if stride == 0 {
        0
    } else {
        output.len() / stride
    };
    let workers = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1)
        .min(rows.max(1));
    if workers <= 1 || rows == 0 {
        body(0, output);
        return;
    }
    let span = rows.div_ceil(workers);
    std::thread::scope(|scope| {
        let mut first = 0;
        let mut rest = output;
        while !rest.is_empty() {
            let take = (span * stride).min(rest.len());
            let (head, tail) = rest.split_at_mut(take);
            let body = &body;
            scope.spawn(move || body(first, head));
            first += take / stride;
            rest = tail;
        }
    });
}

/// Lloyd's algorithm on cosine distance, seeded deterministically.
///
/// Seeded by even spacing rather than at random: Harmony is run inside
/// pipelines whose figures are compared between runs, and a random start would
/// move the correction slightly every time for no reason the user can see.
fn kmeans_cosine(unit: &[f64], n_dims: usize, k: usize) -> Vec<f64> {
    let n = if n_dims == 0 { 0 } else { unit.len() / n_dims };
    if n == 0 || n_dims == 0 {
        return Vec::new();
    }
    let k = k.min(n).max(1);
    let stride = (n as f64 / k as f64).max(1.0);
    let mut centroids = vec![0.0f64; k * n_dims];
    for cluster in 0..k {
        let source = ((cluster as f64 * stride) as usize).min(n - 1);
        centroids[cluster * n_dims..(cluster + 1) * n_dims]
            .copy_from_slice(&unit[source * n_dims..(source + 1) * n_dims]);
    }

    let mut nearest = vec![0u32; n];
    for _ in 0..10 {
        // Assignment is per cell and writes only that cell's slot, so it splits
        // across threads without changing a single sum.
        //
        // One dot product per centroid, not two. `max_by` over an index range
        // re-evaluates its comparator's operands on every comparison, so the
        // running best's dot product was being recomputed k-1 times: exactly
        // twice the arithmetic of the loop below, in the hottest loop of the
        // slowest stage. `>=` reproduces `max_by`'s last-one-wins tie-break.
        let centroids_ref = &centroids;
        par_rows_mut(&mut nearest, 1, |first_cell, slot| {
            for (offset, target) in slot.iter_mut().enumerate() {
                let cell = first_cell + offset;
                let row = &unit[cell * n_dims..(cell + 1) * n_dims];
                let mut best = 0usize;
                let mut best_score = f64::NEG_INFINITY;
                for cluster in 0..k {
                    let score = dot(
                        row,
                        &centroids_ref[cluster * n_dims..(cluster + 1) * n_dims],
                    );
                    if score >= best_score {
                        best_score = score;
                        best = cluster;
                    }
                }
                *target = best as u32;
            }
        });

        // The accumulation stays serial and in cell order, so the centroids are
        // the same floating-point sum whatever the machine's core count.
        let mut sums = vec![0.0f64; k * n_dims];
        let mut counts = vec![0.0f64; k];
        for (cell, &best) in nearest.iter().enumerate() {
            let row = &unit[cell * n_dims..(cell + 1) * n_dims];
            let target = &mut sums[best as usize * n_dims..(best as usize + 1) * n_dims];
            for (accumulator, value) in target.iter_mut().zip(row) {
                *accumulator += *value;
            }
            counts[best as usize] += 1.0;
        }
        for cluster in 0..k {
            if counts[cluster] <= 0.0 {
                continue;
            }
            let centroid = &mut centroids[cluster * n_dims..(cluster + 1) * n_dims];
            centroid.copy_from_slice(&sums[cluster * n_dims..(cluster + 1) * n_dims]);
            for value in centroid.iter_mut() {
                *value /= counts[cluster];
            }
            let norm = centroid.iter().map(|v| v * v).sum::<f64>().sqrt();
            if norm > 1e-12 {
                for value in centroid.iter_mut() {
                    *value /= norm;
                }
            }
        }
    }
    centroids
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Soft cluster membership, penalised for batch imbalance.
///
/// Returns one weight vector per cluster. The penalty compares each cluster's
/// observed batch composition against the composition it would have if batches
/// were spread evenly, and pulls cells towards clusters where their own batch is
/// under-represented. `theta` is how hard it pulls.
/// Returns membership cell-major: cell `i`'s `k` weights are `r[i * k..][..k]`.
///
/// Cell-major rather than cluster-major. Every consumer — the composition
/// reduction here, and the regression that follows — reads all of one cell's
/// clusters together, which cluster-major storage spreads across `k` separate
/// arrays a full cell-count apart. On 30,000 cells and 100 clusters that is a
/// cache miss per cluster per cell; contiguously it is a few lines.
fn soft_assign(
    unit: &[f64],
    n_dims: usize,
    centroids: &[f64],
    sigma: f64,
    theta: f64,
    batch_of: &[usize],
    batch_sizes: &[f64],
) -> Vec<f64> {
    let n = if n_dims == 0 { 0 } else { unit.len() / n_dims };
    let k = if n_dims == 0 {
        0
    } else {
        centroids.len() / n_dims
    };
    let total = n as f64;
    let mut r = vec![0.0f64; n * k];
    if n == 0 || k == 0 {
        return r;
    }

    // Unpenalised assignment first, to have an observed composition to penalise.
    par_rows_mut(&mut r, k, |first_cell, block| {
        for (offset, weights) in block.chunks_mut(k).enumerate() {
            let cell = first_cell + offset;
            let row = &unit[cell * n_dims..(cell + 1) * n_dims];
            let mut sum = 0.0;
            for (cluster, weight) in weights.iter_mut().enumerate() {
                // Cosine distance on unit vectors.
                let value = (-(1.0
                    - dot(row, &centroids[cluster * n_dims..(cluster + 1) * n_dims]))
                    / sigma)
                    .exp();
                *weight = value;
                sum += value;
            }
            for weight in weights.iter_mut() {
                *weight = if sum > 0.0 {
                    *weight / sum
                } else {
                    1.0 / k as f64
                };
            }
        }
    });

    // A few refinements: the penalty depends on the composition, which depends
    // on the penalty.
    let n_batches = batch_sizes.len();
    for _ in 0..3 {
        let mut observed = vec![0.0f64; n_batches * k];
        for (cell, &batch) in batch_of.iter().enumerate() {
            let weights = &r[cell * k..(cell + 1) * k];
            let target = &mut observed[batch * k..(batch + 1) * k];
            for (accumulator, weight) in target.iter_mut().zip(weights) {
                *accumulator += *weight;
            }
        }
        let cluster_mass: Vec<f64> = (0..k)
            .map(|cluster| {
                (0..n_batches)
                    .map(|batch| observed[batch * k + cluster])
                    .sum()
            })
            .collect();

        // The penalty depends only on (batch, cluster), so it is the same for
        // every cell in a batch. Computing it once per batch instead of once per
        // cell removes an `exp`, a `powf` and two divisions from the inner loop.
        let mut penalty = vec![0.0f64; n_batches * k];
        for batch in 0..n_batches {
            for cluster in 0..k {
                let expected = batch_sizes[batch] * cluster_mass[cluster] / total;
                let seen = observed[batch * k + cluster].max(1e-9);
                penalty[batch * k + cluster] = (expected.max(1e-9) / seen).powf(theta);
            }
        }

        par_rows_mut(&mut r, k, |first_cell, block| {
            for (offset, weights) in block.chunks_mut(k).enumerate() {
                let cell = first_cell + offset;
                let row = &unit[cell * n_dims..(cell + 1) * n_dims];
                let batch = &penalty[batch_of[cell] * k..(batch_of[cell] + 1) * k];
                let mut sum = 0.0;
                for (cluster, weight) in weights.iter_mut().enumerate() {
                    let value = (-(1.0
                        - dot(row, &centroids[cluster * n_dims..(cluster + 1) * n_dims]))
                        / sigma)
                        .exp()
                        * batch[cluster];
                    *weight = value;
                    sum += value;
                }
                for weight in weights.iter_mut() {
                    *weight = if sum > 0.0 {
                        *weight / sum
                    } else {
                        1.0 / k as f64
                    };
                }
            }
        });
    }
    r
}

/// Ridge-regress the batch out of one cluster and subtract it.
///
/// The design is an intercept plus one indicator per batch. The intercept's
/// coefficient is deliberately *not* subtracted: it carries where the cluster
/// sits in the embedding, which is the biology. Only the batch coefficients are
/// removed, and the ridge term keeps a batch with few cells in this cluster from
/// producing an enormous correction.
/// All clusters at once, in two sweeps over the embedding instead of two per
/// cluster.
///
/// The per-cluster formulation reads the whole embedding and the whole shift
/// array once for every cluster: on 30,000 cells, 40 dimensions and 100
/// clusters that is 200 passes over ~19 MB per Harmony iteration, and the
/// arithmetic inside is a handful of adds per element. It is bandwidth, not
/// flops. Every cluster's accumulators together are `k * (size^2 + size *
/// n_dims)` — about 100 KB here, comfortably cache-resident — so hoisting the
/// cell loop outside the cluster loop turns those 200 passes into two.
///
/// Each accumulator still sees cells in ascending order and each shift still
/// sums clusters in ascending order, so this is the same floating-point
/// arithmetic in the same sequence, not merely the same value.
#[allow(clippy::too_many_arguments)]
fn correct_all_clusters(
    embedding: &[f64],
    n_dims: usize,
    weights: &[f64],
    n_clusters: usize,
    batch_of: &[usize],
    n_batches: usize,
    ridge: f64,
    shift: &mut [f64],
) {
    if n_clusters == 0 || n_dims == 0 {
        return;
    }
    let size = n_batches + 1;
    let n_cells = batch_of.len();
    let mut normal = vec![0.0f64; n_clusters * size * size];
    let mut rhs = vec![0.0f64; n_clusters * size * n_dims];

    for cell in 0..n_cells {
        let row = &embedding[cell * n_dims..(cell + 1) * n_dims];
        let batch = batch_of[cell] + 1;
        let memberships = &weights[cell * n_clusters..(cell + 1) * n_clusters];
        for (cluster, &weight) in memberships.iter().enumerate() {
            if weight <= 0.0 {
                continue;
            }
            let block = &mut normal[cluster * size * size..(cluster + 1) * size * size];
            block[0] += weight;
            block[batch] += weight;
            block[batch * size] += weight;
            block[batch * size + batch] += weight;

            let base = cluster * size * n_dims;
            let (intercept, indicator) = (base, base + batch * n_dims);
            for d in 0..n_dims {
                let value = weight * row[d];
                rhs[intercept + d] += value;
                rhs[indicator + d] += value;
            }
        }
    }

    // The systems are (batches + 1) square — single digits — so a direct solve
    // per cluster costs nothing next to the sweeps around it.
    let solved: Vec<Option<Vec<Vec<f64>>>> = (0..n_clusters)
        .map(|cluster| {
            let mut a: Vec<Vec<f64>> = (0..size)
                .map(|row| normal[cluster * size * size + row * size..][..size].to_vec())
                .collect();
            // Ridge on the batch terms only, so the intercept stays unpenalised.
            for batch in 1..size {
                a[batch][batch] += ridge;
            }
            let mut b: Vec<Vec<f64>> = (0..size)
                .map(|row| rhs[cluster * size * n_dims + row * n_dims..][..n_dims].to_vec())
                .collect();
            solve_multi(&mut a, &mut b)
        })
        .collect();

    for cell in 0..n_cells {
        let batch = batch_of[cell] + 1;
        let out = &mut shift[cell * n_dims..(cell + 1) * n_dims];
        let memberships = &weights[cell * n_clusters..(cell + 1) * n_clusters];
        for (cluster, &weight) in memberships.iter().enumerate() {
            if weight <= 0.0 {
                continue;
            }
            let Some(coefficients) = &solved[cluster] else {
                continue;
            };
            for (value, coefficient) in out.iter_mut().zip(&coefficients[batch]) {
                *value += weight * *coefficient;
            }
        }
    }
}

/// Gauss-Jordan with partial pivoting, solving for many right-hand sides at
/// once. The system is (batches + 1) square - single digits - so a direct
/// solve is both simplest and fastest.
fn solve_multi(a: &mut [Vec<f64>], b: &mut [Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let width = b.first().map(|row| row.len())?;
    for column in 0..n {
        let pivot = (column..n)
            .max_by(|&x, &y| {
                a[x][column]
                    .abs()
                    .partial_cmp(&a[y][column].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(column);
        if a[pivot][column].abs() < 1e-12 {
            // Singular: a batch absent from this cluster contributes nothing,
            // and skipping beats returning a wild correction.
            return None;
        }
        a.swap(column, pivot);
        b.swap(column, pivot);

        let divisor = a[column][column];
        for value in a[column].iter_mut() {
            *value /= divisor;
        }
        for value in b[column].iter_mut() {
            *value /= divisor;
        }
        for row in 0..n {
            if row == column {
                continue;
            }
            let factor = a[row][column];
            if factor == 0.0 {
                continue;
            }
            for k in 0..n {
                a[row][k] -= factor * a[column][k];
            }
            for k in 0..width {
                b[row][k] -= factor * b[column][k];
            }
        }
    }
    Some(b.to_vec())
}

// â”€â”€ find_all_markers(matrix, clusters, opts?) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Genes that distinguish each cluster from every other cell.
///
/// Seurat's FindAllMarkers, which is the step that turns numbered clusters into
/// cell types: cluster 3 is monocytes because LYZ, CD14 and S100A9 come out of
/// this table, not because of where it sits on a UMAP.
///
/// Each cluster is tested against all remaining cells with a Mann-Whitney
/// U test - the same `wilcoxon` builtin exposes, and Seurat's default. Two
/// pre-filters run first, both because they are the published defaults and
/// because they remove most of the work: a gene detected in almost no cell of
/// either group cannot be a marker, and neither can one whose means barely
/// differ.
///
/// Expression is assumed log1p-normalised, as it is after
/// `normalize_total |> log1p_transform`. The fold change follows Seurat and
/// undoes that before averaging - log2(mean(expm1(x)) + 1) per group - because
/// a difference of mean logs is not the log of a mean ratio, and quoting one as
/// the other overstates small folds.
fn builtin_find_all_markers(args: Vec<Value>) -> Result<Value> {
    let opts: HashMap<String, Value> = match args.get(2) {
        Some(Value::Record(map)) => map.as_ref().clone(),
        _ => HashMap::new(),
    };
    let number = |key: &str, fallback: f64| -> f64 {
        opts.get(key).and_then(|v| v.as_float()).unwrap_or(fallback)
    };
    let min_pct = number("min_pct", 0.01);
    let logfc_threshold = number("logfc_threshold", 0.1);
    let only_positive = opts.get("only_pos").map(|v| v.is_truthy()).unwrap_or(false);

    // Cells x genes, as a column per gene: every test wants one gene across all
    // cells, and both input layouts store cells first.
    let (n_cells, n_genes, columns) = expression_columns(&args[0], "find_all_markers")?;

    let labels: Vec<String> = match &args[1] {
        Value::List(items) => items.iter().map(|v| format!("{v}")).collect(),
        _ => {
            return Err(BioLangError::type_error(
                "find_all_markers() requires a List of cluster labels, one per cell",
                None,
            ))
        }
    };
    if labels.len() != n_cells {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "find_all_markers(): {} cluster labels for {n_cells} cells",
                labels.len()
            ),
            None,
        ));
    }

    let gene_names: Vec<String> = match opts.get("genes") {
        Some(Value::List(items)) => items.iter().map(|v| format!("{v}")).collect(),
        _ => Vec::new(),
    };

    // First-seen order, so the table does not reshuffle between runs.
    let mut cluster_order: Vec<String> = Vec::new();
    let mut members: HashMap<String, Vec<usize>> = HashMap::new();
    for (cell, label) in labels.iter().enumerate() {
        if !members.contains_key(label) {
            cluster_order.push(label.clone());
        }
        members.entry(label.clone()).or_default().push(cell);
    }

    // Seurat's FindAllMarkers returns only tests below this p-value. Without
    // it every gene that clears the fold-change and detection filters is
    // returned, which on a 3-cluster fixture was 156 rows against Seurat's 72 -
    // the same 72, plus 84 that Seurat had discarded as insignificant.
    let return_thresh = number("return_thresh", 0.01);
    // Added to the summed linear expression before averaging, per Seurat's
    // pseudocount.use.
    const PSEUDOCOUNT: f64 = 1.0;

    struct Marker {
        gene: String,
        cluster: String,
        p_value: f64,
        avg_log2fc: f64,
        pct_1: f64,
        pct_2: f64,
    }
    let mut found: Vec<Marker> = Vec::new();

    for cluster in &cluster_order {
        let inside = &members[cluster];
        if inside.is_empty() || inside.len() == n_cells {
            // Nothing to contrast against.
            continue;
        }
        let is_inside = {
            let mut flags = vec![false; n_cells];
            for &cell in inside {
                flags[cell] = true;
            }
            flags
        };
        let outside_count = n_cells - inside.len();

        for gene in 0..n_genes {
            let values = &columns[gene];
            let mut in_group: Vec<f64> = Vec::with_capacity(inside.len());
            let mut out_group: Vec<f64> = Vec::with_capacity(outside_count);
            let (mut detected_in, mut detected_out) = (0usize, 0usize);
            let (mut linear_in, mut linear_out) = (0.0f64, 0.0f64);
            for (cell, &value) in values.iter().enumerate() {
                if is_inside[cell] {
                    in_group.push(value);
                    if value > 0.0 {
                        detected_in += 1;
                    }
                    linear_in += value.exp_m1();
                } else {
                    out_group.push(value);
                    if value > 0.0 {
                        detected_out += 1;
                    }
                    linear_out += value.exp_m1();
                }
            }

            let pct_1 = detected_in as f64 / inside.len() as f64;
            let pct_2 = detected_out as f64 / outside_count as f64;
            if pct_1.max(pct_2) < min_pct {
                continue;
            }

            // Seurat's mean function for log1p data, which adds the
            // pseudocount to the *sum* before dividing by the cell count:
            //
            //   log(x = (rowSums(expm1(x)) + pseudocount.use) / NCOL(x), base = base)
            //
            // Adding it to the mean instead looks equivalent and is not: the
            // pseudocount is effectively 1/n rather than 1, so the two agree
            // only where the group means are large. Measured against Seurat on
            // a 3-cluster fixture, the mean form drifted by up to 0.19 log2
            // units, worst on the strongest markers - exactly the genes a
            // reader is looking at.
            let avg_log2fc = ((linear_in + PSEUDOCOUNT) / inside.len() as f64).log2()
                - ((linear_out + PSEUDOCOUNT) / outside_count as f64).log2();
            if avg_log2fc.abs() < logfc_threshold {
                continue;
            }
            if only_positive && avg_log2fc <= 0.0 {
                continue;
            }

            // Only now, on the few genes that survived, is the test worth running.
            // Seurat runs R's wilcox.test, which applies the continuity
            // correction. Matching it here is the difference between p-values
            // that agree to machine precision and ones that are consistently
            // about 1.4% out.
            let Ok(test) = bl_core::bio_core::stats_ops::mann_whitney_u(
                &in_group,
                &out_group,
                "two_sided",
                true,
            ) else {
                continue;
            };

            if !(test.p_value < return_thresh) {
                continue;
            }

            found.push(Marker {
                gene: gene_names
                    .get(gene)
                    .cloned()
                    .unwrap_or_else(|| format!("gene{gene}")),
                cluster: cluster.clone(),
                p_value: test.p_value,
                avg_log2fc,
                pct_1,
                pct_2,
            });
        }
    }

    // Seurat corrects with Bonferroni over every gene in the assay:
    //
    //   p.adjust(p = de.results$p_val, method = "bonferroni", n = nrow(object))
    //
    // The hypothesis count is the whole assay even when pre-filtering skipped
    // most of the tests, which stops filtering from making adjusted p-values
    // artificially optimistic. This previously applied a Benjamini-Hochberg
    // step-down instead - p * n / rank, held monotone - which is a different
    // and far less conservative correction. It is not what FindAllMarkers
    // reports, and on a 3-cluster fixture it disagreed with Seurat by up to
    // 0.30 on the adjusted p-value while the raw p-values agreed.
    let adjusted: Vec<f64> = found
        .iter()
        .map(|marker| (marker.p_value * n_genes as f64).min(1.0))
        .collect();

    // Most significant first within each cluster, clusters in first-seen order:
    // the reading order for naming cell types.
    let mut order: Vec<usize> = (0..found.len()).collect();
    let rank: HashMap<&String, usize> = cluster_order
        .iter()
        .enumerate()
        .map(|(i, name)| (name, i))
        .collect();
    order.sort_by(|&a, &b| {
        rank[&found[a].cluster]
            .cmp(&rank[&found[b].cluster])
            .then_with(|| {
                found[a]
                    .p_value
                    .partial_cmp(&found[b].p_value)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                found[b]
                    .avg_log2fc
                    .partial_cmp(&found[a].avg_log2fc)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let rows: Vec<Value> = order
        .into_iter()
        .map(|i| {
            let marker = &found[i];
            let mut record = HashMap::new();
            record.insert("gene".to_string(), Value::Str(marker.gene.clone()));
            record.insert("cluster".to_string(), Value::Str(marker.cluster.clone()));
            record.insert("p_value".to_string(), Value::Float(marker.p_value));
            record.insert(
                "p_adj".to_string(),
                Value::Float(adjusted.get(i).copied().unwrap_or(1.0)),
            );
            record.insert("avg_log2fc".to_string(), Value::Float(marker.avg_log2fc));
            record.insert("pct_1".to_string(), Value::Float(marker.pct_1));
            record.insert("pct_2".to_string(), Value::Float(marker.pct_2));
            Value::Record(record.into())
        })
        .collect();

    Ok(Value::List(rows.into()))
}

/// A cells x genes matrix as one vector per gene.
///
/// Both accepted layouts store cells first, and every per-gene test wants the
/// column; transposing once beats walking every row per gene.
pub(crate) fn expression_columns(
    value: &Value,
    who: &str,
) -> Result<(usize, usize, Vec<Vec<f64>>)> {
    match value {
        Value::SparseMatrix(matrix) => {
            let mut columns = vec![vec![0.0; matrix.nrow]; matrix.ncol];
            for row in 0..matrix.nrow {
                let (from, to) = (matrix.indptr[row], matrix.indptr[row + 1]);
                for position in from..to {
                    columns[matrix.indices[position]][row] = matrix.data[position];
                }
            }
            Ok((matrix.nrow, matrix.ncol, columns))
        }
        _ => {
            let rows = require_matrix(value, who)?;
            let n_cells = rows.len();
            let n_genes = rows.first().map(|row| row.len()).unwrap_or(0);
            let mut columns = vec![vec![0.0; n_cells]; n_genes];
            for (cell, row) in rows.iter().enumerate() {
                for (gene, &value) in row.iter().enumerate() {
                    if gene < n_genes {
                        columns[gene][cell] = value;
                    }
                }
            }
            Ok((n_cells, n_genes, columns))
        }
    }
}

// â”€â”€ cell_qc(matrix, gene_names?, mito_prefix="MT-") â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn builtin_cell_qc(args: Vec<Value>) -> Result<Value> {
    let gene_names: Option<Vec<String>> = if args.len() > 1 {
        match &args[1] {
            Value::List(names) => Some(
                names
                    .iter()
                    .map(|v| match v {
                        Value::Str(s) => s.clone(),
                        other => format!("{other}"),
                    })
                    .collect(),
            ),
            Value::Nil => None,
            _ => None,
        }
    } else {
        None
    };

    let mito_prefix = if args.len() > 2 {
        match &args[2] {
            Value::Str(s) => s.clone(),
            _ => "MT-".to_string(),
        }
    } else {
        "MT-".to_string()
    };

    let mito_indices: Vec<usize> = if let Some(ref names) = gene_names {
        names
            .iter()
            .enumerate()
            .filter_map(|(i, n)| {
                if n.starts_with(&mito_prefix) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    } else {
        vec![]
    };

    let columns = vec![
        "cell_idx".to_string(),
        "total_counts".to_string(),
        "n_genes".to_string(),
        "pct_mito".to_string(),
    ];

    let rows = match &args[0] {
        Value::SparseMatrix(matrix) => {
            let totals = matrix.row_sums();
            let detected = matrix.row_nnz();
            let is_mito: Vec<bool> = (0..matrix.ncol)
                .map(|column| mito_indices.contains(&column))
                .collect();
            (0..matrix.nrow)
                .map(|row| {
                    let mito_counts: f64 = (matrix.indptr[row]..matrix.indptr[row + 1])
                        .filter(|&position| is_mito[matrix.indices[position]])
                        .map(|position| matrix.data[position])
                        .sum();
                    let pct_mito = if totals[row] > 0.0 && gene_names.is_some() {
                        mito_counts / totals[row] * 100.0
                    } else {
                        0.0
                    };
                    vec![
                        Value::Int(row as i64),
                        Value::Float(totals[row]),
                        Value::Int(detected[row] as i64),
                        Value::Float(pct_mito),
                    ]
                })
                .collect()
        }
        _ => {
            let matrix = require_matrix(&args[0], "cell_qc")?;
            matrix
                .iter()
                .enumerate()
                .map(|(row_index, row)| {
                    let total: f64 = row.iter().sum();
                    let n_genes_detected = row.iter().filter(|&&value| value > 0.0).count();
                    let mito_counts: f64 = mito_indices
                        .iter()
                        .map(|&index| row.get(index).copied().unwrap_or(0.0))
                        .sum();
                    let pct_mito = if total > 0.0 && gene_names.is_some() {
                        mito_counts / total * 100.0
                    } else {
                        0.0
                    };
                    vec![
                        Value::Int(row_index as i64),
                        Value::Float(total),
                        Value::Int(n_genes_detected as i64),
                        Value::Float(pct_mito),
                    ]
                })
                .collect()
        }
    };

    Ok(Value::Table(Table::new(columns, rows)))
}

// â”€â”€ gene_qc(matrix, gene_names?) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn builtin_gene_qc(args: Vec<Value>) -> Result<Value> {
    let columns = vec![
        "gene_idx".to_string(),
        "n_cells".to_string(),
        "mean_expression".to_string(),
        "pct_dropout".to_string(),
    ];

    let (n_cells, n_genes, n_cells_expr, sums) = match &args[0] {
        Value::SparseMatrix(matrix) => (
            matrix.nrow,
            matrix.ncol,
            matrix.col_nnz(),
            matrix.col_sums(),
        ),
        _ => {
            let matrix = require_matrix(&args[0], "gene_qc")?;
            let n_genes = matrix.first().map(|row| row.len()).unwrap_or(0);
            let mut n_cells_expr = vec![0usize; n_genes];
            let mut sums = vec![0.0f64; n_genes];
            for row in &matrix {
                for (column, value) in row.iter().copied().enumerate() {
                    sums[column] += value;
                    if value > 0.0 {
                        n_cells_expr[column] += 1;
                    }
                }
            }
            (matrix.len(), n_genes, n_cells_expr, sums)
        }
    };

    if n_cells == 0 || n_genes == 0 {
        return Ok(Value::Table(Table::new(columns, vec![])));
    }

    let n_cells_float = n_cells as f64;

    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(n_genes);
    for j in 0..n_genes {
        let mean_expr = sums[j] / n_cells_float;
        let pct_dropout = (n_cells_float - n_cells_expr[j] as f64) / n_cells_float * 100.0;
        rows.push(vec![
            Value::Int(j as i64),
            Value::Int(n_cells_expr[j] as i64),
            Value::Float(mean_expr),
            Value::Float(pct_dropout),
        ]);
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

// â”€â”€ knn_graph(embeddings, k=15) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn neighbour_distance(a: &[f64], b: &[f64], metric: &str) -> f64 {
    if metric.eq_ignore_ascii_case("cosine") {
        let dot: f64 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let left_norm = a.iter().map(|value| value * value).sum::<f64>().sqrt();
        let right_norm = b.iter().map(|value| value * value).sum::<f64>().sqrt();
        if left_norm <= 1e-15 || right_norm <= 1e-15 {
            return if left_norm <= 1e-15 && right_norm <= 1e-15 {
                0.0
            } else {
                1.0
            };
        }
        return (1.0 - dot / (left_norm * right_norm)).clamp(0.0, 2.0);
    }
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

fn exact_neighbour_rows(embeddings: &[Vec<f64>], k: usize, metric: &str) -> Vec<Vec<(usize, f64)>> {
    (0..embeddings.len())
        .map(|i| {
            let mut distances: Vec<(usize, f64)> = (0..embeddings.len())
                .filter(|&j| j != i)
                .map(|j| {
                    (
                        j,
                        neighbour_distance(&embeddings[i], &embeddings[j], metric),
                    )
                })
                .collect();
            distances.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            distances.truncate(k);
            distances
        })
        .collect()
}

fn rp_tree_candidates(
    embeddings: &[Vec<f64>],
    indices: Vec<usize>,
    leaf_size: usize,
    state: &mut u64,
    candidates: &mut [Vec<usize>],
) {
    if indices.len() <= leaf_size {
        for &cell in &indices {
            candidates[cell].extend(indices.iter().copied().filter(|&other| other != cell));
        }
        return;
    }

    // Random-projection-tree split (Dasgupta & Freund): project on the line
    // through two sampled observations and split at the median. Repeated trees
    // give close observations several independent chances to share a leaf.
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let first_position = (*state as usize) % indices.len();
    let first = indices[first_position];
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let mut second_position = (*state as usize) % indices.len();
    if second_position == first_position {
        second_position = (second_position + 1) % indices.len();
    }
    let second = indices[second_position];

    let direction: Vec<f64> = embeddings[first]
        .iter()
        .zip(&embeddings[second])
        .map(|(a, b)| a - b)
        .collect();
    let usable = direction.iter().any(|value| value.abs() > 1e-12);
    let dimensions = embeddings.first().map(|row| row.len()).unwrap_or(0);
    let fallback_dimension = if dimensions == 0 {
        0
    } else {
        ((*state >> 32) as usize) % dimensions
    };
    let mut projected: Vec<(usize, f64)> = indices
        .into_iter()
        .map(|index| {
            let projection = if usable {
                embeddings[index]
                    .iter()
                    .zip(&direction)
                    .map(|(value, axis)| value * axis)
                    .sum()
            } else {
                embeddings[index]
                    .get(fallback_dimension)
                    .copied()
                    .unwrap_or(0.0)
            };
            (index, projection)
        })
        .collect();
    projected.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let right_projected = projected.split_off(projected.len() / 2);
    let left: Vec<usize> = projected.into_iter().map(|(index, _)| index).collect();
    let right: Vec<usize> = right_projected
        .into_iter()
        .map(|(index, _)| index)
        .collect();
    rp_tree_candidates(embeddings, left, leaf_size, state, candidates);
    rp_tree_candidates(embeddings, right, leaf_size, state, candidates);
}

/// Deterministic random-projection-forest search for HBC-scale datasets.
/// Candidate distances are evaluated exactly. Small inputs retain all-pairs
/// search so existing fixtures and small analyses do not change.
pub(crate) fn neighbour_rows_metric(
    embeddings: &[Vec<f64>],
    k: usize,
    metric: &str,
) -> Vec<Vec<(usize, f64)>> {
    const EXACT_LIMIT: usize = 4096;
    if embeddings.len() <= EXACT_LIMIT {
        return exact_neighbour_rows(embeddings, k, metric);
    }

    // Seurat 5.5.1 builds a 50-tree Spotify Annoy Euclidean index and queries
    // it with search.k=-1. Use that exact permissively licensed contract on
    // native builds; the browser and non-Euclidean metrics retain the portable
    // GPU/projection paths below.
    #[cfg(not(target_arch = "wasm32"))]
    if metric.eq_ignore_ascii_case("euclidean") {
        if let Ok(neighbours) = bl_seurat_compat::annoy_euclidean(embeddings, embeddings, k + 1, 50)
        {
            return neighbours
                .into_iter()
                .enumerate()
                .map(|(cell, row)| {
                    row.into_iter()
                        .filter(|(other, _)| *other != cell)
                        .take(k)
                        .collect()
                })
                .collect();
        }
    }

    // Portable GPU top-k search avoids the approximation gap for the graph
    // and UMAP range (normally 15-30 neighbors). Any unsupported shape or
    // driver failure transparently continues into the projection forest.
    if let Ok(Some(neighbours)) = crate::gpu::nearest_rows(embeddings, k, metric) {
        return neighbours;
    }

    let n = embeddings.len();
    // `k` can be much larger than the graph/UMAP range here. Seurat-style
    // anchor filtering asks for 200 cross-dataset neighbours and the mixed
    // search deliberately over-fetches candidates. The old unbounded formula
    // made k=1600 use 3,200-point leaves across 24 trees: 76,800 preallocated
    // indices per cell, or about 18.2 GB for the 29,629-cell HBC object before
    // duplicates. Large-k searches need broader leaves, not an unbounded
    // number of retained candidates per cell.
    // A larger ordinary-graph forest improved individual edge recall on HBC,
    // but made the resulting modularity partition less Seurat-like at the
    // fixed published parameters. Retain the measured 24-tree configuration;
    // the large-k anchor-filter path remains bounded for peak-memory safety.
    let leaf_size = (k.saturating_mul(2)).clamp(32, 256);
    let tree_count = if k > 256 { 8_usize } else { 24_usize };
    let mut candidates: Vec<Vec<usize>> = (0..n)
        .map(|_| Vec::with_capacity(leaf_size * tree_count))
        .collect();
    for tree in 0..tree_count {
        let mut state = 0x9e3779b97f4a7c15_u64 ^ (tree as u64 + 1).wrapping_mul(0x517cc1b727220a95);
        rp_tree_candidates(
            embeddings,
            (0..n).collect(),
            leaf_size,
            &mut state,
            &mut candidates,
        );
    }

    candidates
        .into_iter()
        .enumerate()
        .map(|(cell, mut row)| {
            row.sort_unstable();
            row.dedup();
            let mut distances: Vec<(usize, f64)> = row
                .into_iter()
                .filter(|&other| other != cell)
                .map(|other| {
                    (
                        other,
                        neighbour_distance(&embeddings[cell], &embeddings[other], metric),
                    )
                })
                .collect();
            distances.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            distances.truncate(k);
            distances
        })
        .collect()
}

fn neighbour_rows(embeddings: &[Vec<f64>], k: usize) -> Vec<Vec<(usize, f64)>> {
    neighbour_rows_metric(embeddings, k, "euclidean")
}

fn builtin_sc_umap(args: Vec<Value>) -> Result<Value> {
    let embeddings = require_matrix(&args[0], "sc_umap")?;
    let options: HashMap<String, Value> = match args.get(1) {
        Some(Value::Record(record)) => record.as_ref().clone(),
        Some(_) => {
            return Err(BioLangError::type_error(
                "sc_umap() options must be a Record",
                None,
            ));
        }
        None => HashMap::new(),
    };
    let integer = |name: &str, default: usize| {
        options
            .get(name)
            .and_then(|value| match value {
                Value::Int(number) => Some((*number).max(1) as usize),
                _ => None,
            })
            .unwrap_or(default)
    };
    let number = |name: &str, default: f64| options.get(name).and_then(to_f64).unwrap_or(default);
    let metric = options
        .get("metric")
        .and_then(|value| match value {
            Value::Str(text) => Some(text.as_str()),
            _ => None,
        })
        .unwrap_or("cosine");
    if !metric.eq_ignore_ascii_case("cosine") && !metric.eq_ignore_ascii_case("euclidean") {
        return Err(BioLangError::type_error(
            format!("sc_umap() metric must be 'cosine' or 'euclidean', got '{metric}'"),
            None,
        ));
    }
    let n_components = integer("n_components", 2);
    let n_neighbors = integer("n_neighbors", 30)
        .min(embeddings.len().saturating_sub(1))
        .max(1);
    let n_epochs = integer(
        "n_epochs",
        if embeddings.len() <= 10_000 { 500 } else { 200 },
    );
    let seed = integer("seed", 42) as u64;
    let negative_sample_rate = integer("negative_sample_rate", 5);
    let neighbours = neighbour_rows_metric(&embeddings, n_neighbors, metric);
    let result = bl_core::bio_core::dimreduce_ops::umap_from_knn(
        &neighbours,
        n_components,
        n_epochs,
        number("min_dist", 0.3),
        number("spread", 1.0),
        seed,
        negative_sample_rate,
    );
    Ok(matrix_to_value(result))
}

fn builtin_knn_graph(args: Vec<Value>) -> Result<Value> {
    let embeddings = require_matrix(&args[0], "knn_graph")?;
    let k = if args.len() > 1 {
        require_int(&args[1], "knn_graph")? as usize
    } else {
        15
    };

    let n = embeddings.len();
    if n == 0 {
        return Ok(Value::List((vec![]).into()));
    }
    let k_actual = k.min(n.saturating_sub(1));

    let mut edges: Vec<Value> = Vec::new();
    for (i, neighbours) in neighbour_rows(&embeddings, k_actual)
        .into_iter()
        .enumerate()
    {
        for (j, distance) in neighbours {
            let mut rec = HashMap::new();
            rec.insert("source".to_string(), Value::Int(i as i64));
            rec.insert("target".to_string(), Value::Int(j as i64));
            rec.insert("distance".to_string(), Value::Float(distance));
            edges.push(Value::Record((rec).into()));
        }
    }

    Ok(Value::List((edges).into()))
}

// ── snn_graph(embeddings, k=15, prune=1/15) ──────────────────────────

/// Shared-nearest-neighbour graph with Jaccard weights.
///
/// The graph community detection actually wants. A kNN edge says "these two
/// cells are close"; an SNN edge says "these two cells sit in the same
/// neighbourhood", which is a far stronger statement and the one that survives
/// high dimensions. As dimensionality grows the distances from a point to its
/// nearest and its furthest neighbour converge, so proximity alone stops
/// discriminating exactly where single-cell data lives. Neighbourhood overlap
/// does not degrade that way: two cells of the same type keep sharing
/// neighbours however many dimensions you measure them in.
///
/// For each pair, the weight is `|N(i) ∩ N(j)| / |N(i) ∪ N(j)|` over the
/// neighbour sets, each of which includes the cell itself — a cell is in its
/// own neighbourhood, and leaving it out makes two cells that are each other's
/// sole neighbour score zero. Edges at or below `prune` are dropped entirely,
/// which is where most of the graph goes: chance adjacencies score low and
/// removing them is what stops communities bleeding into each other.
///
/// `prune` defaults to 1/15, matching `FindNeighbors(prune.SNN = 1/15)`.
fn builtin_snn_graph(args: Vec<Value>) -> Result<Value> {
    let embeddings = require_matrix(&args[0], "snn_graph")?;
    let k = if args.len() > 1 {
        require_int(&args[1], "snn_graph")? as usize
    } else {
        15
    };
    let prune = if args.len() > 2 {
        to_f64(&args[2]).unwrap_or(1.0 / 15.0)
    } else {
        1.0 / 15.0
    };

    let n = embeddings.len();
    if n == 0 {
        return Ok(Value::List((vec![]).into()));
    }
    // Seurat's ranked-neighbour matrix contains the query cell itself among
    // its k columns. Keep k-1 other cells so the Jaccard denominator is k.
    let k_actual = k.saturating_sub(1).min(n.saturating_sub(1));

    // Neighbour sets, each including the cell itself.
    let neighbours: Vec<Vec<usize>> = neighbour_rows(&embeddings, k_actual)
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            let mut set: Vec<usize> = std::iter::once(i)
                .chain(row.into_iter().map(|(j, _)| j))
                .collect();
            set.sort_unstable();
            set
        })
        .collect();

    // Inverted index: which cells count `v` among their neighbours. Without it
    // finding the pairs that share anything means comparing every cell to every
    // other, which is the cost this whole structure exists to avoid.
    let mut listed_by: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (cell, set) in neighbours.iter().enumerate() {
        for &member in set {
            listed_by[member].push(cell);
        }
    }

    let mut edges: Vec<Value> = Vec::new();
    let mut shared: HashMap<usize, usize> = HashMap::new();
    for i in 0..n {
        shared.clear();
        for &member in &neighbours[i] {
            for &other in &listed_by[member] {
                // Each undirected pair once.
                if other > i {
                    *shared.entry(other).or_insert(0) += 1;
                }
            }
        }
        for (&j, &count) in shared.iter() {
            let union = neighbours[i].len() + neighbours[j].len() - count;
            if union == 0 {
                continue;
            }
            let jaccard = count as f64 / union as f64;
            // Strictly less than, which is what Seurat's ComputeSNN does:
            //   if (it.value() < prune) { it.valueRef() = 0; }
            // Its documentation says "less than or equal to" and its code says
            // less than. An edge sitting exactly on the threshold is kept.
            if jaccard < prune {
                continue;
            }
            let mut rec = HashMap::new();
            rec.insert("source".to_string(), Value::Int(i as i64));
            rec.insert("target".to_string(), Value::Int(j as i64));
            rec.insert("weight".to_string(), Value::Float(jaccard));
            edges.push(Value::Record((rec).into()));
        }
    }

    Ok(Value::List((edges).into()))
}

// â”€â”€ leiden_cluster(matrix, k, resolution=1.0) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Build a symmetric kNN graph on the embedding, then run real Leiden
// (local moving + refinement + aggregation, connected communities).

fn builtin_leiden_cluster(args: Vec<Value>) -> Result<Value> {
    let embeddings = require_matrix(&args[0], "leiden_cluster")?;
    let k = require_int(&args[1], "leiden_cluster")? as usize;
    let resolution = if args.len() > 2 {
        match &args[2] {
            Value::Float(f) => *f,
            Value::Int(n) => *n as f64,
            _ => 1.0,
        }
    } else {
        1.0
    };

    let n = embeddings.len();
    if n == 0 {
        return Ok(Value::List((vec![]).into()));
    }
    let edges = builtin_knn_graph(vec![matrix_to_value(embeddings), Value::Int(k as i64)])?;
    builtin_leiden_graph(vec![edges, Value::Int(n as i64), Value::Float(resolution)])
}

// â”€â”€ doublet_score(matrix, n_simulated=500) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Louvain on a kNN graph built from an embedding.
///
/// The counterpart to `leiden_cluster`, for reproducing analyses done with
/// Seurat, whose `FindClusters` defaults to `algorithm = 1` â€” Louvain â€” in
/// v5.5.1 as in every earlier version. See `louvain_sparse` in bio-core for why
/// the difference matters and why it is not "fixed" here.
///
/// Resolution defaults to 0.8, Seurat's default, rather than the 1.0 that
/// Leiden convention uses.
fn builtin_louvain_cluster(args: Vec<Value>) -> Result<Value> {
    let embeddings = require_matrix(&args[0], "louvain_cluster")?;
    let k = require_int(&args[1], "louvain_cluster")? as usize;
    let resolution = if args.len() > 2 {
        to_f64(&args[2]).unwrap_or(0.8)
    } else {
        0.8
    };

    let n = embeddings.len();
    if n == 0 {
        return Ok(Value::List((vec![]).into()));
    }
    let edges = builtin_knn_graph(vec![matrix_to_value(embeddings), Value::Int(k as i64)])?;
    let adjacency = leiden_adjacency(&edges, n, "louvain_cluster")?;
    // Ten restarts, which is `FindClusters(n.start = 10)`. A single greedy pass
    // lands in whichever local optimum the node order leads to, and running it
    // longer does not help; running it again from a different order does.
    const N_START: usize = 10;
    let labels =
        bl_core::bio_core::cluster_ops::louvain_sparse_restarts(&adjacency, resolution, N_START, 0);
    Ok(Value::List(
        labels
            .into_iter()
            .map(|label| Value::Int(label as i64))
            .collect::<Vec<_>>()
            .into(),
    ))
}

/// Turn an edge list into a sorted sparse adjacency.
///
/// Shared by `leiden_graph` and `louvain_cluster` so the two cannot drift:
/// they differ only in which community-detection call consumes the result.
fn leiden_adjacency(edges: &Value, n_nodes: usize, who: &str) -> Result<Vec<Vec<(usize, f64)>>> {
    let edges = match edges {
        Value::List(edges) => edges,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "{who}() edges must be List<Record>, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    let mut undirected: HashMap<(usize, usize), f64> = HashMap::new();
    for edge in edges.iter() {
        let record = match edge {
            Value::Record(record) => record,
            other => {
                return Err(BioLangError::type_error(
                    format!(
                        "leiden_graph() edges must be Records, got {}",
                        other.type_of()
                    ),
                    None,
                ))
            }
        };
        let source = record
            .get("source")
            .and_then(|value| match value {
                Value::Int(index) if *index >= 0 => Some(*index as usize),
                _ => None,
            })
            .ok_or_else(|| {
                BioLangError::type_error(
                    "leiden_graph() each edge requires a non-negative Int source",
                    None,
                )
            })?;
        let target = record
            .get("target")
            .and_then(|value| match value {
                Value::Int(index) if *index >= 0 => Some(*index as usize),
                _ => None,
            })
            .ok_or_else(|| {
                BioLangError::type_error(
                    "leiden_graph() each edge requires a non-negative Int target",
                    None,
                )
            })?;
        if source >= n_nodes || target >= n_nodes {
            return Err(BioLangError::runtime(
                ErrorKind::IndexOutOfBounds,
                format!("leiden_graph() edge ({source}, {target}) is outside {n_nodes} nodes"),
                None,
            ));
        }
        if source == target {
            continue;
        }
        let weight = if let Some(weight) = record.get("weight").and_then(to_f64) {
            weight
        } else {
            let distance = record.get("distance").and_then(to_f64).unwrap_or(0.0);
            1.0 / (1.0 + distance.max(0.0))
        };
        if !weight.is_finite() || weight <= 0.0 {
            continue;
        }
        let pair = if source < target {
            (source, target)
        } else {
            (target, source)
        };
        undirected
            .entry(pair)
            .and_modify(|stored| *stored = stored.max(weight))
            .or_insert(weight);
    }

    let mut adjacency = vec![Vec::new(); n_nodes];
    for ((source, target), weight) in undirected {
        adjacency[source].push((target, weight));
        adjacency[target].push((source, weight));
    }
    for neighbors in &mut adjacency {
        neighbors.sort_by_key(|(neighbor, _)| *neighbor);
    }
    Ok(adjacency)
}

/// Louvain on a graph that was built elsewhere.
///
/// The counterpart to `leiden_graph`, and the one to reach for when the graph
/// is an SNN rather than a plain kNN — `louvain_cluster` builds its own kNN
/// internally, which throws away the neighbourhood-overlap weighting that makes
/// the graph worth having.
fn builtin_louvain_graph(args: Vec<Value>) -> Result<Value> {
    let n_nodes = require_int(&args[1], "louvain_graph")?;
    if n_nodes < 0 {
        return Err(BioLangError::type_error(
            "louvain_graph() n_nodes must be non-negative",
            None,
        ));
    }
    let resolution = to_f64(&args[2]).ok_or_else(|| {
        BioLangError::type_error("louvain_graph() resolution must be numeric", None)
    })?;
    let n_start = match args.get(3) {
        Some(value) => require_int(value, "louvain_graph")?.max(1) as usize,
        None => 10,
    };
    let adjacency = leiden_adjacency(&args[0], n_nodes as usize, "louvain_graph")?;
    let labels = bl_core::bio_core::cluster_ops::louvain_seurat_restarts(
        &adjacency, resolution, n_start, 10, 0,
    );
    Ok(Value::List(
        labels
            .into_iter()
            .map(|label| Value::Int(label as i64))
            .collect::<Vec<_>>()
            .into(),
    ))
}

fn builtin_leiden_graph(args: Vec<Value>) -> Result<Value> {
    let edges = match &args[0] {
        Value::List(edges) => edges,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "leiden_graph() edges must be List<Record>, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    let n_nodes = require_int(&args[1], "leiden_graph")?;
    if n_nodes < 0 {
        return Err(BioLangError::type_error(
            "leiden_graph() n_nodes must be non-negative",
            None,
        ));
    }
    let resolution = to_f64(&args[2]).ok_or_else(|| {
        BioLangError::type_error("leiden_graph() resolution must be numeric", None)
    })?;
    let n_nodes = n_nodes as usize;

    let mut undirected: HashMap<(usize, usize), f64> = HashMap::new();
    for edge in edges.iter() {
        let record = match edge {
            Value::Record(record) => record,
            other => {
                return Err(BioLangError::type_error(
                    format!(
                        "leiden_graph() edges must be Records, got {}",
                        other.type_of()
                    ),
                    None,
                ))
            }
        };
        let source = record
            .get("source")
            .and_then(|value| match value {
                Value::Int(index) if *index >= 0 => Some(*index as usize),
                _ => None,
            })
            .ok_or_else(|| {
                BioLangError::type_error(
                    "leiden_graph() each edge requires a non-negative Int source",
                    None,
                )
            })?;
        let target = record
            .get("target")
            .and_then(|value| match value {
                Value::Int(index) if *index >= 0 => Some(*index as usize),
                _ => None,
            })
            .ok_or_else(|| {
                BioLangError::type_error(
                    "leiden_graph() each edge requires a non-negative Int target",
                    None,
                )
            })?;
        if source >= n_nodes || target >= n_nodes {
            return Err(BioLangError::runtime(
                ErrorKind::IndexOutOfBounds,
                format!("leiden_graph() edge ({source}, {target}) is outside {n_nodes} nodes"),
                None,
            ));
        }
        if source == target {
            continue;
        }
        let weight = if let Some(weight) = record.get("weight").and_then(to_f64) {
            weight
        } else {
            let distance = record.get("distance").and_then(to_f64).unwrap_or(0.0);
            1.0 / (1.0 + distance.max(0.0))
        };
        if !weight.is_finite() || weight <= 0.0 {
            continue;
        }
        let pair = if source < target {
            (source, target)
        } else {
            (target, source)
        };
        undirected
            .entry(pair)
            .and_modify(|stored| *stored = stored.max(weight))
            .or_insert(weight);
    }

    let mut adjacency = vec![Vec::new(); n_nodes];
    for ((source, target), weight) in undirected {
        adjacency[source].push((target, weight));
        adjacency[target].push((source, weight));
    }
    for neighbors in &mut adjacency {
        neighbors.sort_by_key(|(neighbor, _)| *neighbor);
    }
    let labels = bl_core::bio_core::cluster_ops::leiden_sparse(&adjacency, resolution);
    Ok(Value::List(
        labels
            .into_iter()
            .map(|label| Value::Int(label as i64))
            .collect::<Vec<_>>()
            .into(),
    ))
}

fn builtin_doublet_score(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "doublet_score")?;
    let n_simulated = if args.len() > 1 {
        require_int(&args[1], "doublet_score")? as usize
    } else {
        500
    };

    let n_cells = mat.len();
    if n_cells < 2 {
        return Ok(Value::List(
            mat.iter()
                .map(|_| Value::Float(0.0))
                .collect::<Vec<_>>()
                .into(),
        ));
    }
    let n_genes = mat[0].len();

    // Generate artificial doublets via deterministic LCG pairs
    let lcg_next = |s: u64| -> u64 {
        s.wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407)
    };
    let mut lcg_state: u64 = 12345;
    let mut artificial: Vec<Vec<f64>> = Vec::with_capacity(n_simulated);
    for _ in 0..n_simulated {
        lcg_state = lcg_next(lcg_state);
        let i = (lcg_state as usize) % n_cells;
        lcg_state = lcg_next(lcg_state);
        let j = (lcg_state as usize) % n_cells;
        let summed: Vec<f64> = (0..n_genes)
            .map(|g| mat[i].get(g).copied().unwrap_or(0.0) + mat[j].get(g).copied().unwrap_or(0.0))
            .collect();
        artificial.push(summed);
    }

    // Pearson correlation between two equal-length vectors
    let pearson = |a: &[f64], b: &[f64]| -> f64 {
        let n = a.len() as f64;
        if n == 0.0 {
            return 0.0;
        }
        let mean_a = a.iter().sum::<f64>() / n;
        let mean_b = b.iter().sum::<f64>() / n;
        let num: f64 = a
            .iter()
            .zip(b.iter())
            .map(|(&x, &y)| (x - mean_a) * (y - mean_b))
            .sum();
        let da: f64 = a
            .iter()
            .map(|&x| (x - mean_a) * (x - mean_a))
            .sum::<f64>()
            .sqrt();
        let db: f64 = b
            .iter()
            .map(|&y| (y - mean_b) * (y - mean_b))
            .sum::<f64>()
            .sqrt();
        if da == 0.0 || db == 0.0 {
            0.0
        } else {
            num / (da * db)
        }
    };

    // For each real cell find the nearest artificial doublet (max correlation)
    let mut raw_scores: Vec<f64> = mat
        .iter()
        .map(|cell| {
            artificial
                .iter()
                .map(|art| pearson(cell, art))
                .fold(f64::NEG_INFINITY, f64::max)
                .max(0.0)
        })
        .collect();

    // Normalise to [0, 1]
    let min_s = raw_scores.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_s = raw_scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max_s - min_s;
    if range > 0.0 {
        for s in &mut raw_scores {
            *s = (*s - min_s) / range;
        }
    }

    Ok(Value::List(
        raw_scores
            .into_iter()
            .map(Value::Float)
            .collect::<Vec<_>>()
            .into(),
    ))
}

// â”€â”€ Section 6 extensions: Seurat-compatible single-cell ops â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn read_lines_from_path(path: &std::path::Path) -> Result<Vec<String>> {
    let file = std::fs::File::open(path).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("read_10x(): cannot open {}: {e}", path.display()),
            None,
        )
    })?;
    let is_gz = path.extension().is_some_and(|e| e == "gz");
    if is_gz {
        #[cfg(feature = "native")]
        {
            use flate2::read::GzDecoder;
            BufReader::new(GzDecoder::new(file))
                .lines()
                .map(|l| {
                    l.map_err(|e| {
                        BioLangError::runtime(
                            ErrorKind::IOError,
                            format!("read_10x(): gz read error: {e}"),
                            None,
                        )
                    })
                })
                .collect()
        }
        #[cfg(not(feature = "native"))]
        Err(BioLangError::runtime(
            ErrorKind::IOError,
            "read_10x(): .gz support requires native feature",
            None,
        ))
    } else {
        BufReader::new(file)
            .lines()
            .map(|l| {
                l.map_err(|e| {
                    BioLangError::runtime(
                        ErrorKind::IOError,
                        format!("read_10x(): read error: {e}"),
                        None,
                    )
                })
            })
            .collect()
    }
}

/// Open a text file for reading, transparently decompressing `.gz`.
fn open_text_reader(path: &std::path::Path) -> Result<Box<dyn BufRead>> {
    let file = std::fs::File::open(path).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("read_10x(): cannot open {}: {e}", path.display()),
            None,
        )
    })?;
    if path.extension().is_some_and(|e| e == "gz") {
        #[cfg(feature = "native")]
        {
            return Ok(Box::new(BufReader::new(flate2::read::GzDecoder::new(file))));
        }
        #[cfg(not(feature = "native"))]
        {
            return Err(BioLangError::runtime(
                ErrorKind::IOError,
                "read_10x(): .gz support requires native feature",
                None,
            ));
        }
    }
    Ok(Box::new(BufReader::new(file)))
}

/// One entry of a Matrix Market file: `(row, column, value)`, still 1-based.
///
/// `u32` for the indices. Matrix Market's own dimensions are unbounded, so the
/// parser checks them, but ten million entries at 24 bytes is 240 MB against
/// 160 MB at 16 — and this vector exists only to be transposed into CSR.
type MtxEntry = (u32, u32, f64);

/// Stream a Matrix Market file into a compact entry list.
///
/// The route this replaces collected every line into a `Vec<String>` first. On
/// a raw 10x matrix that is ten million separately heap-allocated strings —
/// roughly half a gigabyte and ten million allocator round trips — held whole
/// while a second, equally large entry list was built beside it. Nothing needs
/// two lines at once, and the header states the entry count, so the one buffer
/// that is genuinely required can be sized exactly instead of doubled into
/// place.
fn parse_mtx_streaming(path: &std::path::Path) -> Result<(usize, usize, Vec<MtxEntry>)> {
    let mut reader = open_text_reader(path)?;
    let mut line = String::new();
    let read = |reader: &mut Box<dyn BufRead>, line: &mut String| -> Result<usize> {
        line.clear();
        reader.read_line(line).map_err(|e| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("read_10x(): read error: {e}"),
                None,
            )
        })
    };

    // The banner and any comments come first; the first bare line is the size
    // line, `rows columns entries`.
    let (n_rows, n_cols, declared) = loop {
        if read(&mut reader, &mut line)? == 0 {
            return Err(BioLangError::runtime(
                ErrorKind::IOError,
                "read_10x(): malformed matrix.mtx (no size line)",
                None,
            ));
        }
        let text = line.trim();
        if text.is_empty() || text.starts_with('%') {
            continue;
        }
        let mut parts = text.split_ascii_whitespace();
        let rows = parts
            .next()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        let columns = parts
            .next()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        let declared = parts
            .next()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        break (rows, columns, declared);
    };
    if n_rows > u32::MAX as usize || n_cols > u32::MAX as usize {
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!("read_10x(): matrix is {n_rows} x {n_cols}, beyond this reader's range"),
            None,
        ));
    }

    let mut entries: Vec<MtxEntry> = Vec::with_capacity(declared);
    loop {
        if read(&mut reader, &mut line)? == 0 {
            break;
        }
        let text = line.trim();
        if text.is_empty() || text.starts_with('%') {
            continue;
        }
        let mut parts = text.split_ascii_whitespace();
        let (Some(row), Some(column), Some(value)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        entries.push((
            row.parse().unwrap_or(0),
            column.parse().unwrap_or(0),
            value.parse().unwrap_or(0.0),
        ));
    }
    Ok((n_rows, n_cols, entries))
}

/// Transpose gene-major Matrix Market entries into a cell-major CSR matrix.
///
/// A counting sort, not a comparison sort. `SparseMatrix::from_triplets` takes
/// three parallel slices, rebuilds them into a third copy of every entry, and
/// sorts that by `(row, column)` — three ten-million-element allocations and an
/// n log n sort where the row is already a small dense integer. Counting gives
/// the same layout in two linear passes, and because Matrix Market promises no
/// ordering while this type's `get` binary-searches on it, each row is checked
/// and only sorted if the file did not already arrive in order — which, from
/// CellRanger, it does.
fn csr_from_mtx_entries(entries: &[MtxEntry], n_cells: usize, n_genes: usize) -> SparseMatrix {
    let keep = |&(gene_1, cell_1, value): &MtxEntry| -> Option<(usize, usize, f64)> {
        let gene = (gene_1 as usize).checked_sub(1)?;
        let cell = (cell_1 as usize).checked_sub(1)?;
        (gene < n_genes && cell < n_cells && value != 0.0).then_some((gene, cell, value))
    };

    let mut indptr = vec![0usize; n_cells + 1];
    for entry in entries {
        if let Some((_, cell, _)) = keep(entry) {
            indptr[cell + 1] += 1;
        }
    }
    for cell in 0..n_cells {
        indptr[cell + 1] += indptr[cell];
    }

    let nnz = indptr[n_cells];
    let mut indices = vec![0usize; nnz];
    let mut data = vec![0.0f64; nnz];
    let mut cursor = indptr.clone();
    for entry in entries {
        if let Some((gene, cell, value)) = keep(entry) {
            let position = cursor[cell];
            indices[position] = gene;
            data[position] = value;
            cursor[cell] = position + 1;
        }
    }
    drop(cursor);

    for cell in 0..n_cells {
        let (from, to) = (indptr[cell], indptr[cell + 1]);
        if indices[from..to].windows(2).all(|pair| pair[0] < pair[1]) {
            continue;
        }
        let mut row: Vec<(usize, f64)> = indices[from..to]
            .iter()
            .copied()
            .zip(data[from..to].iter().copied())
            .collect();
        row.sort_by_key(|&(gene, _)| gene);
        for (offset, (gene, value)) in row.into_iter().enumerate() {
            indices[from + offset] = gene;
            data[from + offset] = value;
        }
    }

    SparseMatrix {
        indptr,
        indices,
        data,
        nrow: n_cells,
        ncol: n_genes,
        row_names: None,
        col_names: None,
    }
}

// â”€â”€ read_10x(path) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn builtin_read_10x(args: Vec<Value>) -> Result<Value> {
    read_10x_impl(args, false)
}

fn builtin_read_10x_sparse(args: Vec<Value>) -> Result<Value> {
    read_10x_impl(args, true)
}

fn read_10x_impl(args: Vec<Value>, sparse: bool) -> Result<Value> {
    use std::path::Path;
    let dir_str = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => {
            return Err(BioLangError::type_error(
                "read_10x() requires a directory path string",
                None,
            ))
        }
    };
    let dir = Path::new(&dir_str);

    let find_file = |names: &[&str]| -> Option<std::path::PathBuf> {
        names.iter().find_map(|n| {
            let p = dir.join(n);
            if p.exists() {
                Some(p)
            } else {
                None
            }
        })
    };

    let barcodes_path = find_file(&["barcodes.tsv.gz", "barcodes.tsv"]).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("read_10x(): barcodes.tsv not found in {dir_str}"),
            None,
        )
    })?;
    let features_path = find_file(&[
        "features.tsv.gz",
        "features.tsv",
        "genes.tsv.gz",
        "genes.tsv",
        "peaks.bed.gz",
        "peaks.bed",
    ])
    .ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("read_10x(): features.tsv or peaks.bed not found in {dir_str}"),
            None,
        )
    })?;
    let matrix_path = find_file(&["matrix.mtx.gz", "matrix.mtx"]).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("read_10x(): matrix.mtx not found in {dir_str}"),
            None,
        )
    })?;

    let mut barcodes: Vec<String> = read_lines_from_path(&barcodes_path)?
        .into_iter()
        .filter(|l| !l.is_empty())
        .collect();

    let is_peak_bed = features_path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("peaks.bed"));

    // features.tsv is `gene_id \t gene_symbol \t feature_type`. Default to the
    // symbol, matching Seurat's Read10X(gene.column = 2) and scanpy's
    // read_10x_mtx(var_names="gene_symbols"). Downstream steps match on symbols
    // â€” the "MT-" prefix for percent-mito, marker panels, DE output â€” so
    // reading the Ensembl ID here makes percent-mito silently zero.
    // Pass gene_column = 1 to get Ensembl IDs instead.
    let gene_column = if is_peak_bed {
        1
    } else if args.len() > 1 {
        match &args[1] {
            Value::Int(n) if *n == 1 || *n == 2 => *n as usize,
            Value::Int(n) => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("read_10x(): gene_column must be 1 (ID) or 2 (symbol), got {n}"),
                    None,
                ))
            }
            _ => {
                return Err(BioLangError::type_error(
                    "read_10x(): gene_column must be Int",
                    None,
                ))
            }
        }
    } else {
        2
    };

    let feature_lines = read_lines_from_path(&features_path)?;
    let peak_ranges: Option<Vec<(String, i64, i64)>> = is_peak_bed.then(|| {
        feature_lines
            .iter()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .filter_map(|line| {
                let mut columns = line.split('\t');
                let chrom = columns.next()?.trim().to_string();
                let start = columns.next()?.trim().parse::<i64>().ok()?;
                let end = columns.next()?.trim().parse::<i64>().ok()?;
                Some((chrom, start, end))
            })
            .collect()
    });
    let mut genes: Vec<String> = if let Some(ranges) = &peak_ranges {
        ranges
            .iter()
            .map(|(chrom, start, end)| format!("{chrom}:{start}-{end}"))
            .collect()
    } else {
        feature_lines
            .into_iter()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let columns: Vec<&str> = line.split('\t').collect();
                let pick = columns
                    .get(gene_column - 1)
                    .map(|value| value.trim())
                    .unwrap_or("");
                if pick.is_empty() {
                    columns
                        .first()
                        .map(|value| value.trim())
                        .unwrap_or("")
                        .to_string()
                } else {
                    pick.to_string()
                }
            })
            .collect()
    };

    let (n_genes_mtx, n_cells_mtx, entries) = parse_mtx_streaming(&matrix_path)?;

    if is_peak_bed && genes.len() != n_genes_mtx {
        return Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!(
                "read_10x(): peaks.bed has {} rows but matrix.mtx has {n_genes_mtx} features",
                genes.len()
            ),
            None,
        ));
    }

    let n_g = genes.len().max(n_genes_mtx);
    let n_c = barcodes.len().max(n_cells_mtx);

    while genes.len() < n_g {
        genes.push(format!("gene_{}", genes.len() + 1));
    }
    while barcodes.len() < n_c {
        barcodes.push(format!("cell_{}", barcodes.len() + 1));
    }

    let matrix_value = if sparse {
        let mut matrix = csr_from_mtx_entries(&entries, n_c, n_g);
        drop(entries);
        matrix.row_names = Some(barcodes.clone());
        matrix.col_names = Some(genes.clone());
        Value::SparseMatrix(std::sync::Arc::new(matrix))
    } else {
        let mut matrix = vec![vec![0.0f64; n_g]; n_c];
        for (gene_1, cell_1, value) in entries {
            let gene = (gene_1 as usize).saturating_sub(1);
            let cell = (cell_1 as usize).saturating_sub(1);
            if gene < n_g && cell < n_c {
                matrix[cell][gene] += value;
            }
        }
        matrix_to_value(matrix)
    };

    let obs = Table::new(
        vec!["barcode".to_string()],
        barcodes
            .iter()
            .cloned()
            .map(|barcode| vec![Value::Str(barcode)])
            .collect(),
    );
    let var = if let Some(ranges) = &peak_ranges {
        Table::new(
            vec![
                "gene".to_string(),
                "chrom".to_string(),
                "start".to_string(),
                "end".to_string(),
            ],
            ranges
                .iter()
                .zip(genes.iter())
                .map(|((chrom, start, end), name)| {
                    vec![
                        Value::Str(name.clone()),
                        Value::Str(chrom.clone()),
                        Value::Int(*start),
                        Value::Int(*end),
                    ]
                })
                .collect(),
        )
    } else {
        Table::new(
            vec!["gene".to_string()],
            genes
                .iter()
                .cloned()
                .map(|gene| vec![Value::Str(gene)])
                .collect(),
        )
    };
    let mut layers = HashMap::new();
    layers.insert("counts".to_string(), matrix_value.clone());

    let mut rec = HashMap::new();
    rec.insert("matrix".to_string(), matrix_value);
    rec.insert(
        "genes".to_string(),
        Value::List(genes.into_iter().map(Value::Str).collect::<Vec<_>>().into()),
    );
    rec.insert(
        "barcodes".to_string(),
        Value::List(
            barcodes
                .into_iter()
                .map(Value::Str)
                .collect::<Vec<_>>()
                .into(),
        ),
    );
    rec.insert("obs".to_string(), Value::Table(obs));
    rec.insert("var".to_string(), Value::Table(var));
    if let Some(ranges) = peak_ranges {
        rec.insert(
            "peaks".to_string(),
            Value::Table(Table::new(
                vec!["chrom".into(), "start".into(), "end".into()],
                ranges
                    .into_iter()
                    .map(|(chrom, start, end)| {
                        vec![Value::Str(chrom), Value::Int(start), Value::Int(end)]
                    })
                    .collect(),
            )),
        );
        rec.insert("feature_type".to_string(), Value::Str("peaks".into()));
    }
    rec.insert("layers".to_string(), Value::Record(layers.into()));
    rec.insert("n_cells".to_string(), Value::Int(n_c as i64));
    rec.insert("n_genes".to_string(), Value::Int(n_g as i64));
    rec.insert("is_sparse".to_string(), Value::Bool(sparse));
    Ok(Value::Record((rec).into()))
}

fn gene_indices_from_value(val: &Value, func: &str) -> Result<Vec<usize>> {
    match val {
        Value::List(list) => list
            .iter()
            .map(|v| match v {
                Value::Int(n) => Ok(*n as usize),
                _ => Err(BioLangError::type_error(
                    format!("{func}() gene indices must be Int"),
                    None,
                )),
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("{func}() gene indices must be List<Int>"),
            None,
        )),
    }
}

// â”€â”€ cell_cycle_score(matrix, s_gene_indices, g2m_gene_indices) â”€â”€â”€â”€â”€â”€â”€â”€

fn builtin_cell_cycle_score(args: Vec<Value>) -> Result<Value> {
    let matrix = singlecell_matrix(&args[0], "cell_cycle_score")?;
    let s_idx = gene_indices_from_value(&args[1], "cell_cycle_score")?;
    let g2m_idx = gene_indices_from_value(&args[2], "cell_cycle_score")?;
    let (n_cells, n_genes) = matrix.dimensions();
    if s_idx.iter().chain(&g2m_idx).any(|&index| index >= n_genes) {
        return Err(BioLangError::runtime(
            ErrorKind::IndexOutOfBounds,
            "cell_cycle_score() gene index is outside the matrix",
            None,
        ));
    }

    let scores: Vec<Value> = (0..n_cells)
        .map(|cell| {
            let s_score = if s_idx.is_empty() {
                0.0
            } else {
                s_idx
                    .iter()
                    .map(|&gene| matrix.value_at(cell, gene))
                    .sum::<f64>()
                    / s_idx.len() as f64
            };
            let g2m_score = if g2m_idx.is_empty() {
                0.0
            } else {
                g2m_idx
                    .iter()
                    .map(|&gene| matrix.value_at(cell, gene))
                    .sum::<f64>()
                    / g2m_idx.len() as f64
            };
            let phase = if s_score > g2m_score && s_score > 0.1 {
                "S"
            } else if g2m_score >= s_score && g2m_score > 0.1 {
                "G2M"
            } else {
                "G1"
            };
            let mut rec = HashMap::new();
            rec.insert("s_score".to_string(), Value::Float(s_score));
            rec.insert("g2m_score".to_string(), Value::Float(g2m_score));
            rec.insert("phase".to_string(), Value::Str(phase.to_string()));
            Value::Record((rec).into())
        })
        .collect();

    Ok(Value::List((scores).into()))
}

// â”€â”€ module_score(matrix, gene_indices) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn builtin_module_score(args: Vec<Value>) -> Result<Value> {
    let matrix = singlecell_matrix(&args[0], "module_score")?;
    let indices = gene_indices_from_value(&args[1], "module_score")?;
    let (n_cells, n_genes) = matrix.dimensions();
    if indices.iter().any(|&index| index >= n_genes) {
        return Err(BioLangError::runtime(
            ErrorKind::IndexOutOfBounds,
            "module_score() gene index is outside the matrix",
            None,
        ));
    }

    let scores: Vec<Value> = (0..n_cells)
        .map(|cell| {
            let score = if indices.is_empty() {
                0.0
            } else {
                indices
                    .iter()
                    .map(|&gene| matrix.value_at(cell, gene))
                    .sum::<f64>()
                    / indices.len() as f64
            };
            Value::Float(score)
        })
        .collect();

    Ok(Value::List((scores).into()))
}

// â”€â”€ sc_sctransform(matrix, n_variable_features?) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Computes Pearson residuals under a simplified negative-binomial model.

/// Pearson residuals under a fixed-overdispersion negative binomial.
///
/// The residual is non-zero wherever the count is zero â€” `(0 - mu)/sqrt(var)` â€”
/// so the result is genuinely dense and there is no sparse form to return. What
/// it must not do is pay for that density three times over. It used to:
/// `to_dense()` on the input, a second full `Vec<Vec<f64>>` for the residuals,
/// and then `matrix_to_value` boxing every element into a `Value::Float` inside
/// nested `Vec<Value>`. On the 29,629 x 16,681 matrix this book's integration
/// chapter builds, that is roughly 4 GB + 4 GB + 12 GB, and it died with an
/// allocation failure on a 32 GB machine.
///
/// Now: read the sparse input in place, write one flat `Vec<f64>`, and return a
/// `Matrix`. Peak is the density the output genuinely needs.
///
/// That is still 4 GB for every gene, and the pipeline discards most of it at
/// the next step â€” variable-gene selection keeps a couple of thousand columns
/// and drops the rest. The optional second argument caps the output the way
/// `SCTransform(variable.features.n = ...)` does: rank genes by the variance of
/// their residuals, then materialise only the top `n`. Ranking needs two
/// accumulators per gene rather than a stored column, so the cap applies to the
/// peak and not just the return value â€” 16,681 genes down to 3,000 is 4 GB down
/// to 711 MB. The residual values themselves are unchanged; only which columns
/// survive.
///
/// Every call returns a record containing the matrix and its original-axis gene
/// indices, because zero-count genes are absent even when no feature cap is
/// requested and a narrower matrix without that mapping is unsafe.
fn builtin_sc_sctransform(args: Vec<Value>) -> Result<Value> {
    let (n_cells, n_genes, get_row): (usize, usize, Box<dyn Fn(usize, &mut Vec<f64>)>) =
        match &args[0] {
            Value::SparseMatrix(sm) => {
                let sm = sm.clone();
                let (rows, cols) = (sm.nrow, sm.ncol);
                (
                    rows,
                    cols,
                    Box::new(move |i: usize, out: &mut Vec<f64>| {
                        out.clear();
                        out.resize(cols, 0.0);
                        for pos in sm.indptr[i]..sm.indptr[i + 1] {
                            out[sm.indices[pos]] = sm.data[pos];
                        }
                    }),
                )
            }
            other => {
                let dense = require_matrix(other, "sc_sctransform")?;
                let rows = dense.len();
                let cols = if rows > 0 { dense[0].len() } else { 0 };
                (
                    rows,
                    cols,
                    Box::new(move |i: usize, out: &mut Vec<f64>| {
                        out.clear();
                        out.extend_from_slice(&dense[i]);
                    }),
                )
            }
        };

    // `n_variable_features`. Absent, non-positive, or wider than the input all
    // mean "every gene", which is also the shape the one-argument form returns.
    let want = match args.get(1) {
        None => None,
        Some(v) => match require_int(v, "sc_sctransform")? {
            n if n > 0 && (n as usize) < n_genes => Some(n as usize),
            _ => None,
        },
    };

    let latent_covariates = match args.get(2) {
        None | Some(Value::Nil) => Vec::new(),
        Some(Value::List(values)) if values.is_empty() => Vec::new(),
        Some(Value::List(values)) if values.iter().all(|value| value.as_float().is_some()) => {
            vec![values
                .iter()
                .map(|value| value.as_float().unwrap())
                .collect()]
        }
        Some(Value::List(values)) => values
            .iter()
            .map(|column| match column {
                Value::List(items) => items
                    .iter()
                    .map(|value| {
                        value.as_float().ok_or_else(|| {
                            BioLangError::type_error(
                                "sc_sctransform() covariates must be numeric",
                                None,
                            )
                        })
                    })
                    .collect::<Result<Vec<_>>>(),
                other => Err(BioLangError::type_error(
                    format!(
                        "sc_sctransform() covariates must be a numeric List or List<List>, got {}",
                        other.type_of()
                    ),
                    None,
                )),
            })
            .collect::<Result<Vec<_>>>()?,
        Some(other) => {
            return Err(BioLangError::type_error(
                format!(
                    "sc_sctransform() covariates must be a numeric List or List<List>, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    if latent_covariates
        .iter()
        .any(|column| column.len() != n_cells)
    {
        return Err(BioLangError::type_error(
            format!("sc_sctransform() covariates must have one value per cell ({n_cells})"),
            None,
        ));
    }

    if n_cells == 0 || n_genes == 0 {
        return Ok(Value::List((vec![]).into()));
    }

    // The regularized model, which is the method as published. Genes get an
    // overdispersion estimated from their own counts and then smoothed against
    // expression, rather than one number asserted for all of them.
    {
        use bl_core::bio_core::sctransform::{sctransform, GeneColumns, SctOptions};
        let data = if let Value::SparseMatrix(matrix) = &args[0] {
            // `GeneColumns::from_cell_major` deliberately scans twice: once to
            // size every gene column exactly and once to fill it. A sparse
            // matrix can supply those scans directly from CSR. Routing it
            // through `get_row` first cleared and revisited n_cells*n_genes
            // zeros on each pass (over 800 million writes for HBC) even though
            // only the non-zeros are relevant.
            GeneColumns::from_cell_major(n_cells, n_genes, |emit| {
                for cell in 0..n_cells {
                    for position in matrix.indptr[cell]..matrix.indptr[cell + 1] {
                        let count = matrix.data[position];
                        if count != 0.0 {
                            emit(cell, matrix.indices[position], count);
                        }
                    }
                }
            })
        } else {
            GeneColumns::from_cell_major(n_cells, n_genes, |emit| {
                let mut row = Vec::with_capacity(n_genes);
                for cell in 0..n_cells {
                    get_row(cell, &mut row);
                    for (gene, &count) in row.iter().enumerate() {
                        if count != 0.0 {
                            emit(cell, gene, count);
                        }
                    }
                }
            })
        };
        let options = SctOptions {
            n_variable_features: want,
            latent_covariates,
            ..Default::default()
        };
        let result = sctransform(&data, &options);
        let width = result.kept_genes.len();
        let matrix = bl_core::matrix::Matrix::new(result.residuals, n_cells, width)
            .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, &e, None))?;
        // Always paired with original-axis gene indices, even uncapped. Genes
        // containing no counts are absent, while other low-detection genes can
        // read regularized parameters from the expression trend. Returning a
        // narrower matrix without its surviving indices would create exactly
        // the quiet axis mismatch that later appears as a wrong gene name.
        let mut record = HashMap::new();
        record.insert("matrix".to_string(), Value::Matrix(matrix.into()));
        record.insert(
            "genes".to_string(),
            Value::List(
                result
                    .kept_genes
                    .into_iter()
                    .map(|gene| Value::Int(gene as i64))
                    .collect::<Vec<_>>()
                    .into(),
            ),
        );
        record.insert(
            "ranked_genes".to_string(),
            Value::List(
                result
                    .ranked_genes
                    .into_iter()
                    .map(|gene| Value::Int(gene as i64))
                    .collect::<Vec<_>>()
                    .into(),
            ),
        );
        // Raw, pre-regularization estimates, aligned with `fit_genes`. The
        // returned `theta` is the smoothed curve read back per gene, so a
        // systematic difference against another implementation could come from
        // the per-gene estimator or from the smoothing, and the smoothed values
        // alone cannot tell you which.
        record.insert(
            "log_geometric_mean".to_string(),
            Value::List(
                result
                    .log_geometric_mean
                    .into_iter()
                    .map(Value::Float)
                    .collect::<Vec<_>>()
                    .into(),
            ),
        );
        record.insert(
            "raw_theta".to_string(),
            Value::List(
                result
                    .raw_theta
                    .into_iter()
                    .map(Value::Float)
                    .collect::<Vec<_>>()
                    .into(),
            ),
        );
        record.insert(
            "raw_intercept".to_string(),
            Value::List(
                result
                    .raw_intercept
                    .into_iter()
                    .map(Value::Float)
                    .collect::<Vec<_>>()
                    .into(),
            ),
        );
        record.insert(
            "fit_genes".to_string(),
            Value::List(
                result
                    .fit_genes
                    .into_iter()
                    .map(|gene| Value::Int(gene as i64))
                    .collect::<Vec<_>>()
                    .into(),
            ),
        );
        record.insert(
            "fit_cells".to_string(),
            Value::List(
                result
                    .fit_cells
                    .into_iter()
                    .map(|cell| Value::Int(cell as i64))
                    .collect::<Vec<_>>()
                    .into(),
            ),
        );
        record.insert(
            "residual_variance".to_string(),
            Value::List(
                result
                    .residual_variance
                    .into_iter()
                    .map(Value::Float)
                    .collect::<Vec<_>>()
                    .into(),
            ),
        );
        record.insert(
            "residual_variance_stage".to_string(),
            Value::Str("pearson_before_centering_and_covariate_regression".into()),
        );
        record.insert(
            "theta".to_string(),
            Value::List(
                result
                    .theta
                    .into_iter()
                    .map(Value::Float)
                    .collect::<Vec<_>>()
                    .into(),
            ),
        );
        record.insert(
            "intercept".to_string(),
            Value::List(
                result
                    .intercept
                    .into_iter()
                    .map(Value::Float)
                    .collect::<Vec<_>>()
                    .into(),
            ),
        );
        return Ok(Value::Record(record.into()));
    }
}

// â”€â”€ sc_integrate(matrix, batch_ids) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Simplified batch correction: subtract per-batch per-gene mean.

fn builtin_sc_integrate(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "sc_integrate")?;
    let batch_ids: Vec<i64> = match &args[1] {
        Value::List(list) => list
            .iter()
            .map(|v| match v {
                Value::Int(n) => *n,
                Value::Float(f) => *f as i64,
                _ => 0,
            })
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "sc_integrate() batch_ids must be List<Int>",
                None,
            ))
        }
    };

    let n_cells = mat.len();
    if n_cells == 0 {
        return Ok(Value::List((vec![]).into()));
    }
    let n_genes = mat[0].len();

    if batch_ids.len() != n_cells {
        return Err(BioLangError::runtime(
            ErrorKind::ArityError,
            format!(
                "sc_integrate(): batch_ids length {} != n_cells {n_cells}",
                batch_ids.len()
            ),
            None,
        ));
    }

    let mut unique_batches: Vec<i64> = batch_ids.clone();
    unique_batches.sort_unstable();
    unique_batches.dedup();

    let mut batch_means: HashMap<i64, Vec<f64>> = HashMap::new();
    for &b in &unique_batches {
        let mut sums = vec![0.0f64; n_genes];
        let mut count = 0usize;
        for (i, row) in mat.iter().enumerate() {
            if batch_ids[i] == b {
                for (j, &v) in row.iter().enumerate() {
                    if j < n_genes {
                        sums[j] += v;
                    }
                }
                count += 1;
            }
        }
        if count > 0 {
            for s in &mut sums {
                *s /= count as f64;
            }
        }
        batch_means.insert(b, sums);
    }

    let corrected: Vec<Vec<f64>> = mat
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let means = batch_means
                .get(&batch_ids[i])
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            row.iter()
                .enumerate()
                .map(|(j, &v)| v - means.get(j).copied().unwrap_or(0.0))
                .collect()
        })
        .collect();

    Ok(matrix_to_value(corrected))
}

// â”€â”€ diffusion_pseudotime(embeddings, knn_edges, start_cell) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Dijkstra shortest path on the KNN graph from start_cell.

fn builtin_diffusion_pseudotime(args: Vec<Value>) -> Result<Value> {
    let embeddings = require_matrix(&args[0], "diffusion_pseudotime")?;
    let n_cells = embeddings.len();

    let edges: Vec<(usize, usize, f64)> = match &args[1] {
        Value::List(list) => list
            .iter()
            .filter_map(|v| {
                let rec = match v {
                    Value::Record(r) => r,
                    _ => return None,
                };
                let src = match rec.get("source")? {
                    Value::Int(n) => *n as usize,
                    _ => return None,
                };
                let tgt = match rec.get("target")? {
                    Value::Int(n) => *n as usize,
                    _ => return None,
                };
                let dist = match rec.get("distance")? {
                    Value::Float(f) => *f,
                    Value::Int(n) => *n as f64,
                    _ => return None,
                };
                Some((src, tgt, dist))
            })
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "diffusion_pseudotime() knn_edges must be List<Record>",
                None,
            ))
        }
    };

    let start_cell = require_int(&args[2], "diffusion_pseudotime")? as usize;

    if n_cells == 0 {
        return Ok(Value::List((vec![]).into()));
    }
    let start = start_cell.min(n_cells - 1);

    let mut adj: Vec<Vec<(usize, f64)>> = vec![vec![]; n_cells];
    for (src, tgt, d) in edges {
        let w = d.max(1e-10);
        if src < n_cells && tgt < n_cells {
            adj[src].push((tgt, w));
            adj[tgt].push((src, w));
        }
    }

    // Dijkstra with f64 bits as heap key (valid for non-negative distances)
    let mut dist_vec = vec![f64::INFINITY; n_cells];
    dist_vec[start] = 0.0;
    let mut heap: BinaryHeap<(Reverse<u64>, usize)> = BinaryHeap::new();
    heap.push((Reverse(0_f64.to_bits()), start));

    while let Some((Reverse(d_bits), u)) = heap.pop() {
        let d = f64::from_bits(d_bits);
        if d > dist_vec[u] {
            continue;
        }
        for &(v, w) in &adj[u] {
            let nd = d + w;
            if nd < dist_vec[v] {
                dist_vec[v] = nd;
                heap.push((Reverse(nd.to_bits()), v));
            }
        }
    }

    Ok(Value::List(
        dist_vec
            .into_iter()
            .map(|d| Value::Float(if d.is_infinite() { -1.0 } else { d }))
            .collect::<Vec<_>>()
            .into(),
    ))
}

// â”€â”€ Cell-cell communication â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

// â”€â”€ lr_score(matrix, cell_labels, lr_pairs) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// matrix: cells Ã— genes (consistent with other builtins)
// cell_labels: one cluster label string per cell
// lr_pairs: List<[ligand_gene_idx, receptor_gene_idx]>
// Returns Table: sender, receiver, ligand_idx, receptor_idx, score (sorted descending)

fn builtin_lr_score(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "lr_score")?;
    let n_cells = mat.len();

    let cell_labels: Vec<String> = match &args[1] {
        Value::List(list) => list
            .iter()
            .map(|v| match v {
                Value::Str(s) => s.clone(),
                other => format!("{other}"),
            })
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "lr_score() cell_labels must be List<Str>",
                None,
            ))
        }
    };

    if cell_labels.len() != n_cells {
        return Err(BioLangError::runtime(
            ErrorKind::ArityError,
            format!(
                "lr_score(): cell_labels length {} != n_cells {n_cells}",
                cell_labels.len()
            ),
            None,
        ));
    }

    let lr_pairs: Vec<(usize, usize)> = match &args[2] {
        Value::List(list) => list
            .iter()
            .enumerate()
            .map(|(i, pair)| match pair {
                Value::List(p) if p.len() >= 2 => {
                    let li = match &p[0] {
                        Value::Int(n) => *n as usize,
                        _ => {
                            return Err(BioLangError::type_error(
                                format!("lr_score() lr_pairs[{i}][0] must be Int"),
                                None,
                            ))
                        }
                    };
                    let ri = match &p[1] {
                        Value::Int(n) => *n as usize,
                        _ => {
                            return Err(BioLangError::type_error(
                                format!("lr_score() lr_pairs[{i}][1] must be Int"),
                                None,
                            ))
                        }
                    };
                    Ok((li, ri))
                }
                _ => Err(BioLangError::type_error(
                    format!("lr_score() lr_pairs[{i}] must be List<Int> of length >= 2"),
                    None,
                )),
            })
            .collect::<Result<Vec<_>>>()?,
        _ => {
            return Err(BioLangError::type_error(
                "lr_score() lr_pairs must be List<List<Int>>",
                None,
            ))
        }
    };

    let columns: Vec<String> = ["sender", "receiver", "ligand_idx", "receptor_idx", "score"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    if n_cells == 0 || lr_pairs.is_empty() {
        return Ok(Value::Table(Table::new(columns, vec![])));
    }

    let n_genes = mat[0].len();

    // Group cell indices by cluster label
    let mut cluster_cells: HashMap<String, Vec<usize>> = HashMap::new();
    for (cell_idx, label) in cell_labels.iter().enumerate() {
        cluster_cells
            .entry(label.clone())
            .or_default()
            .push(cell_idx);
    }

    // Compute mean expression per gene for each cluster
    let mut cluster_means: HashMap<String, Vec<f64>> = HashMap::new();
    for (cluster, cells) in &cluster_cells {
        let mut means = vec![0.0f64; n_genes];
        let n = cells.len() as f64;
        for &c in cells {
            for (g, &v) in mat[c].iter().enumerate() {
                if g < n_genes {
                    means[g] += v;
                }
            }
        }
        for m in &mut means {
            *m /= n;
        }
        cluster_means.insert(cluster.clone(), means);
    }

    // Deterministic ordering
    let mut clusters: Vec<String> = cluster_means.keys().cloned().collect();
    clusters.sort();

    // Score every sender Ã— receiver Ã— LR pair with score > 0
    let mut scored: Vec<(f64, Vec<Value>)> = Vec::new();
    for sender in &clusters {
        for receiver in &clusters {
            let s_means = &cluster_means[sender];
            let r_means = &cluster_means[receiver];
            for &(li, ri) in &lr_pairs {
                let l_expr = s_means.get(li).copied().unwrap_or(0.0);
                let r_expr = r_means.get(ri).copied().unwrap_or(0.0);
                let score = l_expr * r_expr;
                if score > 0.0 {
                    scored.push((
                        score,
                        vec![
                            Value::Str(sender.clone()),
                            Value::Str(receiver.clone()),
                            Value::Int(li as i64),
                            Value::Int(ri as i64),
                            Value::Float(score),
                        ],
                    ));
                }
            }
        }
    }

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let rows: Vec<Vec<Value>> = scored.into_iter().map(|(_, r)| r).collect();
    Ok(Value::Table(Table::new(columns, rows)))
}

// â”€â”€ lr_aggregate(lr_scores, pathway_map) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// lr_scores: Table from lr_score() â€” sender, receiver, ligand_idx, receptor_idx, score
// pathway_map: Table â€” ligand_idx, receptor_idx, pathway
// Returns Table: sender, receiver, pathway, total_score, n_pairs (sorted descending)

fn builtin_lr_aggregate(args: Vec<Value>) -> Result<Value> {
    let (score_rows, s_col, r_col, li_col, ri_col, sc_col) = match &args[0] {
        Value::Table(t) => {
            let s_col = t.col_index("sender").ok_or_else(|| {
                BioLangError::type_error("lr_aggregate() lr_scores missing 'sender'", None)
            })?;
            let r_col = t.col_index("receiver").ok_or_else(|| {
                BioLangError::type_error("lr_aggregate() lr_scores missing 'receiver'", None)
            })?;
            let li_col = t.col_index("ligand_idx").ok_or_else(|| {
                BioLangError::type_error("lr_aggregate() lr_scores missing 'ligand_idx'", None)
            })?;
            let ri_col = t.col_index("receptor_idx").ok_or_else(|| {
                BioLangError::type_error("lr_aggregate() lr_scores missing 'receptor_idx'", None)
            })?;
            let sc_col = t.col_index("score").ok_or_else(|| {
                BioLangError::type_error("lr_aggregate() lr_scores missing 'score'", None)
            })?;
            (t.rows.clone(), s_col, r_col, li_col, ri_col, sc_col)
        }
        _ => {
            return Err(BioLangError::type_error(
                "lr_aggregate() first arg must be a Table from lr_score()",
                None,
            ))
        }
    };

    let (pm_rows, pm_li, pm_ri, pm_path) = match &args[1] {
        Value::Table(t) => {
            let pm_li = t.col_index("ligand_idx").ok_or_else(|| {
                BioLangError::type_error("lr_aggregate() pathway_map missing 'ligand_idx'", None)
            })?;
            let pm_ri = t.col_index("receptor_idx").ok_or_else(|| {
                BioLangError::type_error("lr_aggregate() pathway_map missing 'receptor_idx'", None)
            })?;
            let pm_path = t.col_index("pathway").ok_or_else(|| {
                BioLangError::type_error("lr_aggregate() pathway_map missing 'pathway'", None)
            })?;
            (t.rows.clone(), pm_li, pm_ri, pm_path)
        }
        _ => {
            return Err(BioLangError::type_error(
                "lr_aggregate() second arg must be a Table with ligand_idx, receptor_idx, pathway",
                None,
            ))
        }
    };

    // Build (ligand_idx, receptor_idx) â†’ pathway lookup
    let mut pathway_lookup: HashMap<(i64, i64), String> = HashMap::new();
    for row in &pm_rows {
        let li = match row.get(pm_li) {
            Some(Value::Int(n)) => *n,
            Some(Value::Float(f)) => *f as i64,
            _ => continue,
        };
        let ri = match row.get(pm_ri) {
            Some(Value::Int(n)) => *n,
            Some(Value::Float(f)) => *f as i64,
            _ => continue,
        };
        let pathway = match row.get(pm_path) {
            Some(Value::Str(s)) => s.clone(),
            _ => continue,
        };
        pathway_lookup.insert((li, ri), pathway);
    }

    // Aggregate (sender, receiver, pathway) â†’ (total_score, n_pairs)
    let mut agg: HashMap<(String, String, String), (f64, usize)> = HashMap::new();
    for row in &score_rows {
        let sender = match row.get(s_col) {
            Some(Value::Str(s)) => s.clone(),
            _ => continue,
        };
        let receiver = match row.get(r_col) {
            Some(Value::Str(s)) => s.clone(),
            _ => continue,
        };
        let li = match row.get(li_col) {
            Some(Value::Int(n)) => *n,
            Some(Value::Float(f)) => *f as i64,
            _ => continue,
        };
        let ri = match row.get(ri_col) {
            Some(Value::Int(n)) => *n,
            Some(Value::Float(f)) => *f as i64,
            _ => continue,
        };
        let score = to_f64(row.get(sc_col).unwrap_or(&Value::Float(0.0))).unwrap_or(0.0);
        if let Some(pathway) = pathway_lookup.get(&(li, ri)) {
            let e = agg
                .entry((sender, receiver, pathway.clone()))
                .or_insert((0.0, 0));
            e.0 += score;
            e.1 += 1;
        }
    }

    let out_columns: Vec<String> = ["sender", "receiver", "pathway", "total_score", "n_pairs"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let mut result: Vec<(f64, Vec<Value>)> = agg
        .into_iter()
        .map(|((sender, receiver, pathway), (total, n))| {
            (
                total,
                vec![
                    Value::Str(sender),
                    Value::Str(receiver),
                    Value::Str(pathway),
                    Value::Float(total),
                    Value::Int(n as i64),
                ],
            )
        })
        .collect();
    result.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let rows: Vec<Vec<Value>> = result.into_iter().map(|(_, r)| r).collect();
    Ok(Value::Table(Table::new(out_columns, rows)))
}

// â”€â”€ Spatial transcriptomics â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

// â”€â”€ spatial_neighbors(coords, k=6) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// coords: Table with columns x, y (one row per spot)
// Returns Table: cell, neighbor, distance (Euclidean), sorted by cell then distance

fn builtin_spatial_neighbors(args: Vec<Value>) -> Result<Value> {
    let (xs, ys) = match &args[0] {
        Value::Table(t) => {
            let x_col = t.col_index("x").ok_or_else(|| {
                BioLangError::type_error("spatial_neighbors() coords must have column 'x'", None)
            })?;
            let y_col = t.col_index("y").ok_or_else(|| {
                BioLangError::type_error("spatial_neighbors() coords must have column 'y'", None)
            })?;
            let xs: Vec<f64> = t
                .rows
                .iter()
                .map(|r| to_f64(r.get(x_col).unwrap_or(&Value::Float(0.0))).unwrap_or(0.0))
                .collect();
            let ys: Vec<f64> = t
                .rows
                .iter()
                .map(|r| to_f64(r.get(y_col).unwrap_or(&Value::Float(0.0))).unwrap_or(0.0))
                .collect();
            (xs, ys)
        }
        _ => {
            return Err(BioLangError::type_error(
                "spatial_neighbors() coords must be Table with x,y columns",
                None,
            ))
        }
    };

    let k = if args.len() > 1 {
        require_int(&args[1], "spatial_neighbors")? as usize
    } else {
        6
    };

    let n = xs.len();
    let k_actual = k.min(n.saturating_sub(1));
    let columns: Vec<String> = ["cell", "neighbor", "distance"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    if n == 0 {
        return Ok(Value::Table(Table::new(columns, vec![])));
    }

    let mut rows: Vec<Vec<Value>> = Vec::new();
    for i in 0..n {
        let mut dists: Vec<(usize, f64)> = (0..n)
            .filter(|&j| j != i)
            .map(|j| {
                let dx = xs[i] - xs[j];
                let dy = ys[i] - ys[j];
                (j, (dx * dx + dy * dy).sqrt())
            })
            .collect();
        dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (j, d) in dists.into_iter().take(k_actual) {
            rows.push(vec![
                Value::Int(i as i64),
                Value::Int(j as i64),
                Value::Float(d),
            ]);
        }
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

// â”€â”€ spatial_moransi(expr_vec, spatial_adj) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// expr_vec: List<Float> â€” gene expression across spots
// spatial_adj: Table with columns cell, neighbor (from spatial_neighbors)
// Returns Float in [-1, 1]: Moran's I spatial autocorrelation

fn builtin_spatial_moransi(args: Vec<Value>) -> Result<Value> {
    let expr: Vec<f64> = match &args[0] {
        Value::List(list) => list.iter().map(|v| to_f64(v).unwrap_or(0.0)).collect(),
        _ => {
            return Err(BioLangError::type_error(
                "spatial_moransi() expr_vec must be List<Float>",
                None,
            ))
        }
    };
    let n = expr.len();

    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    let mut total_w: usize = 0;
    match &args[1] {
        Value::Table(t) => {
            let c_col = t.col_index("cell").ok_or_else(|| {
                BioLangError::type_error(
                    "spatial_moransi() spatial_adj missing 'cell' column",
                    None,
                )
            })?;
            let nb_col = t.col_index("neighbor").ok_or_else(|| {
                BioLangError::type_error(
                    "spatial_moransi() spatial_adj missing 'neighbor' column",
                    None,
                )
            })?;
            for row in &t.rows {
                let c = match row.get(c_col) {
                    Some(Value::Int(i)) => *i as usize,
                    Some(Value::Float(f)) => *f as usize,
                    _ => continue,
                };
                let nb = match row.get(nb_col) {
                    Some(Value::Int(i)) => *i as usize,
                    Some(Value::Float(f)) => *f as usize,
                    _ => continue,
                };
                if c < n && nb < n {
                    adj[c].push(nb);
                    total_w += 1;
                }
            }
        }
        _ => {
            return Err(BioLangError::type_error(
                "spatial_moransi() spatial_adj must be Table from spatial_neighbors()",
                None,
            ))
        }
    }

    if n == 0 || total_w == 0 {
        return Ok(Value::Float(0.0));
    }

    let mean = expr.iter().sum::<f64>() / n as f64;
    let denom: f64 = expr.iter().map(|&x| (x - mean) * (x - mean)).sum();
    if denom < 1e-12 {
        return Ok(Value::Float(0.0));
    }

    // Binary weights: w_ij = 1 if j in adj[i]; W = total_w
    let mut numer = 0.0f64;
    for i in 0..n {
        let xi = expr[i];
        for &j in &adj[i] {
            numer += (xi - mean) * (expr[j] - mean);
        }
    }

    let i_moran = (n as f64 / total_w as f64) * numer / denom;
    Ok(Value::Float(i_moran.clamp(-1.0, 1.0)))
}

// â”€â”€ Cell type annotation â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

// â”€â”€ reference_classify(query_matrix, ref_matrix, ref_labels) â”€â”€â”€â”€â”€â”€â”€â”€â”€
// query_matrix: Table genes Ã— query_cells; ref_matrix: genes Ã— ref_cells
// Returns Table: cell, label (majority-vote over top-5 cosine neighbours), confidence

fn builtin_reference_classify(args: Vec<Value>) -> Result<Value> {
    let q_mat = require_matrix(&args[0], "reference_classify")?; // mat[gene][query_cell]
    let r_mat = require_matrix(&args[1], "reference_classify")?; // mat[gene][ref_cell]
    let ref_labels: Vec<String> = match &args[2] {
        Value::List(list) => list
            .iter()
            .map(|v| match v {
                Value::Str(s) => s.clone(),
                other => format!("{other}"),
            })
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "reference_classify() ref_labels must be List<Str>",
                None,
            ))
        }
    };

    let n_genes_q = q_mat.len();
    let n_genes_r = r_mat.len();
    if n_genes_q != n_genes_r {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("reference_classify(): query has {n_genes_q} genes but ref has {n_genes_r}"),
            None,
        ));
    }
    let n_genes = n_genes_q;
    let n_query = if n_genes > 0 { q_mat[0].len() } else { 0 };
    let n_ref = if n_genes > 0 { r_mat[0].len() } else { 0 };
    if ref_labels.len() != n_ref {
        return Err(BioLangError::runtime(
            ErrorKind::ArityError,
            format!(
                "reference_classify(): ref_labels length {} != n_ref_cells {n_ref}",
                ref_labels.len()
            ),
            None,
        ));
    }

    let k = 5usize.min(n_ref);
    let columns: Vec<String> = ["cell", "label", "confidence"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut rows: Vec<Vec<Value>> = Vec::new();

    for qc in 0..n_query {
        let qvec: Vec<f64> = (0..n_genes)
            .map(|g| q_mat[g].get(qc).copied().unwrap_or(0.0))
            .collect();
        let q_norm: f64 = qvec.iter().map(|&v| v * v).sum::<f64>().sqrt();

        let mut sims: Vec<(usize, f64)> = (0..n_ref)
            .map(|rc| {
                let dot: f64 = (0..n_genes)
                    .map(|g| qvec[g] * r_mat[g].get(rc).copied().unwrap_or(0.0))
                    .sum();
                let r_norm: f64 = (0..n_genes)
                    .map(|g| {
                        let v = r_mat[g].get(rc).copied().unwrap_or(0.0);
                        v * v
                    })
                    .sum::<f64>()
                    .sqrt();
                let sim = if q_norm > 1e-10 && r_norm > 1e-10 {
                    dot / (q_norm * r_norm)
                } else {
                    0.0
                };
                (rc, sim)
            })
            .collect();
        sims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut counts: HashMap<String, usize> = HashMap::new();
        for &(rc, _) in sims.iter().take(k) {
            *counts.entry(ref_labels[rc].clone()).or_insert(0) += 1;
        }
        let (best_label, best_count) = counts
            .into_iter()
            .max_by_key(|(_, c)| *c)
            .unwrap_or_else(|| ("unknown".to_string(), 0));
        let confidence = if k > 0 {
            best_count as f64 / k as f64
        } else {
            0.0
        };

        rows.push(vec![
            Value::Int(qc as i64),
            Value::Str(best_label),
            Value::Float(confidence),
        ]);
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

// â”€â”€ Pseudobulk aggregation â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

// â”€â”€ pseudobulk_aggregate(matrix, cell_labels, sample_labels) â”€â”€â”€â”€â”€â”€â”€â”€â”€
// matrix: cells Ã— genes â€” the orientation every single-cell object uses, so
// obj.matrix can be passed straight in. CSR input is summed without densifying.
// Sums counts per (cluster, sample) group.
// Returns Table: columns = "cluster__sample", rows = genes

fn builtin_pseudobulk_aggregate(args: Vec<Value>) -> Result<Value> {
    let parse_str_list = |v: &Value, name: &str| -> Result<Vec<String>> {
        match v {
            Value::List(list) => Ok(list
                .iter()
                .map(|v| match v {
                    Value::Str(s) => s.clone(),
                    other => format!("{other}"),
                })
                .collect()),
            _ => Err(BioLangError::type_error(
                format!("pseudobulk_aggregate() {name} must be List<Str>"),
                None,
            )),
        }
    };
    let cell_labels = parse_str_list(&args[1], "cell_labels")?;
    let sample_labels = parse_str_list(&args[2], "sample_labels")?;

    // Dense input is materialized as rows-of-cells; CSR stays sparse and is
    // accumulated below straight from its nonzeros.
    let sparse = match &args[0] {
        Value::SparseMatrix(sm) => Some(sm),
        _ => None,
    };
    let dense: Vec<Vec<f64>> = match sparse {
        Some(_) => Vec::new(),
        None => require_matrix(&args[0], "pseudobulk_aggregate")?,
    };
    let (n_cells, n_genes) = match sparse {
        Some(sm) => (sm.nrow, sm.ncol),
        None => (dense.len(), dense.first().map(|r| r.len()).unwrap_or(0)),
    };

    if cell_labels.len() != n_cells {
        return Err(BioLangError::runtime(
            ErrorKind::ArityError,
            format!(
                "pseudobulk_aggregate(): cell_labels length {} != n_cells {n_cells}",
                cell_labels.len()
            ),
            None,
        ));
    }
    if sample_labels.len() != n_cells {
        return Err(BioLangError::runtime(
            ErrorKind::ArityError,
            format!(
                "pseudobulk_aggregate(): sample_labels length {} != n_cells {n_cells}",
                sample_labels.len()
            ),
            None,
        ));
    }

    // Collect unique groups in deterministic sorted order
    let mut group_keys: Vec<String> = (0..n_cells)
        .map(|c| format!("{}__{}", cell_labels[c], sample_labels[c]))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    group_keys.sort();
    let n_groups = group_keys.len();
    let group_to_col: HashMap<String, usize> = group_keys
        .iter()
        .enumerate()
        .map(|(i, k)| (k.clone(), i))
        .collect();

    // Cell â†’ group column
    let cell_group: Vec<usize> = (0..n_cells)
        .map(|c| group_to_col[&format!("{}__{}", cell_labels[c], sample_labels[c])])
        .collect();

    // Accumulate into a genes Ã— groups panel, one pass over the cells.
    let mut sums = vec![vec![0.0f64; n_groups]; n_genes];
    match sparse {
        Some(sm) => {
            for c in 0..n_cells {
                let col = cell_group[c];
                for pos in sm.indptr[c]..sm.indptr[c + 1] {
                    sums[sm.indices[pos]][col] += sm.data[pos];
                }
            }
        }
        None => {
            for (c, row) in dense.iter().enumerate() {
                let col = cell_group[c];
                for (g, &v) in row.iter().enumerate() {
                    if g < n_genes {
                        sums[g][col] += v;
                    }
                }
            }
        }
    }

    let rows: Vec<Vec<Value>> = sums
        .into_iter()
        .map(|row| row.into_iter().map(Value::Float).collect())
        .collect();

    Ok(Value::Table(Table::new(group_keys, rows)))
}

// â”€â”€ Multimodal integration â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

// â”€â”€ wnn_graph(matrix_a, matrix_b, k) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// matrix_a, matrix_b: Table cells Ã— dims (rows=cells) for two modalities
// Returns Table: cell, neighbor, weight (Î± weighted by modality quality)

fn builtin_wnn_graph(args: Vec<Value>) -> Result<Value> {
    let mat_a = require_matrix(&args[0], "wnn_graph")?; // mat[cell][dim]
    let mat_b = require_matrix(&args[1], "wnn_graph")?;
    let k = require_int(&args[2], "wnn_graph")? as usize;

    let n_a = mat_a.len();
    let n_b = mat_b.len();
    if n_a != n_b {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("wnn_graph(): matrix_a has {n_a} cells but matrix_b has {n_b}"),
            None,
        ));
    }
    let n = n_a;
    let k_actual = k.min(n.saturating_sub(1));
    let columns: Vec<String> = ["cell", "neighbor", "weight"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    if n == 0 || k_actual == 0 {
        return Ok(Value::Table(Table::new(columns, vec![])));
    }

    let euclid = |a: &[f64], b: &[f64]| -> f64 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y) * (x - y))
            .sum::<f64>()
            .sqrt()
    };

    // k-NN for each modality
    let knn = |mat: &[Vec<f64>]| -> Vec<Vec<(usize, f64)>> {
        (0..n)
            .map(|i| {
                let mut dists: Vec<(usize, f64)> = (0..n)
                    .filter(|&j| j != i)
                    .map(|j| (j, euclid(&mat[i], &mat[j])))
                    .collect();
                dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                dists.into_iter().take(k_actual).collect()
            })
            .collect()
    };

    let knn_a = knn(&mat_a);
    let knn_b = knn(&mat_b);

    // Per-cell modality weight Î±_i
    let alpha: Vec<f64> = (0..n)
        .map(|i| {
            let mean_a = if knn_a[i].is_empty() {
                0.0
            } else {
                knn_a[i].iter().map(|(_, d)| d).sum::<f64>() / knn_a[i].len() as f64
            };
            let mean_b = if knn_b[i].is_empty() {
                0.0
            } else {
                knn_b[i].iter().map(|(_, d)| d).sum::<f64>() / knn_b[i].len() as f64
            };
            let ea = (-mean_a).exp();
            let eb = (-mean_b).exp();
            let denom = ea + eb;
            if denom > 1e-12 {
                ea / denom
            } else {
                0.5
            }
        })
        .collect();

    // Merge edges from both modalities, keep top-k by weight
    let mut rows: Vec<Vec<Value>> = Vec::new();
    for i in 0..n {
        let ai = alpha[i];
        let mut edges: HashMap<usize, f64> = HashMap::new();
        for &(j, _) in &knn_a[i] {
            edges.entry(j).or_insert(ai);
        }
        for &(j, _) in &knn_b[i] {
            edges.entry(j).or_insert(1.0 - ai);
        }
        let mut sorted: Vec<(usize, f64)> = edges.into_iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (j, w) in sorted.into_iter().take(k_actual) {
            rows.push(vec![
                Value::Int(i as i64),
                Value::Int(j as i64),
                Value::Float(w),
            ]);
        }
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

// â”€â”€ RNA velocity â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

// â”€â”€ velocity_estimate(spliced, unspliced) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
// Deterministic model: Î²_g = mean_spliced_g / (mean_unspliced_g + Îµ)
// velocity[g][c] = unspliced[g][c] * Î²_g - spliced[g][c]
// Returns Table same shape as inputs (genes Ã— cells)

fn builtin_velocity_estimate(args: Vec<Value>) -> Result<Value> {
    let spliced = require_matrix(&args[0], "velocity_estimate")?; // mat[gene][cell]
    let unspliced = require_matrix(&args[1], "velocity_estimate")?;

    let n_genes = spliced.len();
    if n_genes != unspliced.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "velocity_estimate(): spliced has {n_genes} genes but unspliced has {}",
                unspliced.len()
            ),
            None,
        ));
    }
    if n_genes == 0 {
        return Ok(matrix_to_value(vec![]));
    }
    let n_cells = spliced[0].len();
    if n_cells != unspliced[0].len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "velocity_estimate(): spliced has {n_cells} cells but unspliced has {}",
                unspliced[0].len()
            ),
            None,
        ));
    }

    const EPS: f64 = 1e-6;
    let result: Vec<Vec<f64>> = (0..n_genes)
        .map(|g| {
            let s_row = &spliced[g];
            let u_row = &unspliced[g];
            let mean_s = s_row.iter().sum::<f64>() / n_cells.max(1) as f64;
            let mean_u = u_row.iter().sum::<f64>() / n_cells.max(1) as f64;
            let beta = mean_s / (mean_u + EPS);
            (0..n_cells)
                .map(|c| {
                    u_row.get(c).copied().unwrap_or(0.0) * beta
                        - s_row.get(c).copied().unwrap_or(0.0)
                })
                .collect()
        })
        .collect();

    Ok(matrix_to_value(result))
}

// â”€â”€ Section 7: CNV / tumour purity â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

// â”€â”€ cnv_segment(log_ratios, min_segment=5) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn builtin_cnv_segment(args: Vec<Value>) -> Result<Value> {
    let log_ratios: Vec<f64> = match &args[0] {
        Value::List(list) => list.iter().map(|v| to_f64(v).unwrap_or(0.0)).collect(),
        _ => {
            return Err(BioLangError::type_error(
                "cnv_segment() requires List<Float>",
                None,
            ))
        }
    };
    let min_segment = if args.len() > 1 {
        require_int(&args[1], "cnv_segment")? as usize
    } else {
        5
    };

    let n = log_ratios.len();
    if n == 0 {
        return Ok(Value::List((vec![]).into()));
    }

    // CBS approximation: sliding window t-test to detect change points
    let window = min_segment.max(3);
    let mut change_points: Vec<usize> = vec![0];

    if n >= 2 * window {
        for i in window..n.saturating_sub(window) {
            let left = &log_ratios[i.saturating_sub(window)..i];
            let right = &log_ratios[i..i + window];

            let mean_l = left.iter().sum::<f64>() / left.len() as f64;
            let mean_r = right.iter().sum::<f64>() / right.len() as f64;

            let var_l = left
                .iter()
                .map(|&x| (x - mean_l) * (x - mean_l))
                .sum::<f64>()
                / (left.len() as f64 + 1e-10);
            let var_r = right
                .iter()
                .map(|&x| (x - mean_r) * (x - mean_r))
                .sum::<f64>()
                / (right.len() as f64 + 1e-10);
            let pooled_se = ((var_l + var_r) / window as f64).sqrt() + 1e-10;
            let t = (mean_r - mean_l).abs() / pooled_se;

            // t â‰ˆ 2.0 ~ p < 0.05 for moderate degrees of freedom
            if t > 2.0 && i - *change_points.last().unwrap() >= min_segment {
                change_points.push(i);
            }
        }
    }
    change_points.push(n);

    // Build raw segments
    let mut segments: Vec<Value> = Vec::new();
    let mut segment_id: i64 = 0;
    for win in change_points.windows(2) {
        let seg_start = win[0];
        let seg_end = win[1];
        let seg_len = seg_end - seg_start;
        let seg_vals = &log_ratios[seg_start..seg_end];
        let mean_ratio = seg_vals.iter().sum::<f64>() / seg_vals.len().max(1) as f64;

        let mut rec = HashMap::new();
        rec.insert("start".to_string(), Value::Int(seg_start as i64));
        rec.insert("end".to_string(), Value::Int(seg_end as i64));
        rec.insert("n_probes".to_string(), Value::Int(seg_len as i64));
        rec.insert("mean_ratio".to_string(), Value::Float(mean_ratio));
        rec.insert("segment_id".to_string(), Value::Int(segment_id));
        segments.push(Value::Record((rec).into()));
        segment_id += 1;
    }

    // Merge adjacent segments whose means differ by < 0.1 log2
    let mut merged: Vec<HashMap<String, Value>> = Vec::new();
    for seg in segments {
        if let Value::Record(fields) = seg {
            if let Some(last) = merged.last_mut() {
                let last_mean = match last.get("mean_ratio") {
                    Some(Value::Float(f)) => *f,
                    _ => f64::INFINITY,
                };
                let this_mean = match fields.get("mean_ratio") {
                    Some(Value::Float(f)) => *f,
                    _ => f64::INFINITY,
                };
                if (last_mean - this_mean).abs() < 0.1 {
                    let last_start = match last.get("start") {
                        Some(Value::Int(n)) => *n as usize,
                        _ => 0,
                    };
                    let this_end = match fields.get("end") {
                        Some(Value::Int(n)) => *n as usize,
                        _ => 0,
                    };
                    let merged_vals = &log_ratios[last_start..this_end];
                    let merged_mean =
                        merged_vals.iter().sum::<f64>() / merged_vals.len().max(1) as f64;
                    last.insert("end".to_string(), Value::Int(this_end as i64));
                    last.insert("n_probes".to_string(), Value::Int(merged_vals.len() as i64));
                    last.insert("mean_ratio".to_string(), Value::Float(merged_mean));
                    continue;
                }
            }
            merged.push((fields).as_ref().clone());
        }
    }

    // Re-number segment ids
    let result: Vec<Value> = merged
        .into_iter()
        .enumerate()
        .map(|(i, mut fields)| {
            fields.insert("segment_id".to_string(), Value::Int(i as i64));
            Value::Record((fields).into())
        })
        .collect();

    Ok(Value::List((result).into()))
}

// â”€â”€ loh_detect(het_snp_vafs) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn builtin_loh_detect(args: Vec<Value>) -> Result<Value> {
    let vafs: Vec<f64> = match &args[0] {
        Value::List(list) => list.iter().map(|v| to_f64(v).unwrap_or(0.5)).collect(),
        _ => {
            return Err(BioLangError::type_error(
                "loh_detect() requires List<Float>",
                None,
            ))
        }
    };

    let n_snps = vafs.len();
    let empty_rec = || {
        let mut rec = HashMap::new();
        rec.insert("n_snps".to_string(), Value::Int(0));
        rec.insert("n_loh_snps".to_string(), Value::Int(0));
        rec.insert("loh_fraction".to_string(), Value::Float(0.0));
        rec.insert("mean_vaf".to_string(), Value::Float(0.0));
        rec.insert("median_deviation".to_string(), Value::Float(0.0));
        Value::Record((rec).into())
    };

    if n_snps == 0 {
        return Ok(empty_rec());
    }

    let loh_threshold = 0.2; // |VAF - 0.5| > 0.2
    let n_loh = vafs
        .iter()
        .filter(|&&v| (v - 0.5).abs() > loh_threshold)
        .count();
    let loh_fraction = n_loh as f64 / n_snps as f64;
    let mean_vaf = vafs.iter().sum::<f64>() / n_snps as f64;

    let mut deviations: Vec<f64> = vafs.iter().map(|&v| (v - 0.5).abs()).collect();
    deviations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_deviation = if n_snps.is_multiple_of(2) {
        (deviations[n_snps / 2 - 1] + deviations[n_snps / 2]) / 2.0
    } else {
        deviations[n_snps / 2]
    };

    let mut rec = HashMap::new();
    rec.insert("n_snps".to_string(), Value::Int(n_snps as i64));
    rec.insert("n_loh_snps".to_string(), Value::Int(n_loh as i64));
    rec.insert("loh_fraction".to_string(), Value::Float(loh_fraction));
    rec.insert("mean_vaf".to_string(), Value::Float(mean_vaf));
    rec.insert(
        "median_deviation".to_string(),
        Value::Float(median_deviation),
    );
    Ok(Value::Record((rec).into()))
}

// â”€â”€ tumor_purity(vafs) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn builtin_tumor_purity(args: Vec<Value>) -> Result<Value> {
    let vafs: Vec<f64> = match &args[0] {
        Value::List(list) => list.iter().map(|v| to_f64(v).unwrap_or(0.0)).collect(),
        _ => {
            return Err(BioLangError::type_error(
                "tumor_purity() requires List<Float>",
                None,
            ))
        }
    };

    let n_variants = vafs.len();
    if n_variants == 0 {
        let mut rec = HashMap::new();
        rec.insert("estimated_purity".to_string(), Value::Float(0.0));
        rec.insert("dominant_peak_vaf".to_string(), Value::Float(0.0));
        rec.insert("n_variants".to_string(), Value::Int(0));
        return Ok(Value::Record((rec).into()));
    }

    // Histogram with 0.05-width bins over [0, 1]
    let n_bins = 20usize;
    let bin_width = 1.0 / n_bins as f64;
    let mut hist = vec![0usize; n_bins];
    for &v in &vafs {
        let bin = ((v / bin_width) as usize).min(n_bins - 1);
        hist[bin] += 1;
    }

    let (mode_bin, _) = hist
        .iter()
        .enumerate()
        .max_by_key(|(_, &cnt)| cnt)
        .unwrap_or((0, &0));

    let dominant_peak_vaf = (mode_bin as f64 + 0.5) * bin_width;
    // purity â‰ˆ 2 * modal_vaf (diploid, clonal heterozygous variants)
    let estimated_purity = (2.0 * dominant_peak_vaf).clamp(0.0, 1.0);

    let mut rec = HashMap::new();
    rec.insert(
        "estimated_purity".to_string(),
        Value::Float(estimated_purity),
    );
    rec.insert(
        "dominant_peak_vaf".to_string(),
        Value::Float(dominant_peak_vaf),
    );
    rec.insert("n_variants".to_string(), Value::Int(n_variants as i64));
    Ok(Value::Record((rec).into()))
}

// â”€â”€ vaf_to_ccf(vaf, purity, cn_total=2, cn_minor=0) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn builtin_vaf_to_ccf(args: Vec<Value>) -> Result<Value> {
    let vaf = to_f64(&args[0])
        .ok_or_else(|| BioLangError::type_error("vaf_to_ccf() vaf must be a number", None))?;
    let purity = to_f64(&args[1])
        .ok_or_else(|| BioLangError::type_error("vaf_to_ccf() purity must be a number", None))?;
    let cn_total = if args.len() > 2 {
        to_f64(&args[2]).unwrap_or(2.0)
    } else {
        2.0
    };
    // cn_minor_of_variant â€” included for API completeness
    let _cn_minor = if args.len() > 3 {
        to_f64(&args[3]).unwrap_or(0.0)
    } else {
        0.0
    };

    if purity <= 0.0 {
        return Ok(Value::Float(0.0));
    }

    // CCF = vaf * (2*(1-purity) + purity*cn_total) / purity
    let ccf = vaf * (2.0 * (1.0 - purity) + purity * cn_total) / purity;
    Ok(Value::Float(ccf.clamp(0.0, 1.0)))
}

// â”€â”€ mutational_signature(mut_counts_96) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
//
// Simplified 10-signature Ã— 96-context COSMIC SBS matrix.
// NOTE: Values below are APPROXIMATE placeholders for structural correctness.
// Replace columns with official values from cosmic-signatures.cancer.sanger.ac.uk
// (COSMIC v3.3 SBS reference signatures TSV).
//
// Column order: SBS1, SBS2, SBS3, SBS4, SBS5, SBS13, SBS17a, SBS17b, SBS18, SBS40

static COSMIC_SIGS: [[f64; 10]; 96] = [
    [
        0.011, 0.001, 0.003, 0.001, 0.008, 0.001, 0.001, 0.001, 0.002, 0.008,
    ],
    [
        0.009, 0.001, 0.003, 0.002, 0.007, 0.001, 0.001, 0.001, 0.002, 0.007,
    ],
    [
        0.008, 0.001, 0.003, 0.001, 0.008, 0.001, 0.001, 0.001, 0.002, 0.008,
    ],
    [
        0.006, 0.001, 0.003, 0.001, 0.007, 0.001, 0.001, 0.001, 0.002, 0.007,
    ],
    [
        0.014, 0.012, 0.003, 0.001, 0.008, 0.001, 0.001, 0.001, 0.002, 0.008,
    ],
    [
        0.011, 0.010, 0.003, 0.002, 0.007, 0.001, 0.001, 0.001, 0.002, 0.007,
    ],
    [
        0.013, 0.012, 0.003, 0.001, 0.008, 0.001, 0.001, 0.001, 0.002, 0.008,
    ],
    [
        0.009, 0.008, 0.003, 0.001, 0.007, 0.001, 0.001, 0.001, 0.002, 0.007,
    ],
    [
        0.008, 0.001, 0.003, 0.001, 0.008, 0.001, 0.001, 0.001, 0.002, 0.008,
    ],
    [
        0.007, 0.001, 0.003, 0.002, 0.007, 0.001, 0.001, 0.001, 0.002, 0.007,
    ],
    [
        0.007, 0.001, 0.003, 0.001, 0.008, 0.001, 0.001, 0.001, 0.002, 0.008,
    ],
    [
        0.005, 0.001, 0.003, 0.001, 0.007, 0.001, 0.001, 0.001, 0.002, 0.007,
    ],
    [
        0.011, 0.001, 0.003, 0.003, 0.008, 0.001, 0.001, 0.001, 0.002, 0.008,
    ],
    [
        0.009, 0.001, 0.003, 0.002, 0.007, 0.001, 0.001, 0.001, 0.002, 0.007,
    ],
    [
        0.008, 0.001, 0.003, 0.002, 0.008, 0.001, 0.001, 0.001, 0.002, 0.008,
    ],
    [
        0.006, 0.001, 0.003, 0.002, 0.007, 0.001, 0.001, 0.001, 0.002, 0.007,
    ],
    [
        0.001, 0.001, 0.008, 0.001, 0.010, 0.001, 0.001, 0.001, 0.003, 0.010,
    ],
    [
        0.001, 0.001, 0.006, 0.002, 0.009, 0.001, 0.001, 0.001, 0.003, 0.009,
    ],
    [
        0.001, 0.001, 0.008, 0.001, 0.010, 0.001, 0.001, 0.001, 0.003, 0.010,
    ],
    [
        0.001, 0.001, 0.005, 0.001, 0.009, 0.001, 0.001, 0.001, 0.003, 0.009,
    ],
    [
        0.001, 0.001, 0.010, 0.004, 0.010, 0.001, 0.001, 0.001, 0.003, 0.010,
    ],
    [
        0.001, 0.001, 0.008, 0.003, 0.009, 0.001, 0.001, 0.001, 0.003, 0.009,
    ],
    [
        0.001, 0.001, 0.010, 0.003, 0.010, 0.001, 0.001, 0.001, 0.003, 0.010,
    ],
    [
        0.001, 0.001, 0.006, 0.002, 0.009, 0.001, 0.001, 0.001, 0.003, 0.009,
    ],
    [
        0.001, 0.001, 0.006, 0.001, 0.010, 0.001, 0.001, 0.001, 0.003, 0.010,
    ],
    [
        0.001, 0.001, 0.005, 0.002, 0.009, 0.001, 0.001, 0.001, 0.003, 0.009,
    ],
    [
        0.001, 0.001, 0.006, 0.001, 0.010, 0.001, 0.001, 0.001, 0.003, 0.010,
    ],
    [
        0.001, 0.001, 0.004, 0.001, 0.009, 0.001, 0.001, 0.001, 0.003, 0.009,
    ],
    [
        0.001, 0.001, 0.009, 0.005, 0.010, 0.001, 0.001, 0.001, 0.003, 0.010,
    ],
    [
        0.001, 0.001, 0.007, 0.004, 0.009, 0.001, 0.001, 0.001, 0.003, 0.009,
    ],
    [
        0.001, 0.001, 0.009, 0.004, 0.010, 0.001, 0.001, 0.001, 0.003, 0.010,
    ],
    [
        0.001, 0.001, 0.006, 0.003, 0.009, 0.001, 0.001, 0.001, 0.003, 0.009,
    ],
    [
        0.001, 0.050, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.040, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.050, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.035, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.060, 0.003, 0.003, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.048, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.060, 0.003, 0.002, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.042, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.035, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.028, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.035, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.025, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.042, 0.003, 0.003, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.034, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.042, 0.003, 0.002, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.030, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.040, 0.050, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.032, 0.040, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.040, 0.050, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.028, 0.035, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.003, 0.006, 0.001, 0.048, 0.060, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.038, 0.048, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.006, 0.001, 0.048, 0.060, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.034, 0.042, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.028, 0.035, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.022, 0.028, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.028, 0.035, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.020, 0.025, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.003, 0.006, 0.001, 0.034, 0.042, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.027, 0.034, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.006, 0.001, 0.034, 0.042, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.024, 0.030, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.003, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.003, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.003, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.003, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
];

const SIG_NAMES: [&str; 10] = [
    "SBS1", "SBS2", "SBS3", "SBS4", "SBS5", "SBS13", "SBS17a", "SBS17b", "SBS18", "SBS40",
];

fn builtin_mutational_signature(args: Vec<Value>) -> Result<Value> {
    let counts: Vec<f64> = match &args[0] {
        Value::List(list) => list.iter().map(|v| to_f64(v).unwrap_or(0.0)).collect(),
        _ => {
            return Err(BioLangError::type_error(
                "mutational_signature() requires List<Int|Float> of 96 counts",
                None,
            ))
        }
    };

    if counts.len() != 96 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "mutational_signature() requires exactly 96 SBS counts, got {}",
                counts.len()
            ),
            None,
        ));
    }

    let total_mutations: f64 = counts.iter().sum();

    let zero_contributions = || -> Vec<Value> {
        SIG_NAMES
            .iter()
            .map(|&name| {
                let mut rec = HashMap::new();
                rec.insert("signature".to_string(), Value::Str(name.to_string()));
                rec.insert("weight".to_string(), Value::Float(0.0));
                Value::Record((rec).into())
            })
            .collect()
    };

    if total_mutations == 0.0 {
        let mut result = HashMap::new();
        result.insert(
            "contributions".to_string(),
            Value::List((zero_contributions()).into()),
        );
        result.insert("r_squared".to_string(), Value::Float(0.0));
        result.insert("total_mutations".to_string(), Value::Float(0.0));
        return Ok(Value::Record((result).into()));
    }

    // Normalize observed counts to proportions
    let obs: Vec<f64> = counts.iter().map(|&c| c / total_mutations).collect();

    const N_SIGS: usize = 10;
    // Non-negative least squares via projected gradient descent
    let mut weights = vec![1.0 / N_SIGS as f64; N_SIGS];
    let learning_rate = 0.01;
    let n_iter = 1000;

    for _ in 0..n_iter {
        // fitted_i = sum_j COSMIC_SIGS[i][j] * weights[j]
        let mut fitted = vec![0.0f64; 96];
        for (j, &w) in weights.iter().enumerate() {
            for i in 0..96 {
                fitted[i] += COSMIC_SIGS[i][j] * w;
            }
        }

        // gradient_j = 2 * sum_i (fitted_i - obs_i) * COSMIC_SIGS[i][j]
        let mut grad = [0.0f64; N_SIGS];
        for i in 0..96 {
            let residual = fitted[i] - obs[i];
            for j in 0..N_SIGS {
                grad[j] += 2.0 * residual * COSMIC_SIGS[i][j];
            }
        }

        // Gradient step + project onto non-negative orthant
        for j in 0..N_SIGS {
            weights[j] = (weights[j] - learning_rate * grad[j]).max(0.0);
        }

        // Re-normalise to sum = 1
        let wsum: f64 = weights.iter().sum();
        if wsum > 0.0 {
            for w in &mut weights {
                *w /= wsum;
            }
        }
    }

    // Compute RÂ²
    let mut fitted_final = vec![0.0f64; 96];
    for (j, &w) in weights.iter().enumerate() {
        for i in 0..96 {
            fitted_final[i] += COSMIC_SIGS[i][j] * w;
        }
    }
    let obs_mean = obs.iter().sum::<f64>() / 96.0;
    let ss_tot: f64 = obs.iter().map(|&o| (o - obs_mean) * (o - obs_mean)).sum();
    let ss_res: f64 = obs
        .iter()
        .zip(fitted_final.iter())
        .map(|(&o, &f)| (o - f) * (o - f))
        .sum();
    let r_squared = if ss_tot > 0.0 {
        (1.0 - ss_res / ss_tot).clamp(0.0, 1.0)
    } else {
        1.0
    };

    let contributions: Vec<Value> = SIG_NAMES
        .iter()
        .zip(weights.iter())
        .map(|(&name, &w)| {
            let mut rec = HashMap::new();
            rec.insert("signature".to_string(), Value::Str(name.to_string()));
            rec.insert("weight".to_string(), Value::Float(w));
            Value::Record((rec).into())
        })
        .collect();

    let mut result = HashMap::new();
    result.insert(
        "contributions".to_string(),
        Value::List((contributions).into()),
    );
    result.insert("r_squared".to_string(), Value::Float(r_squared));
    result.insert("total_mutations".to_string(), Value::Float(total_mutations));
    Ok(Value::Record((result).into()))
}
