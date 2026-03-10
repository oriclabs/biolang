"""BED Interval Overlap (Real Data): ENCODE H3K27ac vs CTCF peaks"""
from collections import defaultdict

# Load H3K27ac peaks as regions
regions_by_chrom = defaultdict(list)
n_regions = 0
with open("data_real/encode_h3k27ac_peaks.bed") as f:
    for line in f:
        if line.startswith("#") or line.startswith("track"):
            continue
        fields = line.strip().split("\t")
        if len(fields) < 3:
            continue
        chrom, start, end = fields[0], int(fields[1]), int(fields[2])
        regions_by_chrom[chrom].append((start, end))
        n_regions += 1

# Sort regions by start for binary search
for chrom in regions_by_chrom:
    regions_by_chrom[chrom].sort()

# Load CTCF peaks as queries and find overlaps
total_overlaps = 0
n_queries = 0
with open("data_real/encode_ctcf_peaks.bed") as f:
    for line in f:
        if line.startswith("#") or line.startswith("track"):
            continue
        fields = line.strip().split("\t")
        if len(fields) < 3:
            continue
        chrom, q_start, q_end = fields[0], int(fields[1]), int(fields[2])
        n_queries += 1
        for start, end in regions_by_chrom.get(chrom, []):
            if start >= q_end:
                break
            if end > q_start:
                total_overlaps += 1

print(f"H3K27ac peaks (regions): {n_regions}")
print(f"CTCF peaks (queries): {n_queries}")
print(f"Total overlaps: {total_overlaps}")
