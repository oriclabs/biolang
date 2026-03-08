use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::matrix::Matrix;
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

pub fn matrix_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("matrix", Arity::Exact(1)),
        ("matrix_from_table", Arity::Exact(2)),
        ("matrix_to_table", Arity::Exact(1)),
        ("zeros", Arity::Exact(2)),
        ("eye", Arity::Exact(1)),
        ("dim", Arity::Exact(1)),
        ("transpose", Arity::Exact(1)),
        ("mat_add", Arity::Exact(2)),
        ("mat_sub", Arity::Exact(2)),
        ("mat_mul", Arity::Exact(2)),
        ("mat_scale", Arity::Exact(2)),
        ("mat_map", Arity::Exact(2)),
        ("dot", Arity::Exact(2)),
        ("row_sums", Arity::Exact(1)),
        ("col_sums", Arity::Exact(1)),
        ("row_means", Arity::Exact(1)),
        ("col_means", Arity::Exact(1)),
        ("pca", Arity::Exact(2)),
        ("cor_matrix", Arity::Exact(1)),
        ("cov_matrix", Arity::Exact(1)),
        ("dist_matrix", Arity::Exact(2)),
        ("row", Arity::Exact(2)),
        ("mat_col", Arity::Exact(2)),
        ("trace", Arity::Exact(1)),
        ("norm", Arity::Exact(1)),
        ("determinant", Arity::Exact(1)),
        ("inverse", Arity::Exact(1)),
        ("solve", Arity::Exact(2)),
        ("eigenvalues", Arity::Exact(1)),
        ("svd", Arity::Exact(1)),
        ("rank", Arity::Exact(1)),
        ("ones", Arity::Exact(2)),
        ("diag", Arity::Exact(1)),
    ]
}

pub fn is_matrix_builtin(name: &str) -> bool {
    matches!(
        name,
        "matrix"
            | "matrix_from_table"
            | "matrix_to_table"
            | "zeros"
            | "eye"
            | "dim"
            | "transpose"
            | "mat_add"
            | "mat_sub"
            | "mat_mul"
            | "mat_scale"
            | "mat_map"
            | "dot"
            | "row_sums"
            | "col_sums"
            | "row_means"
            | "col_means"
            | "pca"
            | "cor_matrix"
            | "cov_matrix"
            | "dist_matrix"
            | "row"
            | "mat_col"
            | "trace"
            | "norm"
            | "determinant"
            | "inverse"
            | "solve"
            | "eigenvalues"
            | "svd"
            | "rank"
            | "ones"
            | "diag"
    )
}

pub fn call_matrix_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "matrix" => builtin_matrix(args),
        "matrix_from_table" => builtin_matrix_from_table(args),
        "matrix_to_table" => builtin_matrix_to_table(args),
        "zeros" => builtin_zeros(args),
        "eye" => builtin_eye(args),
        "dim" => builtin_dim(args),
        "transpose" => builtin_transpose(args),
        "mat_add" => builtin_mat_add(args),
        "mat_sub" => builtin_mat_sub(args),
        "mat_mul" => builtin_mat_mul(args),
        "mat_scale" => builtin_mat_scale(args),
        "mat_map" => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "mat_map() requires a closure and must be called via HOF dispatch",
            None,
        )),
        "dot" => builtin_dot(args),
        "row_sums" => builtin_row_sums(args),
        "col_sums" => builtin_col_sums(args),
        "row_means" => builtin_row_means(args),
        "col_means" => builtin_col_means(args),
        "pca" => builtin_pca(args),
        "cor_matrix" => builtin_cor_matrix(args),
        "cov_matrix" => builtin_cov_matrix(args),
        "dist_matrix" => builtin_dist_matrix(args),
        "row" => builtin_row(args),
        "mat_col" => builtin_mat_col(args),
        "trace" => builtin_trace(args),
        "norm" => builtin_norm(args),
        "determinant" => builtin_determinant(args),
        "inverse" => builtin_inverse(args),
        "solve" => builtin_solve(args),
        "eigenvalues" => builtin_eigenvalues(args),
        "svd" => builtin_svd(args),
        "rank" => builtin_rank(args),
        "ones" => builtin_ones(args),
        "diag" => builtin_diag(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown matrix builtin '{name}'"),
            None,
        )),
    }
}

fn require_matrix<'a>(val: &'a Value, func: &str) -> Result<&'a Matrix> {
    match val {
        Value::Matrix(m) => Ok(m),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Matrix, got {}", other.type_of()),
            None,
        )),
    }
}

fn require_int(val: &Value, func: &str) -> Result<i64> {
    match val {
        Value::Int(n) => Ok(*n),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Int, got {}", other.type_of()),
            None,
        )),
    }
}

fn require_num(val: &Value, func: &str) -> Result<f64> {
    match val {
        Value::Int(n) => Ok(*n as f64),
        Value::Float(f) => Ok(*f),
        other => Err(BioLangError::type_error(
            format!("{func}() requires number, got {}", other.type_of()),
            None,
        )),
    }
}

fn builtin_matrix(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::List(rows) => {
            let mut data = Vec::new();
            let mut ncol = 0;
            for (i, row) in rows.iter().enumerate() {
                match row {
                    Value::List(items) => {
                        if i == 0 {
                            ncol = items.len();
                        } else if items.len() != ncol {
                            return Err(BioLangError::runtime(
                                ErrorKind::TypeError,
                                format!(
                                    "matrix() row {} has {} cols, expected {}",
                                    i,
                                    items.len(),
                                    ncol
                                ),
                                None,
                            ));
                        }
                        for item in items {
                            let v = match item {
                                Value::Int(n) => *n as f64,
                                Value::Float(f) => *f,
                                _ => {
                                    return Err(BioLangError::type_error(
                                        "matrix() requires numeric values",
                                        None,
                                    ))
                                }
                            };
                            data.push(v);
                        }
                    }
                    _ => {
                        return Err(BioLangError::type_error(
                            "matrix() requires List of Lists",
                            None,
                        ))
                    }
                }
            }
            let nrow = rows.len();
            let m = Matrix::new(data, nrow, ncol)
                .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
            Ok(Value::Matrix(m))
        }
        _ => Err(BioLangError::type_error(
            "matrix() requires List of Lists",
            None,
        )),
    }
}

fn builtin_matrix_from_table(args: Vec<Value>) -> Result<Value> {
    let table = match &args[0] {
        Value::Table(t) => t,
        other => {
            return Err(BioLangError::type_error(
                format!("matrix_from_table() requires Table, got {}", other.type_of()),
                None,
            ))
        }
    };
    let col_names: Vec<String> = match &args[1] {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Str(s) => Ok(s.clone()),
                _ => Err(BioLangError::type_error(
                    "matrix_from_table() column names must be strings",
                    None,
                )),
            })
            .collect::<Result<Vec<_>>>()?,
        _ => {
            return Err(BioLangError::type_error(
                "matrix_from_table() requires List of column names",
                None,
            ))
        }
    };

    let col_indices: Vec<usize> = col_names
        .iter()
        .map(|name| {
            table.col_index(name).ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("column '{name}' not found"),
                    None,
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let nrow = table.num_rows();
    let ncol = col_indices.len();
    let mut data = Vec::with_capacity(nrow * ncol);
    for row in &table.rows {
        for &ci in &col_indices {
            let v = match &row[ci] {
                Value::Int(n) => *n as f64,
                Value::Float(f) => *f,
                _ => f64::NAN,
            };
            data.push(v);
        }
    }

    let mut m = Matrix::new(data, nrow, ncol)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    m.col_names = Some(col_names);
    Ok(Value::Matrix(m))
}

fn builtin_matrix_to_table(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "matrix_to_table")?;
    let columns: Vec<String> = if let Some(ref names) = m.col_names {
        names.clone()
    } else {
        (0..m.ncol).map(|i| format!("V{}", i + 1)).collect()
    };
    let mut rows = Vec::with_capacity(m.nrow);
    for i in 0..m.nrow {
        let row: Vec<Value> = m.row(i).into_iter().map(Value::Float).collect();
        rows.push(row);
    }
    Ok(Value::Table(Table::new(columns, rows)))
}

fn builtin_zeros(args: Vec<Value>) -> Result<Value> {
    let nrow = require_int(&args[0], "zeros")? as usize;
    let ncol = require_int(&args[1], "zeros")? as usize;
    Ok(Value::Matrix(Matrix::zeros(nrow, ncol)))
}

fn builtin_eye(args: Vec<Value>) -> Result<Value> {
    let n = require_int(&args[0], "eye")? as usize;
    Ok(Value::Matrix(Matrix::identity(n)))
}

fn builtin_dim(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "dim")?;
    Ok(Value::List(vec![
        Value::Int(m.nrow as i64),
        Value::Int(m.ncol as i64),
    ]))
}

fn builtin_transpose(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "transpose")?;
    Ok(Value::Matrix(m.transpose()))
}

fn builtin_mat_add(args: Vec<Value>) -> Result<Value> {
    let a = require_matrix(&args[0], "mat_add")?;
    let b = require_matrix(&args[1], "mat_add")?;
    let result = a
        .add(b)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(Value::Matrix(result))
}

fn builtin_mat_sub(args: Vec<Value>) -> Result<Value> {
    let a = require_matrix(&args[0], "mat_sub")?;
    let b = require_matrix(&args[1], "mat_sub")?;
    let result = a
        .sub(b)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(Value::Matrix(result))
}

fn builtin_mat_mul(args: Vec<Value>) -> Result<Value> {
    let a = require_matrix(&args[0], "mat_mul")?;
    let b = require_matrix(&args[1], "mat_mul")?;
    let result = a
        .mul_elementwise(b)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(Value::Matrix(result))
}

fn builtin_mat_scale(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "mat_scale")?;
    let s = require_num(&args[1], "mat_scale")?;
    Ok(Value::Matrix(m.scale(s)))
}

fn builtin_dot(args: Vec<Value>) -> Result<Value> {
    let a = require_matrix(&args[0], "dot")?;
    let b = require_matrix(&args[1], "dot")?;
    let result = a
        .dot(b)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(Value::Matrix(result))
}

fn builtin_row_sums(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "row_sums")?;
    Ok(Value::List(m.row_sums().into_iter().map(Value::Float).collect()))
}

fn builtin_col_sums(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "col_sums")?;
    Ok(Value::List(m.col_sums().into_iter().map(Value::Float).collect()))
}

fn builtin_row_means(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "row_means")?;
    Ok(Value::List(
        m.row_means().into_iter().map(Value::Float).collect(),
    ))
}

fn builtin_col_means(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "col_means")?;
    Ok(Value::List(
        m.col_means().into_iter().map(Value::Float).collect(),
    ))
}

fn builtin_pca(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "pca")?;
    let n_components = require_int(&args[1], "pca")? as usize;

    // Convert matrix to Vec<Vec<f64>> for stats_ops
    let mut rows: Vec<Vec<f64>> = Vec::with_capacity(m.nrow);
    for i in 0..m.nrow {
        rows.push(m.row(i));
    }

    let res = bl_core::bio_core::stats_ops::principal_component_analysis(&rows, n_components)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;

    let mut result = HashMap::new();
    result.insert(
        "explained_variance".to_string(),
        Value::List(res.explained_variance.into_iter().map(Value::Float).collect()),
    );
    result.insert(
        "explained_variance_ratio".to_string(),
        Value::List(
            res.explained_variance_ratio
                .into_iter()
                .map(Value::Float)
                .collect(),
        ),
    );

    // components as Matrix
    if !res.components.is_empty() {
        let comp_nrow = res.components.len();
        let comp_ncol = res.components[0].len();
        let comp_data: Vec<f64> = res.components.into_iter().flatten().collect();
        if let Ok(comp_mat) = Matrix::new(comp_data, comp_nrow, comp_ncol) {
            result.insert("components".to_string(), Value::Matrix(comp_mat));
        }
    }

    // transformed data as Matrix
    if !res.transformed_data.is_empty() {
        let td_nrow = res.transformed_data.len();
        let td_ncol = res.transformed_data[0].len();
        let td_data: Vec<f64> = res.transformed_data.into_iter().flatten().collect();
        if let Ok(td_mat) = Matrix::new(td_data, td_nrow, td_ncol) {
            result.insert("transformed".to_string(), Value::Matrix(td_mat));
        }
    }

    Ok(Value::Record(result))
}

fn builtin_cor_matrix(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "cor_matrix")?;
    let ncol = m.ncol;
    let cols: Vec<Vec<f64>> = (0..ncol).map(|j| m.col(j)).collect();
    let mut data = vec![0.0; ncol * ncol];
    for i in 0..ncol {
        for j in 0..ncol {
            data[i * ncol + j] = bl_core::bio_core::stats_ops::pearson_correlation(&cols[i], &cols[j]);
        }
    }
    let result = Matrix::new(data, ncol, ncol)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(Value::Matrix(result))
}

fn builtin_cov_matrix(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "cov_matrix")?;
    let ncol = m.ncol;
    let nrow = m.nrow;
    let cols: Vec<Vec<f64>> = (0..ncol).map(|j| m.col(j)).collect();
    let means: Vec<f64> = cols.iter().map(|c| c.iter().sum::<f64>() / nrow as f64).collect();
    let mut data = vec![0.0; ncol * ncol];
    for i in 0..ncol {
        for j in 0..ncol {
            let cov: f64 = (0..nrow)
                .map(|k| (cols[i][k] - means[i]) * (cols[j][k] - means[j]))
                .sum::<f64>()
                / (nrow - 1) as f64;
            data[i * ncol + j] = cov;
        }
    }
    let result = Matrix::new(data, ncol, ncol)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(Value::Matrix(result))
}

fn builtin_dist_matrix(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "dist_matrix")?;
    let method = match &args[1] {
        Value::Str(s) => s.as_str(),
        _ => {
            return Err(BioLangError::type_error(
                "dist_matrix() requires Str method",
                None,
            ))
        }
    };

    let nrow = m.nrow;
    let mut data = vec![0.0; nrow * nrow];
    for i in 0..nrow {
        let ri = m.row(i);
        for j in (i + 1)..nrow {
            let rj = m.row(j);
            let d = match method {
                "euclidean" => {
                    ri.iter()
                        .zip(&rj)
                        .map(|(a, b)| (a - b).powi(2))
                        .sum::<f64>()
                        .sqrt()
                }
                "manhattan" => ri.iter().zip(&rj).map(|(a, b)| (a - b).abs()).sum::<f64>(),
                _ => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!("dist_matrix() unknown method '{method}'"),
                        None,
                    ))
                }
            };
            data[i * nrow + j] = d;
            data[j * nrow + i] = d;
        }
    }
    let result = Matrix::new(data, nrow, nrow)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(Value::Matrix(result))
}

fn builtin_row(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "row")?;
    let i = match &args[1] {
        Value::Int(n) => *n as usize,
        other => {
            return Err(BioLangError::type_error(
                format!("row() index must be Int, got {}", other.type_of()),
                None,
            ))
        }
    };
    if i >= m.nrow {
        return Err(BioLangError::runtime(
            ErrorKind::IndexOutOfBounds,
            format!("row index {} out of bounds for {}x{} matrix", i, m.nrow, m.ncol),
            None,
        ));
    }
    Ok(Value::List(m.row(i).into_iter().map(Value::Float).collect()))
}

fn builtin_mat_col(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "mat_col")?;
    let j = match &args[1] {
        Value::Int(n) => *n as usize,
        other => {
            return Err(BioLangError::type_error(
                format!("mat_col() index must be Int, got {}", other.type_of()),
                None,
            ))
        }
    };
    if j >= m.ncol {
        return Err(BioLangError::runtime(
            ErrorKind::IndexOutOfBounds,
            format!("col index {} out of bounds for {}x{} matrix", j, m.nrow, m.ncol),
            None,
        ));
    }
    Ok(Value::List(m.col(j).into_iter().map(Value::Float).collect()))
}

fn builtin_trace(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "trace")?;
    let t = m.trace().map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(Value::Float(t))
}

fn builtin_norm(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "norm")?;
    Ok(Value::Float(m.norm()))
}

fn builtin_determinant(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "determinant")?;
    let d = m.determinant().map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(Value::Float(d))
}

fn builtin_inverse(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "inverse")?;
    let inv = m.inverse().map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(Value::Matrix(inv))
}

fn builtin_solve(args: Vec<Value>) -> Result<Value> {
    let a = require_matrix(&args[0], "solve")?;
    let b_vec: Vec<f64> = match &args[1] {
        Value::List(items) => items
            .iter()
            .enumerate()
            .map(|(i, v)| match v {
                Value::Int(n) => Ok(*n as f64),
                Value::Float(f) => Ok(*f),
                other => Err(BioLangError::type_error(
                    format!("solve() b[{}] must be numeric, got {}", i, other.type_of()),
                    None,
                )),
            })
            .collect::<Result<Vec<_>>>()?,
        other => {
            return Err(BioLangError::type_error(
                format!("solve() requires List for b, got {}", other.type_of()),
                None,
            ))
        }
    };
    let x = a.solve(&b_vec).map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(Value::List(x.into_iter().map(Value::Float).collect()))
}

fn builtin_eigenvalues(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "eigenvalues")?;
    let (vals, vecs) = m.eigen().map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    let mut result = HashMap::new();
    result.insert(
        "values".to_string(),
        Value::List(vals.into_iter().map(Value::Float).collect()),
    );
    result.insert("vectors".to_string(), Value::Matrix(vecs));
    Ok(Value::Record(result))
}

fn builtin_svd(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "svd")?;
    let (u, s, vt) = m.svd().map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    let mut result = HashMap::new();
    result.insert("u".to_string(), Value::Matrix(u));
    result.insert(
        "d".to_string(),
        Value::List(s.into_iter().map(Value::Float).collect()),
    );
    result.insert("vt".to_string(), Value::Matrix(vt));
    Ok(Value::Record(result))
}

fn builtin_rank(args: Vec<Value>) -> Result<Value> {
    let m = require_matrix(&args[0], "rank")?;
    let r = m.rank().map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(Value::Int(r as i64))
}

fn builtin_ones(args: Vec<Value>) -> Result<Value> {
    let nrow = require_int(&args[0], "ones")? as usize;
    let ncol = require_int(&args[1], "ones")? as usize;
    let data = vec![1.0; nrow * ncol];
    let m = Matrix::new(data, nrow, ncol)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(Value::Matrix(m))
}

fn builtin_diag(args: Vec<Value>) -> Result<Value> {
    let values: Vec<f64> = match &args[0] {
        Value::List(items) => items
            .iter()
            .enumerate()
            .map(|(i, v)| match v {
                Value::Int(n) => Ok(*n as f64),
                Value::Float(f) => Ok(*f),
                other => Err(BioLangError::type_error(
                    format!("diag() element {} must be numeric, got {}", i, other.type_of()),
                    None,
                )),
            })
            .collect::<Result<Vec<_>>>()?,
        other => {
            return Err(BioLangError::type_error(
                format!("diag() requires List, got {}", other.type_of()),
                None,
            ))
        }
    };
    let n = values.len();
    let mut data = vec![0.0; n * n];
    for (i, &v) in values.iter().enumerate() {
        data[i * n + i] = v;
    }
    let m = Matrix::new(data, n, n)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, e, None))?;
    Ok(Value::Matrix(m))
}

