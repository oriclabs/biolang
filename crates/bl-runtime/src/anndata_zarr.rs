//! Native AnnData `.zarr` reader/writer (pure Rust, no C/HDF5).
//!
//! Implements the subset of the Zarr v2 spec that AnnData uses, so scRNA-seq
//! objects can be exchanged with Scanpy/anndata without a container or libhdf5:
//!
//! - `read_anndata(path)`  → sc object `{matrix, genes, barcodes, n_cells, n_genes}`
//! - `write_anndata(path, obj)` → writes an AnnData `.zarr` store
//!
//! Supported: dense and CSR/CSC `X`, `obs/_index` + `var/_index` string arrays,
//! numeric dtypes `<f4/<f8/<i4/<i8/<u4/…`, and the `blosc`/`gzip`/`zlib`/raw
//! compressors. `blosc` — the zarr and anndata default — is handled by
//! [`crate::blosc`], a pure-Rust decoder, so this path needs no C libraries.
//! Blosc chunks using the `snappy` inner codec are the one remaining gap.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::sparse_matrix::SparseMatrix;
use bl_core::value::{Arity, Table, Value};

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub fn anndata_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("read_anndata", Arity::Exact(1)),
        ("write_anndata", Arity::Exact(2)),
    ]
}

pub fn is_anndata_builtin(name: &str) -> bool {
    matches!(name, "read_anndata" | "write_anndata")
}

pub fn call_anndata_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "read_anndata" => {
            let path = str_arg(&args[0], "read_anndata")?;
            read_anndata_dir(Path::new(&path))
        }
        "write_anndata" => {
            let path = str_arg(&args[0], "write_anndata")?;
            write_anndata_dir(Path::new(&path), &args[1])?;
            Ok(Value::Str(path))
        }
        _ => Err(io_err(format!("unknown anndata builtin '{name}'"))),
    }
}

fn io_err(msg: impl Into<String>) -> BioLangError {
    BioLangError::runtime(ErrorKind::IOError, msg.into(), None)
}

fn str_arg(v: &Value, func: &str) -> Result<String> {
    match v {
        Value::Str(s) => Ok(s.clone()),
        other => Err(BioLangError::type_error(
            format!("{func}() requires a path (Str), got {}", other.type_of()),
            None,
        )),
    }
}

// ── Zarr v2 primitives ───────────────────────────────────────────────────────

fn read_json(path: &Path) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

struct ZArray {
    dir: PathBuf,
    shape: Vec<usize>,
    chunks: Vec<usize>,
    dtype: String,
    compressor: Option<String>,
    is_vlen_utf8: bool,
    fill_f64: f64,
}

fn usize_vec(v: &serde_json::Value) -> Result<Vec<usize>> {
    v.as_array()
        .ok_or_else(|| io_err("expected JSON array"))?
        .iter()
        .map(|x| {
            x.as_u64()
                .map(|n| n as usize)
                .ok_or_else(|| io_err("expected non-negative integer"))
        })
        .collect()
}

fn open_array(dir: &Path) -> Result<ZArray> {
    let meta = read_json(&dir.join(".zarray"))
        .ok_or_else(|| io_err(format!("no .zarray in {}", dir.display())))?;
    let shape = usize_vec(&meta["shape"])?;
    let chunks = usize_vec(&meta["chunks"])?;
    let dtype = meta["dtype"].as_str().unwrap_or("<f8").to_string();

    let compressor = match &meta["compressor"] {
        serde_json::Value::Null => None,
        c => {
            let id = c["id"].as_str().unwrap_or("");
            match id {
                "gzip" | "zlib" | "blosc" => Some(id.to_string()),
                other => {
                    return Err(io_err(format!(
                        "unsupported zarr compressor '{other}' in {}; re-save the AnnData store with a blosc, gzip or zlib compressor",
                        dir.display()
                    )))
                }
            }
        }
    };

    let is_vlen_utf8 = meta["filters"]
        .as_array()
        .map(|fs| fs.iter().any(|f| f["id"].as_str() == Some("vlen-utf8")))
        .unwrap_or(false);

    let fill_f64 = meta["fill_value"].as_f64().unwrap_or(0.0);

    Ok(ZArray {
        dir: dir.to_path_buf(),
        shape,
        chunks,
        dtype,
        compressor,
        is_vlen_utf8,
        fill_f64,
    })
}

impl ZArray {
    fn decompress(&self, raw: Vec<u8>) -> Result<Vec<u8>> {
        match self.compressor.as_deref() {
            None => Ok(raw),
            Some("gzip") => {
                let mut d = flate2::read::GzDecoder::new(&raw[..]);
                let mut out = Vec::new();
                d.read_to_end(&mut out)
                    .map_err(|e| io_err(format!("gzip: {e}")))?;
                Ok(out)
            }
            Some("zlib") => {
                let mut d = flate2::read::ZlibDecoder::new(&raw[..]);
                let mut out = Vec::new();
                d.read_to_end(&mut out)
                    .map_err(|e| io_err(format!("zlib: {e}")))?;
                Ok(out)
            }
            Some("blosc") => crate::blosc::decompress(&raw),
            Some(other) => Err(io_err(format!("unsupported compressor '{other}'"))),
        }
    }

    fn chunk_bytes(&self, coords: &[usize]) -> Result<Option<Vec<u8>>> {
        let name = if coords.is_empty() {
            "0".to_string()
        } else {
            coords
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(".")
        };
        let path = self.dir.join(&name);
        if !path.exists() {
            return Ok(None);
        }
        let raw = std::fs::read(&path).map_err(|e| io_err(format!("read chunk: {e}")))?;
        Ok(Some(self.decompress(raw)?))
    }

    /// Assemble the full array (C-order flat) from its chunks, using `decode`
    /// to turn a decompressed chunk into `chunk_len` elements.
    fn assemble<T: Clone>(&self, fill: T, decode: impl Fn(&[u8]) -> Vec<T>) -> Result<Vec<T>> {
        let ndim = self.shape.len();
        let total: usize = self.shape.iter().product();
        if total == 0 {
            return Ok(Vec::new());
        }
        let mut out = vec![fill; total];
        let chunk_len: usize = self.chunks.iter().product();
        if chunk_len == 0 {
            return Ok(out);
        }
        let nchunks: Vec<usize> = (0..ndim)
            .map(|d| self.shape[d].div_ceil(self.chunks[d]))
            .collect();
        let total_chunks: usize = nchunks.iter().product::<usize>().max(1);

        for ci in 0..total_chunks {
            let cc = unravel(ci, &nchunks);
            let Some(bytes) = self.chunk_bytes(&cc)? else {
                continue; // missing chunk → fill value
            };
            let elems = decode(&bytes);
            for li in 0..chunk_len {
                let lc = unravel(li, &self.chunks);
                let mut gc = vec![0usize; ndim];
                let mut inbounds = true;
                for d in 0..ndim {
                    gc[d] = cc[d] * self.chunks[d] + lc[d];
                    if gc[d] >= self.shape[d] {
                        inbounds = false;
                        break;
                    }
                }
                if inbounds && li < elems.len() {
                    out[ravel(&gc, &self.shape)] = elems[li].clone();
                }
            }
        }
        Ok(out)
    }

    fn read_numeric(&self) -> Result<Vec<f64>> {
        let (order, kind, width) = parse_dtype(&self.dtype)?;
        let le = order != '>';
        let fill = self.fill_f64;
        self.assemble(fill, |bytes| decode_numeric(bytes, kind, width, le))
    }

    fn read_strings(&self) -> Result<Vec<String>> {
        if !self.is_vlen_utf8 {
            return Err(io_err(format!(
                "{} is not a vlen-utf8 string array",
                self.dir.display()
            )));
        }
        self.assemble(String::new(), decode_vlen_utf8)
    }
}

fn unravel(mut idx: usize, dims: &[usize]) -> Vec<usize> {
    let n = dims.len();
    let mut coord = vec![0usize; n];
    for d in (0..n).rev() {
        coord[d] = idx % dims[d];
        idx /= dims[d];
    }
    coord
}

fn ravel(coord: &[usize], dims: &[usize]) -> usize {
    let mut idx = 0;
    for d in 0..dims.len() {
        idx = idx * dims[d] + coord[d];
    }
    idx
}

fn parse_dtype(dt: &str) -> Result<(char, char, usize)> {
    let chars: Vec<char> = dt.chars().collect();
    if chars.len() < 3 {
        return Err(io_err(format!("cannot parse numeric dtype '{dt}'")));
    }
    let width: usize = dt[2..]
        .parse()
        .map_err(|_| io_err(format!("bad dtype width in '{dt}'")))?;
    Ok((chars[0], chars[1], width))
}

fn decode_numeric(bytes: &[u8], kind: char, width: usize, le: bool) -> Vec<f64> {
    if width == 0 {
        return Vec::new();
    }
    let n = bytes.len() / width;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let s = &bytes[i * width..i * width + width];
        let v = match (kind, width) {
            ('f', 8) => f64::from_le_bytes(le8(s, le)),
            ('f', 4) => f32::from_le_bytes(le4(s, le)) as f64,
            ('i', 8) => i64::from_le_bytes(le8(s, le)) as f64,
            ('i', 4) => i32::from_le_bytes(le4(s, le)) as f64,
            ('i', 2) => i16::from_le_bytes(le2(s, le)) as f64,
            ('i', 1) => (s[0] as i8) as f64,
            ('u', 8) => u64::from_le_bytes(le8(s, le)) as f64,
            ('u', 4) => u32::from_le_bytes(le4(s, le)) as f64,
            ('u', 2) => u16::from_le_bytes(le2(s, le)) as f64,
            ('u', 1) | ('b', 1) => s[0] as f64,
            _ => 0.0,
        };
        out.push(v);
    }
    out
}

fn le2(s: &[u8], le: bool) -> [u8; 2] {
    let mut a = [s[0], s[1]];
    if !le {
        a.reverse();
    }
    a
}
fn le4(s: &[u8], le: bool) -> [u8; 4] {
    let mut a = [s[0], s[1], s[2], s[3]];
    if !le {
        a.reverse();
    }
    a
}
fn le8(s: &[u8], le: bool) -> [u8; 8] {
    let mut a = [s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]];
    if !le {
        a.reverse();
    }
    a
}

/// numcodecs VLenUTF8: [u32 count][per element: u32 len + utf8 bytes].
fn decode_vlen_utf8(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if bytes.len() < 4 {
        return out;
    }
    let count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let mut i = 4;
    for _ in 0..count {
        if i + 4 > bytes.len() {
            break;
        }
        let len = u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]) as usize;
        i += 4;
        if i + len > bytes.len() {
            break;
        }
        out.push(String::from_utf8_lossy(&bytes[i..i + len]).into_owned());
        i += len;
    }
    out
}

// ── AnnData read ─────────────────────────────────────────────────────────────

fn read_anndata_dir(root: &Path) -> Result<Value> {
    if !root.exists() {
        return Err(io_err(format!("{} does not exist", root.display())));
    }
    let x_dir = root.join("X");

    let (mut matrix, n_obs, n_var, is_sparse) = if x_dir.join(".zarray").exists() {
        // Dense 2D X.
        let arr = open_array(&x_dir)?;
        if arr.shape.len() != 2 {
            return Err(io_err("X must be 2-dimensional"));
        }
        let (r, c) = (arr.shape[0], arr.shape[1]);
        (
            dense_matrix_value(reshape(arr.read_numeric()?, r, c)),
            r,
            c,
            false,
        )
    } else if x_dir.join(".zgroup").exists() {
        // Sparse CSR/CSC X.
        let attrs = read_json(&x_dir.join(".zattrs")).unwrap_or(serde_json::Value::Null);
        let enc = attrs["encoding-type"].as_str().unwrap_or("csr_matrix");
        let shape = usize_vec(&attrs["shape"])
            .map_err(|_| io_err("sparse X missing 'shape' in .zattrs"))?;
        let (n_obs, n_var) = (shape[0], shape[1]);
        let data = open_array(&x_dir.join("data"))?.read_numeric()?;
        let indices: Vec<usize> = open_array(&x_dir.join("indices"))?
            .read_numeric()?
            .iter()
            .map(|v| *v as usize)
            .collect();
        let indptr: Vec<usize> = open_array(&x_dir.join("indptr"))?
            .read_numeric()?
            .iter()
            .map(|v| *v as usize)
            .collect();
        let sparse = if enc.contains("csc") {
            sparse_from_csc(data, indices, indptr, n_obs, n_var)?
        } else {
            sparse_from_csr(data, indices, indptr, n_obs, n_var)?
        };
        (Value::SparseMatrix(std::sync::Arc::new(sparse)), n_obs, n_var, true)
    } else {
        return Err(io_err(format!(
            "no X array or group under {}",
            root.display()
        )));
    };

    let genes = read_index(&root.join("var"), n_var)?;
    let barcodes = read_index(&root.join("obs"), n_obs)?;
    if let Value::SparseMatrix(sparse) = &mut matrix {
        // Names arrive after the matrix is built, and the Arc is not shared yet,
        // so make_mut writes in place rather than cloning the whole matrix.
        let owned = std::sync::Arc::make_mut(sparse);
        owned.row_names = Some(barcodes.clone());
        owned.col_names = Some(genes.clone());
    }

    let mut rec = HashMap::new();
    rec.insert("matrix".into(), matrix.clone());
    rec.insert(
        "genes".into(),
        Value::List(
            genes
                .iter()
                .cloned()
                .map(Value::Str)
                .collect::<Vec<_>>()
                .into(),
        ),
    );
    rec.insert(
        "barcodes".into(),
        Value::List(
            barcodes
                .iter()
                .cloned()
                .map(Value::Str)
                .collect::<Vec<_>>()
                .into(),
        ),
    );
    rec.insert("n_cells".into(), Value::Int(n_obs as i64));
    rec.insert("n_genes".into(), Value::Int(n_var as i64));
    rec.insert("is_sparse".into(), Value::Bool(is_sparse));
    rec.insert(
        "obs".into(),
        Value::Table(Table::new(
            vec!["barcode".into()],
            barcodes
                .iter()
                .map(|barcode| vec![Value::Str(barcode.clone())])
                .collect(),
        )),
    );
    rec.insert(
        "var".into(),
        Value::Table(Table::new(
            vec!["gene".into()],
            genes
                .iter()
                .map(|gene| vec![Value::Str(gene.clone())])
                .collect(),
        )),
    );
    let mut layers = HashMap::new();
    layers.insert("X".into(), matrix);
    rec.insert("layers".into(), Value::Record(layers.into()));
    Ok(Value::Record((rec).into()))
}

/// Read a dataframe's `_index` string array (the name of the index column is in
/// the group's `.zattrs["_index"]`, defaulting to `_index`). Falls back to
/// positional names if absent.
fn read_index(df_dir: &Path, n: usize) -> Result<Vec<String>> {
    let index_name = read_json(&df_dir.join(".zattrs"))
        .and_then(|a| a["_index"].as_str().map(String::from))
        .unwrap_or_else(|| "_index".to_string());
    let index_dir = df_dir.join(&index_name);
    if index_dir.join(".zarray").exists() {
        let names = open_array(&index_dir)?.read_strings()?;
        if names.len() == n {
            return Ok(names);
        }
    }
    Ok((0..n).map(|i| i.to_string()).collect())
}

fn reshape(flat: Vec<f64>, rows: usize, cols: usize) -> Vec<Vec<f64>> {
    let mut out = Vec::with_capacity(rows);
    for r in 0..rows {
        out.push(flat[r * cols..(r + 1) * cols].to_vec());
    }
    out
}

fn dense_matrix_value(matrix: Vec<Vec<f64>>) -> Value {
    Value::List(
        matrix
            .into_iter()
            .map(|row| Value::List(row.into_iter().map(Value::Float).collect::<Vec<_>>().into()))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn sparse_from_csr(
    data: Vec<f64>,
    indices: Vec<usize>,
    indptr: Vec<usize>,
    n_obs: usize,
    n_var: usize,
) -> Result<SparseMatrix> {
    if indptr.len() != n_obs + 1
        || data.len() != indices.len()
        || indptr.last().copied() != Some(data.len())
        || indices.iter().any(|index| *index >= n_var)
    {
        return Err(io_err("invalid CSR arrays in AnnData X"));
    }
    Ok(SparseMatrix {
        indptr,
        indices,
        data,
        nrow: n_obs,
        ncol: n_var,
        row_names: None,
        col_names: None,
    })
}

fn sparse_from_csc(
    data: Vec<f64>,
    indices: Vec<usize>,
    indptr: Vec<usize>,
    n_obs: usize,
    n_var: usize,
) -> Result<SparseMatrix> {
    if indptr.len() != n_var + 1
        || data.len() != indices.len()
        || indptr.last().copied() != Some(data.len())
        || indices.iter().any(|index| *index >= n_obs)
    {
        return Err(io_err("invalid CSC arrays in AnnData X"));
    }
    let mut rows = Vec::with_capacity(data.len());
    let mut columns = Vec::with_capacity(data.len());
    let mut values = Vec::with_capacity(data.len());
    for c in 0..n_var {
        for k in indptr[c]..indptr[c + 1] {
            rows.push(indices[k]);
            columns.push(c);
            values.push(data[k]);
        }
    }
    Ok(SparseMatrix::from_triplets(
        &rows, &columns, &values, n_obs, n_var,
    ))
}

// ── AnnData write ────────────────────────────────────────────────────────────

fn write_anndata_dir(root: &Path, obj: &Value) -> Result<()> {
    let rec = match obj {
        Value::Record(m) | Value::Map(m) => m,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "write_anndata() requires a single-cell object (Record), got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };

    let matrix = rec
        .get("matrix")
        .ok_or_else(|| io_err("write_anndata(): object has no 'matrix' field"))?;
    let (n_obs, n_var) = match matrix {
        Value::SparseMatrix(matrix) => (matrix.nrow, matrix.ncol),
        Value::List(_) => {
            let dense = get_matrix(rec)?;
            (dense.len(), dense.first().map(|row| row.len()).unwrap_or(0))
        }
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "write_anndata(): 'matrix' must be a Matrix or SparseMatrix, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    let genes = get_str_list(rec, "genes", n_var);
    let barcodes = get_str_list(rec, "barcodes", n_obs);

    std::fs::create_dir_all(root).map_err(|e| io_err(format!("mkdir: {e}")))?;
    write_json(
        &root.join(".zgroup"),
        &serde_json::json!({"zarr_format": 2}),
    )?;
    write_json(
        &root.join(".zattrs"),
        &serde_json::json!({"encoding-type": "anndata", "encoding-version": "0.1.0"}),
    )?;

    match matrix {
        Value::SparseMatrix(matrix) => write_sparse_matrix(&root.join("X"), matrix)?,
        Value::List(_) => {
            let dense = get_matrix(rec)?;
            let flat: Vec<f64> = dense.iter().flatten().copied().collect();
            write_numeric_array(&root.join("X"), &[n_obs, n_var], &flat)?;
        }
        _ => unreachable!(),
    }

    // var / obs dataframes with a string _index.
    write_dataframe(&root.join("var"), &genes)?;
    write_dataframe(&root.join("obs"), &barcodes)?;
    Ok(())
}

fn get_matrix(rec: &HashMap<String, Value>) -> Result<Vec<Vec<f64>>> {
    let m = rec
        .get("matrix")
        .ok_or_else(|| io_err("write_anndata(): object has no 'matrix' field"))?;
    let rows = match m {
        Value::List(r) => r,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "write_anndata(): 'matrix' must be a List of rows, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    let mut out = Vec::with_capacity(rows.len());
    for row in rows.iter() {
        match row {
            Value::List(cells) => out.push(
                cells
                    .iter()
                    .map(|v| match v {
                        Value::Float(f) => *f,
                        Value::Int(n) => *n as f64,
                        _ => 0.0,
                    })
                    .collect(),
            ),
            other => {
                return Err(BioLangError::type_error(
                    format!(
                        "write_anndata(): matrix rows must be Lists, got {}",
                        other.type_of()
                    ),
                    None,
                ))
            }
        }
    }
    Ok(out)
}

fn get_str_list(rec: &HashMap<String, Value>, key: &str, n: usize) -> Vec<String> {
    match rec.get(key) {
        Some(Value::List(items)) => items
            .iter()
            .map(|v| match v {
                Value::Str(s) => s.clone(),
                other => format!("{other:?}"),
            })
            .collect(),
        _ => (0..n).map(|i| i.to_string()).collect(),
    }
}

fn write_json(path: &Path, value: &serde_json::Value) -> Result<()> {
    let text = serde_json::to_string(value).map_err(|e| io_err(format!("json: {e}")))?;
    std::fs::write(path, text).map_err(|e| io_err(format!("write {}: {e}", path.display())))
}

fn gzip(data: &[u8]) -> Result<Vec<u8>> {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(data)
        .map_err(|e| io_err(format!("gzip: {e}")))?;
    enc.finish().map_err(|e| io_err(format!("gzip: {e}")))
}

fn write_numeric_array(dir: &Path, shape: &[usize], flat: &[f64]) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(|e| io_err(format!("mkdir: {e}")))?;
    let zarray = serde_json::json!({
        "zarr_format": 2,
        "shape": shape,
        "chunks": shape,          // single chunk
        "dtype": "<f8",
        "compressor": {"id": "gzip", "level": 5},
        "fill_value": 0.0,
        "order": "C",
        "filters": serde_json::Value::Null,
    });
    write_json(&dir.join(".zarray"), &zarray)?;

    let mut bytes = Vec::with_capacity(flat.len() * 8);
    for v in flat {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    let chunk_name = vec!["0"; shape.len().max(1)].join(".");
    std::fs::write(dir.join(chunk_name), gzip(&bytes)?)
        .map_err(|e| io_err(format!("write chunk: {e}")))
}

fn write_usize_array(dir: &Path, flat: &[usize]) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(|e| io_err(format!("mkdir: {e}")))?;
    let zarray = serde_json::json!({
        "zarr_format": 2,
        "shape": [flat.len()],
        "chunks": [flat.len().max(1)],
        "dtype": "<i8",
        "compressor": {"id": "gzip", "level": 5},
        "fill_value": 0,
        "order": "C",
        "filters": serde_json::Value::Null,
    });
    write_json(&dir.join(".zarray"), &zarray)?;

    let mut bytes = Vec::with_capacity(flat.len() * 8);
    for value in flat {
        let value = i64::try_from(*value)
            .map_err(|_| io_err("sparse index exceeds AnnData int64 range"))?;
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    std::fs::write(dir.join("0"), gzip(&bytes)?).map_err(|e| io_err(format!("write chunk: {e}")))
}

fn write_sparse_matrix(dir: &Path, matrix: &SparseMatrix) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(|e| io_err(format!("mkdir: {e}")))?;
    write_json(&dir.join(".zgroup"), &serde_json::json!({"zarr_format": 2}))?;
    write_json(
        &dir.join(".zattrs"),
        &serde_json::json!({
            "encoding-type": "csr_matrix",
            "encoding-version": "0.1.0",
            "shape": [matrix.nrow, matrix.ncol],
        }),
    )?;
    write_numeric_array(&dir.join("data"), &[matrix.data.len()], &matrix.data)?;
    write_usize_array(&dir.join("indices"), &matrix.indices)?;
    write_usize_array(&dir.join("indptr"), &matrix.indptr)
}

fn write_dataframe(dir: &Path, index: &[String]) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(|e| io_err(format!("mkdir: {e}")))?;
    write_json(&dir.join(".zgroup"), &serde_json::json!({"zarr_format": 2}))?;
    write_json(
        &dir.join(".zattrs"),
        &serde_json::json!({
            "encoding-type": "dataframe",
            "encoding-version": "0.2.0",
            "_index": "_index",
            "column-order": [],
        }),
    )?;
    write_string_array(&dir.join("_index"), index)
}

fn write_string_array(dir: &Path, strings: &[String]) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(|e| io_err(format!("mkdir: {e}")))?;
    let zarray = serde_json::json!({
        "zarr_format": 2,
        "shape": [strings.len()],
        "chunks": [strings.len().max(1)],
        "dtype": "|O",
        "compressor": {"id": "gzip", "level": 5},
        "fill_value": "",
        "order": "C",
        "filters": [{"id": "vlen-utf8"}],
    });
    write_json(&dir.join(".zarray"), &zarray)?;

    // numcodecs VLenUTF8 encoding.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(strings.len() as u32).to_le_bytes());
    for s in strings {
        bytes.extend_from_slice(&(s.len() as u32).to_le_bytes());
        bytes.extend_from_slice(s.as_bytes());
    }
    std::fs::write(dir.join("0"), gzip(&bytes)?).map_err(|e| io_err(format!("write chunk: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "bl_anndata_{tag}_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        p
    }

    fn sc_object() -> Value {
        let matrix = Value::List(
            (vec![
                Value::List((vec![Value::Float(0.0), Value::Float(2.0), Value::Float(3.0)]).into()),
                Value::List((vec![Value::Float(4.0), Value::Float(0.0), Value::Float(6.0)]).into()),
            ])
            .into(),
        );
        let mut rec = HashMap::new();
        rec.insert("matrix".into(), matrix);
        rec.insert(
            "genes".into(),
            Value::List(
                (vec![
                    Value::Str("GeneA".into()),
                    Value::Str("GeneB".into()),
                    Value::Str("GeneC".into()),
                ])
                .into(),
            ),
        );
        rec.insert(
            "barcodes".into(),
            Value::List((vec![Value::Str("CELL_1".into()), Value::Str("CELL_2".into())]).into()),
        );
        Value::Record((rec).into())
    }

    fn sparse_sc_object() -> Value {
        let mut matrix = SparseMatrix::from_dense(&[vec![0.0, 2.0, 3.0], vec![4.0, 0.0, 6.0]]);
        matrix.row_names = Some(vec!["CELL_1".into(), "CELL_2".into()]);
        matrix.col_names = Some(vec!["GeneA".into(), "GeneB".into(), "GeneC".into()]);
        let mut rec = HashMap::new();
        rec.insert("matrix".into(), Value::SparseMatrix(std::sync::Arc::new(matrix)));
        rec.insert(
            "genes".into(),
            Value::List(
                vec![
                    Value::Str("GeneA".into()),
                    Value::Str("GeneB".into()),
                    Value::Str("GeneC".into()),
                ]
                .into(),
            ),
        );
        rec.insert(
            "barcodes".into(),
            Value::List(vec![Value::Str("CELL_1".into()), Value::Str("CELL_2".into())].into()),
        );
        Value::Record(rec.into())
    }

    fn field<'a>(v: &'a Value, key: &str) -> &'a Value {
        match v {
            Value::Record(m) => m.get(key).unwrap(),
            _ => panic!("not a record"),
        }
    }

    #[test]
    fn round_trip_dense_and_strings() {
        let dir = tmp_dir("rt");
        write_anndata_dir(&dir, &sc_object()).unwrap();
        let back = read_anndata_dir(&dir).unwrap();

        assert_eq!(field(&back, "n_cells"), &Value::Int(2));
        assert_eq!(field(&back, "n_genes"), &Value::Int(3));

        // Gene / cell names survive the vlen-utf8 round trip.
        let genes = match field(&back, "genes") {
            Value::List(l) => l.clone(),
            _ => panic!(),
        };
        assert_eq!(genes[0], Value::Str("GeneA".into()));
        assert_eq!(genes[2], Value::Str("GeneC".into()));
        assert_eq!(
            field(&back, "barcodes"),
            &Value::List((vec![Value::Str("CELL_1".into()), Value::Str("CELL_2".into())]).into())
        );

        // Matrix values (including the zeros) survive.
        let rows = match field(&back, "matrix") {
            Value::List(r) => r.clone(),
            _ => panic!(),
        };
        assert_eq!(
            rows[0],
            Value::List((vec![Value::Float(0.0), Value::Float(2.0), Value::Float(3.0)]).into())
        );
        assert_eq!(
            rows[1],
            Value::List((vec![Value::Float(4.0), Value::Float(0.0), Value::Float(6.0)]).into())
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reads_csr_sparse_x() {
        // Hand-build a minimal CSR AnnData store: 2 cells x 3 genes.
        // Dense: [[0,2,3],[4,0,6]] → data=[2,3,4,6], indices=[1,2,0,2], indptr=[0,2,4].
        let dir = tmp_dir("csr");
        std::fs::create_dir_all(dir.join("X")).unwrap();
        write_json(&dir.join(".zgroup"), &serde_json::json!({"zarr_format":2})).unwrap();
        write_json(
            &dir.join("X").join(".zgroup"),
            &serde_json::json!({"zarr_format":2}),
        )
        .unwrap();
        write_json(
            &dir.join("X").join(".zattrs"),
            &serde_json::json!({"encoding-type":"csr_matrix","shape":[2,3]}),
        )
        .unwrap();
        write_numeric_array(&dir.join("X").join("data"), &[4], &[2.0, 3.0, 4.0, 6.0]).unwrap();
        write_numeric_array(&dir.join("X").join("indices"), &[4], &[1.0, 2.0, 0.0, 2.0]).unwrap();
        write_numeric_array(&dir.join("X").join("indptr"), &[3], &[0.0, 2.0, 4.0]).unwrap();
        write_dataframe(
            &dir.join("var"),
            &["GeneA".into(), "GeneB".into(), "GeneC".into()],
        )
        .unwrap();
        write_dataframe(&dir.join("obs"), &["CELL_1".into(), "CELL_2".into()]).unwrap();

        let back = read_anndata_dir(&dir).unwrap();
        let matrix = match field(&back, "matrix") {
            Value::SparseMatrix(matrix) => matrix,
            other => panic!("expected sparse matrix, got {other:?}"),
        };
        assert_eq!(
            matrix.to_dense(),
            vec![vec![0.0, 2.0, 3.0], vec![4.0, 0.0, 6.0]]
        );
        assert_eq!(field(&back, "is_sparse"), &Value::Bool(true));
        assert!(matches!(field(&back, "obs"), Value::Table(table) if table.rows.len() == 2));
        assert!(matches!(field(&back, "var"), Value::Table(table) if table.rows.len() == 3));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn round_trip_sparse_without_densifying() {
        let dir = tmp_dir("sparse_rt");
        write_anndata_dir(&dir, &sparse_sc_object()).unwrap();
        let back = read_anndata_dir(&dir).unwrap();

        let matrix = match field(&back, "matrix") {
            Value::SparseMatrix(matrix) => matrix,
            other => panic!("expected sparse matrix, got {other:?}"),
        };
        assert_eq!((matrix.nrow, matrix.ncol, matrix.nnz()), (2, 3, 4));
        assert_eq!(
            matrix.to_dense(),
            vec![vec![0.0, 2.0, 3.0], vec![4.0, 0.0, 6.0]]
        );
        assert_eq!(
            matrix.row_names.as_deref(),
            Some(&["CELL_1".to_string(), "CELL_2".to_string()][..])
        );
        assert_eq!(
            matrix.col_names.as_deref(),
            Some(
                &[
                    "GeneA".to_string(),
                    "GeneB".to_string(),
                    "GeneC".to_string(),
                ][..]
            )
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
