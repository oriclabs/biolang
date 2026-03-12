# Knowledge Graphs

BioLang includes a built-in graph data structure for modeling biological networks — protein interactions, gene regulatory networks, metabolic pathways, and more.

## Creating Graphs

```
# Empty undirected graph
let g = graph()

# Directed graph
let g = graph(true)
```

## Adding Nodes and Edges

Nodes are identified by string IDs and can carry arbitrary attributes:

```
let g = graph()
let g = add_node(g, "BRCA1", {biotype: "protein_coding", chrom: "chr17"})
let g = add_node(g, "TP53", {biotype: "protein_coding", chrom: "chr17"})
let g = add_edge(g, "BRCA1", "TP53", {score: 0.99, source: "STRING"})
```

Adding an edge auto-creates nodes that don't exist yet:

```
let g = graph()
let g = add_edge(g, "A", "B")  # both A and B are created
has_node(g, "A")                # true
```

## Querying the Graph

```
# Build a small PPI network
let g = graph()
let g = add_edge(g, "BRCA1", "TP53", {score: 0.99})
let g = add_edge(g, "TP53", "MDM2", {score: 0.97})
let g = add_edge(g, "BRCA1", "BARD1", {score: 0.95})
let g = add_edge(g, "MDM2", "CDKN2A", {score: 0.85})

# Direct neighbors
neighbors(g, "TP53")           # ["BRCA1", "MDM2"]

# Node degree
degree(g, "BRCA1")             # 2

# Shortest path (BFS)
shortest_path(g, "BARD1", "CDKN2A")  # ["BARD1", "BRCA1", "TP53", "MDM2", "CDKN2A"]

# All nodes and edges
nodes(g)                       # ["BARD1", "BRCA1", "CDKN2A", "MDM2", "TP53"]
edges(g)                       # Table with from, to, weight columns
```

## Graph Analysis

### Connected Components

Find disconnected subnetworks:

```
let g = graph()
let g = add_edge(g, "BRCA1", "TP53")
let g = add_edge(g, "BRCA1", "BARD1")
let g = add_node(g, "ISOLATED_GENE")
let g = add_edge(g, "CDK2", "CCND1")

let components = connected_components(g)
print("Number of components: " + str(len(components)))
# 3: [BRCA1, TP53, BARD1], [ISOLATED_GENE], [CDK2, CCND1]
```

### Induced Subgraph

Extract a subgraph containing only specified nodes and their connecting edges:

```
let g = graph()
let g = add_edge(g, "A", "B")
let g = add_edge(g, "B", "C")
let g = add_edge(g, "C", "D")

let sub = subgraph(g, ["A", "B", "C"])
has_edge(sub, "A", "B")    # true
has_edge(sub, "C", "D")    # false (D not in subgraph)
```

### Node Attributes

```
let g = graph()
let g = add_node(g, "EGFR", {
    biotype: "protein_coding",
    chrom: "chr7",
    pathway: "EGFR signaling"
})

let attrs = node_attr(g, "EGFR")
print(attrs.pathway)    # "EGFR signaling"
```

## Removing Nodes and Edges

```
let g = graph()
let g = add_edge(g, "A", "B")
let g = add_edge(g, "B", "C")

# Remove a single edge
let g = remove_edge(g, "A", "B")
has_edge(g, "A", "B")    # false

# Remove a node (also removes all connected edges)
let g = remove_node(g, "B")
has_node(g, "B")          # false
```

## Directed vs Undirected

By default, graphs are undirected — edges go both ways:

```
let g = graph()
let g = add_edge(g, "A", "B")
neighbors(g, "B")    # ["A"] — B sees A as neighbor
```

For directed graphs (e.g., regulatory networks):

```
let g = graph(true)
let g = add_edge(g, "TF", "TARGET")
neighbors(g, "TF")      # ["TARGET"]
neighbors(g, "TARGET")  # [] — directed, no reverse edge
```

## Real-World Example: STRING Network Analysis

```
# requires: internet connection
# Fetch protein interactions from STRING
let network = string_network(["BRCA1"], 9606)

# Build graph from API results — network is list of {protein_a, protein_b, score}
let g = graph()
let g = network |> reduce(g, |g, edge|
    add_edge(g, edge.protein_a, edge.protein_b, {score: edge.score})
)

# Find hub genes (highest degree)
let gene_degrees = nodes(g) |> map(|n| {gene: n, deg: degree(g, n)})
gene_degrees
  |> sort_by(|r| r.deg, desc: true)
  |> take(10)
  |> each(|r| print(r.gene + ": " + str(r.deg) + " interactions"))

# Check connectivity
let components = connected_components(g)
print("Connected components: " + str(len(components)))
```

## Builtin Reference

| Function | Args | Description |
|---|---|---|
| `graph()` | `[directed]` | Create empty graph (default: undirected) |
| `add_node(g, id, [attrs])` | graph, Str, [Record] | Add node with optional attributes |
| `add_edge(g, from, to, [attrs])` | graph, Str, Str, [Record] | Add edge (auto-creates nodes) |
| `remove_node(g, id)` | graph, Str | Remove node and its edges |
| `remove_edge(g, from, to)` | graph, Str, Str | Remove first matching edge |
| `neighbors(g, id)` | graph, Str | List of neighbor node IDs |
| `degree(g, id)` | graph, Str | Number of edges for a node |
| `shortest_path(g, from, to)` | graph, Str, Str | BFS shortest path (nil if none) |
| `connected_components(g)` | graph | List of node-ID lists per component |
| `nodes(g)` | graph | Sorted list of all node IDs |
| `edges(g)` | graph | Table with from, to, weight columns |
| `has_node(g, id)` | graph, Str | Bool |
| `has_edge(g, from, to)` | graph, Str, Str | Bool |
| `subgraph(g, ids)` | graph, List[Str] | Induced subgraph |
| `node_attr(g, id)` | graph, Str | Attributes record for a node |
