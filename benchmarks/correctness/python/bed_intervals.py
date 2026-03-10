"""Parse BED, compute total span, count per chromosome, merge overlapping. Output JSON."""
import json
import sys
from collections import defaultdict

intervals = []
chrom_counts = defaultdict(int)
with open("data/promoters.bed") as f:
    for line in f:
        if line.startswith("#") or line.strip() == "":
            continue
        fields = line.strip().split("\t")
        chrom = fields[0]
        start = int(fields[1])
        end = int(fields[2])
        intervals.append((chrom, start, end))
        chrom_counts[chrom] += 1

total_span = sum(end - start for _, start, end in intervals)

# Merge overlapping intervals per chromosome
by_chrom = defaultdict(list)
for chrom, start, end in intervals:
    by_chrom[chrom].append((start, end))

merged_count = 0
for chrom in by_chrom:
    sorted_ivs = sorted(by_chrom[chrom])
    merged = [sorted_ivs[0]]
    for start, end in sorted_ivs[1:]:
        if start <= merged[-1][1]:
            merged[-1] = (merged[-1][0], max(merged[-1][1], end))
        else:
            merged.append((start, end))
    merged_count += len(merged)

json.dump({
    "n_intervals": len(intervals),
    "total_span": total_span,
    "per_chromosome": dict(sorted(chrom_counts.items())),
    "merged_count": merged_count
}, sys.stdout, indent=2)
print()
