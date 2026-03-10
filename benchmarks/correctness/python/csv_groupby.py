"""Group CSV by cohort, compute count and mean depth. Output JSON."""
import json
import sys
import csv
from collections import defaultdict

groups = defaultdict(list)
with open("data/samples.csv") as f:
    reader = csv.DictReader(f)
    for row in reader:
        groups[row["cohort"]].append(float(row["depth"]))

results = {}
for cohort in sorted(groups.keys()):
    depths = groups[cohort]
    results[cohort] = {
        "count": len(depths),
        "mean_depth": round(sum(depths) / len(depths), 6)
    }

json.dump({"groups": results}, sys.stdout, indent=2)
print()
