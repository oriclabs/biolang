"""Shannon and Simpson diversity over GFF feature types. Output JSON."""
import json
import sys
import math
from collections import Counter

counts = Counter()
total = 0
with open("data/annotations.gff3") as f:
    for line in f:
        if line.startswith("#") or not line.strip():
            continue
        parts = line.rstrip("\n").split("\t")
        if len(parts) >= 3:
            counts[parts[2]] += 1
            total += 1

ps = [counts[k] / total for k in sorted(counts)]
shannon = -sum(p * math.log(p) for p in ps)
simpson = 1 - sum(p * p for p in ps)

json.dump({"n_categories": len(ps), "n_observations": total,
           "shannon": round(shannon, 9), "simpson": round(simpson, 9)},
          sys.stdout, indent=2)
print()
