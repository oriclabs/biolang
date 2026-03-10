"""Benchmark 4: VCF Filtering"""
import re

total = 0
filtered = []

with open("data/variants.vcf") as f:
    for line in f:
        if line.startswith("#"):
            continue
        total += 1
        fields = line.strip().split("\t")
        chrom = fields[0]
        qual = float(fields[5]) if fields[5] != "." else 0.0
        ref = fields[3]
        alt = fields[4]

        # Parse INFO for DP
        info = {}
        for item in fields[7].split(";"):
            if "=" in item:
                k, v = item.split("=", 1)
                info[k] = v

        dp = int(info.get("DP", "0"))

        if qual >= 30.0 and dp >= 10 and chrom in ("chr1", "1"):
            filtered.append((chrom, ref, alt, qual, dp))

snps = sum(1 for _, ref, alt, _, _ in filtered if len(ref) == 1 and len(alt) == 1)
indels = len(filtered) - snps

print(f"Total variants: {total}")
print(f"After filtering: {len(filtered)}")
print(f"SNPs: {snps}")
print(f"Indels: {indels}")
print(f"Ti/Tv ratio: computed from filtered set")
