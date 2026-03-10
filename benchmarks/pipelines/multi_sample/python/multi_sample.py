"""Multi-Sample Pipeline: Load sheet -> Per-sample stats -> Aggregate
Demonstrates looping over samples with aggregation."""
import csv
from collections import defaultdict
import statistics

# Stage 1: Load sample sheet
samples = {}
with open("data/samples.csv") as f:
    for row in csv.DictReader(f):
        samples[row["sample_id"]] = {
            "depth": float(row["depth"]),
            "quality": float(row["quality"]),
            "read_count": int(row["read_count"]),
        }

# Stage 2: Load metadata
metadata = {}
with open("data/metadata.csv") as f:
    for row in csv.DictReader(f):
        metadata[row["sample_id"]] = row

# Stage 3: Join
joined = []
for sid, s in samples.items():
    if sid in metadata:
        joined.append({**s, **metadata[sid]})

# Stage 4: Per-cohort analysis
by_cohort = defaultdict(list)
for s in joined:
    by_cohort[s["cohort"]].append(s)

print("Multi-Sample Pipeline Results:")
print(f"  Total samples: {len(joined)}")
print(f"  Cohorts: {len(by_cohort)}")
print()

for cohort, rows in sorted(by_cohort.items()):
    depths = [r["depth"] for r in rows]
    quals = [r["quality"] for r in rows]
    reads = [r["read_count"] for r in rows]
    print(f"  {cohort}:")
    print(f"    Samples: {len(rows)}")
    print(f"    Mean depth: {round(statistics.mean(depths), 1)}")
    print(f"    Mean quality: {round(statistics.mean(quals), 1)}")
    print(f"    Total reads: {sum(reads)}")

all_depths = [s["depth"] for s in joined]
all_quals = [s["quality"] for s in joined]
print()
print(f"  Overall mean depth: {round(statistics.mean(all_depths), 1)}")
print(f"  Overall mean quality: {round(statistics.mean(all_quals), 1)}")
print(f"  Depth std dev: {round(statistics.stdev(all_depths), 2)}")
