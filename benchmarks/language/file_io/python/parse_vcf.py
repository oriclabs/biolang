"""File I/O: Parse VCF (50K variants, ~2.3 MB)"""
count = 0
with open("data/variants.vcf") as f:
    for line in f:
        if not line.startswith("#"):
            count += 1
print(f"Records: {count}")
