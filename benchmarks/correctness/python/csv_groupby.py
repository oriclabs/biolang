"""Group CSV by cohort, compute count and mean age. Output JSON."""
import json
import sys
import csv
from collections import defaultdict

groups = defaultdict(list)
with open("data/metadata.csv") as f:
    reader = csv.DictReader(f)
    for row in reader:
        groups[row["cohort"]].append(float(row["age"]))

results = {}
for cohort in sorted(groups.keys()):
    ages = groups[cohort]
    results[cohort] = {
        "count": len(ages),
        "mean_age": round(sum(ages) / len(ages), 6)
    }

json.dump({"groups": results}, sys.stdout, indent=2)
print()
