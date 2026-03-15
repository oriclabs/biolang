import json
import csv

groups = {}
with open("real_data/clinvar_variants.csv") as f:
    reader = csv.DictReader(f)
    for row in reader:
        sig = row["clnsig"]
        vlen = float(row["var_len"])
        if sig not in groups:
            groups[sig] = {"count": 0, "total_len": 0.0}
        groups[sig]["count"] += 1
        groups[sig]["total_len"] += vlen

result = {}
for sig, g in groups.items():
    result[sig] = {"count": g["count"], "mean_var_len": g["total_len"] / g["count"]}

print(json.dumps({"groups": result}))
