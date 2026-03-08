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

```
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
| `index_of(list, item) -> Int \| None` | First index of `item`, or None |
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

```
# Example: enumerate quality-filtered reads
let good_reads = reads
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
| `each(coll, fn) -> None` | Execute `fn` for side effects on every element |
| `flat_map(coll, fn) -> List` | Map then flatten one level |
| `take_while(coll, fn) -> List` | Take leading elements while predicate holds |
| `any(coll, fn) -> Bool` | True if `fn` returns true for at least one element |
| `all(coll, fn) -> Bool` | True if `fn` returns true for every element |
| `none(coll, fn) -> Bool` | True if `fn` returns true for no elements |
| `find(coll, fn) -> Any \| None` | First element satisfying `fn` |
| `find_index(coll, fn) -> Int \| None` | Index of first element satisfying `fn` |
| `par_map(coll, fn) -> List` | Parallel map across available cores |
| `par_filter(coll, fn) -> List` | Parallel filter across available cores |
| `mat_map(matrix, fn) -> Matrix` | Apply `fn` element-wise to a matrix |
| `try_call(fn, args) -> Result` | Call `fn` with `args`, capturing errors instead of panicking |

```
# Example: parallel GC content across a genome's chromosomes
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
| `summarize(tbl, ...aggs) -> Table` | Aggregate columns (sum, mean, etc.) |
| `group_by(tbl, col) -> GroupedTable` | Group rows by a column value (table variant) |
| `join(a, b, on, how?) -> Table` | Join two tables on a key column |
| `csv(path) -> Table` | Read a CSV file into a table |
| `tsv(path) -> Table` | Read a TSV file into a table |
| `write_csv(tbl, path) -> None` | Write a table to CSV |
| `write_tsv(tbl, path) -> None` | Write a table to TSV |
| `nrow(tbl) -> Int` | Number of rows |
| `ncol(tbl) -> Int` | Number of columns |
| `colnames(tbl) -> List[Str]` | Column name list |
| `row_names(tbl) -> List[Str]` | Row name list (if set) |

```
# Example: summarize variant counts per chromosome
tsv("variants.tsv")
  |> group_by("chrom")
  |> summarize(count: len, mean_qual: |rows| mean(rows |> map(|r| r.quality)))
  |> write_csv("chrom_summary.csv")
```

---

## Bio File I/O

Read and write standard bioinformatics file formats. Readers return lazy
streams that integrate with pipes; writers flush on completion.

| Builtin | Description |
|---|---|
| `read_fasta(path) -> List[Record]` | Parse FASTA; each record has `id`, `desc`, `seq` fields |
| `read_fastq(path) -> List[Record]` | Parse FASTQ; each record has `id`, `seq`, `quality`, `length` fields |
| `read_vcf(path) -> List[Record]` | Parse VCF; each record has `chrom`, `pos`, `ref`, `alt`, `qual`, `info` fields |
| `read_bed(path) -> List[Record]` | Parse BED; each record has `chrom`, `start`, `end` and optional fields |
| `read_gff(path) -> List[Record]` | Parse GFF/GTF; each record has `seqid`, `type`, `start`, `end`, `attributes` |
| `write_fasta(records, path) -> None` | Write records to FASTA format |
| `write_fastq(records, path) -> None` | Write records to FASTQ format |
| `write_bed(records, path) -> None` | Write records to BED format |

```
# Example: filter FASTQ reads by quality and write survivors
read_fastq("sample_R1.fastq.gz")
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
| `query_overlaps(tree, query) -> List[Interval]` | All intervals overlapping the query |
| `query_nearest(tree, query, k?) -> List[Interval]` | K nearest intervals to the query |
| `coverage(intervals) -> List[Record]` | Per-base or per-region coverage depth |
| `merge_intervals(intervals, dist?) -> List[Interval]` | Merge overlapping or nearby intervals |
| `intersect_intervals(a, b) -> List[Interval]` | Pairwise intersection of two interval sets |
| `subtract_intervals(a, b) -> List[Interval]` | Regions in `a` not covered by `b` |

```
# Example: find promoter-peak overlaps
let promoters = read_bed("examples/sample-data/promoters.bed") |> map(|r| interval(r.chrom, r.start, r.end))
let peaks     = read_bed("examples/sample-data/chip_peaks.bed") |> map(|r| interval(r.chrom, r.start, r.end))
let tree      = interval_tree(peaks)
let hits      = promoters |> flat_map(|p| query_overlaps(tree, p))
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

```
# Example: compute Ti/Tv ratio for a VCF
let vars = read_vcf("examples/sample-data/calls.vcf") |> filter(|v| v.qual > 30)
let summary = variant_summary(vars)
print("Ti/Tv = " + str(summary.ti_tv_ratio) + ", SNPs = " + str(summary.snp_count))
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
| `fisher_exact(table) -> Record` | Fisher's exact test on a 2x2 contingency table |
| `p_adjust(pvals, method?) -> List[Float]` | Multiple testing correction (default: Benjamini-Hochberg) |
| `lm(xs, ys) -> Record` | Simple linear regression; returns `{slope, intercept, r_squared}` |
| `ks_test(xs, ys) -> Record` | Kolmogorov-Smirnov test |
| `mean_phred(quals) -> Float` | Mean Phred quality score from a quality string |

```
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
| `identity(n) -> Matrix` | n x n identity matrix |
| `zeros(rows, cols) -> Matrix` | Matrix of zeros |
| `ones(rows, cols) -> Matrix` | Matrix of ones |
| `diag(values) -> Matrix` | Diagonal matrix from a list of values |
| `mat_map(m, fn) -> Matrix` | Apply `fn` element-wise |

```
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

```
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
| `matches(s, pattern) -> Bool` | True if regex `pattern` matches |
| `format(template, ...args) -> Str` | Printf-style formatting |

BioLang also supports **f-strings** for inline interpolation:

```
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

```
# Example: route processing based on sequence type
let seq = read_fasta("input.fa") |> first() |> |r| r.seq
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
| `ncbi_search(db, query, max?) -> List[Record]` | Search NCBI Entrez databases |
| `ncbi_gene(gene_id) -> Record` | Fetch NCBI Gene record |
| `ncbi_sequence(acc) -> Record` | Fetch sequence by accession from NCBI Nucleotide/Protein |
| `ensembl_gene(ensembl_id) -> Record` | Ensembl gene lookup by Ensembl ID |
| `ensembl_symbol(species, symbol) -> Record` | Ensembl gene lookup by species and symbol |
| `ensembl_vep(variants) -> List[Record]` | Variant Effect Predictor annotation |
| `uniprot_search(query, max?) -> List[Record]` | Search UniProt by keyword or accession |
| `uniprot_entry(acc) -> Record` | Full UniProt entry |
| `ucsc_sequence(genome, chrom, start, end) -> Dna` | Fetch genomic sequence from UCSC DAS |
| `kegg_get(entry) -> Record` | Retrieve a KEGG database entry |
| `kegg_find(db, query) -> List[Record]` | Search KEGG databases |
| `string_network(proteins, species?) -> Record` | STRING protein-protein interaction network |
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

```
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
| `print(val) -> None` | Print a value followed by a newline |
| `assert(cond, msg?) -> None` | Panic with `msg` if `cond` is false |
| `sleep(ms) -> None` | Pause execution for `ms` milliseconds |
| `now() -> Float` | Current timestamp in seconds since epoch |
| `elapsed(start) -> Float` | Seconds elapsed since `start` (from `now()`) |
| `bp(n) -> Int` | Identity; documents that `n` is in base pairs |
| `kb(n) -> Int` | Convert kilobases to base pairs (`n * 1000`) |
| `mb(n) -> Int` | Convert megabases to base pairs (`n * 1_000_000`) |
| `gb(n) -> Int` | Convert gigabases to base pairs (`n * 1_000_000_000`) |
| `to_json(val) -> Str` | Serialize any value to a JSON string |
| `from_json(s) -> Any` | Parse a JSON string into a BioLang value |
| `env(name) -> Str \| None` | Read an environment variable |
| `exit(code?) -> Never` | Terminate the process with an exit code (default: 0) |

```
# Example: time a heavy operation
let t0 = now()
let result = read_fasta("genome.fa")
  |> flat_map(|r| find_orfs(r.seq, 300))
print("Found " + str(len(result)) + " ORFs in " + str(elapsed(t0)) + "s")
```
