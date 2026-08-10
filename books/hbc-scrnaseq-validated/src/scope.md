# Scope and reproducibility

The workflow expects two unmodified 10x matrices in the repository root:

```text
ctrl_raw/
  barcodes.tsv
  genes.tsv
  matrix.mtx
stim_raw/
  barcodes.tsv
  genes.tsv
  matrix.mtx
```

Run the validation notebook from that root:

```powershell
target/release/bl.exe notebook packages/singlecell/examples/hbc_course_validation.bln
```

BioLang automatically uses a compatible GPU for the large sketched CCA block
products and otherwise falls back to deterministic f64 CPU calculations. Every
script and notebook run now prints the selected policy, adapter, and graphics
backend in its run header. You can also inspect the environment with:

```powershell
target/release/bl.exe doctor
```

To avoid GPU use for a particular command, pass the global flag from either
side of the subcommand:

```powershell
target/release/bl.exe notebook packages/singlecell/examples/hbc_course_validation.bln --no-gpu
```

For notebooks launched by another process, set `BIOLANG_GPU=off`. Automatic
detection is the default; `--gpu` or `BIOLANG_GPU=on` explicitly restores it.
The validation summary records `countsketch_subspace_gpu` or
`countsketch_subspace_cpu`, so an accelerated run is auditable rather than
inferred from hardware presence.

The notebook checks the exact cell and gene counts before it starts expensive
normalization. It writes a feature manifest, cell manifest, marker table, UMAP
SVG, and short run summary. Bulky generated PCs remain ignored. The compact
evidence required to audit the published cluster, feature, and marker metrics
is committed under `src/evidence/2026-08-09/` with SHA-256 hashes for the
remaining generated files.

## Analysis contract

The book distinguishes three kinds of agreement:

1. **Exact checkpoints** compare integer counts and joined cell identities.
2. **Structural comparisons** use feature-set overlap, adjusted Rand index,
   mapped cluster accuracy, and neighborhood Jaccard similarity.
3. **Biological comparisons** ask whether known marker programs are recovered,
   without assuming that cluster number 10 in one run must be cluster number 10
   in another.

The external R environment is not a build or runtime dependency. Its runner
scripts live in a clearly marked validation-only directory, and no GPL package
source, translated implementation, linked library, or serialized model is part
of BioLang. Only independently produced CSV/text evidence is retained.
