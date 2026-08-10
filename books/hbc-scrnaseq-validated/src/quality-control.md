# Counts and quality control

Droplet matrices are mostly zero. `sc.load` retains that sparsity and returns a
single-cell record containing the count matrix, barcodes, and gene names.

```biolang
import "singlecell" as sc

let ctrl_raw = sc.load("ctrl_raw")
let stim_raw = sc.load("stim_raw")
```

For each barcode, the HBC filter combines four signals:

- at least 500 observed molecules;
- at least 250 detected genes;
- `log10(genes) / log10(molecules) > 0.80`;
- less than 20 percent mitochondrial molecules.

The novelty ratio removes low-complexity droplets that can pass either count
threshold by itself. The mitochondrial threshold removes cells whose RNA pool
is unusually dominated by mitochondrial transcripts.

```biolang
fn hbc_filter(obj) {
    let metrics = cell_qc(obj.matrix, obj.genes)
    let genes = col(metrics, "n_genes")
    let umis = col(metrics, "total_counts")
    let mito = col(metrics, "pct_mito")
    let keep = range(0, obj.n_cells) |> filter(|i| {
        umis[i] >= 500.0 and
        genes[i] >= 250.0 and
        log10(genes[i]) / log10(umis[i]) > 0.80 and
        mito[i] < 20.0
    })
    sc_subset_cells(obj, keep)
}
```

After independently filtering both samples, the matrices are merged and genes
detected in fewer than ten cells are discarded.

```biolang
let ctrl = hbc_filter(ctrl_raw)
let stim = hbc_filter(stim_raw)
let filtered = sc.merge(ctrl, stim, "ctrl", "stim")
    |> sc.filter_genes(10)
```

This stage matches the checkpoints in the HBC
[quality-control lesson](https://hbctraining.github.io/Intro-to-scRNAseq-Quarto/lessons/05_quality_control.html)
and its downstream [PCA lesson](https://hbctraining.github.io/Intro-to-scRNAseq/lessons/06_theory_of_PCA.html)
exactly:

| Checkpoint | Control | Stimulated | Combined |
|---|---:|---:|---:|
| retained cells | 14,847 | 14,782 | 29,629 |
| retained genes | — | — | 14,065 |

An exact match is important: downstream comparisons would be ambiguous if the
methods were not operating on the same cells and genes.
