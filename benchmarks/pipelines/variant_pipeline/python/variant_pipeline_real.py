"""Variant Pipeline (Real Data): ClinVar -> Filter -> Classify -> Summarize
Uses real ClinVar 20K variants (GRCh38)."""
from collections import defaultdict

# Stage 1: Read VCF
variants = []
with open("data_real/clinvar_diverse.vcf") as f:
    for line in f:
        if line.startswith("#"):
            continue
        fields = line.strip().split("\t")
        if len(fields) < 8:
            continue
        chrom, pos, _, ref, alt = fields[0], int(fields[1]), fields[2], fields[3], fields[4]
        filt = fields[6]
        info = dict(kv.split("=", 1) for kv in fields[7].split(";") if "=" in kv)
        clnsig = info.get("CLNSIG", "")
        variants.append({"chrom": chrom, "pos": pos, "ref": ref, "alt": alt,
                         "filter": filt, "clnsig": clnsig})

# Stage 2: Filter (ClinVar uses "." for PASS)
filtered = [v for v in variants if v["filter"] in (".", "PASS")]

# Stage 3: Classify
for v in filtered:
    v["variant_type"] = "SNP" if len(v["ref"]) == 1 and len(v["alt"]) == 1 else "INDEL"

# Stage 4: Summarize by chromosome
by_chrom = defaultdict(list)
for v in filtered:
    by_chrom[v["chrom"]].append(v)

print("Variant Pipeline (ClinVar Real Data):")
print(f"  Chromosomes analyzed: {len(by_chrom)}")
summaries = []
for chrom, vs in by_chrom.items():
    snps = sum(1 for v in vs if v["variant_type"] == "SNP")
    indels = len(vs) - snps
    summaries.append((chrom, len(vs), snps, indels))

for chrom, total, snps, indels in sorted(summaries, key=lambda x: -x[1])[:5]:
    print(f"  {chrom}: {total} variants ({snps} SNPs, {indels} indels)")
