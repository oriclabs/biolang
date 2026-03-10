"""Filter VCF by QUAL>=30, count variants per chromosome. Output JSON."""
import json
import sys

chrom_counts = {}
total = 0
passed = 0
with open("data/variants.vcf") as f:
    for line in f:
        if line.startswith("#"):
            continue
        total += 1
        fields = line.strip().split("\t")
        qual = float(fields[5]) if fields[5] != "." else 0.0
        if qual >= 30:
            passed += 1
            chrom = fields[0]
            chrom_counts[chrom] = chrom_counts.get(chrom, 0) + 1

# Sort by chromosome name
sorted_counts = dict(sorted(chrom_counts.items()))
json.dump({"total_variants": total, "passed_qual30": passed,
           "per_chromosome": sorted_counts}, sys.stdout, indent=2)
print()
