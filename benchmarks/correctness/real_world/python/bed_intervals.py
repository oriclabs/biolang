import json

intervals = []
with open("real_data/ecoli_genes.bed") as f:
    for line in f:
        parts = line.strip().split("\t")
        if len(parts) >= 3:
            intervals.append((parts[0], int(parts[1]), int(parts[2])))

n = len(intervals)
total_span = sum(end - start for _, start, end in intervals)

per_chrom = {}
for chrom, _, _ in intervals:
    per_chrom[chrom] = per_chrom.get(chrom, 0) + 1

# Merge overlapping
sorted_ivs = sorted(intervals, key=lambda x: (x[0], x[1]))
merged = []
if sorted_ivs:
    cur_chrom, cur_start, cur_end = sorted_ivs[0]
    for chrom, start, end in sorted_ivs[1:]:
        if chrom == cur_chrom and start <= cur_end:
            cur_end = max(cur_end, end)
        else:
            merged.append((cur_chrom, cur_start, cur_end))
            cur_chrom, cur_start, cur_end = chrom, start, end
    merged.append((cur_chrom, cur_start, cur_end))

print(json.dumps({"n_intervals": n, "total_span": total_span, "per_chromosome": per_chrom, "merged_count": len(merged)}))
