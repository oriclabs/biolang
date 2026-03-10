"""Count features by type from GFF3. Output JSON."""
import json
import sys
from collections import Counter

counts = Counter()
total = 0
with open("data/annotations.gff") as f:
    for line in f:
        if line.startswith("#") or line.strip() == "":
            continue
        fields = line.strip().split("\t")
        if len(fields) >= 3:
            counts[fields[2]] += 1
            total += 1

sorted_counts = dict(sorted(counts.items()))
json.dump({"total_features": total, "by_type": sorted_counts}, sys.stdout, indent=2)
print()
