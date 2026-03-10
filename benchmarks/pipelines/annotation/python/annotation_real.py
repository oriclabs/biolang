"""Annotation Pipeline (Real Data): ClinVar variants -> filter -> annotate with Ensembl genes"""
import csv
from collections import defaultdict

# Load ClinVar variants
variants = []
with open("data_real/clinvar_diverse.vcf") as f:
    for line in f:
        if line.startswith("#"):
            continue
        fields = line.strip().split("\t")
        if len(fields) < 8:
            continue
        filt = fields[6]
        variants.append({"chrom": fields[0], "pos": int(fields[1]), "filter": filt})

# Filter
filtered = [v for v in variants if v["filter"] in (".", "PASS")]

# Count per chromosome
chrom_counts = defaultdict(int)
for v in filtered:
    chrom_counts[v["chrom"]] += 1

# Load real gene annotations
annotations = []
with open("data_real/gene_annotations.csv") as f:
    for row in csv.DictReader(f):
        annotations.append(row)

# Pathway summary
pathway_counts = defaultdict(int)
for a in annotations:
    pathway_counts[a["pathway"]] += 1

print("Annotation Pipeline (Real Data):")
print(f"  ClinVar variants: {len(variants)}")
print(f"  After filter: {len(filtered)}")
print(f"  Chromosomes with variants: {len(chrom_counts)}")
print(f"  Ensembl genes (chr22): {len(annotations)}")
print("  Pathways:")
for pathway, count in sorted(pathway_counts.items(), key=lambda x: -x[1])[:5]:
    print(f"    {pathway}: {count} genes")
