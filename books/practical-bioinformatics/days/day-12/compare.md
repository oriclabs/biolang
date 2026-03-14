# Day 12: Language Comparison --- Finding Variants in Genomes

## Line Counts

| Operation | BioLang | Python (base) | R (base) |
|-----------|---------|---------------|----------|
| Load VCF | 1 | 15 | 12 |
| Classify variant type | 1 (property) | 5 (function) | 5 (function) |
| Ts/Tv ratio | 1 | 5 | 6 |
| Het/Hom ratio | 1 | 8 | 10 |
| Variant summary | 1 | 8 | 8 |
| Quality filter | 2 | 1 | 1 |
| VEP annotation | 1 | N/A | N/A |
| Export CSV | 2 | 5 | 2 |
| **Total script** | **~65** | **~110** | **~105** |

## Key Differences

### Loading a VCF File

```
# BioLang — built-in VCF reader, returns list of Variant values
let variants = read_vcf("data/variants.vcf")

# Python — manual parsing or cyvcf2
variants = []
with open("data/variants.vcf") as f:
    for line in f:
        if line.startswith("#"): continue
        fields = line.strip().split("\t")
        # ... parse 10 fields manually

# R — manual parsing or VariantAnnotation
lines <- readLines("data/variants.vcf")
data_lines <- lines[!grepl("^#", lines)]
# ... parse tab-separated fields
```

### Variant Type Classification

```
# BioLang — computed property on Variant value
v.variant_type   # "Snp", "Indel", "Mnp", "Other"
v.is_snp         # true/false
v.is_indel       # true/false

# Python — manual function
def variant_type(ref, alt):
    if len(ref) == 1 and len(alt) == 1: return "Snp"
    elif len(ref) != len(alt): return "Indel"
    else: return "Mnp"

# R — manual function
variant_type <- function(ref, alt) {
  if (nchar(ref) == 1 && nchar(alt) == 1) "Snp"
  else if (nchar(ref) != nchar(alt)) "Indel"
  else "Mnp"
}
```

### Transition/Transversion Ratio

```
# BioLang — single function call
tstv_ratio(variants)

# Python — manual classification
TRANSITIONS = {("A","G"), ("G","A"), ("C","T"), ("T","C")}
ts = sum(1 for v in snps if (v["ref"], v["alt"]) in TRANSITIONS)
tv = len(snps) - ts
ratio = ts / tv

# R — manual classification
transitions <- c("AG", "GA", "CT", "TC")
ts_count <- sum(mapply(function(r,a) paste0(r,a) %in% transitions, snps$ref, snps$alt))
tv_count <- nrow(snps) - ts_count
ratio <- ts_count / tv_count
```

### Quality Filtering

```
# BioLang — pipe chain with built-in properties
let passed = variants |> filter(|v| v.filter == "PASS" and v.qual >= 30)

# Python — list comprehension
passed = [v for v in variants if v["filter"] == "PASS" and v["qual"] >= 30]

# R — subsetting
passed <- variants[variants$filter == "PASS" & variants$qual >= 30, ]
```

### VEP Annotation

```
# BioLang — built-in API client
let annotation = ensembl_vep("17:7577120:G:A")

# Python — requires requests library + manual API call
import requests
url = "https://rest.ensembl.org/vep/human/region/17:7577120:7577120/A"
resp = requests.get(url, headers={"Content-Type": "application/json"})

# R — requires httr + manual API call
library(httr)
resp <- GET("https://rest.ensembl.org/vep/human/region/17:7577120:7577120/A",
            add_headers("Content-Type" = "application/json"))
```

## Observations

- BioLang's `read_vcf()` returns structured Variant values with computed properties (`.is_snp`, `.variant_type`, `.is_transition`). Python and R require manual parsing and classification functions.
- BioLang provides `tstv_ratio()`, `het_hom_ratio()`, and `variant_summary()` as single-call builtins. These are 5-10 lines each in Python/R.
- BioLang's `ensembl_vep()` wraps the Ensembl REST API in one call. Python/R require HTTP client libraries and manual JSON parsing.
- The pipe-and-filter pattern (`|> filter(|v| ...)`) is more readable than nested list comprehensions or data frame subsetting for multi-step filtering.
- For production VCF work, Python users typically use cyvcf2 or pysam, and R users use VariantAnnotation — but these add significant dependency overhead.
