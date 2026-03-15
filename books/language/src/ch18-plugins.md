# Chapter 15: Plugins

BioLang ships with a rich set of built-in functions, but bioinformatics is vast.
You may need to wrap a niche aligner, call an R statistical package, or connect
to a lab-specific LIMS API. The plugin system lets you extend BioLang with
external tools written in Python, R, TypeScript/Deno, or native binaries,
all communicating over a simple JSON protocol on stdin/stdout.

## Plugin Architecture

A BioLang plugin is a standalone program that:

1. Reads JSON requests from stdin
2. Processes the request
3. Writes JSON responses to stdout

BioLang manages the plugin lifecycle: it starts the subprocess when the plugin
is first called, keeps it alive for subsequent calls, and shuts it down when
the script ends. This avoids process startup overhead on repeated invocations.

## The Plugin Manifest

Every plugin has a `plugin.json` file that describes its interface.

```json
{
  "name": "blast-wrapper",
  "version": "1.0.0",
  "description": "BLAST+ sequence search wrapper",
  "kind": "python",
  "entry": "blast_plugin.py",
  "functions": [
    {
      "name": "blastn",
      "description": "Nucleotide BLAST search",
      "params": {
        "query": {"type": "string", "description": "Path to query FASTA"},
        "db": {"type": "string", "description": "BLAST database name or path"},
        "evalue": {"type": "number", "default": 1e-5, "description": "E-value threshold"},
        "max_hits": {"type": "integer", "default": 10, "description": "Maximum hits to return"},
        "threads": {"type": "integer", "default": 4, "description": "CPU threads"}
      },
      "returns": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "subject": "string",
            "identity": "number",
            "evalue": "number",
            "bitscore": "number",
            "alignment_length": "integer"
          }
        }
      }
    },
    {
      "name": "blastp",
      "description": "Protein BLAST search",
      "params": {
        "query": {"type": "string", "description": "Path to query FASTA"},
        "db": {"type": "string", "description": "BLAST database name or path"},
        "evalue": {"type": "number", "default": 1e-5},
        "max_hits": {"type": "integer", "default": 10}
      },
      "returns": {
        "type": "array"
      }
    },
    {
      "name": "makeblastdb",
      "description": "Create a BLAST database from a FASTA file",
      "params": {
        "input": {"type": "string", "description": "Input FASTA path"},
        "dbtype": {"type": "string", "enum": ["nucl", "prot"], "description": "Sequence type"},
        "out": {"type": "string", "description": "Output database prefix"}
      },
      "returns": {
        "type": "object"
      }
    }
  ]
}
```

The `kind` field tells BioLang how to launch the plugin:

| Kind | Interpreter |
|---|---|
| `python` | `python3` (or `python` on Windows) |
| `r` | `Rscript` |
| `deno` | `deno run` |
| `typescript` | `deno run` (same as deno) |
| `native` | Direct executable |

## Communication Protocol

BioLang sends JSON messages to the plugin's stdin and reads JSON responses from
stdout. Each message is a single line of JSON terminated by a newline.

### Request Format

```json
{"id": "req_001", "function": "blastn", "params": {"query": "input.fa", "db": "nt", "evalue": 1e-10}}
```

### Response Format

Success:
```json
{"id": "req_001", "result": [{"subject": "NM_000546.6", "identity": 99.5, "evalue": 0.0, "bitscore": 1842}]}
```

Error:
```json
{"id": "req_001", "error": {"code": "BLAST_FAILED", "message": "Database nt not found"}}
```

### Lifecycle Messages

BioLang sends a shutdown message when the script ends:
```json
{"id": "shutdown", "function": "__shutdown__", "params": {}}
```

The plugin should clean up resources and exit.

## Installing Plugins

### From a Directory

```bash
bl add ./my-plugins/blast-wrapper
```

This copies (or symlinks) the plugin into `~/.biolang/plugins/blast-wrapper/`.

### From a Git Repository

```bash
bl add https://github.com/lab/biolang-deseq2.git
```

BioLang clones the repository into the plugins directory.

### Removing Plugins

```bash
bl remove blast-wrapper
```

### Listing Installed Plugins

From the command line:
```bash
bl plugins
```

From the REPL:
```bash
:plugins
```

Both show the plugin name, version, kind, and available functions.

## Using Plugins in Scripts

Once installed, import a plugin by name and call its functions.

```biolang
import "blast-wrapper"

# Build a custom database
makeblastdb(input: "my_sequences.fa", dbtype: "nucl", out: "custom_db")

# Search against it
let hits = blastn(query: "query.fa", db: "custom_db",
                  evalue: 1e-20, max_hits: 5)

hits |> each(|h| print(h.subject + " identity=" + str(h.identity)
                       + "% evalue=" + str(h.evalue)))
```

Plugin functions integrate seamlessly with pipes and other BioLang features.

```biolang
import "blast-wrapper"

read_fasta("data/sequences.fasta")
  |> filter(|r| len(r.seq) > kb(1))
  |> write_fasta("long_contigs.fa")

blastn(query: "long_contigs.fa", db: "nt", evalue: 1e-30, threads: 8)
  |> filter(|h| h.identity > 95.0)
  |> sort_by(|h| -h.bitscore)
  |> write_tsv("blast_hits.tsv")
```

## Example: Python BLAST Wrapper Plugin

Here is the complete Python implementation for the `blast-wrapper` plugin
described above.

### Directory Structure

```
blast-wrapper/
  plugin.json        # manifest (shown earlier)
  blast_plugin.py    # implementation
```

### blast_plugin.py

```python
import json
import subprocess
import sys
import csv
from io import StringIO


def blastn(params):
    cmd = [
        "blastn",
        "-query", params["query"],
        "-db", params["db"],
        "-evalue", str(params.get("evalue", 1e-5)),
        "-max_target_seqs", str(params.get("max_hits", 10)),
        "-num_threads", str(params.get("threads", 4)),
        "-outfmt", "6 sseqid pident evalue bitscore length",
    ]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        return {"error": {"code": "BLAST_FAILED", "message": result.stderr.strip()}}

    hits = []
    reader = csv.reader(StringIO(result.stdout), delimiter="\t")
    for row in reader:
        hits.append({
            "subject": row[0],
            "identity": float(row[1]),
            "evalue": float(row[2]),
            "bitscore": float(row[3]),
            "alignment_length": int(row[4]),
        })
    return {"result": hits}


def blastp(params):
    cmd = [
        "blastp",
        "-query", params["query"],
        "-db", params["db"],
        "-evalue", str(params.get("evalue", 1e-5)),
        "-max_target_seqs", str(params.get("max_hits", 10)),
        "-outfmt", "6 sseqid pident evalue bitscore length",
    ]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        return {"error": {"code": "BLAST_FAILED", "message": result.stderr.strip()}}

    hits = []
    reader = csv.reader(StringIO(result.stdout), delimiter="\t")
    for row in reader:
        hits.append({
            "subject": row[0],
            "identity": float(row[1]),
            "evalue": float(row[2]),
            "bitscore": float(row[3]),
            "alignment_length": int(row[4]),
        })
    return {"result": hits}


def makeblastdb(params):
    cmd = [
        "makeblastdb",
        "-in", params["input"],
        "-dbtype", params["dbtype"],
        "-out", params["out"],
    ]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        return {"error": {"code": "MAKEBLASTDB_FAILED", "message": result.stderr.strip()}}
    return {"result": {"database": params["out"], "status": "created"}}


DISPATCH = {
    "blastn": blastn,
    "blastp": blastp,
    "makeblastdb": makeblastdb,
}


def main():
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        request = json.loads(line)

        func_name = request["function"]
        if func_name == "__shutdown__":
            break

        handler = DISPATCH.get(func_name)
        if handler is None:
            response = {"id": request["id"],
                        "error": {"code": "UNKNOWN_FUNCTION",
                                  "message": f"No function: {func_name}"}}
        else:
            resp = handler(request["params"])
            response = {"id": request["id"], **resp}

        sys.stdout.write(json.dumps(response) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
```

Install and use:

```biolang
bl add ./blast-wrapper

# In a BioLang script:
import "blast-wrapper"

let hits = blastn(query: "primers.fa", db: "refseq_genomic",
                  evalue: 1e-5, max_hits: 20)
hits |> filter(|h| h.identity == 100.0)
    |> each(|h| print("Perfect match: " + h.subject))
```

## Example: R DESeq2 Plugin for Differential Expression

### Directory Structure

```
deseq2-plugin/
  plugin.json
  deseq2_plugin.R
```

### plugin.json

```json
{
  "name": "deseq2",
  "version": "1.0.0",
  "description": "DESeq2 differential expression analysis",
  "kind": "r",
  "entry": "deseq2_plugin.R",
  "functions": [
    {
      "name": "deseq2_de",
      "description": "Run DESeq2 differential expression on a count matrix",
      "params": {
        "counts_file": {"type": "string", "description": "Path to raw counts TSV (genes x samples)"},
        "metadata_file": {"type": "string", "description": "Path to sample metadata TSV"},
        "design": {"type": "string", "default": "~ condition", "description": "DESeq2 design formula"},
        "contrast": {"type": "array", "description": "Contrast vector, e.g. ['condition', 'treated', 'control']"},
        "alpha": {"type": "number", "default": 0.05, "description": "Adjusted p-value threshold"},
        "lfc_threshold": {"type": "number", "default": 0.0, "description": "Log2 fold change threshold for lfcShrink"}
      },
      "returns": {
        "type": "object",
        "properties": {
          "results_file": "string",
          "n_significant": "integer",
          "n_up": "integer",
          "n_down": "integer"
        }
      }
    },
    {
      "name": "deseq2_normalize",
      "description": "Return DESeq2-normalized counts",
      "params": {
        "counts_file": {"type": "string"},
        "metadata_file": {"type": "string"},
        "design": {"type": "string", "default": "~ condition"}
      },
      "returns": {
        "type": "object",
        "properties": {
          "normalized_file": "string"
        }
      }
    }
  ]
}
```

### deseq2_plugin.R

```r
library(jsonlite)
library(DESeq2)

deseq2_de <- function(params) {
  counts <- read.delim(params$counts_file, row.names = 1)
  metadata <- read.delim(params$metadata_file, row.names = 1)

  # Ensure sample order matches
  metadata <- metadata[columns(counts), , drop = FALSE]

  dds <- DESeqDataSetFromMatrix(
    countData = counts,
    colData = metadata,
    design = as.formula(params$design)
  )

  dds <- DESeq(dds)

  contrast <- params$contrast
  res <- results(dds, contrast = contrast, alpha = params$alpha)

  if (params$lfc_threshold > 0) {
    res <- lfcShrink(dds, contrast = contrast, res = res, type = "ashr")
  }

  res_df <- as.data.frame(res)
  res_df$gene <- rownames(res_df)
  out_file <- sub("\\.tsv$", "_deseq2_results.tsv", params$counts_file)
  write.table(res_df, out_file, sep = "\t", quote = FALSE, row.names = FALSE)

  sig <- res_df[!is.na(res_df$padj) & res_df$padj < params$alpha, ]

  list(
    result = list(
      results_file = out_file,
      n_significant = len(sig),
      n_up = sum(sig$log2FoldChange > 0),
      n_down = sum(sig$log2FoldChange < 0)
    )
  )
}

deseq2_normalize <- function(params) {
  counts <- read.delim(params$counts_file, row.names = 1)
  metadata <- read.delim(params$metadata_file, row.names = 1)
  metadata <- metadata[columns(counts), , drop = FALSE]

  dds <- DESeqDataSetFromMatrix(
    countData = counts,
    colData = metadata,
    design = as.formula(params$design)
  )
  dds <- estimateSizeFactors(dds)
  norm_counts <- counts(dds, normalized = TRUE)

  out_file <- sub("\\.tsv$", "_normalized.tsv", params$counts_file)
  write.table(as.data.frame(norm_counts), out_file, sep = "\t", quote = FALSE)

  list(result = list(normalized_file = out_file))
}

dispatch <- list(
  deseq2_de = deseq2_de,
  deseq2_normalize = deseq2_normalize
)

# Main loop: read JSON from stdin, dispatch, write JSON to stdout
con <- file("stdin", "r")
while (TRUE) {
  line <- readLines(con, n = 1)
  if (length(line) == 0) break

  request <- fromJSON(line)
  if (request$`function` == "__shutdown__") break

  handler <- dispatch[[request$`function`]]
  if (is.null(handler)) {
    response <- list(
      id = request$id,
      error = list(code = "UNKNOWN_FUNCTION",
                   message = paste("No function:", request$`function`))
    )
  } else {
    resp <- tryCatch(
      handler(request$params),
      error = function(e) list(error = list(code = "R_ERROR", message = e$message))
    )
    response <- c(list(id = request$id), resp)
  }

  cat(toJSON(response, auto_unbox = TRUE), "\n")
  flush(stdout())
}
close(con)
```

### Using the DESeq2 Plugin

```biolang
# requires: internet connection (for ncbi_gene calls below)
# requires: NCBI_API_KEY (optional, increases rate limit)
bl add ./deseq2-plugin

# In a BioLang script:
import "deseq2"

# Run differential expression
let de = deseq2_de(
  counts_file: "raw_counts.tsv",
  metadata_file: "sample_metadata.tsv",
  design: "~ condition",
  contrast: ["condition", "treated", "control"],
  alpha: 0.05,
  lfc_threshold: 1.0
)

print(str(de.n_significant) + " DE genes ("
      + str(de.n_up) + " up, " + str(de.n_down) + " down)")

# Read the results back into BioLang for further analysis
let results = tsv(de.results_file)
  |> filter(|r| r.padj != nil and r.padj < 0.01)
  |> sort_by(|r| r.padj)

# Cross-reference top hits with bio APIs
results |> take(10) |> each(|r| {
  let gene_info = ncbi_gene(r.gene)
  print(r.gene + " log2FC=" + str(r.log2FoldChange)
        + " padj=" + str(r.padj)
        + " — " + gene_info.name)
})

# Get normalized counts for downstream analysis
let norm = deseq2_normalize(
  counts_file: "raw_counts.tsv",
  metadata_file: "sample_metadata.tsv"
)
print("Normalized counts: " + norm.normalized_file)
```

## Writing a TypeScript/Deno Plugin

For plugins that connect to REST APIs or need async I/O, Deno is a good choice.

### lims-connector/plugin.json

```json
{
  "name": "lims-connector",
  "version": "1.0.0",
  "description": "Connect to lab LIMS for sample metadata",
  "kind": "deno",
  "entry": "lims_plugin.ts",
  "functions": [
    {
      "name": "lims_samples",
      "description": "Fetch sample metadata from LIMS",
      "params": {
        "project": {"type": "string", "description": "LIMS project ID"},
        "status": {"type": "string", "default": "all", "description": "Filter by status"}
      },
      "returns": {"type": "array"}
    },
    {
      "name": "lims_submit_results",
      "description": "Push analysis results back to LIMS",
      "params": {
        "sample_id": {"type": "string"},
        "results_file": {"type": "string"},
        "qc_status": {"type": "string", "enum": ["pass", "fail", "warning"]}
      },
      "returns": {"type": "object"}
    }
  ]
}
```

### lims_plugin.ts

```typescript
import { readLines } from "https://deno.land/std/io/mod.ts";

const LIMS_URL = Deno.env.get("LIMS_API_URL") || "https://lims.example.com/api";
const LIMS_TOKEN = Deno.env.get("LIMS_API_TOKEN") || "";

interface Request {
  id: string;
  function: string;
  params: Record<string, unknown>;
}

async function limsSamples(params: Record<string, unknown>) {
  const project = params.project as string;
  const status = (params.status as string) || "all";

  const url = `${LIMS_URL}/projects/${project}/samples?status=${status}`;
  const resp = await fetch(url, {
    headers: { "Authorization": `Bearer ${LIMS_TOKEN}` },
  });

  if (!resp.ok) {
    return { error: { code: "LIMS_ERROR", message: await resp.text() } };
  }

  const data = await resp.json();
  return { result: data.samples };
}

async function limsSubmitResults(params: Record<string, unknown>) {
  const sampleId = params.sample_id as string;
  const resultsFile = params.results_file as string;
  const qcStatus = params.qc_status as string;

  const results = await Deno.readTextFile(resultsFile);

  const resp = await fetch(`${LIMS_URL}/samples/${sampleId}/results`, {
    method: "POST",
    headers: {
      "Authorization": `Bearer ${LIMS_TOKEN}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ results: JSON.parse(results), qc_status: qcStatus }),
  });

  if (!resp.ok) {
    return { error: { code: "LIMS_ERROR", message: await resp.text() } };
  }

  return { result: { sample_id: sampleId, status: "submitted" } };
}

const dispatch: Record<string, (p: Record<string, unknown>) => Promise<unknown>> = {
  lims_samples: limsSamples,
  lims_submit_results: limsSubmitResults,
};

for await (const line of readLines(Deno.stdin)) {
  if (!line.trim()) continue;

  const request: Request = JSON.parse(line);
  if (request.function === "__shutdown__") break;

  const handler = dispatch[request.function];
  let response;
  if (!handler) {
    response = {
      id: request.id,
      error: { code: "UNKNOWN_FUNCTION", message: `No function: ${request.function}` },
    };
  } else {
    const resp = await handler(request.params);
    response = { id: request.id, ...resp };
  }

  console.log(JSON.stringify(response));
}
```

### Using the LIMS Plugin

```biolang
import "lims-connector"

# Fetch samples from the LIMS
let samples = lims_samples(project: "WGS-2024-042", status: "sequenced")

# Process each sample
let results = samples |> par_map(|sample| {
  let bam = tool("bwa-mem2", "-t 8 -x sr " + "GRCh38.fa " + sample.fastq_r1 + " " + sample.fastq_r2)
    |> tool("samtools", "sort -@ 4")
  let stats = tool("samtools", "flagstat " + bam)
  let depth = tool("samtools", "depth -a " + bam) |> mean()

  let qc = if stats.mapped_pct > 90 and depth > 30 then "pass"
           else if stats.mapped_pct > 80 then "warning"
           else "fail"

  let result = {sample_id: sample.id, mapped_pct: stats.mapped_pct,
                mean_depth: depth, bam: bam}
  result |> write_json(sample.id + "_results.json")

  # Push results back to LIMS
  lims_submit_results(sample_id: sample.id,
                      results_file: sample.id + "_results.json",
                      qc_status: qc)
  result
})
```

## Plugin Discovery

BioLang searches for plugins in this order:

1. `./plugins/` in the current working directory
2. `~/.biolang/plugins/` in the user home directory
3. Paths listed in the `BIOLANG_PLUGIN_PATH` environment variable

Each directory is scanned for subdirectories containing a `plugin.json` file.

## Plugin Best Practices

**Keep plugins focused.** A plugin should wrap one tool or one API. If you have
BLAST and HMMER, make them separate plugins.

**Use stderr for logging.** BioLang captures stdout for the JSON protocol.
Diagnostic messages, progress indicators, and debug output should go to stderr.

**Handle errors gracefully.** Return error responses instead of crashing. The
`error` response format lets BioLang surface meaningful messages to the user.

**Document parameters.** The `description` fields in `plugin.json` are shown
by `:plugins` in the REPL and `bl plugins` on the command line.

**Pin versions.** If your plugin depends on an external tool, document the
required version in the manifest description so users know what to install.

## Summary

The plugin system extends BioLang with any language that can read and write
JSON on stdio. Write a `plugin.json` manifest, implement the stdin/stdout
dispatch loop, and install with `bl add`. Plugins integrate fully with pipes,
parallel operations, and the rest of BioLang. Python plugins wrap command-line
bioinformatics tools, R plugins bring statistical packages like DESeq2, and
Deno/TypeScript plugins connect to REST APIs and external services.
