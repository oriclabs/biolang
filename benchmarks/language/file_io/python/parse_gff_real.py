"""File I/O: Parse real Ensembl GFF3 annotations (chr22)"""
genes = 0
exons = 0
total = 0
with open("data_real/ensembl_chr22.gff3") as f:
    for line in f:
        if line.startswith("#"):
            continue
        fields = line.strip().split("\t")
        if len(fields) < 9:
            continue
        total += 1
        if fields[2] == "gene":
            genes += 1
        elif fields[2] == "exon":
            exons += 1

print(f"Total features: {total}")
print(f"Genes: {genes}")
print(f"Exons: {exons}")
