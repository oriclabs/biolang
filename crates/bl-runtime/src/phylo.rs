//! Phylogenetics builtins.
//!
//! Functions: nw_parse, tree_leaves, patristic_distance,
//! nw_to_distance_matrix, upgma.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

// ── Registry ─────────────────────────────────────────────────────────

pub fn phylo_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("nw_parse", Arity::Exact(1)),
        ("tree_leaves", Arity::Exact(1)),
        ("patristic_distance", Arity::Exact(3)),
        ("nw_to_distance_matrix", Arity::Exact(1)),
        ("upgma", Arity::Exact(2)),
    ]
}

pub fn is_phylo_builtin(name: &str) -> bool {
    matches!(
        name,
        "nw_parse" | "tree_leaves" | "patristic_distance" | "nw_to_distance_matrix" | "upgma"
    )
}

pub fn call_phylo_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "nw_parse" => builtin_nw_parse(args),
        "tree_leaves" => builtin_tree_leaves(args),
        "patristic_distance" => builtin_patristic_distance(args),
        "nw_to_distance_matrix" => builtin_nw_to_distance_matrix(args),
        "upgma" => builtin_upgma(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown phylo builtin '{name}'"),
            None,
        )),
    }
}

// ── Tree node ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Node {
    id: i64,
    parent: i64, // -1 = root
    label: String,
    branch_length: f64,
}

// ── Newick parser ────────────────────────────────────────────────────

fn parse_newick(s: &str) -> Result<Vec<Node>> {
    let s = s.trim().trim_end_matches(';');
    let chars: Vec<char> = s.chars().collect();
    let mut nodes: Vec<Node> = Vec::new();
    let mut id_counter: i64 = 0;
    let mut stack: Vec<i64> = Vec::new(); // parent stack

    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '(' => {
                // New internal node — push placeholder; will be filled when we see ')'
                let id = id_counter;
                id_counter += 1;
                let parent = stack.last().copied().unwrap_or(-1);
                nodes.push(Node {
                    id,
                    parent,
                    label: String::new(),
                    branch_length: 0.0,
                });
                stack.push(id);
                i += 1;
            }
            ')' => {
                // Close current group; parse optional label and branch length
                i += 1;
                let (label, bl, consumed) = parse_label_and_bl(&chars, i);
                i += consumed;
                let node_id = stack.pop().ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::TypeError,
                        "nw_parse(): unbalanced parentheses".to_string(),
                        None,
                    )
                })?;
                // Update the internal node we created when we saw '('
                if let Some(n) = nodes.iter_mut().find(|n| n.id == node_id) {
                    n.label = label;
                    n.branch_length = bl;
                }
            }
            ',' => {
                i += 1;
            }
            _ => {
                // Leaf: parse label and branch length
                let (label, bl, consumed) = parse_label_and_bl(&chars, i);
                i += consumed;
                let parent = stack.last().copied().unwrap_or(-1);
                nodes.push(Node {
                    id: id_counter,
                    parent,
                    label,
                    branch_length: bl,
                });
                id_counter += 1;
            }
        }
    }
    Ok(nodes)
}

/// Parse `label:branch_length` starting at position i; return (label, bl, chars_consumed).
fn parse_label_and_bl(chars: &[char], start: usize) -> (String, f64, usize) {
    let mut i = start;
    let mut label = String::new();
    // Read label (stops at : , ) ( whitespace)
    while i < chars.len() && !matches!(chars[i], ':' | ',' | ')' | '(' | ';') {
        label.push(chars[i]);
        i += 1;
    }
    let mut bl = 0.0f64;
    if i < chars.len() && chars[i] == ':' {
        i += 1;
        let mut bl_str = String::new();
        while i < chars.len()
            && (chars[i].is_ascii_digit()
                || chars[i] == '.'
                || chars[i] == '-'
                || chars[i] == 'e'
                || chars[i] == 'E')
        {
            bl_str.push(chars[i]);
            i += 1;
        }
        bl = bl_str.parse().unwrap_or(0.0);
    }
    (label.trim().to_string(), bl, i - start)
}

fn nodes_to_table(nodes: &[Node]) -> Value {
    let columns = ["node", "parent", "label", "branch_length"]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let rows = nodes
        .iter()
        .map(|n| {
            vec![
                Value::Int(n.id),
                Value::Int(n.parent),
                Value::Str(n.label.clone()),
                Value::Float(n.branch_length),
            ]
        })
        .collect();
    Value::Table(Table::new(columns, rows))
}

fn table_to_nodes(table: &Table) -> Result<Vec<Node>> {
    let id_col = col_index(table, "node", "tree")?;
    let parent_col = col_index(table, "parent", "tree")?;
    let label_col = col_index(table, "label", "tree")?;
    let bl_col = col_index(table, "branch_length", "tree")?;

    table
        .rows
        .iter()
        .map(|row| {
            let id = match &row[id_col] {
                Value::Int(n) => *n,
                _ => 0,
            };
            let parent = match &row[parent_col] {
                Value::Int(n) => *n,
                _ => -1,
            };
            let label = match &row[label_col] {
                Value::Str(s) => s.clone(),
                _ => String::new(),
            };
            let bl = match &row[bl_col] {
                Value::Float(f) => *f,
                Value::Int(n) => *n as f64,
                _ => 0.0,
            };
            Ok(Node {
                id,
                parent,
                label,
                branch_length: bl,
            })
        })
        .collect()
}

fn col_index(table: &Table, name: &str, func: &str) -> Result<usize> {
    table.columns.iter().position(|c| c == name).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::NameError,
            format!("{func}(): column '{name}' not found"),
            None,
        )
    })
}

fn require_table<'a>(val: &'a Value, func: &str) -> Result<&'a Table> {
    match val {
        Value::Table(t) => Ok(t),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires Table"),
            None,
        )),
    }
}

fn to_f64(v: &Value) -> f64 {
    match v {
        Value::Float(f) => *f,
        Value::Int(n) => *n as f64,
        _ => 0.0,
    }
}

fn require_str<'a>(val: &'a Value, func: &str) -> Result<&'a str> {
    match val {
        Value::Str(s) => Ok(s.as_str()),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires Str"),
            None,
        )),
    }
}

// ── nw_parse ─────────────────────────────────────────────────────────

fn builtin_nw_parse(args: Vec<Value>) -> Result<Value> {
    let s = require_str(&args[0], "nw_parse")?;
    let nodes = parse_newick(s)?;
    Ok(nodes_to_table(&nodes))
}

// ── tree_leaves ──────────────────────────────────────────────────────

fn builtin_tree_leaves(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "tree_leaves")?;
    let nodes = table_to_nodes(table)?;

    // Leaves = nodes that are not the parent of any other node
    let parent_ids: std::collections::HashSet<i64> = nodes.iter().map(|n| n.parent).collect();
    let leaves: Vec<Value> = nodes
        .iter()
        .filter(|n| !parent_ids.contains(&n.id) && !n.label.is_empty())
        .map(|n| Value::Str(n.label.clone()))
        .collect();

    Ok(Value::List((leaves).into()))
}

// ── patristic_distance ───────────────────────────────────────────────

fn builtin_patristic_distance(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "patristic_distance")?;
    let label_a = require_str(&args[1], "patristic_distance")?;
    let label_b = require_str(&args[2], "patristic_distance")?;
    let nodes = table_to_nodes(table)?;

    // Build id → node map and label → id map
    let id_map: HashMap<i64, &Node> = nodes.iter().map(|n| (n.id, n)).collect();
    let label_map: HashMap<&str, i64> = nodes
        .iter()
        .filter(|n| !n.label.is_empty())
        .map(|n| (n.label.as_str(), n.id))
        .collect();

    let id_a = label_map.get(label_a).copied().ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::NameError,
            format!("patristic_distance(): label '{label_a}' not found"),
            None,
        )
    })?;
    let id_b = label_map.get(label_b).copied().ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::NameError,
            format!("patristic_distance(): label '{label_b}' not found"),
            None,
        )
    })?;

    // Walk from each leaf to root, collecting (id → cumulative_dist)
    fn path_to_root(start: i64, id_map: &HashMap<i64, &Node>) -> HashMap<i64, f64> {
        let mut dist = HashMap::new();
        let mut cur = start;
        let mut acc = 0.0;
        loop {
            dist.insert(cur, acc);
            if let Some(n) = id_map.get(&cur) {
                if n.parent == -1 {
                    break;
                }
                acc += n.branch_length;
                cur = n.parent;
            } else {
                break;
            }
        }
        dist
    }

    let path_a = path_to_root(id_a, &id_map);
    let path_b = path_to_root(id_b, &id_map);

    // Find LCA: first node in path_a that's also in path_b
    let lca_dist_a = path_a
        .iter()
        .filter(|(id, _)| path_b.contains_key(id))
        .min_by(|(_, da), (_, db)| da.partial_cmp(db).unwrap_or(std::cmp::Ordering::Equal));

    if let Some((&lca_id, &da)) = lca_dist_a {
        let db = path_b[&lca_id];
        Ok(Value::Float(da + db))
    } else {
        Ok(Value::Float(0.0))
    }
}

// ── nw_to_distance_matrix ────────────────────────────────────────────

fn builtin_nw_to_distance_matrix(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "nw_to_distance_matrix")?;
    let nodes = table_to_nodes(table)?;

    let parent_ids: std::collections::HashSet<i64> = nodes.iter().map(|n| n.parent).collect();
    let leaves: Vec<&Node> = nodes
        .iter()
        .filter(|n| !parent_ids.contains(&n.id) && !n.label.is_empty())
        .collect();

    let leaf_labels: Vec<String> = leaves.iter().map(|n| n.label.clone()).collect();
    let n = leaves.len();
    let id_map: HashMap<i64, &Node> = nodes.iter().map(|n| (n.id, n)).collect();

    fn path_to_root(start: i64, id_map: &HashMap<i64, &Node>) -> HashMap<i64, f64> {
        let mut dist = HashMap::new();
        let mut cur = start;
        let mut acc = 0.0;
        loop {
            dist.insert(cur, acc);
            if let Some(n) = id_map.get(&cur) {
                if n.parent == -1 {
                    break;
                }
                acc += n.branch_length;
                cur = n.parent;
            } else {
                break;
            }
        }
        dist
    }

    let paths: Vec<HashMap<i64, f64>> =
        leaves.iter().map(|n| path_to_root(n.id, &id_map)).collect();

    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    for i in 0..n {
        let mut row = Vec::new();
        for j in 0..n {
            if i == j {
                row.push(Value::Float(0.0));
                continue;
            }
            // Find LCA distance
            let lca = paths[i]
                .iter()
                .filter(|(id, _)| paths[j].contains_key(id))
                .min_by(|(_, da), (_, db)| da.partial_cmp(db).unwrap_or(std::cmp::Ordering::Equal));
            let d = lca.map(|(&id, &da)| da + paths[j][&id]).unwrap_or(0.0);
            row.push(Value::Float(d));
        }
        out_rows.push(row);
    }

    Ok(Value::Table(Table::new(leaf_labels, out_rows)))
}

// ── upgma ─────────────────────────────────────────────────────────────

fn builtin_upgma(args: Vec<Value>) -> Result<Value> {
    let labels = match &args[0] {
        Value::List(l) => l
            .iter()
            .map(|v| match v {
                Value::Str(s) => s.clone(),
                _ => v.to_string(),
            })
            .collect::<Vec<_>>(),
        _ => {
            return Err(BioLangError::type_error(
                "upgma() labels must be List<Str>",
                None,
            ))
        }
    };

    let dist_table = require_table(&args[1], "upgma")?;
    let n = labels.len();
    if n == 0 {
        return Ok(Value::Str("();".to_string()));
    }

    // Extract distance matrix from Table
    let mut dist: Vec<Vec<f64>> = dist_table
        .rows
        .iter()
        .map(|row| row.iter().map(to_f64).collect::<Vec<f64>>())
        .collect::<Vec<Vec<f64>>>();

    // Pad or trim to n×n
    dist.resize(n, vec![0.0_f64; n]);
    for row in dist.iter_mut() {
        row.resize(n, 0.0_f64);
    }

    // UPGMA: each cluster is represented by a Newick string and a size
    let mut clusters: Vec<String> = labels.to_vec();
    let mut sizes: Vec<usize> = vec![1; n];
    let mut active: Vec<usize> = (0..n).collect();
    // Height of each cluster above the tips. Leaves sit at zero; a merge places
    // the new node at half the distance between the clusters it joins, which is
    // what makes UPGMA's tips equidistant from the root.
    let mut heights: Vec<f64> = vec![0.0; n];

    while active.len() > 1 {
        // Find minimum distance pair
        let mut min_d = f64::INFINITY;
        let mut mi = 0usize;
        let mut mj = 1usize;
        for ii in 0..active.len() {
            for jj in (ii + 1)..active.len() {
                let a = active[ii];
                let b = active[jj];
                if dist[a][b] < min_d {
                    min_d = dist[a][b];
                    mi = ii;
                    mj = jj;
                }
            }
        }

        let a = active[mi];
        let b = active[mj];
        let half = min_d / 2.0;

        // New cluster label
        // Branch length is the rise from each child to the new node, not the
        // node height itself. This used to emit a literal ":0" for every branch
        // while computing `half` and then discarding it with `let _ = half`, so
        // every tree came back topologically correct and quantitatively empty -
        // useless to anything that reads branch lengths.
        let branch_a = (half - heights[a]).max(0.0);
        let branch_b = (half - heights[b]).max(0.0);
        let new_label = format!(
            "({}:{:.6},{}:{:.6})",
            clusters[a], branch_a, clusters[b], branch_b
        );
        let new_size = sizes[a] + sizes[b];

        // Update distances for merged cluster using UPGMA average
        for &k in &active {
            if k == a || k == b {
                continue;
            }
            let new_d =
                (dist[a][k] * sizes[a] as f64 + dist[b][k] * sizes[b] as f64) / new_size as f64;
            dist[a][k] = new_d;
            dist[k][a] = new_d;
        }

        clusters[a] = new_label;
        sizes[a] = new_size;
        heights[a] = half;

        // Remove b from active
        active.remove(mj);
    }

    let root = active[0];
    Ok(Value::Str(format!("({});", clusters[root])))
}
