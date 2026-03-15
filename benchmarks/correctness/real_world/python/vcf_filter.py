import json

total = 0
pathogenic = []
with open("real_data/clinvar.vcf") as f:
    for line in f:
        if line.startswith("#"):
            continue
        total += 1
        parts = line.strip().split("\t")
        if len(parts) < 8:
            continue
        info = parts[7]
        for field in info.split(";"):
            if field.startswith("CLNSIG="):
                sig = field.split("=", 1)[1]
                if sig == "Pathogenic" or sig.startswith("Pathogenic/") or sig.startswith("Pathogenic|"):
                    pathogenic.append(parts[0])
                break

per_chrom = {}
for c in pathogenic:
    per_chrom[c] = per_chrom.get(c, 0) + 1

print(json.dumps({"total_variants": total, "pathogenic_count": len(pathogenic), "per_chromosome": per_chrom}))
