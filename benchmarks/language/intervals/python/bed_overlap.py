"""BED Interval Overlap: find overlapping regions between two BED files"""
from collections import defaultdict

# Load regions
regions_by_chrom = defaultdict(list)
with open("data/regions.bed") as f:
    count = 0
    for line in f:
        fields = line.strip().split("\t")
        chrom, start, end = fields[0], int(fields[1]), int(fields[2])
        regions_by_chrom[chrom].append((start, end))
        count += 1
n_regions = count

# Sort regions by start for binary search
for chrom in regions_by_chrom:
    regions_by_chrom[chrom].sort()

# Load queries and find overlaps
total_overlaps = 0
n_queries = 0
with open("data/queries.bed") as f:
    for line in f:
        fields = line.strip().split("\t")
        chrom, q_start, q_end = fields[0], int(fields[1]), int(fields[2])
        n_queries += 1
        for start, end in regions_by_chrom.get(chrom, []):
            if start >= q_end:
                break
            if end > q_start:
                total_overlaps += 1

print(f"Regions: {n_regions}")
print(f"Queries: {n_queries}")
print(f"Total overlaps: {total_overlaps}")
