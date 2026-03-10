"""Annotation Pipeline: Variants -> filter -> annotate with gene info -> pathway summary"""
import csv
from collections import defaultdict

# Load variants
variants = []
with open("data/variants.vcf") as f:
    for line in f:
        if line.startswith("#"):
            continue
        fields = line.strip().split("\t")
        qual = float(fields[5]) if fields[5] != "." else 0
        variants.append({"chrom": fields[0], "pos": int(fields[1]), "qual": qual})

# Filter
filtered = [v for v in variants if v["qual"] >= 30]

# Count per chromosome
chrom_counts = defaultdict(int)
for v in filtered:
    chrom_counts[v["chrom"]] += 1

# Load annotations
annotations = []
with open("data/gene_annotations.csv") as f:
    for row in csv.DictReader(f):
        annotations.append(row)

# Pathway summary
pathway_counts = defaultdict(int)
for a in annotations:
    pathway_counts[a["pathway"]] += 1

print("Annotation Pipeline:")
print(f"  Total variants: {len(variants)}")
print(f"  After quality filter: {len(filtered)}")
print(f"  Chromosomes with variants: {len(chrom_counts)}")
print(f"  Annotated genes: {len(annotations)}")
print("  Pathways:")
for pathway, count in sorted(pathway_counts.items(), key=lambda x: -x[1])[:5]:
    print(f"    {pathway}: {count} genes")
