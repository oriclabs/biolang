"""RNA-seq Mini Pipeline: counts -> normalize -> fold change -> DE genes"""
import csv
import math

# Load conditions
conditions = {}
with open("data/conditions.csv") as f:
    for row in csv.DictReader(f):
        conditions[row["sample"]] = row["condition"]

# Load counts
with open("data/gene_counts.csv") as f:
    reader = csv.DictReader(f)
    sample_cols = [c for c in reader.fieldnames if c != "gene"]
    counts = list(reader)

control_samples = [s for s in sample_cols if conditions.get(s) == "control"]
treatment_samples = [s for s in sample_cols if conditions.get(s) == "treatment"]

# Compute fold changes
results = []
for row in counts:
    ctrl_vals = [float(row[s]) for s in control_samples]
    treat_vals = [float(row[s]) for s in treatment_samples]
    ctrl_mean = sum(ctrl_vals) / len(ctrl_vals) if ctrl_vals else 0
    treat_mean = sum(treat_vals) / len(treat_vals) if treat_vals else 0
    fc = treat_mean / ctrl_mean if ctrl_mean > 0 else 0
    log2fc = math.log2(fc) if fc > 0 else 0
    results.append({
        "gene": row["gene"],
        "ctrl_mean": round(ctrl_mean, 1),
        "treat_mean": round(treat_mean, 1),
        "fold_change": round(fc, 2),
        "log2fc": round(log2fc, 2),
    })

de_genes = [r for r in results if abs(r["log2fc"]) >= 0.585]
up = [r for r in de_genes if r["log2fc"] > 0]
down = [r for r in de_genes if r["log2fc"] < 0]

print("RNA-seq Mini Pipeline:")
print(f"  Genes: {len(counts)}")
print(f"  Samples: {len(sample_cols)}")
print(f"  DE genes (|log2FC| >= 0.585): {len(de_genes)}")
print(f"  Upregulated: {len(up)}")
print(f"  Downregulated: {len(down)}")
