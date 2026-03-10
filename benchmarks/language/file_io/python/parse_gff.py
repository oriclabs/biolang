"""File I/O: Parse GFF3 annotations (5,000 genes with exons)"""
total = 0
genes = 0
exons = 0
with open("data/annotations.gff3") as f:
    for line in f:
        if line.startswith("#"):
            continue
        total += 1
        fields = line.strip().split("\t")
        if len(fields) >= 3:
            if fields[2] == "gene":
                genes += 1
            elif fields[2] == "exon":
                exons += 1

print(f"Total features: {total}")
print(f"Genes: {genes}")
print(f"Exons: {exons}")
