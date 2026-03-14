# Day 28: Language Comparison --- Clinical Variant Report

## Line Counts

| Operation | BioLang | Python | R |
|-----------|---------|--------|---|
| VCF parsing | 1 (built-in) | 15 | 10 |
| TSV loading | 1 (built-in) | 6 | 1 |
| Info field extraction | 9 | 5 | 7 |
| VCF validation | 11 | 8 | 8 |
| Quality filter | 7 | 8 | 8 |
| Frequency filter | 8 | 7 | 7 |
| Panel filter | 5 | 3 | 2 |
| Scoring functions | 20 | 18 | 16 |
| Variant scorer | 22 | 22 | 20 |
| QC stats | 10 | 12 | 11 |
| Report builder | 50 | 60 | 70 |
| Main pipeline | 18 | 22 | 28 |
| Annotation/joins | 4 | 16 | 8 |
| I/O boilerplate | 0 (built-in) | 12 | 6 |
| **Total** | **~165** | **~215** | **~205** |

## Key Differences

### VCF Parsing

```
# BioLang --- one built-in call
let variants = read_vcf("data/patient.vcf")

# Python --- manual parsing (15 lines) or pyvcf3 dependency
variants = []
with open(path) as f:
    for line in f:
        if line.startswith("##"): continue
        if line.startswith("#CHROM"):
            headers = line.lstrip("#").split("\t")
            continue
        fields = line.split("\t")
        row = {h: fields[i] for i, h in enumerate(headers)}
        variants.append(row)

# R --- manual parsing or VariantAnnotation (heavy Bioconductor dep)
lines <- readLines(path)
lines <- lines[!grepl("^##", lines)]
header_line <- lines[grepl("^#CHROM", lines)]
headers <- tolower(strsplit(sub("^#", "", header_line), "\t")[[1]])
```

### Table Joins for Annotation

```
# BioLang --- built-in join_tables()
let with_genes = join_tables(annotated, gene_db, "gene")
let with_clinvar = join_tables(with_genes, clinvar_db, "variant_key")

# Python --- manual dictionary lookup (16 lines)
gene_lookup = {g["gene"]: g for g in gene_db}
clinvar_lookup = {c["variant_key"]: c for c in clinvar_db}
for v in variants:
    gene_info = gene_lookup.get(v["gene"], {})
    for k, val in gene_info.items():
        v[k] = val
    # ... similar for clinvar

# R --- dplyr left_join
annotated <- qc_passed %>%
  left_join(gene_db, by = "gene") %>%
  left_join(clinvar_db, by = "variant_key")
```

### Error Handling in Pipelines

```
# BioLang --- try/catch inline
let af = try { float(extract_info_field(row.info, "AF")) } catch err { 0.0 }

# Python --- try/except
try:
    af = float(extract_info_field(row.get("info", ""), "AF"))
except (ValueError, KeyError):
    af = 0.0

# R --- tryCatch
af <- tryCatch(
  as.numeric(extract_info_field(row$info, "AF")),
  error = function(e) 0.0,
  warning = function(w) 0.0
)
```

### Pipe-Based Filtering

```
# BioLang --- pipe chain
let rare_variants = with_clinvar |> frequency_filter(0.01)
let panel_variants = rare_variants |> panel_filter(cancer_panel)
let classified = panel_variants |> map(|row| score_variant(row))

# Python --- sequential assignment
rare_variants = frequency_filter(annotated, 0.01)
panel_matched = panel_filter(rare_variants, cancer_panel)
classified = [score_variant(v) for v in panel_matched]

# R --- dplyr pipe
classified <- annotated %>%
  frequency_filter(0.01) %>%
  panel_filter(cancer_panel) %>%
  score_variants()
```

## Summary

BioLang's advantages for clinical genomics:

1. **Built-in VCF parsing** eliminates 10--15 lines of boilerplate or heavy dependencies
2. **Built-in table joins** replace manual dictionary lookups or dplyr dependency
3. **Inline try/catch** is more concise than Python's try/except blocks or R's tryCatch
4. **Pipe chains** make the multi-stage filtering pipeline readable as a linear flow
5. **No dependency management** --- all bio format I/O and table operations are built in

R's dplyr pipe syntax is comparable in readability, but requires package installation and has heavier VCF parsing overhead. Python is the most verbose due to manual VCF parsing and dictionary-based joins.
