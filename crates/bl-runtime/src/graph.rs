use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::{HashMap, HashSet, VecDeque};

pub fn graph_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("graph", Arity::Range(0, 1)),
        ("add_node", Arity::Range(2, 3)),
        ("add_edge", Arity::Range(3, 4)),
        ("remove_node", Arity::Exact(2)),
        ("remove_edge", Arity::Exact(3)),
        ("neighbors", Arity::Exact(2)),
        ("degree", Arity::Exact(2)),
        ("shortest_path", Arity::Exact(3)),
        ("connected_components", Arity::Exact(1)),
        ("nodes", Arity::Exact(1)),
        ("edges", Arity::Exact(1)),
        ("has_node", Arity::Exact(2)),
        ("has_edge", Arity::Exact(3)),
        ("subgraph", Arity::Exact(2)),
        ("node_attr", Arity::Exact(2)),
    ]
}

pub fn is_graph_builtin(name: &str) -> bool {
    matches!(
        name,
        "graph"
            | "add_node"
            | "add_edge"
            | "remove_node"
            | "remove_edge"
            | "neighbors"
            | "degree"
            | "shortest_path"
            | "connected_components"
            | "nodes"
            | "edges"
            | "has_node"
            | "has_edge"
            | "subgraph"
            | "node_attr"
    )
}

pub fn call_graph_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "graph" => builtin_graph(args),
        "add_node" => builtin_add_node(args),
        "add_edge" => builtin_add_edge(args),
        "remove_node" => builtin_remove_node(args),
        "remove_edge" => builtin_remove_edge(args),
        "neighbors" => builtin_neighbors(args),
        "degree" => builtin_degree(args),
        "shortest_path" => builtin_shortest_path(args),
        "connected_components" => builtin_connected_components(args),
        "nodes" => builtin_nodes(args),
        "edges" => builtin_edges(args),
        "has_node" => builtin_has_node(args),
        "has_edge" => builtin_has_edge(args),
        "subgraph" => builtin_subgraph(args),
        "node_attr" => builtin_node_attr(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown graph builtin '{name}'"),
            None,
        )),
    }
}

// Graph is stored as a Record with:
//   nodes: Map { node_id -> Record{attrs} }
//   edges: List [ Record{from, to, weight, attrs} ]
//   directed: Bool

fn extract_graph(val: &Value) -> Result<&HashMap<String, Value>> {
    match val {
        Value::Record(map) | Value::Map(map) => {
            if map.contains_key("_graph") {
                Ok(map)
            } else {
                Err(BioLangError::type_error("expected a graph value", None))
            }
        }
        _ => Err(BioLangError::type_error(
            format!("expected graph, got {}", val.type_of()),
            None,
        )),
    }
}

fn get_nodes(g: &HashMap<String, Value>) -> HashMap<String, Value> {
    match g.get("nodes") {
        Some(Value::Map(m) | Value::Record(m)) => m.clone(),
        _ => HashMap::new(),
    }
}

fn get_edges(g: &HashMap<String, Value>) -> Vec<Value> {
    match g.get("edges") {
        Some(Value::List(v)) => v.clone(),
        _ => Vec::new(),
    }
}

fn is_directed(g: &HashMap<String, Value>) -> bool {
    matches!(g.get("directed"), Some(Value::Bool(true)))
}

fn make_graph(nodes: HashMap<String, Value>, edges: Vec<Value>, directed: bool) -> Value {
    let mut map = HashMap::new();
    map.insert("_graph".into(), Value::Bool(true));
    map.insert("nodes".into(), Value::Map(nodes));
    map.insert("edges".into(), Value::List(edges));
    map.insert("directed".into(), Value::Bool(directed));
    Value::Record(map)
}

fn edge_endpoints(e: &Value) -> Option<(String, String)> {
    match e {
        Value::Record(m) | Value::Map(m) => {
            let from = m.get("from")?.as_str()?.to_string();
            let to = m.get("to")?.as_str()?.to_string();
            Some((from, to))
        }
        _ => None,
    }
}

/// graph() or graph({directed: true})
fn builtin_graph(args: Vec<Value>) -> Result<Value> {
    let directed = if !args.is_empty() {
        match &args[0] {
            Value::Record(m) | Value::Map(m) => {
                matches!(m.get("directed"), Some(Value::Bool(true)))
            }
            Value::Bool(b) => *b,
            _ => false,
        }
    } else {
        false
    };
    Ok(make_graph(HashMap::new(), Vec::new(), directed))
}

/// add_node(g, id) or add_node(g, id, {attrs})
fn builtin_add_node(args: Vec<Value>) -> Result<Value> {
    let g = extract_graph(&args[0])?;
    let id = match &args[1] {
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("add_node() id must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let attrs = if args.len() > 2 {
        args[2].clone()
    } else {
        Value::Record(HashMap::new())
    };
    let mut nodes = get_nodes(g);
    let edges = get_edges(g);
    let directed = is_directed(g);
    nodes.insert(id, attrs);
    Ok(make_graph(nodes, edges, directed))
}

/// add_edge(g, from, to) or add_edge(g, from, to, {weight: 1.0, ...})
fn builtin_add_edge(args: Vec<Value>) -> Result<Value> {
    let g = extract_graph(&args[0])?;
    let from = match &args[1] {
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("add_edge() from must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let to = match &args[2] {
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("add_edge() to must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let mut nodes = get_nodes(g);
    let mut edges = get_edges(g);
    let directed = is_directed(g);
    // Auto-add nodes if missing
    nodes.entry(from.clone()).or_insert(Value::Record(HashMap::new()));
    nodes.entry(to.clone()).or_insert(Value::Record(HashMap::new()));
    let mut edge_map = HashMap::new();
    edge_map.insert("from".into(), Value::Str(from));
    edge_map.insert("to".into(), Value::Str(to));
    if args.len() > 3 {
        if let Value::Record(attrs) | Value::Map(attrs) = &args[3] {
            for (k, v) in attrs {
                edge_map.insert(k.clone(), v.clone());
            }
        }
    }
    edges.push(Value::Record(edge_map));
    Ok(make_graph(nodes, edges, directed))
}

/// remove_node(g, id)
fn builtin_remove_node(args: Vec<Value>) -> Result<Value> {
    let g = extract_graph(&args[0])?;
    let id = match &args[1] {
        Value::Str(s) => s.as_str(),
        other => {
            return Err(BioLangError::type_error(
                format!("remove_node() id must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let mut nodes = get_nodes(g);
    let edges = get_edges(g);
    let directed = is_directed(g);
    nodes.remove(id);
    let edges: Vec<Value> = edges
        .into_iter()
        .filter(|e| {
            edge_endpoints(e)
                .map(|(f, t)| f != id && t != id)
                .unwrap_or(true)
        })
        .collect();
    Ok(make_graph(nodes, edges, directed))
}

/// remove_edge(g, from, to)
fn builtin_remove_edge(args: Vec<Value>) -> Result<Value> {
    let g = extract_graph(&args[0])?;
    let from = match &args[1] {
        Value::Str(s) => s.as_str(),
        other => {
            return Err(BioLangError::type_error(
                format!("remove_edge() from must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let to = match &args[2] {
        Value::Str(s) => s.as_str(),
        other => {
            return Err(BioLangError::type_error(
                format!("remove_edge() to must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let nodes = get_nodes(g);
    let edges = get_edges(g);
    let directed = is_directed(g);
    let mut removed = false;
    let edges: Vec<Value> = edges
        .into_iter()
        .filter(|e| {
            if removed {
                return true;
            }
            if let Some((f, t)) = edge_endpoints(e) {
                if f == from && t == to {
                    removed = true;
                    return false;
                }
            }
            true
        })
        .collect();
    Ok(make_graph(nodes, edges, directed))
}

/// neighbors(g, node_id) → List[Str]
fn builtin_neighbors(args: Vec<Value>) -> Result<Value> {
    let g = extract_graph(&args[0])?;
    let id = match &args[1] {
        Value::Str(s) => s.as_str(),
        other => {
            return Err(BioLangError::type_error(
                format!("neighbors() id must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let edges = get_edges(g);
    let directed = is_directed(g);
    let mut nbrs: Vec<String> = Vec::new();
    for e in &edges {
        if let Some((f, t)) = edge_endpoints(e) {
            if f == id {
                nbrs.push(t);
            } else if !directed && t == id {
                nbrs.push(f);
            }
        }
    }
    nbrs.sort();
    nbrs.dedup();
    Ok(Value::List(nbrs.into_iter().map(Value::Str).collect()))
}

/// degree(g, node_id) → Int
fn builtin_degree(args: Vec<Value>) -> Result<Value> {
    let g = extract_graph(&args[0])?;
    let id = match &args[1] {
        Value::Str(s) => s.as_str(),
        other => {
            return Err(BioLangError::type_error(
                format!("degree() id must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let edges = get_edges(g);
    let directed = is_directed(g);
    let mut count = 0i64;
    for e in &edges {
        if let Some((f, t)) = edge_endpoints(e) {
            if f == id || (!directed && t == id) {
                count += 1;
            }
        }
    }
    Ok(Value::Int(count))
}

/// shortest_path(g, from, to) → List[Str] (BFS)
fn builtin_shortest_path(args: Vec<Value>) -> Result<Value> {
    let g = extract_graph(&args[0])?;
    let from = match &args[1] {
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("shortest_path() from must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let to = match &args[2] {
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("shortest_path() to must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let edges = get_edges(g);
    let directed = is_directed(g);

    // Build adjacency list
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for e in &edges {
        if let Some((f, t)) = edge_endpoints(e) {
            adj.entry(f.clone()).or_default().push(t.clone());
            if !directed {
                adj.entry(t).or_default().push(f);
            }
        }
    }

    // BFS
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<Vec<String>> = VecDeque::new();
    queue.push_back(vec![from.clone()]);
    visited.insert(from);

    while let Some(path) = queue.pop_front() {
        let current = path.last().unwrap();
        if *current == to {
            return Ok(Value::List(path.into_iter().map(Value::Str).collect()));
        }
        if let Some(neighbors) = adj.get(current) {
            for nbr in neighbors {
                if !visited.contains(nbr) {
                    visited.insert(nbr.clone());
                    let mut new_path = path.clone();
                    new_path.push(nbr.clone());
                    queue.push_back(new_path);
                }
            }
        }
    }

    Ok(Value::Nil) // no path found
}

/// connected_components(g) → List[List[Str]]
fn builtin_connected_components(args: Vec<Value>) -> Result<Value> {
    let g = extract_graph(&args[0])?;
    let nodes = get_nodes(g);
    let edges = get_edges(g);

    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for id in nodes.keys() {
        adj.entry(id.clone()).or_default();
    }
    for e in &edges {
        if let Some((f, t)) = edge_endpoints(e) {
            adj.entry(f.clone()).or_default().push(t.clone());
            adj.entry(t.clone()).or_default().push(f);
        }
    }

    let mut visited: HashSet<String> = HashSet::new();
    let mut components: Vec<Value> = Vec::new();

    for node in nodes.keys() {
        if visited.contains(node) {
            continue;
        }
        let mut component = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(node.clone());
        visited.insert(node.clone());
        while let Some(current) = queue.pop_front() {
            component.push(Value::Str(current.clone()));
            if let Some(nbrs) = adj.get(&current) {
                for nbr in nbrs {
                    if !visited.contains(nbr) {
                        visited.insert(nbr.clone());
                        queue.push_back(nbr.clone());
                    }
                }
            }
        }
        components.push(Value::List(component));
    }

    Ok(Value::List(components))
}

/// nodes(g) → List[Str]
fn builtin_nodes(args: Vec<Value>) -> Result<Value> {
    let g = extract_graph(&args[0])?;
    let nodes = get_nodes(g);
    let mut ids: Vec<String> = nodes.keys().cloned().collect();
    ids.sort();
    Ok(Value::List(ids.into_iter().map(Value::Str).collect()))
}

/// edges(g) → Table{from, to, weight}
fn builtin_edges(args: Vec<Value>) -> Result<Value> {
    let g = extract_graph(&args[0])?;
    let edges = get_edges(g);
    let cols = vec!["from".into(), "to".into(), "weight".into()];
    let mut rows = Vec::new();
    for e in &edges {
        if let Some((f, t)) = edge_endpoints(e) {
            let w = match e {
                Value::Record(m) | Value::Map(m) => match m.get("weight") {
                    Some(Value::Float(f)) => Value::Float(*f),
                    Some(Value::Int(i)) => Value::Int(*i),
                    _ => Value::Float(1.0),
                },
                _ => Value::Float(1.0),
            };
            rows.push(vec![Value::Str(f), Value::Str(t), w]);
        }
    }
    Ok(Value::Table(Table::new(cols, rows)))
}

/// has_node(g, id) → Bool
fn builtin_has_node(args: Vec<Value>) -> Result<Value> {
    let g = extract_graph(&args[0])?;
    let id = match &args[1] {
        Value::Str(s) => s.as_str(),
        other => {
            return Err(BioLangError::type_error(
                format!("has_node() id must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let nodes = get_nodes(g);
    Ok(Value::Bool(nodes.contains_key(id)))
}

/// has_edge(g, from, to) → Bool
fn builtin_has_edge(args: Vec<Value>) -> Result<Value> {
    let g = extract_graph(&args[0])?;
    let from = match &args[1] {
        Value::Str(s) => s.as_str(),
        other => {
            return Err(BioLangError::type_error(
                format!("has_edge() from must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let to = match &args[2] {
        Value::Str(s) => s.as_str(),
        other => {
            return Err(BioLangError::type_error(
                format!("has_edge() to must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let edges = get_edges(g);
    let found = edges.iter().any(|e| {
        edge_endpoints(e)
            .map(|(f, t)| f == from && t == to)
            .unwrap_or(false)
    });
    Ok(Value::Bool(found))
}

/// subgraph(g, node_ids) → Graph (induced subgraph)
fn builtin_subgraph(args: Vec<Value>) -> Result<Value> {
    let g = extract_graph(&args[0])?;
    let keep: HashSet<String> = match &args[1] {
        Value::List(items) => items
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "subgraph() second arg must be List of Str",
                None,
            ))
        }
    };
    let nodes = get_nodes(g);
    let edges = get_edges(g);
    let directed = is_directed(g);
    let sub_nodes: HashMap<String, Value> = nodes
        .into_iter()
        .filter(|(k, _)| keep.contains(k))
        .collect();
    let sub_edges: Vec<Value> = edges
        .into_iter()
        .filter(|e| {
            edge_endpoints(e)
                .map(|(f, t)| keep.contains(&f) && keep.contains(&t))
                .unwrap_or(false)
        })
        .collect();
    Ok(make_graph(sub_nodes, sub_edges, directed))
}

/// node_attr(g, id) → Record (attributes for a node)
fn builtin_node_attr(args: Vec<Value>) -> Result<Value> {
    let g = extract_graph(&args[0])?;
    let id = match &args[1] {
        Value::Str(s) => s.as_str(),
        other => {
            return Err(BioLangError::type_error(
                format!("node_attr() id must be Str, got {}", other.type_of()),
                None,
            ))
        }
    };
    let nodes = get_nodes(g);
    match nodes.get(id) {
        Some(v) => Ok(v.clone()),
        None => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("node_attr(): node '{id}' not found"),
            None,
        )),
    }
}
