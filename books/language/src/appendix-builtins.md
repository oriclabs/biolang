# Appendix A: Builtin Reference

BioLang ships with a comprehensive standard library of builtins designed for
bioinformatics workflows. Every function listed here is available without
imports -- they are part of the language runtime.

---

## Sequence Operations

Builtins that operate on bio-typed sequences (`dna`, `rna`, `protein`).

| Builtin | Description |
|---|---|
| `complement(seq) -> Dna \| Rna` | Watson-Crick complement of a nucleotide sequence |
| `reverse_complement(seq) -> Dna \| Rna` | Reverse complement -- the opposing strand |
| `transcribe(seq) -> Rna` | Transcribe DNA to RNA (T to U) |
| `translate(seq) -> Protein` | Translate an RNA or DNA coding sequence to amino acids |
| `gc_content(seq) -> Float` | GC fraction of a nucleotide sequence (0.0 -- 1.0) |
| `find_motif(seq, pattern) -> List[Int]` | All start positions where `pattern` occurs in `seq` |
| `hamming_distance(a, b) -> Int` | Number of mismatched positions between equal-length sequences |
| `edit_distance(a, b) -> Int` | Edit distance between two sequences |
| `find_orfs(seq, min_len?) -> List[Record]` | Open reading frames with start, stop, and frame fields |
| `restriction_sites(seq, enzyme?) -> List[Record]` | Recognition sites for restriction enzymes |
| `tm(seq) -> Float` | Melting temperature estimate for a short oligonucleotide |
| `slice(seq, start, end) -> Dna \| Rna \| Protein` | Extract a subsequence by 0-based coordinates |

```biolang
# Example: quick primer analysis
let primer = dna"ATCGATCGATCG"
let rc     = reverse_complement(primer)
let temp   = tm(primer)
print("Primer Tm = " + str(temp) + "C, reverse complement = " + str(rc))
```

---

## Collection Operations

General-purpose operations on lists, records, and sets.

| Builtin | Description |
|---|---|
| `len(coll) -> Int` | Number of elements in a list, string, or sequence |
| `push(list, item) -> List` | Append an element, returning a new list |
| `pop(list) -> List` | Remove the last element, returning a new list |
| `concat(a, b) -> List` | Concatenate two lists |
| `flatten(nested) -> List` | Flatten one level of nesting |
| `reverse(list) -> List` | Reverse element order |
| `contains(coll, item) -> Bool` | True if `item` is present |
| `index_of(list, item) -> Int \| Nil` | First index of `item`, or nil |
| `last(list) -> Any` | Last element |
| `first(list) -> Any` | First element |
| `head(list, n) -> List` | First `n` elements |
| `tail(list, n) -> List` | Last `n` elements |
| `unique(list) -> List` | Remove duplicates, preserving order |
| `zip(a, b) -> List` | Pair elements from two lists into a list of tuples |
| `enumerate(list) -> List` | Pair each element with its 0-based index |
| `chunk(list, size) -> List[List]` | Split into fixed-size sublists |
| `window(list, size) -> List[List]` | Sliding window of the given size |
| `scan(list, init, fn) -> List` | Running accumulation (like reduce but keeps intermediates) |
| `range(start, end, step?) -> List` | Integer range |
| `set(list) -> Set` | Convert a list to a deduplicated set |
| `keys(record) -> List[Str]` | Field names of a record |
| `values(record) -> List` | Field values of a record |
| `has_key(record, key) -> Bool` | True if the record contains the named field |
| `sort_by(list, fn) -> List` | Sort by a key function |
| `group_by(list, fn) -> Record` | Group elements by a key function into a record of lists |
| `partition(list, fn) -> [List, List]` | Split into elements that pass and fail a predicate |

```biolang
# Example: enumerate quality-filtered reads
let good_reads = read_fastq("data/reads.fastq")
  |> filter(|r| mean_phred(r.quality) > 30)
  |> enumerate()
  |> head(5)
```

---

## Higher-Order Functions

Functions that accept other functions as arguments -- the backbone of
BioLang's pipeline style.

| Builtin | Description |
|---|---|
| `map(coll, fn) -> List` | Apply `fn` to every element |
| `filter(coll, fn) -> List` | Keep elements where `fn` returns true |
| `reduce(coll, init, fn) -> Any` | Fold elements into a single value |
| `sort(coll, fn?) -> List` | Sort, optionally by comparator |
| `each(coll, fn) -> Nil` | Execute `fn` for side effects on every element |
| `flat_map(coll, fn) -> List` | Map then flatten one level |
| `take_while(coll, fn) -> List` | Take leading elements while predicate holds |
| `any(coll, fn) -> Bool` | True if `fn` returns true for at least one element |
| `all(coll, fn) -> Bool` | True if `fn` returns true for every element |
| `none(coll, fn) -> Bool` | True if `fn` returns true for no elements |
| `find(coll, fn) -> Any \| Nil` | First element satisfying `fn` |
| `find_index(coll, fn) -> Int \| Nil` | Index of first element satisfying `fn` |
| `par_map(coll, fn) -> List` | Parallel map across available cores |
| `par_filter(coll, fn) -> List` | Parallel filter across available cores |
| `mat_map(matrix, fn) -> Matrix` | Apply `fn` element-wise to a matrix |
| `try_call(fn, args) -> Result` | Call `fn` with `args`, capturing errors instead of panicking |

```biolang
# Example: parallel GC content across a genome's chromosomes
let chromosomes = [
  {name: "chrA", seq: dna"ATGCGCGTAA"},
  {name: "chrB", seq: dna"ATATATGCAT"}
]
let gc_values = chromosomes
  |> par_map(|chr| {name: chr.name, gc: gc_content(chr.seq)})
  |> sort_by(|r| r.gc)
```

---

## Table Operations

Tabular data manipulation inspired by dataframe semantics -- designed for
sample sheets, variant tables, and expression matrices.

| Builtin | Description |
|---|---|
| `table(data) -> Table` | Create a table from a list of records |
| `select(tbl, ...cols) -> Table` | Pick columns by name |
| `mutate(tbl, name, fn) -> Table` | Add or transform a column |
| `summarize(grouped, \|key, rows\| {...}) -> Table` | Aggregate grouped data via closure |
| `group_by(tbl, col) -> GroupedTable` | Group rows by a column value (table variant) |
| `inner_join(a, b, on) -> Table` | Keep rows whose key occurs in both tables |
| `left_join(a, b, on) -> Table` | Keep every left row and attach matching right columns |
| `csv(path) -> Table` | Read a CSV file into a table |
| `tsv(path) -> Table` | Read a TSV file into a table |
| `write_tsv(tbl, path) -> Nil` | Write a table to CSV |
| `write_tsv(tbl, path) -> Nil` | Write a table to TSV |
| `len(tbl) -> Int` | Number of rows |
| `ncols(tbl) -> Int` | Number of columns |
| `columns(tbl) -> List[Str]` | Column name list |
| `row_names(tbl) -> List[Str]` | Row name list (if set) |

```biolang
# Example: summarize variant counts per chromosome
tsv("variants.tsv")
  |> group_by("chrom")
  |> summarize(|chrom, rows| {count: len(rows), mean_qual: mean(col(rows, "quality"))})
  |> write_tsv("chrom_summary.csv")
```

---

## Bio File I/O

Read and write standard bioinformatics file formats. Readers return lazy
streams that integrate with pipes; writers flush on completion.

| Builtin | Description |
|---|---|
| `read_fasta(path) -> Table` | Parse FASTA; columns are `id`, `description`, `seq`, and `length` |
| `read_fastq(path) -> Table` | Parse FASTQ; columns are `id`, `description`, `seq`, `length`, and `quality` |
| `read_vcf(path) -> Table` | Parse VCF; columns include `chrom`, `pos`, `ref`, `alt`, `qual`, and `info` |
| `read_bed(path) -> Table` | Parse BED; columns include `chrom`, `start`, and `end` |
| `read_gff(path) -> Table` | Parse GFF/GTF; columns include `seqid`, `type`, `start`, `end`, and raw `attributes` text |
| `write_fasta(records, path) -> Nil` | Write records to FASTA format |
| `write_fastq(records, path) -> Nil` | Write records to FASTQ format |
| `write_bed(records, path) -> Nil` | Write records to BED format |

```biolang
# Example: filter FASTQ reads by quality and write survivors
read_fastq("data/reads.fastq")
  |> filter(|r| mean_phred(r.quality) > 30)
  |> write_fastq("sample_R1.filtered.fq")
```

---

## Genomic Intervals

Interval arithmetic for coordinate-based genomic analysis. Intervals carry
`chrom`, `start`, `end`, and optional `strand` and `data` fields.

| Builtin | Description |
|---|---|
| `interval(chrom, start, end, strand?) -> Interval` | Create a genomic interval |
| `interval_tree(intervals) -> IntervalTree` | Build an index for fast overlap queries |
| `query_overlaps(tree, chrom, start, end) -> Table` | Rows overlapping a half-open query range |
| `count_overlaps(tree, chrom, start, end) -> Int` | Number of rows overlapping a query range |
| `coverage(tree) -> Table` | Coverage segments with `chrom`, `start`, `end`, and `depth` |
| `merge_intervals(intervals, dist?) -> List[Interval]` | Merge overlapping or nearby intervals |
| `intersect(a, b) -> List[Interval]` | Pairwise intersection of two interval sets |
| `subtract(a, b) -> List[Interval]` | Regions in `a` not covered by `b` |

```biolang
# Example: find promoter-peak overlaps
let promoters = read_bed("data/regions.bed") |> map(|r| interval(r.chrom, r.start, r.end))
let peaks     = read_bed("data/exons.bed") |> map(|r| interval(r.chrom, r.start, r.end))
let tree      = interval_tree(peaks)
let hits      = promoters |> flat_map(|p| query_overlaps(tree, p.chrom, p.start, p.end))
print("Found " + str(len(hits)) + " promoter-peak overlaps")
```

---

## Variants

Builtins for working with genetic variant records. Variant objects carry
`chrom`, `pos`, `ref`, `alt`, `qual`, and `info` fields.

| Builtin | Description |
|---|---|
| `variant(chrom, pos, ref, alt) -> Variant` | Construct a variant record |
| `is_snp(v) -> Bool` | True if single-nucleotide polymorphism |
| `is_indel(v) -> Bool` | True if insertion or deletion |
| `is_transition(v) -> Bool` | True if purine-purine or pyrimidine-pyrimidine substitution |
| `is_transversion(v) -> Bool` | True if purine-pyrimidine substitution |
| `variant_type(v) -> Str` | Classification string: "snp", "ins", "del", "mnv", "complex" |
| `is_het(v) -> Bool` | True if heterozygous genotype |
| `is_hom_ref(v) -> Bool` | True if homozygous reference |
| `is_hom_alt(v) -> Bool` | True if homozygous alternate |
| `is_multiallelic(v) -> Bool` | True if more than one alt allele |
| `parse_vcf_info(info_str) -> Record` | Parse a VCF INFO field string into a record |
| `variant_summary(variants) -> Record` | Aggregate counts of SNPs, indels, Ti/Tv ratio, het/hom ratio |

```biolang
# Example: compute Ti/Tv ratio for a VCF
let vars = vcf_filter(vcf_parse(read_text("data/variants.vcf")), 30)
let summary = variant_summary(vars)
let snp_rows = summary |> filter(|row| row.type_ == "SNP")
let snp_count = if len(snp_rows) > 0 { snp_rows[0].count } else { 0 }
print(summary)
print("Ti/Tv = " + str(titv_ratio(vars)) + ", SNPs = " + str(snp_count))
```

---

## Statistics

Statistical functions for quality control, expression analysis, and
hypothesis testing.

| Builtin | Description |
|---|---|
| `mean(xs) -> Float` | Arithmetic mean |
| `median(xs) -> Float` | Median value |
| `stdev(xs) -> Float` | Sample standard deviation |
| `variance(xs) -> Float` | Sample variance |
| `quantile(xs, q) -> Float` | Quantile at fraction `q` (0.0 -- 1.0) |
| `min(xs) -> Num` | Minimum value |
| `max(xs) -> Num` | Maximum value |
| `sum(xs) -> Num` | Sum of all elements |
| `cor(xs, ys) -> Float` | Pearson correlation coefficient |
| `ttest(xs, ys) -> Record` | Two-sample t-test; returns `{statistic, pvalue}` |
| `chi_square(observed, expected) -> Record` | Chi-squared test; returns `{statistic, pvalue, df}` |
| `wilcoxon(xs, ys) -> Record` | Wilcoxon rank-sum test |
| `anova(groups) -> Record` | One-way ANOVA across groups |
| `fisher_exact(a, b, c, d) -> Record` | Fisher's exact test on a 2x2 contingency table |
| `p_adjust(pvals, method?) -> List[Float]` | Multiple testing correction (default: Benjamini-Hochberg) |
| `lm(xs, ys) -> Record` | Simple linear regression; returns `{slope, intercept, r_squared}` |
| `ks_test(xs, ys) -> Record` | Kolmogorov-Smirnov test |
| `mean_phred(quals) -> Float` | Mean Phred quality score from a quality string |

```biolang
# Example: differential expression significance
let control   = [5.2, 4.8, 5.1, 4.9]
let treatment = [8.1, 7.5, 8.3, 7.9]
let result    = ttest(control, treatment)
print("p-value = " + str(result.pvalue))
```

---

## Linear Algebra

Matrix operations for expression matrices, PCA, distance calculations, and
numerical biology.

| Builtin | Description |
|---|---|
| `matrix(data) -> Matrix` | Create a matrix from a list of lists (row-major) |
| `transpose(m) -> Matrix` | Transpose rows and columns |
| `mat_mul(a, b) -> Matrix` | Matrix multiplication |
| `determinant(m) -> Float` | Determinant of a square matrix |
| `inverse(m) -> Matrix` | Matrix inverse |
| `eigenvalues(m) -> List[Float]` | Eigenvalues of a square matrix |
| `svd(m) -> Record` | Singular value decomposition; returns `{u, s, vt}` |
| `solve(a, b) -> Matrix` | Solve the linear system Ax = b |
| `trace(m) -> Float` | Sum of diagonal elements |
| `norm(m, p?) -> Float` | Matrix or vector norm (default: Frobenius / L2) |
| `rank(m) -> Int` | Numerical rank |
| `eye(n) -> Matrix` | n x n identity matrix |
| `zeros(rows, cols) -> Matrix` | Matrix of zeros |
| `ones(rows, cols) -> Matrix` | Matrix of ones |
| `diag(values) -> Matrix` | Diagonal matrix from a list of values |
| `mat_map(m, fn) -> Matrix` | Apply `fn` element-wise |

```biolang
# Example: PCA on a gene expression matrix
let expr = tsv("examples/sample-data/counts.tsv") |> table()
let m    = matrix(expr |> select("gene_a", "gene_b", "gene_c"))
let decomp = svd(m)
print("Top 3 singular values: " + str(head(decomp.s, 3)))
```

---

## Math

Standard mathematical functions available for scoring, normalization, and
modeling.

| Builtin | Description |
|---|---|
| `abs(x) -> Num` | Absolute value |
| `ceil(x) -> Int` | Round up to nearest integer |
| `floor(x) -> Int` | Round down to nearest integer |
| `round(x, digits?) -> Float` | Round to `digits` decimal places (default: 0) |
| `sqrt(x) -> Float` | Square root |
| `log(x) -> Float` | Natural logarithm |
| `log2(x) -> Float` | Base-2 logarithm (common in fold-change analysis) |
| `log10(x) -> Float` | Base-10 logarithm |
| `exp(x) -> Float` | Euler's number raised to `x` |
| `pow(base, exp) -> Float` | Exponentiation |
| `sin(x) -> Float` | Sine |
| `cos(x) -> Float` | Cosine |
| `tan(x) -> Float` | Tangent |
| `ode_solve(fn, y0, t_span, dt?) -> List[Record]` | Numerical ODE integration (Runge-Kutta) |

```biolang
# Example: log2 fold-change between conditions
let control   = 12.5
let treatment = 50.0
let lfc = log2(treatment / control)
print("Log2 fold-change = " + str(lfc))
```

---

## String Operations

Text manipulation for parsing identifiers, annotations, and formatted output.

| Builtin | Description |
|---|---|
| `split(s, delim) -> List[Str]` | Split string on delimiter |
| `join(list, delim) -> Str` | Join list elements into a string |
| `trim(s) -> Str` | Strip leading and trailing whitespace |
| `upper(s) -> Str` | Convert to uppercase |
| `lower(s) -> Str` | Convert to lowercase |
| `starts_with(s, prefix) -> Bool` | True if `s` begins with `prefix` |
| `ends_with(s, suffix) -> Bool` | True if `s` ends with `suffix` |
| `replace(s, from, to) -> Str` | Replace all occurrences |
| `regex_match(s, pattern) -> Bool` | True if regex `pattern` matches |
| `format(template, ...args) -> Str` | Printf-style formatting |

BioLang also supports **f-strings** for inline interpolation:

```biolang
# Example: parse a FASTA header
let header = ">sp|P12345|MYG_HUMAN Myoglobin OS=Homo sapiens"
let parts  = split(header, "|")
let acc    = parts[1]
print("Accession: " + acc)
```

---

## Type Operations

Runtime type inspection and conversion -- useful for dynamic dispatch in
pipelines that handle mixed bio types.

| Builtin | Description |
|---|---|
| `type(val) -> Str` | Runtime type name as a string |
| `is_dna(val) -> Bool` | True if val is a DNA sequence |
| `is_rna(val) -> Bool` | True if val is an RNA sequence |
| `is_protein(val) -> Bool` | True if val is a protein sequence |
| `is_interval(val) -> Bool` | True if val is a genomic interval |
| `is_variant(val) -> Bool` | True if val is a variant record |
| `is_record(val) -> Bool` | True if val is a record |
| `is_list(val) -> Bool` | True if val is a list |
| `is_table(val) -> Bool` | True if val is a table |
| `is_nil(val) -> Bool` | True if val is nil |
| `is_int(val) -> Bool` | True if val is an integer |
| `is_float(val) -> Bool` | True if val is a float |
| `is_str(val) -> Bool` | True if val is a string |
| `is_bool(val) -> Bool` | True if val is a boolean |
| `into(val, target_type) -> Any` | Convert between compatible types |

```biolang
# Example: route processing based on sequence type
let seq = read_fasta("data/sequences.fasta") |> first() |> |r| r.seq
if is_dna(seq) then
  print("DNA, GC = " + str(gc_content(seq)))
else if is_protein(seq) then
  print("Protein, length = " + str(len(seq)))
```

---

## Bio APIs

Remote database queries for annotation enrichment. All API builtins are
async-aware and return structured records.

| Builtin | Description |
|---|---|
| `ncbi_search(db, query, max?) -> List[Str]` | Search NCBI Entrez databases (returns ID list) |
| `ncbi_gene(symbol, max?) -> Record or List[Str]` | Gene lookup: Record if single match, else ID list |
| `ncbi_sequence(acc) -> Str` | Fetch sequence by accession as FASTA text |
| `ensembl_gene(ensembl_id) -> Record` | Ensembl gene lookup by Ensembl ID |
| `ensembl_symbol(species, symbol) -> Record` | Ensembl gene lookup by species and symbol |
| `ensembl_vep(variants) -> List[Record]` | Variant Effect Predictor annotation |
| `uniprot_search(query, max?) -> List[Record]` | Search UniProt by keyword or accession |
| `uniprot_entry(acc) -> Record` | Full UniProt entry |
| `ucsc_sequence(genome, chrom, start, end) -> Dna` | Fetch genomic sequence from UCSC DAS |
| `kegg_get(entry) -> Record` | Retrieve a KEGG database entry |
| `kegg_find(db, query) -> List[Record]` | Search KEGG databases |
| `string_network(proteins, species) -> List[Record]` | STRING interactions: {protein_a, protein_b, score} |
| `pdb_entry(pdb_id) -> Record` | Fetch PDB structure metadata |
| `reactome_pathways(gene) -> List[Record]` | Reactome pathway memberships for a gene |
| `go_term(go_id) -> Record` | Gene Ontology term details |
| `go_annotations(gene, species?) -> List[Record]` | GO annotations for a gene |
| `cosmic_gene(symbol) -> Record` | COSMIC cancer gene census entry |
| `datasets_gene(symbol, taxon?) -> Record` | NCBI Datasets gene data |
| `biomart_query(dataset, attributes, filters?) -> Table` | BioMart query returning a table |
| `nfcore_list(sort_by?, limit?) -> List[Record]` | List nf-core pipelines |
| `nfcore_search(query, limit?) -> List[Record]` | Search nf-core pipelines by name/topic |
| `nfcore_info(name) -> Record` | Detailed nf-core pipeline metadata |
| `nfcore_releases(name) -> List[Record]` | Release history for an nf-core pipeline |
| `nfcore_params(name) -> Record` | Parameter schema for an nf-core pipeline |
| `biocontainers_search(query, limit?) -> List[Record]` | Search BioContainers registry |
| `biocontainers_popular(limit?) -> List[Record]` | Popular BioContainers tools |
| `biocontainers_info(tool) -> Record` | Detailed tool info with versions |
| `biocontainers_versions(tool) -> List[Record]` | All versions with container image URIs |
| `galaxy_search(query, limit?) -> List[Record]` | Search Galaxy ToolShed repositories |
| `galaxy_popular(limit?) -> List[Record]` | Popular Galaxy ToolShed tools |
| `galaxy_categories() -> List[Record]` | Galaxy ToolShed tool categories |
| `galaxy_tool(owner, name) -> Record` | Galaxy ToolShed repository details |
| `nf_parse(path) -> Record` | Parse a Nextflow .nf file into a structured Record |
| `nf_to_bl(record) -> Str` | Generate BioLang pipeline code from parsed Nextflow |
| `galaxy_to_bl(record) -> Str` | Generate BioLang pipeline code from Galaxy workflow |
| `api_endpoints() -> Record` | Show current API endpoint URLs |

```biolang
# requires: internet connection
# Example: annotate a gene list with pathway data
let genes = ["BRCA1", "TP53", "EGFR"]
genes |> each(|g| {
  let pathways = reactome_pathways(g)
  print(g + ": " + str(len(pathways)) + " pathways")
})
```

---

## Utility

General-purpose helpers for debugging, timing, unit conversion, and
serialization.

| Builtin | Description |
|---|---|
| `print(val) -> Nil` | Print a value followed by a newline |
| `assert cond, msg?` | Abort with `msg` if `cond` is false |
| `sleep(ms) -> Nil` | Pause execution for `ms` milliseconds |
| `now() -> Str` | Current UTC time in ISO 8601 format |
| `timestamp() -> Int` | Current Unix timestamp in seconds; subtract two values to measure elapsed time |
| `bp(n) -> Int` | Identity; documents that `n` is in base pairs |
| `kb(n) -> Int` | Convert kilobases to base pairs (`n * 1000`) |
| `mb(n) -> Int` | Convert megabases to base pairs (`n * 1_000_000`) |
| `gb(n) -> Int` | Convert gigabases to base pairs (`n * 1_000_000_000`) |
| `json_stringify(val) -> Str` | Serialize any value to a JSON string |
| `json_parse(s) -> Any` | Parse a JSON string into a BioLang value |
| `env(name) -> Str \| Nil` | Read an environment variable |
| `exit(code?) -> Never` | Terminate the process with an exit code (default: 0) |

```biolang
# Example: time a heavy operation
let t0 = timestamp()
let result = read_fasta("data/sequences.fasta")
  |> flat_map(|r| find_orfs(r.seq, 300))
print("Found " + str(len(result)) + " ORFs in " + str(timestamp() - t0) + "s")
```

---

## Single-Cell Analysis

Builtins for single-cell RNA-seq workflows. Higher-level operations are
available via the `singlecell`, `cellchat`, `spatial`, `velocity`,
`celltypes`, `multimodal`, and `grn` packages.

| Builtin | Arity | Description |
|---|---|---|
| `lr_score(matrix, cell_labels, lr_pairs)` | 3 | Pairwise ligand-receptor scoring between clusters |
| `lr_aggregate(lr_scores, pathway_map)` | 2 | Pathway-level aggregation of LR scores |
| `spatial_neighbors(coords, k)` | 2 | 2-D k-NN adjacency from spatial coordinates |
| `spatial_moransi(expr_vec, spatial_adj)` | 2 | Moran's I spatial autocorrelation statistic |
| `reference_classify(query, ref_matrix, ref_labels)` | 3 | Cosine k-NN label transfer from a reference dataset |
| `pseudobulk_aggregate(matrix, cell_labels, sample_labels)` | 3 | Sum cells×genes counts per (cluster, sample); rows are genes |
| `wnn_graph(matrix_a, matrix_b, k)` | 3 | Weighted nearest-neighbor graph for multimodal integration |
| `velocity_estimate(spliced, unspliced)` | 2 | RNA velocity (β·u − s) per gene per cell |

```biolang
import "singlecell" as sc
import "cellchat"   as cc

let obj = sc.load("data/pbmc3k/filtered_gene_bc_matrices/hg19/")
  |> sc.filter_cells(200, 5000, 20.0)
  |> sc.normalize
  |> sc.variable_genes(2000)
  |> sc.neighbors(15)
  |> sc.cluster

let scores = cc.score(obj)
cc.top(scores, 20)
cc.senders(scores)
cc.pathways(scores) |> take(10)
```

---

## Variants & Population Genetics

Builtins for VCF-level variant analysis and population genetics statistics.
The `variants` and `popgen` packages provide higher-level workflows.

### Variant Parsing

| Builtin | Arity | Description |
|---|---|---|
| `vcf_parse(text)` | 1 | Parse VCF text into a table of variant records |
| `vcf_filter(variants, min_qual, pass_only?)` | 1-3 | Filter a parsed VCF table by minimum QUAL and optional `PASS` status |
| `titv_ratio(variants)` | 1 | Transition / transversion ratio |
| `variant_summary(variants)` | 1 | Aggregate SNP, insertion, deletion, and MNV counts |
| `allele_freq(variants, field?)` | 1-2 | Extract an INFO allele-frequency field (default `AF`) |

### Population Genetics

| Builtin | Arity | Description |
|---|---|---|
| `hwe_test(n_aa, n_ab, n_bb)` | 3 | Hardy-Weinberg chi-square test |
| `fst_weir_cockerham(pop1, pop2)` | 2 | Weir-Cockerham Fst from matching `[n_ref, n_total]` rows |
| `tajima_d(site_differences, n_sequences)` | 2 | Tajima's D neutrality test statistic |
| `ld_r2(geno_a, geno_b)` | 2 | Linkage disequilibrium r² between two variants |
| `allele_freq_spectrum(counts, n_sequences?)` | 1-2 | Folded site frequency spectrum |
| `nucleotide_diversity(geno_matrix)` | 1 | Nucleotide diversity (π) across sites |

```biolang
import "variants" as vcf
import "popgen"   as pg

let pass = vcf.load_filtered("results/calls.vcf", 30.0, true)
vcf.summary(pass)
vcf.titv(pass)

pg.hwe(360, 480, 160)
pg.fst([45, 12, 89], [78, 5, 92], 200, 200)
pg.tajima(15, 20, 1000)
```

---

## Bulk RNA-seq

Builtins for loading and normalising bulk RNA-seq quantification output.

| Builtin | Arity | Description |
|---|---|---|
| `parse_salmon(quant_sf)` | 1 | Parse Salmon `quant.sf` text into a gene expression table |
| `parse_featurecounts(counts_txt)` | 1 | Parse featureCounts output into a counts table |
| `size_factors(count_matrix)` | 1 | DESeq2-style median-ratio size factors |
| `filter_low_counts(count_matrix, min_count, min_samples)` | 3 | Remove genes with fewer than `min_count` in `min_samples` samples |
| `tpm_matrix(count_matrix, gene_lengths)` | 2 | Convert raw counts to TPM |
| `sample_correlation(count_matrix)` | 1 | Pairwise Pearson correlation between samples |

```biolang
import "rnaseq"       as rna
import "differential" as de

let counts = rna.from_featurecounts("results/counts.txt")
let counts = rna.filter_genes(counts, 10, 3)
let norm   = rna.normalize_sf(counts)

let conditions = ["ctrl", "ctrl", "ctrl", "treated", "treated", "treated"]
let results = de.de_wald(norm, [0, 1, 2], [3, 4, 5])
results |> filter(|r| r.padj <= 0.05 && abs(r.log2fc) >= 1.0)
```

---

## Phylogenetics

Builtins for Newick tree parsing and distance-based phylogenetics.

| Builtin | Arity | Description |
|---|---|---|
| `nw_parse(newick_str)` | 1 | Parse a Newick string into a tree record |
| `tree_leaves(tree)` | 1 | List all leaf names in the tree |
| `patristic_distance(tree, leaf_a, leaf_b)` | 3 | Sum of branch lengths between two leaves |
| `nw_to_distance_matrix(tree)` | 1 | All-pairs patristic distance matrix |
| `upgma(leaves, dist_matrix)` | 2 | UPGMA tree reconstruction from a distance matrix |

```biolang
import "phylo" as ph

let tree = ph.load("data/sarscov2_sequences.nwk")
ph.n_leaves(tree)
ph.distance(tree, "Alpha", "Delta")

let dmat    = ph.distance_matrix(tree)
let rebuilt = ph.build_upgma(ph.leaves(tree), dmat)
```

---

## ChIP-seq / ATAC-seq

Builtins for peak-level quality control and consensus peak generation.

| Builtin | Arity | Description |
|---|---|---|
| `merge_peaks(peaks)` | 1 | Merge overlapping NarrowPeak records |
| `consensus_peaks(peak_lists, min_samples)` | 2 | Peaks present in at least `min_samples` replicates |
| `frip_score(reads_in_peaks, total_reads)` | 2 | Fraction of Reads In Peaks quality metric |
| `tss_enrichment(peaks, tss_sites)` | 2 | TSS enrichment score for ATAC QC |
| `peak_annotation(peaks, annotations)` | 2 | Annotate peaks with nearest genomic features |

```biolang
import "chipseq" as cs

let ctrl    = cs.load_narrowpeak("results/ctrl_peaks.narrowPeak")
let treated = cs.load_narrowpeak("results/treated_peaks.narrowPeak")

cs.qc_report(ctrl, 320000, 25000000)

let consensus = cs.consensus([cs.merge(ctrl), cs.merge(treated)], 2)
cs.n_peaks(consensus)
```

---

## Microbiome

Builtins for alpha/beta diversity, rarefaction, and taxonomic composition.

| Builtin | Arity | Description |
|---|---|---|
| `alpha_diversity(count_vec, method)` | 2 | Shannon, Simpson, or Chao1 alpha diversity |
| `beta_diversity(matrix_a, matrix_b, method)` | 3 | Bray-Curtis or Jaccard beta diversity |
| `rarefaction(count_vec, depth)` | 2 | Subsample counts to `depth` without replacement |
| `relative_abundance(count_vec)` | 1 | Fractional abundances (sums to 1.0) |
| `taxonomic_collapse(otu_table, taxonomy, level)` | 3 | Aggregate OTU counts at a given taxonomic rank |

```biolang
import "microbiome" as mb

let otus = read_csv("data/feature_table.tsv")
mb.alpha_table(otus, "shannon")

let rarefied = mb.rarefy_matrix(otus, 5000)
mb.beta(rarefied, "bray_curtis")

let taxonomy = read_csv("data/taxonomy.tsv")
mb.top_taxa(taxonomy, 10, "genus")
```

---

## Statistics (Extended)

Higher-level statistical testing beyond the core `ttest`, `chi_square`, and
`p_adjust` builtins. The `statistics` package wraps these with convenient
defaults.

| Builtin | Arity | Description |
|---|---|---|
| `bh_adjust(p_values)` | 1 | Benjamini-Hochberg FDR correction, order preserved |
| `bonferroni_adjust(p_values)` | 1 | Bonferroni correction, clamped to 1.0 |
| `fisher_exact(a, b, c, d)` | 4 | Two-sided Fisher's exact test; returns `{p_value, odds_ratio}` |
| `chi_square(observed, expected)` | 2 | Chi-squared goodness-of-fit; returns `{statistic, df, p_value}` |
| `permutation_test(a, b, n)` | 3 | Permutation test with `n` shuffles; returns p-value |
| `bootstrap_ci(values, n, conf)` | 3 | Percentile bootstrap CI; returns `{mean, lower, upper, std_err}` |
| `genomic_inflation(p_values)` | 1 | Genomic inflation factor λ from GWAS p-values |
| `pearson_correlation(x, y)` | 2 | Pearson r, zero-variance safe |

```biolang
import "statistics" as stat

let p_values = [0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 0.9]
let adj_bh   = stat.bh(p_values)
stat.significant(adj_bh, 0.05)
stat.inflation(p_values)

let group_a = [2.1, 2.3, 1.9, 2.5, 2.2]
let group_b = [3.1, 2.8, 3.4, 2.9, 3.2]
stat.permtest(group_a, group_b, 10000)
```

---

## RT-qPCR

Builtins for ΔCt/ΔΔCt analysis, standard-curve efficiency, and geNorm
reference gene selection.

| Builtin | Arity | Description |
|---|---|---|
| `delta_ct(sample_ct, ref_ct)` | 2 | ΔCt = sample_ct − reference_ct |
| `delta_delta_ct(sample_dct, control_dct)` | 2 | Fold change = 2^(−ΔΔCt) |
| `pcr_efficiency(cts, log_dilutions)` | 2 | Efficiency from standard curve; returns `{efficiency, slope}` |
| `reference_normalize(ct_table, ref_indices)` | 2 | Per-sample normalization using reference gene mean |
| `genorm_stability(ct_table, ref_indices)` | 2 | geNorm M-score stability ranking |

```biolang
import "qpcr" as qpcr

let standard_cts  = [15.2, 18.5, 21.8, 25.1, 28.4]
let log_dilutions = [0.0, -1.0, -2.0, -3.0, -4.0]
qpcr.efficiency(standard_cts, log_dilutions)

let dct         = qpcr.dct(28.5, 22.0)
let fold_change = qpcr.ddct(dct, 6.2)
```

---

## Proteomics

Builtins for MaxQuant LFQ data: loading, normalization, imputation, and
differential abundance.

| Builtin | Arity | Description |
|---|---|---|
| `load_maxquant(text)` | 1 | Parse MaxQuant `proteinGroups.txt` into a matrix |
| `log2_transform(matrix)` | 1 | Log2-transform all intensity values |
| `quantile_normalize(matrix)` | 1 | Quantile normalization across samples |
| `impute_minvalue(matrix, percentile)` | 2 | Impute missing values with a low-percentile constant |
| `protein_ttest(matrix, group_a, group_b)` | 3 | Per-protein two-sample t-test; returns `{log2fc, p_value}` |
| `volcano_data(test_result, fc, p)` | 3 | Classify each protein as up/down/unchanged |

```biolang
import "proteomics" as prot

let mat  = prot.full_pipeline("data/proteinGroups.txt")
let de   = prot.ttest(mat, [0, 1, 2], [3, 4, 5])
let hits = prot.significant(de, 1.0, 0.05)
prot.volcano(de, 1.0, 0.05)
```

---

## DNA Methylation

Builtins for RRBS/EPIC array data: beta/M-value conversion, DMR calling,
CpG density, and epigenetic clocks.

| Builtin | Arity | Description |
|---|---|---|
| `beta_to_mvalue(beta_vec)` | 1 | Convert β values to M-values (logit) |
| `mvalue_to_beta(m_vec)` | 1 | Convert M-values back to β (inverse logit) |
| `dmr_find(beta_matrix, group_a, group_b, min_delta, min_cpgs)` | 5 | Find differentially methylated regions |
| `cpg_density(positions, window)` | 2 | CpG density per window of base pairs |
| `epigenetic_age(beta_vec, coefs)` | 2 | Predicted age from a linear clock model (e.g. Horvath) |
| `differential_methylation(beta_matrix, group_a, group_b)` | 3 | Site-level ΔBeta and p-value for each CpG |

```biolang
import "methylation" as meth

# let m_matrix = beta_matrix |> rows |> map(fn(r) -> meth.to_mvalue(r))
# let dmrs = meth.find_dmrs(beta_matrix, [0,1,2,3,4], [5,6,7,8,9])
# let diff = meth.diff_cpgs(beta_matrix, [0,1,2], [3,4,5])
# meth.significant_cpgs(diff, 0.15, 0.05)

let positions = [100, 120, 145, 300, 320, 340, 360]
meth.density(positions, 200)
```

---

## Protein Structure

Builtins for PDB parsing, structural alignment (Kabsch RMSD), contact maps,
and secondary structure assignment.

| Builtin | Arity | Description |
|---|---|---|
| `pdb_parse(text)` | 1 | Parse PDB ATOM records into a table with chain/residue/coordinate fields |
| `rmsd(coords_a, coords_b)` | 2 | Optimal Kabsch RMSD between two Cα coordinate sets |
| `contact_map(coords, dist)` | 2 | Boolean contact matrix at `dist` Å cutoff |
| `secondary_structure(coords)` | 1 | DSSP-lite assignment: H (helix), E (sheet), C (coil) per residue |
| `backbone_angles(coords)` | 1 | Phi/psi dihedral angles for Ramachandran analysis |

```biolang
import "structure" as st

let pred    = st.load("data/alphafold_prediction.pdb")
let exp     = st.load("data/experimental.pdb")
let pred_ca = st.ca_atoms(pred)
let exp_ca  = st.ca_atoms(exp)

st.rmsd(pred_ca, exp_ca)
st.ss_composition(pred_ca)
st.contacts(pred_ca, 8.0)
```

---

## Biological Networks

Builtins for protein-protein interaction network analysis: centrality,
shortest paths, connected components, and disease module enrichment.

| Builtin | Arity | Description |
|---|---|---|
| `load_ppi(text)` | 1 | Parse STRING TSV edge list into a table of `{source, target, score}` |
| `degree_centrality(edges)` | 1 | Degree (number of neighbours) for every node |
| `betweenness_centrality(edges)` | 1 | Brandes BFS betweenness for every node |
| `shortest_path(edges, src, tgt)` | 3 | BFS shortest path between two nodes |
| `connected_components(edges)` | 1 | Union-Find component labels for every node |
| `network_enrichment(gene_list, edges, bg_size)` | 3 | Hypergeometric enrichment of a gene set in the network |

```biolang
import "network" as net

let ppi = net.load_string("data/9606.protein.links.v12.0.txt")
  |> net.filter_score(0.7)

net.hub_genes(ppi, 20)
let lcc = net.largest_component(ppi)

let brca = ["TP53", "BRCA1", "BRCA2", "ATM", "CHEK2"]
net.path(lcc, "TP53", "BRCA1")
net.enrichment(brca, ppi, 20000)
```

---

## Copy Number Variation (cnv)

Builtins for CNV analysis from WGS/WES read-depth data.

| Builtin | Arity | Description |
|---|---|---|
| `log2_ratios(tumor, normal)` | 2 | log2((tumor+1)/(normal+1)) per bin |
| `cbs_segment(ratios)` | 1 | Circular binary segmentation → Table of segments |
| `cn_call(segments, ploidy)` | 2 | Integer CN from log2-ratio segments |
| `allele_specific_cn(baf, ratio)` | 2 | Major/minor allele CN from B-allele freq + ratio |
| `cnv_summary(segments)` | 1 | Fraction altered, n_segments, mean ratio |

```biolang
import "cnv" as cnv

# let ratios   = cnv.ratios(tumor_depths, normal_depths)
# let segments = cnv.segment(ratios)
# cnv.call(segments, ploidy=2)
# cnv.summary(segments)

# Allele-specific CN from SNP heterozygotes
# let baf = read_csv("tumor_snps.tsv") |> col("baf")
# cnv.allelic(baf, ratios)
```

---

## Hi-C & 3D Genomics (hic)

Builtins for chromatin conformation capture data.

| Builtin | Arity | Description |
|---|---|---|
| `ice_normalize(matrix)` | 1 | Iterative correction (ICE) of contact matrix |
| `insulation_score(matrix, window)` | 2 | Diamond insulation score per bin |
| `tad_boundaries(scores, min_delta)` | 2 | Local minima in insulation score → TAD boundaries |
| `distance_decay(matrix)` | 1 | Mean contact frequency vs genomic distance |
| `expected_contacts(matrix)` | 1 | Distance-decay expected contact matrix |

```biolang
import "hic" as hic

# let norm   = hic.normalize(contact_matrix)
# let scores = hic.insulation(norm, 10)
# let tads   = hic.boundaries(scores, delta=0.15)
# hic.decay(norm)
```

---

## ATAC-seq (atac)

Builtins for ATAC-seq quality control and fragment analysis.

| Builtin | Arity | Description |
|---|---|---|
| `fragment_size_dist(lengths)` | 1 | Histogram of fragment lengths in 10-bp bins |
| `nfr_enrichment(lengths)` | 1 | NFR/mono-nucleosome ratio (>1.5 = good quality) |
| `nucleosome_fractions(lengths)` | 1 | Sub-NFR / NFR / mono / di / tri fractions |
| `tss_enrichment_score(lengths, dists, flank)` | 3 | TSS signal / background ratio |
| `atac_qc(lengths)` | 1 | Combined QC record: NFR fraction, median size, etc. |

```biolang
import "atac" as atac

# let frag_lengths = read_csv("fragments.tsv") |> col("tlen") |> map(fn(x) -> abs(x))
# atac.qc(frag_lengths)
# atac.nfr(frag_lengths)
# atac.sizes(frag_lengths)
# atac.nucleosomes(frag_lengths)
```

---

## Drug Response (drug)

Builtins for dose-response modelling and combination synergy.

| Builtin | Arity | Description |
|---|---|---|
| `fit_ic50(concentrations, viabilities)` | 2 | 4-parameter logistic fit → `{ic50, slope, top, bottom, r2}` |
| `dose_response_curve(concs, ic50, slope, top, bottom)` | 5 | Evaluate 4PL model at given concentrations |
| `auc_response(concentrations, viabilities)` | 2 | Area under dose-response curve (trapezoidal, log10-normalized) |
| `bliss_synergy(viab_a, viab_b, viab_combo)` | 3 | Bliss independence synergy score |
| `loewe_synergy(ic50_a, ic50_b, conc_a, conc_b, obs)` | 5 | Loewe additivity synergy (1 - CI) |
| `drug_rank(ic50_table, ascending)` | 2 | Rank drugs by IC50 |

```biolang
import "drug" as drug

let concs  = [0.001, 0.01, 0.1, 1.0, 10.0, 100.0, 1000.0, 10000.0]
let viabs  = [98.0, 95.0, 88.0, 70.0, 45.0, 20.0, 8.0, 3.0]

let params = drug.fit(concs, viabs)
drug.auc(concs, viabs)
drug.bliss(75.0, 60.0, 30.0)
drug.loewe(1.0, 2.0, 0.5, 1.0)
```

---

## GWAS (gwas)

Builtins for genome-wide association study summary statistics.

| Builtin | Arity | Description |
|---|---|---|
| `parse_sumstats(text)` | 1 | Auto-detect column names (CHR/BP/SNP/P/BETA/SE/A1/A2/MAF) |
| `manhattan_data(sumstats)` | 1 | Cumulative positions + -log10(p) for Manhattan plot |
| `qq_data(pvals)` | 1 | Expected vs observed -log10(p) for QQ plot |
| `clump(sumstats, p_threshold, window_kb)` | 3 | Greedy distance-based LD clumping |
| `top_loci(sumstats, p_threshold)` | 2 | Genome-wide significant hits |
| `lambda_gc(pvals)` | 1 | Genomic inflation factor λ |

```biolang
import "gwas" as gwas

let ss   = gwas.load("data/my_gwas_sumstats.txt")
gwas.lambda(ss.pval)
let hits = gwas.hits(ss)
let loci = gwas.clump(ss, 5e-8, 250)
gwas.manhattan(ss)
gwas.qq(ss.pval)
```

---

## Genomic Annotation (annotation)

Builtins for GTF/GFF3 parsing and genomic interval annotation.

| Builtin | Arity | Description |
|---|---|---|
| `parse_gtf(text)` | 1 | Parse GTF or GFF3 → Table with gene_id, gene_name, chrom, start, end, strand |
| `gene_bodies(gtf)` | 1 | Collapse transcripts to gene-level min/max intervals |
| `promoters(gtf, upstream, downstream)` | 3 | Strand-aware TSS ± window intervals |
| `interval_overlap(query, subject)` | 2 | Chrom-aware interval overlap between two Tables |
| `annotate_peaks(peaks, gtf)` | 2 | Nearest gene + Promoter/Intragenic/Distal classification |
| `gene_id_map(gtf)` | 1 | Ensembl ID → gene name mapping Table |

```biolang
import "annotation" as ann

let gtf   = ann.load("data/gencode.v45.annotation.gtf")
let genes = ann.genes(gtf)
let proms = ann.promoters(gtf, 2000, 200)
let idmap = ann.id_map(gtf)

# let peaks = read_csv("peaks.narrowPeak")
# ann.annotate(peaks, gtf)
# ann.overlap(peaks, atac_peaks)
```
