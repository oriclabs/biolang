"""Benchmark 5: CSV Wrangling"""
import csv
from collections import defaultdict
from statistics import mean

# Read samples
with open("data/samples.csv") as f:
    samples = list(csv.DictReader(f))
for s in samples:
    s["depth"] = float(s["depth"])
    s["quality"] = float(s["quality"])
    s["read_count"] = int(s["read_count"])

# Read metadata
with open("data/metadata.csv") as f:
    metadata = {row["sample_id"]: row for row in csv.DictReader(f)}

# Join
joined = []
for s in samples:
    if s["sample_id"] in metadata:
        merged = {**s, **metadata[s["sample_id"]]}
        joined.append(merged)

# Group by cohort
groups = defaultdict(list)
for row in joined:
    groups[row["cohort"]].append(row)

# Summarize
summary = []
for cohort, rows in groups.items():
    summary.append({
        "cohort": cohort,
        "count": len(rows),
        "mean_depth": mean(r["depth"] for r in rows),
        "mean_quality": mean(r["quality"] for r in rows),
        "total_reads": sum(r["read_count"] for r in rows),
    })

summary.sort(key=lambda x: x["mean_depth"], reverse=True)

print("Cohort Summary:")
for row in summary:
    print(f"  {row['cohort']}: n={row['count']}, depth={row['mean_depth']:.1f}, "
          f"qual={row['mean_quality']:.1f}, reads={row['total_reads']}")

# High-quality filter
hq = [r for r in joined if r["quality"] >= 30 and r["depth"] >= 20]
print(f"\nHigh-quality samples: {len(hq)} / {len(joined)}")
