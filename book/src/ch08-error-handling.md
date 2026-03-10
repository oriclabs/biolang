# Error Handling

Biological data is messy. FASTA files contain malformed headers, VCF records
have missing INFO fields, and remote databases time out. BioLang provides
layered error-handling mechanisms so you can write code that degrades
gracefully instead of crashing mid-pipeline.

## try/catch Blocks

Wrap any code that might fail in `try`. If an error occurs, control transfers
to the `catch` block, where you can inspect the error and decide how to
proceed.

```biolang
let records = try {
    read_vcf("variants.vcf")
} catch e {
    print("Failed to parse VCF: " + e.message)
    []
}
```

`try/catch` is an expression -- it returns the value of whichever branch
executes:

```biolang
let depth = try {
    let info = parse_info(variant.info_str)
    int(info["DP"])
} catch e {
    0  # default when DP is absent or unparseable
}
```

### Catching Specific Errors

You can pattern-match on error types inside `catch`:

```biolang
try {
    let result = fetch_uniprot(accession)
    process_protein(result)
} catch e {
    match e.kind {
        "network_error" => {
            print("Network unreachable, using cached data")
            load_cache("uniprot", accession)
        }
        "parse_error" => {
            print("Malformed response for " + accession + ", skipping")
            nil
        }
        _ => {
            print("Unexpected error: " + e.message)
            nil
        }
    }
}
```

## Error Propagation

When a function cannot handle an error meaningfully, let it propagate to the
caller.

```biolang
fn load_reference(path) {
    guard file_exists(path) else {
        error("Reference file not found: " + path)
    }
    let records = read_fasta(path)
    guard len(records) > 0 else {
        error("Reference file is empty: " + path)
    }
    records
}

# Caller decides how to handle it
try {
    let ref_seqs = load_reference("hg38.fa")
    run_alignment(reads, ref_seqs)
} catch e {
    print("Pipeline aborted: " + e.message)
}
```

The `error()` function raises an exception that unwinds to the nearest `catch`.

## Retry Loops

Network calls to biological databases are unreliable. Use a loop with
`try/catch` to retry a block up to a specified number of attempts.

```biolang
let gene_info = nil
let attempts = 0
while attempts < 3 {
    try {
        gene_info = ncbi_gene("BRCA1")
        break
    } catch e {
        attempts = attempts + 1
        if attempts >= 3 { error("All attempts failed: " + e.message) }
        sleep(2000)
    }
}
```

If all attempts fail, the last error propagates. Combine with an outer
`try/catch` for a fully defensive pattern:

```biolang
let annotation = try {
    let result = nil
    let attempts = 0
    while attempts < 5 {
        try {
            result = ensembl_gene("ENSG00000139618")
            break
        } catch e {
            attempts = attempts + 1
            if attempts >= 5 { error("All attempts failed: " + e.message) }
            sleep(1000)
        }
    }
    result
} catch e {
    print("Ensembl unavailable after 5 attempts: " + e.message)
    {gene_name: "BRCA2", source: "fallback_cache"}
}
```

### Exponential Backoff

For high-traffic APIs, increase delay between retries using a helper:

```biolang
fn fetch_with_backoff(url, max_attempts: 5) {
    let attempt = 0
    let result = nil

    while attempt < max_attempts {
        try {
            result = http_get(url)
            break
        } catch e {
            attempt = attempt + 1
            if attempt >= max_attempts {
                error("All " + str(max_attempts) + " attempts failed: " + e.message)
            }
            let wait = 1000 * pow(2, attempt - 1)  # 1s, 2s, 4s, 8s, 16s
            print("Attempt " + str(attempt) + " failed, retrying in " + str(wait) + "ms")
            sleep(wait)
        }
    }
    result
}

let blast_result = fetch_with_backoff("https://blast.ncbi.nlm.nih.gov/blast/Blast.cgi?RID=" + rid)
```

## Null Coalescing: ??

The `??` operator returns the left-hand side if it is not `nil`; otherwise it
returns the right-hand side. This is indispensable for fields that may be
absent.

```biolang
let depth = variant.info?.DP ?? 0
let gene_name = annotation?.gene_name ?? "unknown"
let af = variant.info?.AF ?? variant.info?.MAF ?? 0.0
```

`??` chains naturally. The first non-nil value wins:

```biolang
let symbol = record.hugo_symbol ?? record.gene_id ?? record.locus_tag ?? "uncharacterised"
```

## Optional Chaining: ?.

The `?.` operator short-circuits a field access chain when any intermediate
value is `nil`. Instead of crashing, the entire expression evaluates to `nil`.

```biolang
# Without optional chaining -- crashes if info is nil or CLNSIG is absent
let sig = variant.info.CLNSIG

# With optional chaining -- returns nil safely
let sig = variant.info?.CLNSIG

# Deep chains
let protein_change = variant.annotation?.consequences?.protein?.hgvs
```

Combine with `??` for a complete default:

```biolang
let consequence = variant.annotation?.consequences?.most_severe ?? "unknown"
```

## Defensive Patterns for Bio Data

### Missing Fields in Records

Biological file formats are under-specified. A BED file might have 3 columns
or 12. A VCF INFO field might omit half the keys. Build defensive accessors.

```biolang
fn safe_info(variant, key, default: nil) {
    try {
        let info = parse_info(variant.info_str)
        info[key] ?? default
    } catch e {
        default
    }
}

let dp = safe_info(v, "DP", default: 0)
let af = safe_info(v, "AF", default: 0.0)
let clnsig = safe_info(v, "CLNSIG", default: "not_reported")
```

### Malformed Records

Skip bad records instead of aborting the whole file:

```biolang
fn robust_parse(lines, parser) {
    let good = []
    let bad = 0

    for (i, line) in enumerate(lines) {
        try {
            good = good + [parser(line)]
        } catch e {
            bad = bad + 1
            print("Skipped line " + str(i + 1) + ": " + e.message)
        }
    }

    if bad > 0 {
        print("Warning: " + str(bad) + " malformed records skipped")
    }
    good
}
```

### Nil-safe Aggregation

When computing statistics over optional fields, filter out nil values first:

```biolang
let qualities = variants
    |> map(|v| v.qual)
    |> filter(|q| q != nil)

let mean_qual = if len(qualities) > 0 { mean(qualities) } else { 0.0 }
```

## Example: Robust FASTA Parser with try/catch

Parse a FASTA file that may contain corrupted entries mixed with valid ones.

```biolang
fn parse_fasta_robust(path) {
    let raw = read_lines(path)
    let records = []
    let current_header = nil
    let current_seq = ""
    let errors = []

    for line in raw {
        if starts_with(line, ">") {
            # Flush previous record
            if current_header != nil {
                try {
                    guard len(current_seq) > 0 else {
                        error("Empty sequence for " + current_header)
                    }
                    guard is_dna(current_seq) else {
                        error("Invalid characters in " + current_header)
                    }
                    records = records + [{header: current_header, seq: current_seq}]
                } catch e {
                    errors = errors + [{header: current_header, reason: e.message}]
                }
            }
            current_header = slice(line, 1, len(line)) |> trim()
            current_seq = ""
        } else {
            current_seq = current_seq + trim(line)
        }
    }

    # Flush last record
    if current_header != nil {
        try {
            guard len(current_seq) > 0 else { error("Empty sequence") }
            guard is_dna(current_seq) else { error("Invalid characters") }
            records = records + [{header: current_header, seq: current_seq}]
        } catch e {
            errors = errors + [{header: current_header, reason: e.message}]
        }
    }

    print("Parsed " + str(len(records)) + " records, " + str(len(errors)) + " errors")
    for err in errors {
        print("  SKIP: " + err.header + " (" + err.reason + ")")
    }
    records
}

let sequences = parse_fasta_robust("mixed_quality.fasta")
```

## Example: Retrying API Calls to NCBI

Query NCBI's E-utilities for gene summaries, handling the rate limit and
transient failures that are common on public APIs.

```biolang
fn fetch_gene_summaries(gene_ids) {
    let results = []

    for id in gene_ids {
        let summary = try {
            let result = nil
            let attempts = 0
            while attempts < 3 {
                try {
                    let url = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi"
                        + "?db=gene&id=" + str(id) + "&retmode=json"
                    let resp = http_get(url)
                    let data = parse_json(resp.body)
                    guard data?.result != nil else {
                        error("No result field in response")
                    }
                    result = data.result[str(id)]
                    break
                } catch e {
                    attempts = attempts + 1
                    if attempts >= 3 { error("All attempts failed: " + e.message) }
                    sleep(1500)
                }
            }
            result
        } catch e {
            print("Failed to fetch gene " + str(id) + ": " + e.message)
            {gene_id: id, name: "unknown", description: "fetch_failed"}
        }

        results = results + [summary]

        # Respect NCBI rate limit: max 3 requests per second without API key
        sleep(400)
    }
    results
}

let gene_ids = [672, 675, 7157, 4609]  # BRCA1, BRCA2, TP53, MYC
let summaries = fetch_gene_summaries(gene_ids)

for s in summaries {
    print(str(s.gene_id ?? s.uid) + ": " + (s.name ?? "unknown") + " - " + (s.description ?? "N/A"))
}
```

## Example: Processing VCF with Missing INFO Fields

Real VCF files have inconsistent INFO columns. Some records carry `AF`, others
do not. Some have `CLNSIG`, most do not. Use `?.` and `??` to process them
uniformly.

```biolang
let variants = read_vcf("clinvar.vcf")

let annotated = variants |> map(|v| {
    let info = parse_info(v.info_str)

    let af = info?.AF ?? info?.CAF ?? 0.0
    let clinical_sig = info?.CLNSIG ?? "not_reported"
    let gene = split(info?.GENEINFO ?? "", ":") |> first() ?? "intergenic"
    let review_status = info?.CLNREVSTAT ?? "no_assertion"
    let origin = info?.ORIGIN ?? "unknown"

    {
        chrom: v.chrom,
        pos: v.pos,
        ref: v.ref,
        alt: v.alt,
        gene: gene,
        clinical_significance: clinical_sig,
        allele_frequency: af,
        review_status: review_status,
        origin: origin
    }
})

# Filter to pathogenic variants with at least two-star review
let pathogenic = annotated
    |> filter(|v| v.clinical_significance == "Pathogenic"
               || v.clinical_significance == "Likely_pathogenic")
    |> filter(|v| v.review_status != "no_assertion")

print("Pathogenic/likely pathogenic variants: " + str(len(pathogenic)))

for v in pathogenic |> take(10) {
    print(v.gene + " " + v.chrom + ":" + str(v.pos)
        + " " + v.ref + ">" + v.alt
        + " AF=" + str(v.allele_frequency))
}

# Write a clean report
write_csv(pathogenic, "pathogenic_report.csv")
```

## Summary

| Mechanism | Syntax | Purpose |
|---|---|---|
| try/catch | `try { } catch e { }` | Graceful failure handling |
| error() | `error("msg")` | Raise an exception |
| retry loop | `while attempts < n { try { ... break } catch { ... sleep(ms) } }` | Transient-failure recovery |
| ?? | `val ?? default` | nil substitution |
| ?. | `obj?.field` | Safe field access |
| guard | `guard cond else { }` | Precondition assertion |

Layer these mechanisms: `guard` at function entry to reject invalid inputs,
`?.` and `??` for data access, `try/catch` around I/O and parsing, and retry
loops for network calls. Together they produce pipelines that finish with partial
results instead of crashing with none.
