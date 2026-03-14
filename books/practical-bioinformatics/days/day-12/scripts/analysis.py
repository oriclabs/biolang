# Day 12: Finding Variants in Genomes — Python equivalent
# Uses cyvcf2 for VCF parsing and pandas for tabulation

import csv
import os
from collections import Counter

# ── Step 1: Load and Explore VCF ────────────────────────────────────────

print("=== Step 1: Loading VCF ===")

# Simple VCF parser (avoids heavy dependency for small files)
variants = []
with open("data/variants.vcf") as f:
    for line in f:
        line = line.strip()
        if line.startswith("#") or not line:
            continue
        fields = line.split("\t")
        chrom, pos, vid, ref, alt, qual, filt, info, fmt, sample = fields[:10]
        # Parse genotype from FORMAT:SAMPLE
        gt_idx = fmt.split(":").index("GT") if "GT" in fmt else 0
        gt = sample.split(":")[gt_idx]
        variants.append({
            "chrom": chrom, "pos": int(pos), "id": vid,
            "ref": ref, "alt": alt,
            "qual": float(qual) if qual != "." else 0.0,
            "filter": filt, "info": info, "gt": gt
        })

print(f"Total variants loaded: {len(variants)}")

v = variants[0]
print(f"First variant:")
print(f"  Chrom: {v['chrom']}")
print(f"  Position: {v['pos']}")
print(f"  ID: {v['id']}")
print(f"  Ref: {v['ref']}, Alt: {v['alt']}")
print(f"  Quality: {v['qual']}")
print(f"  Filter: {v['filter']}")
print()

# ── Step 2: Variant Classification ──────────────────────────────────────

print("=== Step 2: Variant Classification ===")


def variant_type(ref_a, alt_a):
    if len(ref_a) == 1 and len(alt_a) == 1:
        return "Snp"
    elif len(ref_a) != len(alt_a):
        return "Indel"
    else:
        return "Mnp"


for v in variants:
    v["type"] = variant_type(v["ref"], v["alt"])

snps = [v for v in variants if v["type"] == "Snp"]
indels = [v for v in variants if v["type"] == "Indel"]
print(f"SNPs: {len(snps)}")
print(f"Indels: {len(indels)}")

print("First 10 variants with types:")
for v in variants[:10]:
    print(f"  {v['chrom']}:{v['pos']} {v['ref']}>{v['alt']} ({v['type']})")
print()

# ── Step 3: Transition/Transversion Ratio ───────────────────────────────

print("=== Step 3: Ts/Tv Ratio ===")

TRANSITIONS = {("A", "G"), ("G", "A"), ("C", "T"), ("T", "C")}


def is_transition(ref_a, alt_a):
    return (ref_a, alt_a) in TRANSITIONS


ts = sum(1 for v in snps if is_transition(v["ref"], v["alt"]))
tv = len(snps) - ts
ratio = ts / tv if tv > 0 else 0.0
print(f"Ts/Tv ratio: {round(ratio, 2)}")
print("Expected ~2.0 for whole genome sequencing")
print(f"Transitions: {ts}")
print(f"Transversions: {tv}")
print()

# ── Step 4: Quality Filtering ───────────────────────────────────────────

print("=== Step 4: Quality Filtering ===")

passed = [v for v in variants if v["filter"] == "PASS"]
print(f"PASS variants: {len(passed)} / {len(variants)}")

high_qual = [v for v in variants if v["filter"] == "PASS" and v["qual"] >= 30]
print(f"PASS + quality >= 30: {len(high_qual)}")

low_qual = [v for v in variants if v["filter"] != "PASS"]
print(f"Filtered out (non-PASS): {len(low_qual)}")
for v in low_qual:
    print(f"  {v['chrom']}:{v['pos']} {v['ref']}>{v['alt']} qual={v['qual']} filter={v['filter']}")
print()

# ── Step 5: Variant Summary ────────────────────────────────────────────

print("=== Step 5: Variant Summary ===")

type_counts = Counter(v["type"] for v in variants)
print(f"Total alleles: {len(variants)}")
print(f"  SNPs: {type_counts.get('Snp', 0)}")
print(f"  Indels: {type_counts.get('Indel', 0)}")
print(f"  MNPs: {type_counts.get('Mnp', 0)}")
print(f"  Transitions: {ts}")
print(f"  Transversions: {tv}")
print(f"  Ts/Tv ratio: {round(ratio, 2)}")
print(f"  Multiallelic: {sum(1 for v in variants if ',' in v['alt'])}")
print()

# ── Step 6: Chromosome Distribution ────────────────────────────────────

print("=== Step 6: Chromosome Distribution ===")

chrom_counts = Counter(v["chrom"] for v in variants)
for chrom in sorted(chrom_counts.keys(), key=lambda c: (len(c), c)):
    print(f"  {chrom}: {chrom_counts[chrom]}")
print()

# ── Step 7: Het/Hom Ratio ──────────────────────────────────────────────

print("=== Step 7: Het/Hom Ratio ===")


def is_het(gt):
    sep = "|" if "|" in gt else "/"
    alleles = [a for a in gt.split(sep) if a != "."]
    return len(alleles) >= 2 and len(set(alleles)) > 1


def is_hom_alt(gt):
    sep = "|" if "|" in gt else "/"
    alleles = [a for a in gt.split(sep) if a != "."]
    return len(alleles) >= 2 and len(set(alleles)) == 1 and alleles[0] != "0"


het_count = sum(1 for v in variants if is_het(v["gt"]))
hom_count = sum(1 for v in variants if is_hom_alt(v["gt"]))
hh_ratio = het_count / hom_count if hom_count > 0 else 0.0
print(f"Het/Hom ratio: {round(hh_ratio, 2)}")
print("Expected ~1.5-2.0 for diploid organisms")
print(f"Heterozygous: {het_count}")
print(f"Homozygous alt: {hom_count}")
print()

# ── Step 8: VEP Annotation (skip in Python — requires requests + Ensembl API) ──

print("=== Step 8: VEP Annotation (requires internet) ===")
print("  Skipping in Python version (use requests + Ensembl REST API)")
print()

# ── Step 9: Complete Filtering Pipeline ─────────────────────────────────

print("=== Step 9: Complete Filtering Pipeline ===")

pipeline_results = [
    {
        "chrom": v["chrom"], "pos": v["pos"], "id": v["id"],
        "ref": v["ref"], "alt": v["alt"], "qual": v["qual"],
        "type": v["type"]
    }
    for v in variants
    if v["filter"] == "PASS" and v["qual"] >= 30
]

print(f"Variants after filtering: {len(pipeline_results)}")
filtered_snps = sum(1 for v in pipeline_results if v["type"] == "Snp")
filtered_indels = sum(1 for v in pipeline_results if v["type"] == "Indel")
print(f"  Filtered SNPs: {filtered_snps}")
print(f"  Filtered Indels: {filtered_indels}")

os.makedirs("results", exist_ok=True)
with open("results/classified_variants.csv", "w", newline="") as f:
    writer = csv.DictWriter(f, fieldnames=["chrom", "pos", "id", "ref", "alt", "qual", "type"])
    writer.writeheader()
    writer.writerows(pipeline_results)
print("Results saved to results/classified_variants.csv")
print()

print("=== Analysis Complete ===")
