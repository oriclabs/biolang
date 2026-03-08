use bl_core::error::{BioLangError, Result};
use bl_core::matrix::Matrix;
use bl_core::sparse_matrix::SparseMatrix;
use bl_core::value::{Arity, Value};

/// Returns the list of (name, arity) for all sparse matrix builtins.
pub fn sparse_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("sparse_matrix", Arity::Range(1, 3)),
        ("to_dense", Arity::Exact(1)),
        ("to_sparse", Arity::Exact(1)),
        ("sparse_get", Arity::Exact(3)),
        ("nnz", Arity::Exact(1)),
        ("sparse_row_sums", Arity::Exact(1)),
        ("sparse_col_sums", Arity::Exact(1)),
        ("normalize_sparse", Arity::Exact(2)),
    ]
}

/// Check if a name is a known sparse builtin.
pub fn is_sparse_builtin(name: &str) -> bool {
    matches!(
        name,
        "sparse_matrix"
            | "to_dense"
            | "to_sparse"
            | "sparse_get"
            | "nnz"
            | "sparse_row_sums"
            | "sparse_col_sums"
            | "normalize_sparse"
    )
}

/// Execute a sparse matrix builtin by name.
pub fn call_sparse_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "sparse_matrix" => builtin_sparse_matrix(args),
        "to_dense" => builtin_to_dense(args),
        "to_sparse" => builtin_to_sparse(args),
        "sparse_get" => builtin_sparse_get(args),
        "nnz" => builtin_nnz(args),
        "sparse_row_sums" => builtin_sparse_row_sums(args),
        "sparse_col_sums" => builtin_sparse_col_sums(args),
        "normalize_sparse" => builtin_normalize_sparse(args),
        _ => Err(BioLangError::runtime(
            bl_core::error::ErrorKind::NameError,
            &format!("unknown sparse builtin: {name}"),
            None,
        )),
    }
}

fn builtin_sparse_matrix(args: Vec<Value>) -> Result<Value> {
    // 3-arg form: sparse_matrix(nrow, ncol, entries) where entries is List[Record{row, col, val}]
    if args.len() == 3 {
        let nrow = match &args[0] {
            Value::Int(n) => *n as usize,
            other => return Err(BioLangError::type_error(
                format!("sparse_matrix() nrow must be Int, got {}", other.type_of()), None,
            )),
        };
        let ncol = match &args[1] {
            Value::Int(n) => *n as usize,
            other => return Err(BioLangError::type_error(
                format!("sparse_matrix() ncol must be Int, got {}", other.type_of()), None,
            )),
        };
        let entries = match &args[2] {
            Value::List(items) => items,
            other => return Err(BioLangError::type_error(
                format!("sparse_matrix() entries must be List, got {}", other.type_of()), None,
            )),
        };
        let mut rows = Vec::with_capacity(entries.len());
        let mut cols = Vec::with_capacity(entries.len());
        let mut vals = Vec::with_capacity(entries.len());
        for entry in entries {
            match entry {
                Value::Record(map) => {
                    let r = match map.get("row") {
                        Some(Value::Int(n)) => *n as usize,
                        Some(Value::Float(f)) => *f as usize,
                        _ => return Err(BioLangError::type_error("sparse_matrix() entry missing 'row'", None)),
                    };
                    let c = match map.get("col") {
                        Some(Value::Int(n)) => *n as usize,
                        Some(Value::Float(f)) => *f as usize,
                        _ => return Err(BioLangError::type_error("sparse_matrix() entry missing 'col'", None)),
                    };
                    let v = match map.get("val") {
                        Some(Value::Float(f)) => *f,
                        Some(Value::Int(n)) => *n as f64,
                        _ => return Err(BioLangError::type_error("sparse_matrix() entry missing 'val'", None)),
                    };
                    rows.push(r);
                    cols.push(c);
                    vals.push(v);
                }
                other => return Err(BioLangError::type_error(
                    format!("sparse_matrix() entries must be Records, got {}", other.type_of()), None,
                )),
            }
        }
        let sm = SparseMatrix::from_triplets(&rows, &cols, &vals, nrow, ncol);
        return Ok(Value::SparseMatrix(sm));
    }

    match &args[0] {
        // From triplets: Record{rows: List[Int], cols: List[Int], vals: List[Float], nrow: Int, ncol: Int}
        Value::Record(map) => {
            let rows = extract_usize_list(map.get("rows"), "rows")?;
            let cols = extract_usize_list(map.get("cols"), "cols")?;
            let vals = extract_f64_list(map.get("vals"), "vals")?;
            let nrow = extract_usize(map.get("nrow"), "nrow")?;
            let ncol = extract_usize(map.get("ncol"), "ncol")?;
            let sm = SparseMatrix::from_triplets(&rows, &cols, &vals, nrow, ncol);
            Ok(Value::SparseMatrix(sm))
        }
        // From nested lists (dense)
        Value::List(outer) => {
            let mut dense = Vec::new();
            for row_val in outer {
                match row_val {
                    Value::List(inner) => {
                        let row: Vec<f64> = inner
                            .iter()
                            .map(|v| match v {
                                Value::Float(f) => *f,
                                Value::Int(n) => *n as f64,
                                _ => 0.0,
                            })
                            .collect();
                        dense.push(row);
                    }
                    _ => {
                        return Err(BioLangError::type_error(
                            "sparse_matrix() from List requires nested Lists",
                            None,
                        ))
                    }
                }
            }
            let sm = SparseMatrix::from_dense(&dense);
            Ok(Value::SparseMatrix(sm))
        }
        other => Err(BioLangError::type_error(
            format!(
                "sparse_matrix() requires Record{{rows,cols,vals,nrow,ncol}} or List[List], got {}",
                other.type_of()
            ),
            None,
        )),
    }
}

fn builtin_to_dense(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::SparseMatrix(sm) => {
            let dense = sm.to_dense();
            let data: Vec<f64> = dense.iter().flat_map(|row| row.iter().copied()).collect();
            let mut m = Matrix::new(data, sm.nrow, sm.ncol)
                .map_err(|e| BioLangError::runtime(bl_core::error::ErrorKind::TypeError, &e, None))?;
            m.row_names = sm.row_names.clone();
            m.col_names = sm.col_names.clone();
            Ok(Value::Matrix(m))
        }
        other => Err(BioLangError::type_error(
            format!("to_dense() requires SparseMatrix, got {}", other.type_of()),
            None,
        )),
    }
}

fn builtin_to_sparse(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Matrix(m) => {
            let mut dense = Vec::with_capacity(m.nrow);
            for i in 0..m.nrow {
                dense.push(m.row(i));
            }
            let mut sm = SparseMatrix::from_dense(&dense);
            sm.row_names = m.row_names.clone();
            sm.col_names = m.col_names.clone();
            Ok(Value::SparseMatrix(sm))
        }
        other => Err(BioLangError::type_error(
            format!("to_sparse() requires Matrix, got {}", other.type_of()),
            None,
        )),
    }
}

fn builtin_sparse_get(args: Vec<Value>) -> Result<Value> {
    let sm = match &args[0] {
        Value::SparseMatrix(sm) => sm,
        other => {
            return Err(BioLangError::type_error(
                format!("sparse_get() requires SparseMatrix, got {}", other.type_of()),
                None,
            ))
        }
    };
    let i = match &args[1] {
        Value::Int(n) => *n as usize,
        other => {
            return Err(BioLangError::type_error(
                format!("sparse_get() row index must be Int, got {}", other.type_of()),
                None,
            ))
        }
    };
    let j = match &args[2] {
        Value::Int(n) => *n as usize,
        other => {
            return Err(BioLangError::type_error(
                format!("sparse_get() col index must be Int, got {}", other.type_of()),
                None,
            ))
        }
    };
    Ok(Value::Float(sm.get(i, j)))
}

fn builtin_nnz(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::SparseMatrix(sm) => Ok(Value::Int(sm.nnz() as i64)),
        other => Err(BioLangError::type_error(
            format!("nnz() requires SparseMatrix, got {}", other.type_of()),
            None,
        )),
    }
}

fn builtin_sparse_row_sums(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::SparseMatrix(sm) => {
            let sums = sm.row_sums();
            Ok(Value::List(sums.into_iter().map(Value::Float).collect()))
        }
        other => Err(BioLangError::type_error(
            format!("sparse_row_sums() requires SparseMatrix, got {}", other.type_of()),
            None,
        )),
    }
}

fn builtin_sparse_col_sums(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::SparseMatrix(sm) => {
            let sums = sm.col_sums();
            Ok(Value::List(sums.into_iter().map(Value::Float).collect()))
        }
        other => Err(BioLangError::type_error(
            format!("sparse_col_sums() requires SparseMatrix, got {}", other.type_of()),
            None,
        )),
    }
}

fn builtin_normalize_sparse(args: Vec<Value>) -> Result<Value> {
    let sm = match &args[0] {
        Value::SparseMatrix(sm) => sm,
        other => {
            return Err(BioLangError::type_error(
                format!("normalize_sparse() requires SparseMatrix, got {}", other.type_of()),
                None,
            ))
        }
    };
    let method = match &args[1] {
        Value::Str(s) => s.as_str(),
        other => {
            return Err(BioLangError::type_error(
                format!("normalize_sparse() method must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let result = match method {
        "log1p_cpm" => sm.normalize_log1p_cpm(),
        "scale" => sm.normalize_scale(),
        _ => {
            return Err(BioLangError::runtime(
                bl_core::error::ErrorKind::TypeError,
                &format!("normalize_sparse() unknown method: {method}. Use 'log1p_cpm' or 'scale'"),
                None,
            ))
        }
    };
    Ok(Value::SparseMatrix(result))
}

// ── Helpers ──────────────────────────────────────────────────────

fn extract_usize_list(val: Option<&Value>, field: &str) -> Result<Vec<usize>> {
    match val {
        Some(Value::List(items)) => items
            .iter()
            .map(|v| match v {
                Value::Int(n) => Ok(*n as usize),
                other => Err(BioLangError::type_error(
                    format!("sparse_matrix() {field} must contain Ints, got {}", other.type_of()),
                    None,
                )),
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("sparse_matrix() missing or invalid field: {field}"),
            None,
        )),
    }
}

fn extract_f64_list(val: Option<&Value>, field: &str) -> Result<Vec<f64>> {
    match val {
        Some(Value::List(items)) => items
            .iter()
            .map(|v| match v {
                Value::Float(f) => Ok(*f),
                Value::Int(n) => Ok(*n as f64),
                other => Err(BioLangError::type_error(
                    format!("sparse_matrix() {field} must contain numbers, got {}", other.type_of()),
                    None,
                )),
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("sparse_matrix() missing or invalid field: {field}"),
            None,
        )),
    }
}

fn extract_usize(val: Option<&Value>, field: &str) -> Result<usize> {
    match val {
        Some(Value::Int(n)) => Ok(*n as usize),
        _ => Err(BioLangError::type_error(
            format!("sparse_matrix() missing or invalid field: {field}"),
            None,
        )),
    }
}
