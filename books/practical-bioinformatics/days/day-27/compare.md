# Day 27: Language Comparison --- Building Tools and Plugins

## Line Counts

| Operation | BioLang | Python | R |
|-----------|---------|--------|---|
| Sequence validation | 9 | 7 | 8 |
| GC classification | 11 | 10 | 10 |
| Motif finding | 7 | 15 | 14 |
| Batch GC | 11 | 12 | 11 |
| Sequence summary | 13 | 16 | 14 |
| Length stats (QC) | 9 | 10 | 10 |
| GC distribution (QC) | 8 | 8 | 8 |
| Flag outliers (QC) | 16 | 19 | 16 |
| QC summary | 12 | 13 | 13 |
| Format report | 9 | 9 | 9 |
| Main pipeline | 45 | 55 | 65 |
| Import / module setup | 2 | 5 | 0 (source) |
| I/O (FASTA reading) | 0 (built-in) | 14 | 16 |
| **Total** | **~60** | **~195** | **~210** |

## Key Differences

### Module System

```
# BioLang --- import with namespace, one line
import "lib/seq_utils.bl" as seq
let gc = seq.classify_gc("ATCGATCG")

# Python --- standard import, but functions must be in proper package
from lib.seq_utils import classify_gc
gc = classify_gc("ATCGATCG")
# Requires __init__.py in lib/, sys.path manipulation, or package install

# R --- source() loads into global scope, no namespacing
source("lib/seq_utils.R")
gc <- classify_gc("ATCGATCG")
# Name collisions are silent and dangerous
```

### Module Organization

```
# BioLang --- any .bl file is a module, no boilerplate
# Just write functions and import

# Python --- requires package structure
lib/
  __init__.py         # required for package
  seq_utils.py
  qc.py
# Or use sys.path.insert(0, "lib")

# R --- no module system, just source()
# Must manage load order manually
# Packages require DESCRIPTION, NAMESPACE, R/ directory
```

### Error Handling in Libraries

```
# BioLang --- error() throws, try/catch catches
fn validate_dna(seq) {
    if len(invalid) > 0 { error("Invalid DNA") }
    return upper_seq
}
let result = try { validate_dna("ATXCG") } catch err { str(err) }

# Python --- raise/try/except, similar pattern
def validate_dna(seq):
    if invalid:
        raise ValueError("Invalid DNA")
    return upper_seq
try:
    result = validate_dna("ATXCG")
except ValueError as e:
    result = str(e)

# R --- stop/tryCatch, more verbose
validate_dna <- function(seq) {
    if (length(invalid) > 0) stop("Invalid DNA")
    upper_seq
}
result <- tryCatch(
    validate_dna("ATXCG"),
    error = function(e) conditionMessage(e)
)
```

### Plugin System

```
# BioLang --- built-in plugin system with JSON protocol
import "kmer-tools" as kmer
let freq = kmer.kmer_freq({ sequence: "ATCG", k: 3 })
# Automatic: spawns process, sends JSON, parses response

# Python --- subprocess with manual JSON handling
import subprocess, json
request = json.dumps({"op": "kmer_freq", "params": {"sequence": "ATCG", "k": 3}})
result = subprocess.run(["python3", "plugin/main.py"],
                        input=request, capture_output=True, text=True)
output = json.loads(result.stdout)
# 5 lines minimum, no manifest/discovery system

# R --- system2 with manual JSON
library(jsonlite)
request <- toJSON(list(op = "kmer_freq", params = list(sequence = "ATCG", k = 3)))
result <- system2("python3", "plugin/main.py", input = request, stdout = TRUE)
output <- fromJSON(result)
# 4 lines minimum, requires jsonlite, no discovery system
```

## Why BioLang Is More Concise

1. **Built-in module system.** `import "path" as name` with zero boilerplate. No `__init__.py`, no `sys.path` hacks, no `source()` load-order issues.

2. **Namespace imports.** `import ... as name` avoids collisions cleanly. R has no equivalent; Python requires explicit `from ... import ...` or alias syntax.

3. **Built-in FASTA/FASTQ I/O.** `read_fasta()` is a single function call. Python and R require 14--16 lines to parse FASTA manually (or an external library).

4. **Plugin system with discovery.** `plugin.json` manifest + `bl add/remove/plugins` CLI. Python and R require manual subprocess management and custom discovery logic.

5. **Pipe-first composition.** `sequences |> map(...) |> filter(...) |> to_table()` reads left to right. Python and R require nested function calls or intermediate variables.
