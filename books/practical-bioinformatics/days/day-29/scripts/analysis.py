"""RNA-seq differential expression analysis pipeline."""

import csv
import math
import os
from pathlib import Path
from scipy import stats


def read_tsv(path):
    rows = []
    with open(path) as f:
        reader = csv.DictReader(f, delimiter="\t")
        for row in reader:
            rows.append(row)
    return rows


def write_tsv(rows, path):
    if not rows:
        return
    with open(path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=rows[0].keys(), delimiter="\t")
        writer.writeheader()
        writer.writerows(rows)


def main():
    counts = read_tsv("data/counts.tsv")
    samples = read_tsv("data/samples.tsv")
    gene_info = read_tsv("data/gene_info.tsv")

    sample_ids = [s["sample_id"] for s in samples]
    ctrl_ids = [s["sample_id"] for s in samples if s["condition"] == "control"]
    treat_ids = [s["sample_id"] for s in samples if s["condition"] == "treated"]

    min_total = 10
    filtered = []
    for row in counts:
        total = sum(int(row[sid]) for sid in sample_ids)
        if total >= min_total:
            filtered.append(row)

    lib_sizes = {}
    for sid in sample_ids:
        lib_sizes[sid] = sum(int(row[sid]) for row in counts)

    cpm = []
    for row in filtered:
        cpm_row = {"gene": row["gene"]}
        for sid in sample_ids:
            raw = int(row[sid])
            cpm_row[sid] = round(raw / lib_sizes[sid] * 1_000_000, 2)
        cpm.append(cpm_row)

    de_results = []
    for row in cpm:
        ctrl_vals = [row[sid] for sid in ctrl_ids]
        treat_vals = [row[sid] for sid in treat_ids]

        ctrl_mean = sum(ctrl_vals) / len(ctrl_vals)
        treat_mean = sum(treat_vals) / len(treat_vals)

        pseudocount = 0.01
        log2fc = math.log2((treat_mean + pseudocount) / (ctrl_mean + pseudocount))

        try:
            _, pval = stats.ttest_ind(ctrl_vals, treat_vals)
        except Exception:
            pval = 1.0

        if math.isnan(pval):
            pval = 1.0

        de_results.append({
            "gene": row["gene"],
            "ctrl_mean": round(ctrl_mean, 2),
            "treat_mean": round(treat_mean, 2),
            "log2fc": round(log2fc, 4),
            "pvalue": pval,
            "direction": "up" if log2fc > 0 else "down",
        })

    de_results.sort(key=lambda x: x["pvalue"])
    m = len(de_results)

    padj_raw = []
    for i in range(m):
        rank = i + 1
        adj = de_results[i]["pvalue"] * m / rank
        padj_raw.append(min(adj, 1.0))

    monotonic = [1.0] * m
    running_min = 1.0
    for i in range(m - 1, -1, -1):
        if padj_raw[i] < running_min:
            running_min = padj_raw[i]
        monotonic[i] = running_min

    corrected = []
    for i in range(m):
        row = de_results[i].copy()
        row["padj"] = round(monotonic[i], 6)
        corrected.append(row)

    fc_threshold = 1.0
    fdr_threshold = 0.05

    significant = [
        r for r in corrected
        if abs(r["log2fc"]) > fc_threshold and r["padj"] < fdr_threshold
    ]

    up_genes = [r for r in significant if r["direction"] == "up"]
    down_genes = [r for r in significant if r["direction"] == "down"]

    # Volcano data
    volcano_data = []
    for row in corrected:
        if row["padj"] > 0:
            neg_log10_p = -math.log10(row["padj"])
        else:
            neg_log10_p = 10.0
        volcano_data.append({
            "gene": row["gene"],
            "log2fc": row["log2fc"],
            "neg_log10_padj": round(neg_log10_p, 4),
        })

    os.makedirs("data/output", exist_ok=True)

    write_tsv(significant, "data/output/de_genes.tsv")

    fc_values = [abs(g["log2fc"]) for g in significant]
    summary_lines = [
        "=== RNA-seq Differential Expression Summary ===",
        "",
        f"Total genes in count matrix: {len(counts)}",
        f"Genes after low-count filter: {len(filtered)}",
        f"Significant DE genes (|log2FC| > {fc_threshold}, FDR < {fdr_threshold}): {len(significant)}",
        f"  Up-regulated: {len(up_genes)}",
        f"  Down-regulated: {len(down_genes)}",
        "",
        f"Mean |log2FC| of DE genes: {round(sum(fc_values)/len(fc_values), 2)}",
        f"Median |log2FC| of DE genes: {round(sorted(fc_values)[len(fc_values)//2], 2)}",
        f"Max |log2FC|: {round(max(fc_values), 2)}",
        "",
        "Output files:",
        "  data/output/de_genes.tsv       - Significant DE gene table",
        "  data/output/volcano.svg        - Volcano plot (not generated in Python version)",
        "  data/output/summary.txt        - This summary",
    ]

    with open("data/output/summary.txt", "w") as f:
        f.write("\n".join(summary_lines))

    for line in summary_lines:
        print(line)


if __name__ == "__main__":
    main()
