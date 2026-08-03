//! Protein interaction network / graph analysis builtins.
//!
//! Functions: load_ppi, degree_centrality, betweenness_centrality,
//!            shortest_path, connected_components, network_enrichment.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::{HashMap, HashSet, VecDeque};

// ── Registry ─────────────────────────────────────────────────────────

pub fn network_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("load_ppi", Arity::Range(1, 2)),
        ("degree_centrality", Arity::Exact(1)),
        ("betweenness_centrality", Arity::Exact(1)),
        ("shortest_path", Arity::Exact(3)),
        ("connected_components", Arity::Exact(1)),
        ("network_enrichment", Arity::Exact(3)),
    ]
}

pub fn is_network_builtin(name: &str) -> bool {
    matches!(
        name,
        "load_ppi"
            | "degree_centrality"
            | "betweenness_centrality"
            | "shortest_path"
            | "connected_components"
            | "network_enrichment"
    )
}

pub fn call_network_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "load_ppi" => builtin_load_ppi(args),
        "degree_centrality" => builtin_degree_centrality(args),
        "betweenness_centrality" => builtin_betweenness_centrality(args),
        "shortest_path" => builtin_shortest_path(args),
        "connected_components" => builtin_connected_components(args),
        "network_enrichment" => builtin_network_enrichment(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown network builtin: {name}"),
            None,
        )),
    }
}

// ── Graph representation ──────────────────────────────────────────────
// Adjacency list keyed by node name with optional edge weight.

type Graph = HashMap<String, Vec<(String, f64)>>;

fn graph_from_table(t: &bl_core::value::Table, func: &str) -> Result<Graph> {
    // Expected columns: node1, node2, [score/weight]
    if t.columns.len() < 2 {
        return Err(BioLangError::type_error(
            format!("{func}() table must have at least 2 columns (node1, node2)"),
            None,
        ));
    }
    let weight_col = t.columns.get(2);
    let mut g: Graph = HashMap::new();
    for row in &t.rows {
        let n1 = match row.first() {
            Some(Value::Str(s)) => s.clone(),
            _ => continue,
        };
        let n2 = match row.get(1) {
            Some(Value::Str(s)) => s.clone(),
            _ => continue,
        };
        let w: f64 = if weight_col.is_some() {
            match row.get(2) {
                Some(Value::Float(f)) => *f,
                Some(Value::Int(n)) => *n as f64,
                _ => 1.0,
            }
        } else {
            1.0
        };
        g.entry(n1.clone()).or_default().push((n2.clone(), w));
        g.entry(n2.clone()).or_default().push((n1.clone(), w));
    }
    Ok(g)
}

fn graph_from_value(val: &Value, func: &str) -> Result<Graph> {
    match val {
        Value::Table(t) => graph_from_table(t, func),
        Value::Map(m) => {
            // Map of node → List of neighbor strings
            let mut g: Graph = HashMap::new();
            for (node, neighbors) in m.iter() {
                let nbrs = match neighbors {
                    Value::List(l) => l,
                    _ => {
                        return Err(BioLangError::type_error(
                            format!("{func}() map values must be Lists of neighbors"),
                            None,
                        ))
                    }
                };
                for nb in nbrs.iter() {
                    let nb_str = match nb {
                        Value::Str(s) => s.clone(),
                        _ => {
                            return Err(BioLangError::type_error(
                                format!("{func}() neighbor must be Str"),
                                None,
                            ))
                        }
                    };
                    g.entry(node.clone())
                        .or_default()
                        .push((nb_str.clone(), 1.0));
                    g.entry(nb_str).or_default().push((node.clone(), 1.0));
                }
            }
            Ok(g)
        }
        _ => Err(BioLangError::type_error(
            format!("{func}() requires Table or Map"),
            None,
        )),
    }
}

// ── load_ppi(tsv_text, min_score=400) → Table ─────────────────────────
// Loads STRING-format TSV (protein1, protein2, combined_score).

fn builtin_load_ppi(args: Vec<Value>) -> Result<Value> {
    let text = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err(BioLangError::type_error("load_ppi() requires Str", None)),
    };
    let min_score: f64 = if args.len() > 1 {
        match &args[1] {
            Value::Float(f) => *f,
            Value::Int(n) => *n as f64,
            _ => 400.0,
        }
    } else {
        400.0
    };
    let mut rows: Vec<Vec<Value>> = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 2 {
            continue;
        }
        let p1 = cols[0].to_string();
        let p2 = cols[1].to_string();
        let score: f64 = cols.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000.0);
        if score < min_score {
            continue;
        }
        rows.push(vec![Value::Str(p1), Value::Str(p2), Value::Float(score)]);
    }
    Ok(Value::Table(Table::new(
        vec![
            "protein1".to_string(),
            "protein2".to_string(),
            "score".to_string(),
        ],
        rows,
    )))
}

// ── degree_centrality(graph) → Table ─────────────────────────────────

fn builtin_degree_centrality(args: Vec<Value>) -> Result<Value> {
    let g = graph_from_value(&args[0], "degree_centrality")?;
    let n = g.len() as f64;
    let denom = if n > 1.0 { n - 1.0 } else { 1.0 };
    let mut rows: Vec<Vec<Value>> = g
        .iter()
        .map(|(node, nbrs)| {
            let deg = nbrs.len() as f64;
            vec![
                Value::Str(node.clone()),
                Value::Int(nbrs.len() as i64),
                Value::Float(deg / denom),
            ]
        })
        .collect();
    rows.sort_by(|a, b| {
        let da = match &a[1] {
            Value::Int(n) => *n,
            _ => 0,
        };
        let db = match &b[1] {
            Value::Int(n) => *n,
            _ => 0,
        };
        db.cmp(&da)
    });
    Ok(Value::Table(Table::new(
        vec![
            "node".to_string(),
            "degree".to_string(),
            "centrality".to_string(),
        ],
        rows,
    )))
}

// ── betweenness_centrality(graph) → Table ────────────────────────────
// Brandes algorithm (unweighted BFS variant).

fn builtin_betweenness_centrality(args: Vec<Value>) -> Result<Value> {
    let g = graph_from_value(&args[0], "betweenness_centrality")?;
    let nodes: Vec<String> = g.keys().cloned().collect();
    let n = nodes.len();
    let node_idx: HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect();
    let mut betweenness = vec![0.0f64; n];
    for s in 0..n {
        let mut stack: Vec<usize> = Vec::new();
        let mut pred: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut sigma = vec![0.0f64; n];
        sigma[s] = 1.0;
        let mut dist = vec![-1i64; n];
        dist[s] = 0;
        let mut queue: VecDeque<usize> = VecDeque::new();
        queue.push_back(s);
        while let Some(v) = queue.pop_front() {
            stack.push(v);
            let v_name = &nodes[v];
            if let Some(nbrs) = g.get(v_name) {
                for (nb_name, _) in nbrs {
                    if let Some(&w) = node_idx.get(nb_name.as_str()) {
                        if dist[w] < 0 {
                            queue.push_back(w);
                            dist[w] = dist[v] + 1;
                        }
                        if dist[w] == dist[v] + 1 {
                            sigma[w] += sigma[v];
                            pred[w].push(v);
                        }
                    }
                }
            }
        }
        let mut delta = vec![0.0f64; n];
        while let Some(w) = stack.pop() {
            for &v in &pred[w] {
                delta[v] += (sigma[v] / sigma[w]) * (1.0 + delta[w]);
            }
            if w != s {
                betweenness[w] += delta[w];
            }
        }
    }
    // Normalise: divide by (n-1)(n-2) for directed, (n-1)(n-2)/2 for undirected
    let norm = if n > 2 {
        ((n - 1) * (n - 2)) as f64 / 2.0
    } else {
        1.0
    };
    let mut rows: Vec<Vec<Value>> = nodes
        .iter()
        .enumerate()
        .map(|(i, name)| {
            vec![
                Value::Str(name.clone()),
                Value::Float(betweenness[i] / norm),
            ]
        })
        .collect();
    rows.sort_by(|a, b| {
        let fa = match &a[1] {
            Value::Float(f) => *f,
            _ => 0.0,
        };
        let fb = match &b[1] {
            Value::Float(f) => *f,
            _ => 0.0,
        };
        fb.partial_cmp(&fa).unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(Value::Table(Table::new(
        vec!["node".to_string(), "betweenness".to_string()],
        rows,
    )))
}

// ── shortest_path(graph, source, target) → List ───────────────────────
// BFS shortest path returning list of node names.

fn builtin_shortest_path(args: Vec<Value>) -> Result<Value> {
    let g = graph_from_value(&args[0], "shortest_path")?;
    let source = match &args[1] {
        Value::Str(s) => s.clone(),
        _ => {
            return Err(BioLangError::type_error(
                "shortest_path() source must be Str",
                None,
            ))
        }
    };
    let target = match &args[2] {
        Value::Str(s) => s.clone(),
        _ => {
            return Err(BioLangError::type_error(
                "shortest_path() target must be Str",
                None,
            ))
        }
    };
    if source == target {
        return Ok(Value::List((vec![Value::Str(source)]).into()));
    }
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<Vec<String>> = VecDeque::new();
    visited.insert(source.clone());
    queue.push_back(vec![source.clone()]);
    while let Some(path) = queue.pop_front() {
        let current = path.last().unwrap();
        if let Some(nbrs) = g.get(current) {
            for (nb, _) in nbrs {
                if !visited.contains(nb) {
                    let mut new_path = path.clone();
                    new_path.push(nb.clone());
                    if nb == &target {
                        return Ok(Value::List(
                            new_path
                                .into_iter()
                                .map(Value::Str)
                                .collect::<Vec<_>>()
                                .into(),
                        ));
                    }
                    visited.insert(nb.clone());
                    queue.push_back(new_path);
                }
            }
        }
    }
    Ok(Value::Nil) // no path
}

// ── connected_components(graph) → Table ──────────────────────────────
// Union-Find to identify connected components.

fn builtin_connected_components(args: Vec<Value>) -> Result<Value> {
    let g = graph_from_value(&args[0], "connected_components")?;
    let nodes: Vec<String> = g.keys().cloned().collect();
    let n = nodes.len();
    let node_idx: HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect();
    let mut parent: Vec<usize> = (0..n).collect();
    let mut rank: Vec<u8> = vec![0; n];
    fn find(parent: &mut Vec<usize>, x: usize) -> usize {
        if parent[x] != x {
            parent[x] = find(parent, parent[x]);
        }
        parent[x]
    }
    fn union(parent: &mut Vec<usize>, rank: &mut Vec<u8>, x: usize, y: usize) {
        let px = find(parent, x);
        let py = find(parent, y);
        if px == py {
            return;
        }
        if rank[px] < rank[py] {
            parent[px] = py;
        } else if rank[px] > rank[py] {
            parent[py] = px;
        } else {
            parent[py] = px;
            rank[px] += 1;
        }
    }
    for (node, nbrs) in &g {
        if let Some(&ni) = node_idx.get(node.as_str()) {
            for (nb, _) in nbrs {
                if let Some(&nj) = node_idx.get(nb.as_str()) {
                    union(&mut parent, &mut rank, ni, nj);
                }
            }
        }
    }
    // Group nodes by component root
    let mut comp_map: HashMap<usize, Vec<String>> = HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        let root = find(&mut parent, i);
        comp_map.entry(root).or_default().push(node.clone());
    }
    let mut comp_list: Vec<Vec<String>> = comp_map.into_values().collect();
    comp_list.sort_by(|a, b| b.len().cmp(&a.len()));
    let rows: Vec<Vec<Value>> = comp_list
        .iter()
        .enumerate()
        .map(|(ci, members)| {
            vec![
                Value::Int(ci as i64 + 1),
                Value::Int(members.len() as i64),
                Value::Str(members.join(",")),
            ]
        })
        .collect();
    Ok(Value::Table(Table::new(
        vec![
            "component".to_string(),
            "size".to_string(),
            "nodes".to_string(),
        ],
        rows,
    )))
}

// ── network_enrichment(graph, gene_set, background_size) → Table ──────
// Hypergeometric-based enrichment of a gene set in the network.

fn builtin_network_enrichment(args: Vec<Value>) -> Result<Value> {
    let g = graph_from_value(&args[0], "network_enrichment")?;
    let gene_set: HashSet<String> = match &args[1] {
        Value::List(l) => l
            .iter()
            .filter_map(|v| match v {
                Value::Str(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "network_enrichment() gene_set must be List",
                None,
            ))
        }
    };
    let background: u64 = match &args[2] {
        Value::Int(n) => *n as u64,
        Value::Float(f) => *f as u64,
        _ => {
            return Err(BioLangError::type_error(
                "network_enrichment() background_size must be Int",
                None,
            ))
        }
    };
    let network_nodes: HashSet<&str> = g.keys().map(String::as_str).collect();
    let k = gene_set
        .iter()
        .filter(|g| network_nodes.contains(g.as_str()))
        .count() as u64;
    let m = gene_set.len() as u64; // gene set size
    let n_net = network_nodes.len() as u64; // network size
    let b = background.max(n_net); // background population
                                   // p-value: hypergeometric P(X >= k) where we draw n_net from b, gene_set has m hits
    let p = hypergeometric_sf(k, b, m, n_net);
    // Fold enrichment
    let expected = if b > 0 {
        (m as f64) * (n_net as f64) / (b as f64)
    } else {
        0.0
    };
    let fold = if expected > 0.0 {
        k as f64 / expected
    } else {
        0.0
    };
    let rows = vec![vec![
        Value::Int(k as i64),
        Value::Int(m as i64),
        Value::Int(n_net as i64),
        Value::Int(b as i64),
        Value::Float(p),
        Value::Float(fold),
    ]];
    Ok(Value::Table(Table::new(
        vec![
            "overlap".to_string(),
            "set_size".to_string(),
            "network_size".to_string(),
            "background".to_string(),
            "pvalue".to_string(),
            "fold_enrichment".to_string(),
        ],
        rows,
    )))
}

/// Hypergeometric survival function P(X >= k): 1 - CDF(k-1).
/// Uses log-factorial approximation for large values.
fn hypergeometric_sf(k: u64, n: u64, m: u64, draws: u64) -> f64 {
    if k == 0 {
        return 1.0;
    }
    // P(X = x) = C(m,x)*C(n-m, draws-x) / C(n, draws)
    let log_denom = log_binom(n, draws);
    let mut p_ge_k = 0.0f64;
    let x_max = m.min(draws);
    for x in k..=x_max {
        let log_p =
            log_binom(m, x) + log_binom(n.saturating_sub(m), draws.saturating_sub(x)) - log_denom;
        p_ge_k += log_p.exp();
    }
    p_ge_k.min(1.0)
}

fn log_binom(n: u64, k: u64) -> f64 {
    if k > n {
        return f64::NEG_INFINITY;
    }
    let k = k.min(n - k);
    (0..k)
        .map(|i| ((n - i) as f64).ln() - ((i + 1) as f64).ln())
        .sum::<f64>()
}
