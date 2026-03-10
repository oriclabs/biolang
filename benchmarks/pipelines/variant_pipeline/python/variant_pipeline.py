"""Variant Pipeline: Read -> Filter -> Classify -> Summarize
Multi-stage pipeline for variant analysis."""
from collections import defaultdict

# Stage 1: Read VCF
variants = []
with open("data/variants.vcf") as f:
    for line in f:
        if line.startswith("#"):
            continue
        fields = line.strip().split("\t")
        chrom, pos, _, ref, alt = fields[0], int(fields[1]), fields[2], fields[3], fields[4]
        qual = float(fields[5]) if fields[5] != "." else 0
        info = dict(kv.split("=") for kv in fields[7].split(";") if "=" in kv)
        dp = int(info.get("DP", 0))
        variants.append({"chrom": chrom, "pos": pos, "ref": ref, "alt": alt, "qual": qual, "dp": dp})

# Stage 2: Filter
filtered = [v for v in variants if v["qual"] >= 30 and v["dp"] >= 10]

# Stage 3: Classify
for v in filtered:
    v["variant_type"] = "SNP" if len(v["ref"]) == 1 and len(v["alt"]) == 1 else "INDEL"

# Stage 4: Summarize by chromosome
by_chrom = defaultdict(list)
for v in filtered:
    by_chrom[v["chrom"]].append(v)

print("Variant Pipeline Results:")
print(f"  Chromosomes analyzed: {len(by_chrom)}")
summaries = []
for chrom, vs in by_chrom.items():
    snps = sum(1 for v in vs if v["variant_type"] == "SNP")
    indels = len(vs) - snps
    mean_qual = sum(v["qual"] for v in vs) / len(vs)
    summaries.append((chrom, len(vs), snps, indels, mean_qual))

for chrom, total, snps, indels, mq in sorted(summaries, key=lambda x: -x[1])[:5]:
    print(f"  {chrom}: {total} variants ({snps} SNPs, {indels} indels, mean QUAL {round(mq, 1)})")
