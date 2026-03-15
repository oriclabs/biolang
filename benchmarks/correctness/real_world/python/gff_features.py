import json

total = 0
by_type = {}
with open("real_data/ecoli_annotation.gff") as f:
    for line in f:
        if line.startswith("#") or not line.strip():
            continue
        total += 1
        parts = line.strip().split("\t")
        if len(parts) >= 3:
            ftype = parts[2]
            by_type[ftype] = by_type.get(ftype, 0) + 1

print(json.dumps({"total_features": total, "by_type": by_type}))
