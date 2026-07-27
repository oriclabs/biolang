//! Protein structure analysis builtins.
//!
//! Functions: pdb_parse, rmsd, contact_map, secondary_structure, backbone_angles.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::{BTreeMap, HashMap};

// ── Registry ─────────────────────────────────────────────────────────

pub fn structure_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("pdb_parse", Arity::Exact(1)),
        ("rmsd", Arity::Exact(2)),
        ("contact_map", Arity::Range(1, 2)),
        ("secondary_structure", Arity::Exact(1)),
        ("backbone_angles", Arity::Exact(1)),
    ]
}

pub fn is_structure_builtin(name: &str) -> bool {
    matches!(
        name,
        "pdb_parse" | "rmsd" | "contact_map" | "secondary_structure" | "backbone_angles"
    )
}

pub fn call_structure_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "pdb_parse" => builtin_pdb_parse(args),
        "rmsd" => builtin_rmsd(args),
        "contact_map" => builtin_contact_map(args),
        "secondary_structure" => builtin_secondary_structure(args),
        "backbone_angles" => builtin_backbone_angles(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown structure builtin: {name}"),
            None,
        )),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

fn val_to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Float(f) => Some(*f),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

fn require_coord_list(val: &Value, func: &str) -> Result<Vec<[f64; 3]>> {
    match val {
        Value::Table(t) => t
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                if row.len() < 3 {
                    return Err(BioLangError::type_error(
                        format!("{func}() row {i} has fewer than 3 columns"),
                        None,
                    ));
                }
                let x = val_to_f64(&row[0]).ok_or_else(|| {
                    BioLangError::type_error(format!("{func}() non-numeric x at row {i}"), None)
                })?;
                let y = val_to_f64(&row[1]).ok_or_else(|| {
                    BioLangError::type_error(format!("{func}() non-numeric y at row {i}"), None)
                })?;
                let z = val_to_f64(&row[2]).ok_or_else(|| {
                    BioLangError::type_error(format!("{func}() non-numeric z at row {i}"), None)
                })?;
                Ok([x, y, z])
            })
            .collect(),
        Value::List(rows) => rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let coords = match row {
                    Value::List(c) => c,
                    _ => {
                        return Err(BioLangError::type_error(
                            format!("{func}() row {i} must be a list"),
                            None,
                        ))
                    }
                };
                if coords.len() < 3 {
                    return Err(BioLangError::type_error(
                        format!("{func}() row {i} needs at least 3 elements"),
                        None,
                    ));
                }
                let x = val_to_f64(&coords[0]).ok_or_else(|| {
                    BioLangError::type_error(format!("{func}() non-numeric x at row {i}"), None)
                })?;
                let y = val_to_f64(&coords[1]).ok_or_else(|| {
                    BioLangError::type_error(format!("{func}() non-numeric y at row {i}"), None)
                })?;
                let z = val_to_f64(&coords[2]).ok_or_else(|| {
                    BioLangError::type_error(format!("{func}() non-numeric z at row {i}"), None)
                })?;
                Ok([x, y, z])
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires List<[x,y,z]> or Table"),
            None,
        )),
    }
}

fn row_f64(row: &[Value], col: usize) -> f64 {
    val_to_f64(row.get(col).unwrap_or(&Value::Float(0.0))).unwrap_or(0.0)
}

// ── pdb_parse(pdb_text) → Table ───────────────────────────────────────

fn builtin_pdb_parse(args: Vec<Value>) -> Result<Value> {
    let text = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => {
            return Err(BioLangError::type_error(
                "pdb_parse() requires Str",
                None,
            ))
        }
    };
    let mut rows: Vec<Vec<Value>> = Vec::new();
    for line in text.lines() {
        let rec = line.get(..6).map(str::trim).unwrap_or("").trim();
        if !matches!(rec, "ATOM" | "HETATM") {
            continue;
        }
        let serial = line.get(6..11).map(str::trim).unwrap_or("").to_string();
        let atom_name = line.get(12..16).map(str::trim).unwrap_or("").to_string();
        let res_name = line.get(17..20).map(str::trim).unwrap_or("").to_string();
        let chain = line.get(21..22).map(str::trim).unwrap_or("").to_string();
        let resseq = line.get(22..26).map(str::trim).unwrap_or("").to_string();
        let x: f64 = line.get(30..38).and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
        let y: f64 = line.get(38..46).and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
        let z: f64 = line.get(46..54).and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
        let bfactor: f64 = line.get(60..66).and_then(|s| s.trim().parse().ok()).unwrap_or(0.0);
        let element = line.get(76..78).map(str::trim).unwrap_or("").to_string();
        rows.push(vec![
            Value::Str(rec.to_string()),
            Value::Str(serial),
            Value::Str(atom_name),
            Value::Str(res_name),
            Value::Str(chain),
            Value::Str(resseq),
            Value::Float(x),
            Value::Float(y),
            Value::Float(z),
            Value::Float(bfactor),
            Value::Str(element),
        ]);
    }
    Ok(Value::Table(Table::new(
        vec![
            "record".to_string(),
            "serial".to_string(),
            "atom".to_string(),
            "resname".to_string(),
            "chain".to_string(),
            "resseq".to_string(),
            "x".to_string(),
            "y".to_string(),
            "z".to_string(),
            "bfactor".to_string(),
            "element".to_string(),
        ],
        rows,
    )))
}

// ── rmsd(coords_a, coords_b) → Float ─────────────────────────────────
// Kabsch algorithm with 3×3 SVD via one-sided Jacobi iteration.

fn builtin_rmsd(args: Vec<Value>) -> Result<Value> {
    let a = require_coord_list(&args[0], "rmsd")?;
    let b = require_coord_list(&args[1], "rmsd")?;
    if a.len() != b.len() || a.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "rmsd() coordinate sets must be non-empty and equal length".to_string(),
            None,
        ));
    }
    let n = a.len() as f64;
    let ca = centroid(&a);
    let cb = centroid(&b);
    let ac: Vec<[f64; 3]> = a.iter().map(|p| sub3(*p, ca)).collect();
    let bc: Vec<[f64; 3]> = b.iter().map(|p| sub3(*p, cb)).collect();
    let h = cov3(&ac, &bc);
    let (u, _s, vt) = svd3(h);
    let d = det3(mat3_mul(transpose3(vt), transpose3(u)));
    let sign = if d < 0.0 { -1.0_f64 } else { 1.0_f64 };
    let mut vt2 = vt;
    for k in 0..3 {
        vt2[2][k] *= sign;
    }
    let r = mat3_mul(transpose3(vt2), transpose3(u));
    let rmsd_sq = ac
        .iter()
        .zip(bc.iter())
        .map(|(pa, pb)| {
            let rot = apply_rot(&r, *pa);
            let d0 = rot[0] - pb[0];
            let d1 = rot[1] - pb[1];
            let d2 = rot[2] - pb[2];
            d0 * d0 + d1 * d1 + d2 * d2
        })
        .sum::<f64>()
        / n;
    Ok(Value::Float(rmsd_sq.sqrt()))
}

fn centroid(pts: &[[f64; 3]]) -> [f64; 3] {
    let n = pts.len() as f64;
    let mut s = [0.0f64; 3];
    for p in pts {
        s[0] += p[0];
        s[1] += p[1];
        s[2] += p[2];
    }
    [s[0] / n, s[1] / n, s[2] / n]
}

fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cov3(a: &[[f64; 3]], b: &[[f64; 3]]) -> [[f64; 3]; 3] {
    let mut h = [[0.0f64; 3]; 3];
    for (pa, pb) in a.iter().zip(b.iter()) {
        for i in 0..3 {
            for j in 0..3 {
                h[i][j] += pa[i] * pb[j];
            }
        }
    }
    h
}

fn transpose3(m: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut t = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            t[i][j] = m[j][i];
        }
    }
    t
}

fn mat3_mul(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut c = [[0.0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                c[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    c
}

fn det3(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

fn apply_rot(r: &[[f64; 3]; 3], p: [f64; 3]) -> [f64; 3] {
    [
        r[0][0] * p[0] + r[0][1] * p[1] + r[0][2] * p[2],
        r[1][0] * p[0] + r[1][1] * p[1] + r[1][2] * p[2],
        r[2][0] * p[0] + r[2][1] * p[1] + r[2][2] * p[2],
    ]
}

/// Two-sided Jacobi SVD for a 3×3 matrix. Returns (U, S, V^T).
/// When a singular value is zero the corresponding U column is set to
/// the cross product of the other two so that U remains orthogonal.
fn svd3(a: [[f64; 3]; 3]) -> ([[f64; 3]; 3], [f64; 3], [[f64; 3]; 3]) {
    let ata = mat3_mul(transpose3(a), a);
    let (v, s_sq) = jacobi_eigen3(ata);
    // Sort descending by singular value for stability
    let mut order = [0usize, 1, 2];
    order.sort_by(|&i, &j| s_sq[j].partial_cmp(&s_sq[i]).unwrap_or(std::cmp::Ordering::Equal));
    let s_sq_sorted = [s_sq[order[0]], s_sq[order[1]], s_sq[order[2]]];
    let v_sorted = [
        [v[0][order[0]], v[0][order[1]], v[0][order[2]]],
        [v[1][order[0]], v[1][order[1]], v[1][order[2]]],
        [v[2][order[0]], v[2][order[1]], v[2][order[2]]],
    ];
    let s = [
        s_sq_sorted[0].max(0.0).sqrt(),
        s_sq_sorted[1].max(0.0).sqrt(),
        s_sq_sorted[2].max(0.0).sqrt(),
    ];
    let av = mat3_mul(a, v_sorted);
    let mut u = [[0.0f64; 3]; 3];
    let mut nonzero = [false; 3];
    for j in 0..3 {
        if s[j] > 1e-12 {
            for i in 0..3 {
                u[i][j] = av[i][j] / s[j];
            }
            nonzero[j] = true;
        }
    }
    // Fill any zero-singular-value columns to keep U orthogonal
    for j in 0..3 {
        if !nonzero[j] {
            let j1 = (j + 1) % 3;
            let j2 = (j + 2) % 3;
            let col1 = [u[0][j1], u[1][j1], u[2][j1]];
            let col2 = [u[0][j2], u[1][j2], u[2][j2]];
            let cross = cross3(col1, col2);
            let len = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
            if len > 1e-12 {
                u[0][j] = cross[0] / len;
                u[1][j] = cross[1] / len;
                u[2][j] = cross[2] / len;
            }
        }
    }
    let vt = transpose3(v_sorted);
    (u, s, vt)
}

/// Jacobi eigendecomposition of a symmetric 3×3 matrix.
/// Returns (V, eigenvalues) where A = V * diag(eigenvalues) * V^T.
fn jacobi_eigen3(mut a: [[f64; 3]; 3]) -> ([[f64; 3]; 3], [f64; 3]) {
    let mut v = [[0.0f64; 3]; 3];
    v[0][0] = 1.0;
    v[1][1] = 1.0;
    v[2][2] = 1.0;
    for _ in 0..50 {
        let mut max_val = 0.0f64;
        let mut p = 0usize;
        let mut q = 1usize;
        for i in 0..3 {
            for j in (i + 1)..3 {
                if a[i][j].abs() > max_val {
                    max_val = a[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if max_val < 1e-12 {
            break;
        }
        let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
        let t = if theta.abs() < 1e12 {
            theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt())
        } else {
            0.5 / theta
        };
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;
        let app = a[p][p];
        let aqq = a[q][q];
        let apq = a[p][q];
        a[p][p] = c * c * app - 2.0 * s * c * apq + s * s * aqq;
        a[q][q] = s * s * app + 2.0 * s * c * apq + c * c * aqq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;
        for r in 0..3 {
            if r != p && r != q {
                let arp = a[r][p];
                let arq = a[r][q];
                a[r][p] = c * arp - s * arq;
                a[p][r] = a[r][p];
                a[r][q] = s * arp + c * arq;
                a[q][r] = a[r][q];
            }
        }
        for r in 0..3 {
            let vrp = v[r][p];
            let vrq = v[r][q];
            v[r][p] = c * vrp - s * vrq;
            v[r][q] = s * vrp + c * vrq;
        }
    }
    let eigenvalues = [a[0][0], a[1][1], a[2][2]];
    (v, eigenvalues)
}

// ── contact_map(coords, cutoff=8.0) → Table ───────────────────────────

fn builtin_contact_map(args: Vec<Value>) -> Result<Value> {
    let coords = require_coord_list(&args[0], "contact_map")?;
    let cutoff = if args.len() > 1 {
        val_to_f64(&args[1]).unwrap_or(8.0)
    } else {
        8.0
    };
    let cutoff_sq = cutoff * cutoff;
    let mut rows: Vec<Vec<Value>> = Vec::new();
    for i in 0..coords.len() {
        for j in (i + 1)..coords.len() {
            let dx = coords[i][0] - coords[j][0];
            let dy = coords[i][1] - coords[j][1];
            let dz = coords[i][2] - coords[j][2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            if dist_sq <= cutoff_sq {
                rows.push(vec![
                    Value::Int(i as i64),
                    Value::Int(j as i64),
                    Value::Float(dist_sq.sqrt()),
                ]);
            }
        }
    }
    Ok(Value::Table(Table::new(
        vec!["i".to_string(), "j".to_string(), "distance".to_string()],
        rows,
    )))
}

// ── secondary_structure(coords_or_pdb_table) → Table ─────────────────
// DSSP-lite: assigns H/E/C per residue using Cα distance heuristics.

fn builtin_secondary_structure(args: Vec<Value>) -> Result<Value> {
    let ca_coords = extract_ca_coords(&args[0], "secondary_structure")?;
    let n = ca_coords.len();
    let mut assignments: Vec<&str> = vec!["C"; n];
    // Alpha helix: Cα(i)–Cα(i+4) distance ≈ 5–6 Å
    for i in 0..n.saturating_sub(4) {
        let d = dist3(ca_coords[i], ca_coords[i + 4]);
        if (4.5..=6.5).contains(&d) {
            for k in i..=(i + 4).min(n - 1) {
                assignments[k] = "H";
            }
        }
    }
    // Beta strand: consecutive Cα distances 3.2–4.0 Å (extended chain)
    for i in 0..n.saturating_sub(1) {
        if assignments[i] == "H" || assignments[i + 1] == "H" {
            continue;
        }
        let d = dist3(ca_coords[i], ca_coords[i + 1]);
        if (3.2..=4.0).contains(&d) {
            if assignments[i] == "C" {
                assignments[i] = "E";
            }
            if assignments[i + 1] == "C" {
                assignments[i + 1] = "E";
            }
        }
    }
    let rows: Vec<Vec<Value>> = (0..n)
        .map(|i| {
            vec![
                Value::Int(i as i64),
                Value::Str(assignments[i].to_string()),
            ]
        })
        .collect();
    Ok(Value::Table(Table::new(
        vec!["residue".to_string(), "ss".to_string()],
        rows,
    )))
}

fn extract_ca_coords(val: &Value, func: &str) -> Result<Vec<[f64; 3]>> {
    if let Value::Table(t) = val {
        let x_col = t.columns.iter().position(|c| c == "x");
        let y_col = t.columns.iter().position(|c| c == "y");
        let z_col = t.columns.iter().position(|c| c == "z");
        let atom_col = t.columns.iter().position(|c| c == "atom");
        if let (Some(xi), Some(yi), Some(zi)) = (x_col, y_col, z_col) {
            return Ok(t
                .rows
                .iter()
                .filter(|row| {
                    if let Some(ai) = atom_col {
                        matches!(row.get(ai), Some(Value::Str(s)) if s == "CA")
                    } else {
                        true
                    }
                })
                .map(|row| [row_f64(row, xi), row_f64(row, yi), row_f64(row, zi)])
                .collect());
        }
    }
    require_coord_list(val, func)
}

fn dist3(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

// ── backbone_angles(pdb_table) → Table ───────────────────────────────
// Phi/psi dihedral angles per residue from N, CA, C backbone atoms.

fn builtin_backbone_angles(args: Vec<Value>) -> Result<Value> {
    let t = match &args[0] {
        Value::Table(t) => t.clone(),
        _ => {
            return Err(BioLangError::type_error(
                "backbone_angles() requires a Table from pdb_parse()",
                None,
            ))
        }
    };
    let atom_col = t.columns.iter().position(|c| c == "atom").unwrap_or(2);
    let resseq_col = t.columns.iter().position(|c| c == "resseq").unwrap_or(5);
    let xi = t.columns.iter().position(|c| c == "x").unwrap_or(6);
    let yi = t.columns.iter().position(|c| c == "y").unwrap_or(7);
    let zi = t.columns.iter().position(|c| c == "z").unwrap_or(8);
    // Group backbone atoms by residue ID
    let mut residues: BTreeMap<String, HashMap<String, [f64; 3]>> = BTreeMap::new();
    for row in &t.rows {
        let atom = match row.get(atom_col) {
            Some(Value::Str(s)) => s.clone(),
            _ => continue,
        };
        if !matches!(atom.as_str(), "N" | "CA" | "C") {
            continue;
        }
        let res = match row.get(resseq_col) {
            Some(Value::Str(s)) => s.clone(),
            _ => continue,
        };
        let xyz = [row_f64(row, xi), row_f64(row, yi), row_f64(row, zi)];
        residues.entry(res).or_default().insert(atom, xyz);
    }
    let res_list: Vec<(String, HashMap<String, [f64; 3]>)> = residues.into_iter().collect();
    let mut rows: Vec<Vec<Value>> = Vec::new();
    for i in 0..res_list.len() {
        let (ref res_id, ref atoms) = res_list[i];
        let ca = match atoms.get("CA") {
            Some(c) => *c,
            None => continue,
        };
        let n_atom = match atoms.get("N") {
            Some(c) => *c,
            None => continue,
        };
        let c_atom = match atoms.get("C") {
            Some(c) => *c,
            None => continue,
        };
        let phi = if i > 0 {
            res_list[i - 1]
                .1
                .get("C")
                .map(|c_prev| dihedral(*c_prev, n_atom, ca, c_atom).to_degrees())
                .unwrap_or(f64::NAN)
        } else {
            f64::NAN
        };
        let psi = if i + 1 < res_list.len() {
            res_list[i + 1]
                .1
                .get("N")
                .map(|n_next| dihedral(n_atom, ca, c_atom, *n_next).to_degrees())
                .unwrap_or(f64::NAN)
        } else {
            f64::NAN
        };
        rows.push(vec![
            Value::Str(res_id.clone()),
            Value::Float(phi),
            Value::Float(psi),
        ]);
    }
    Ok(Value::Table(Table::new(
        vec!["resseq".to_string(), "phi".to_string(), "psi".to_string()],
        rows,
    )))
}

fn dihedral(a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> f64 {
    let b1 = sub3(b, a);
    let b2 = sub3(c, b);
    let b3 = sub3(d, c);
    let n1 = cross3(b1, b2);
    let n2 = cross3(b2, b3);
    let m1 = cross3(n1, b2);
    let b2_len = (b2[0] * b2[0] + b2[1] * b2[1] + b2[2] * b2[2]).sqrt();
    if b2_len < 1e-12 {
        return 0.0;
    }
    let x = dot3(n1, n2);
    let y = dot3(m1, n2) / b2_len;
    y.atan2(x)
}

fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
