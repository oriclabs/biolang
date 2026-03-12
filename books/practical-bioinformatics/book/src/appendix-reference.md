# Appendix D: Quick Reference Card

A concise reference for BioLang syntax, builtins, REPL commands, and CLI usage.

## Language Syntax

### Variables

```bio
let x = 42
let name = "BRCA1"
let seq = dna"ATGCGATCG"
let rna_seq = rna"AUGCGAUCG"
let protein = protein"MARS"
```

Reassignment (updates an existing binding):

```bio
x = 100
```

### Types

| Type | Example | Notes |
|------|---------|-------|
| `Int` | `42` | Integer |
| `Float` | `3.14` | Floating-point |
| `Str` | `"hello"` | String |
| `Bool` | `true`, `false` | Boolean |
| `Nil` | `nil` | Null value |
| `DNA` | `dna"ATGC"` | DNA sequence |
| `RNA` | `rna"AUGC"` | RNA sequence |
| `Protein` | `protein"MARS"` | Amino acid sequence |
| `List` | `[1, 2, 3]` | Ordered collection |
| `Record` | `{name: "A", val: 1}` | Named fields |
| `Table` | `to_table(rows, cols)` | 2D data structure |
| `Interval` | `interval("chr1", 100, 200)` | Genomic region |
| `Function` | `fn(x) { x + 1 }` | Named function |
| `Closure` | `\|x\| x + 1` | Anonymous function |
| `Stream` | `stream_fastq(path)` | Lazy iterator |

### Operators

| Operator | Meaning | Example |
|----------|---------|---------|
| `+` `-` `*` `/` | Arithmetic | `3 + 4` |
| `%` | Modulo | `17 % 5` |
| `**` | Power | `2 ** 10` |
| `==` `!=` | Equality | `x == 5` |
| `<` `>` `<=` `>=` | Comparison | `x > 0` |
| `and` `or` `not` | Logical | `x > 0 and x < 10` |
| `\|>` | Pipe | `x \|> f()` |
| `~` | Approximate | Pattern matching |
| `..` | Range | `1..10` |

### Pipe Syntax

The pipe operator passes the left-hand value as the first argument to the right-hand function:

```bio
# These are equivalent:
x |> f(y)
f(x, y)

# Chaining multiple operations:
data
  |> filter(|r| r.quality > 30)
  |> map(|r| gc_content(r.sequence))
  |> mean()
```

### Functions

Named functions:

```bio
let square = fn(x) {
  x * x
}
```

Closures (anonymous functions / lambdas):

```bio
|x| x * 2
|a, b| a + b
|r| r.quality >= 30
```

### Records

```bio
let gene = {name: "TP53", chrom: "chr17", start: 7571720}
gene.name        # Access field: "TP53"
keys(gene)       # ["name", "chrom", "start"]
values(gene)     # ["TP53", "chr17", 7571720]
```

### Lists

```bio
let nums = [1, 2, 3, 4, 5]
nums[0]          # First element: 1
len(nums)        # Length: 5
nums |> map(|x| x * 2)    # [2, 4, 6, 8, 10]
nums |> filter(|x| x > 3) # [4, 5]
```

### Tables

```bio
let t = to_table(rows, ["name", "value", "score"])
t |> select(["name", "score"])
t |> where(|row| row.score > 0.5)
t |> mutate("log_score", |row| log2(row.score))
t |> summarize(|key, rows| {category: key, mean_score: mean(rows |> col("score"))})
t |> group_by("category")
t |> sort_by("score", "desc")
```

### Control Flow

```bio
# If/else
if x > 0 then
  println("positive")
else
  println("non-positive")
end

# For loop
for item in items do
  println(item)
end

# While loop
while x > 0 do
  x = x - 1
end
```

### Error Handling

```bio
try
  let data = read_fasta("missing.fa")
catch e
  println(f"Error: {e}")
end
```

> **Requires CLI:** This example uses file I/O not available in the browser. Run with `bl run`.

### Imports

```bio
import "utils.bl"
import "helpers.bl" as h
h.my_function()
```

## Builtins by Category

### Sequence Operations

| Function | Description |
|----------|-------------|
| `gc_content(seq)` | GC fraction (0.0-1.0) |
| `complement(seq)` | Complementary strand |
| `reverse_complement(seq)` | Reverse complement |
| `translate(seq)` | DNA/RNA to protein |
| `kmers(seq, k)` | List of k-mers |
| `find_motif(seq, pattern)` | Find motif positions |

### File I/O

| Function | Description |
|----------|-------------|
| `read_fasta(path)` | Read FASTA file, returns list of records |
| `read_fastq(path)` | Read FASTQ file, returns list of records |
| `read_csv(path)` | Read CSV file, returns table |
| `read_vcf(path)` | Read VCF file, returns list of variant records |
| `read_bed(path)` | Read BED file, returns list of interval records |
| `read_gff(path)` | Read GFF/GTF file, returns list of feature records |
| `write_csv(table, path)` | Write table to CSV |
| `write_fasta(records, path)` | Write records to FASTA |

### Streaming

| Function | Description |
|----------|-------------|
| `stream_fastq(path)` | Lazy FASTQ iterator (memory-efficient) |
| `stream_fasta(path)` | Lazy FASTA iterator (memory-efficient) |

### Table Operations

| Function | Description |
|----------|-------------|
| `to_table(rows, columns)` | Create table from row data and column names |
| `select(table, columns)` | Select columns by name |
| `where(table, predicate)` | Filter rows by condition |
| `mutate(table, name, func)` | Add or transform a column |
| `summarize(table, name, func, col)` | Aggregate a column |
| `join_tables(t1, t2, key)` | Join two tables on a key column |
| `group_by(table, column)` | Group rows by column value |
| `sort_by(table, column, order)` | Sort rows (`"asc"` or `"desc"`) |

### Statistics

| Function | Description |
|----------|-------------|
| `mean(list)` | Arithmetic mean |
| `median(list)` | Median value |
| `stdev(list)` | Standard deviation |
| `var(list)` | Variance |
| `t_test(list1, list2)` | Two-sample t-test |
| `cor(list1, list2)` | Pearson correlation |

### Math

| Function | Description |
|----------|-------------|
| `log(x)` | Natural logarithm |
| `log2(x)` | Base-2 logarithm |
| `log10(x)` | Base-10 logarithm |
| `abs(x)` | Absolute value |
| `sqrt(x)` | Square root |
| `pow(base, exp)` | Exponentiation |
| `round(x)` | Round to nearest integer |
| `ceil(x)` | Round up |
| `floor(x)` | Round down |

### Visualization

| Function | Description |
|----------|-------------|
| `scatter(x, y, opts)` | Scatter plot |
| `bar(labels, values, opts)` | Bar chart |
| `hist(values, opts)` | Histogram |
| `heatmap(matrix, opts)` | Heatmap |
| `box(groups, opts)` | Box plot |
| `line(x, y, opts)` | Line chart |
| `volcano(log2fc, pvals, opts)` | Volcano plot |
| `dotplot(data, opts)` | Dot plot |
| `phylo_tree(tree, opts)` | Phylogenetic tree |

### String Operations

| Function | Description |
|----------|-------------|
| `split(str, delimiter)` | Split string into list |
| `join(list, delimiter)` | Join list into string |
| `trim(str)` | Remove leading/trailing whitespace |
| `upper(str)` | Convert to uppercase |
| `lower(str)` | Convert to lowercase |
| `contains(str, substring)` | Check if substring exists |
| `starts_with(str, prefix)` | Check prefix |
| `ends_with(str, suffix)` | Check suffix |
| `replace(str, old, new)` | Replace occurrences |

### Higher-Order Functions

| Function | Description |
|----------|-------------|
| `map(collection, func)` | Transform each element |
| `filter(collection, func)` | Keep elements matching predicate |
| `reduce(collection, func, init)` | Fold into single value |
| `sort(collection, func)` | Sort by comparison function |
| `each(collection, func)` | Execute function for each element (no return) |
| `flatten(nested_list)` | Flatten one level of nesting |
| `group_by(list, func)` | Group elements by key function |
| `par_map(collection, func)` | Parallel map (multi-threaded) |
| `par_filter(collection, func)` | Parallel filter (multi-threaded) |

### API Access

| Function | Description |
|----------|-------------|
| `ncbi_search(db, query)` | Search NCBI database |
| `ncbi_gene(symbol, species)` | Get gene info from NCBI |
| `ncbi_sequence(id)` | Fetch sequence by accession |
| `ensembl_gene(id_or_symbol)` | Get gene info from Ensembl |
| `ensembl_vep(hgvs)` | Variant Effect Predictor |
| `uniprot_search(query)` | Search UniProt |
| `uniprot_entry(accession)` | Get UniProt entry |
| `ucsc_sequence(genome, chrom, start, end)` | Get UCSC sequence |
| `kegg_get(id)` | Get KEGG entry |
| `kegg_find(db, query)` | Search KEGG |
| `go_term(id)` | Get Gene Ontology term |
| `go_annotations(gene)` | Get GO annotations |
| `string_network(genes, species)` | STRING protein network |
| `pdb_entry(id)` | Get PDB structure entry |
| `reactome_pathways(gene)` | Get Reactome pathways |
| `cosmic_gene(symbol)` | COSMIC cancer mutations |
| `datasets_gene(symbol)` | NCBI Datasets gene info |

### Utility Functions

| Function | Description |
|----------|-------------|
| `println(value)` | Print to stdout with newline |
| `len(collection)` | Length of list, string, or table |
| `typeof(value)` | Type name as string |
| `keys(record)` | Record field names |
| `values(record)` | Record field values |
| `range(start, end)` | Integer range |
| `zip(list1, list2)` | Pair elements from two lists |
| `json_encode(value)` | Convert to JSON string |
| `json_decode(str)` | Parse JSON string to value |

### File System

| Function | Description |
|----------|-------------|
| `file_exists(path)` | Check if file exists |
| `read_lines(path)` | Read file as list of lines |
| `write_lines(lines, path)` | Write list of lines to file |
| `mkdir(path)` | Create directory |
| `list_dir(path)` | List directory contents |

### LLM Integration

| Function | Description |
|----------|-------------|
| `chat(prompt)` | Send prompt to configured LLM, returns response |

## REPL Commands

Type these at the `bl>` prompt (they start with `:`):

| Command | Description |
|---------|-------------|
| `:help` | Show all available REPL commands |
| `:env` | Display all variables in the current environment |
| `:reset` | Clear the environment and start fresh |
| `:load file.bl` | Load and execute a BioLang script |
| `:save file.bl` | Save the current session history to a file |
| `:time expression` | Execute an expression and print elapsed time |
| `:type expression` | Show the type of an expression without executing it |
| `:profile expression` | Profile execution with detailed timing |
| `:plugins` | List available plugins |
| `:history` | Show command history for the session |
| `:plot` | Display the most recently generated plot |

## CLI Commands

The `bl` command-line tool:

| Command | Description |
|---------|-------------|
| `bl run script.bl` | Execute a BioLang script |
| `bl repl` | Start interactive REPL (also: `bl` with no args) |
| `bl lsp` | Start the Language Server Protocol server |
| `bl init project-name` | Scaffold a new project directory |
| `bl plugins` | List installed plugins |

### Common Usage Patterns

Run a script:

```bash
bl run analysis.bl
```

Run a one-liner:

```bash
bl -e 'gc_content(dna"ATGCGATCG") |> println()'
```

Start the REPL and load a file:

```bash
bl repl
bl> :load helpers.bl
bl> my_function("input.fasta")
```

Run with environment variables:

```bash
NCBI_API_KEY=your-key bl run fetch_genes.bl
```

## Common Patterns

### Read, Filter, Analyze

```bio
read_fastq("data.fastq")
  |> filter(|r| r.quality >= 30)
  |> map(|r| gc_content(r.sequence))
  |> mean()
```

> **Requires CLI:** This example uses file I/O not available in the browser. Run with `bl run`.

### Stream Large Files

```bio
stream_fastq("huge.fastq")
  |> filter(|r| len(r.sequence) >= 100)
  |> each(|r| println(r.name))
```

> **Requires CLI:** This example uses file I/O not available in the browser. Run with `bl run`.

### Build a Summary Table

```bio
let reads = read_fastq("sample.fastq")
let rows = reads |> map(|r| {
  name: r.name,
  length: len(r.sequence),
  gc: gc_content(r.sequence),
  quality: r.quality
})
let t = to_table(rows, ["name", "length", "gc", "quality"])
t |> sort_by("gc", "desc") |> write_csv("summary.csv")
```

> **Requires CLI:** This example uses file I/O not available in the browser. Run with `bl run`.

### Fetch and Analyze from Database

```bio
let gene = ncbi_gene("TP53", "human")
let seq = ncbi_sequence(gene.id)
let motifs = find_motif(seq, "TATA")
println(f"Found {len(motifs)} TATA boxes in TP53")
```

> **Requires CLI:** This example uses network APIs not available in the browser. Run with `bl run`.

### Multi-Step Pipeline with Error Handling

```bio
try
  let variants = read_vcf("sample.vcf")
  let filtered = variants
    |> filter(|v| v.quality >= 30)
    |> filter(|v| v.alt != ".")
  println(f"Kept {len(filtered)} of {len(variants)} variants")
  write_csv(to_table(filtered, keys(filtered[0])), "filtered.csv")
catch e
  println(f"Pipeline failed: {e}")
end
```

> **Requires CLI:** This example uses file I/O not available in the browser. Run with `bl run`.

### Parallel Processing

```bio
let files = list_dir("fastq/") |> filter(|f| ends_with(f, ".fastq"))
let results = files |> par_map(|f| {
  let reads = read_fastq(f)
  {
    file: f,
    count: len(reads),
    mean_gc: reads |> map(|r| gc_content(r.sequence)) |> mean()
  }
})
to_table(results, ["file", "count", "mean_gc"]) |> write_csv("batch_results.csv")
```

> **Requires CLI:** This example uses file I/O not available in the browser. Run with `bl run`.
