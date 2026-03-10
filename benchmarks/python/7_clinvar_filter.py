"""Benchmark 7: Real-World ClinVar Variant Analysis
Filter and classify ClinVar variants from NCBI."""

total = 0
pathogenic = []

with open("data_real/clinvar_20k.vcf") as f:
    for line in f:
        if line.startswith("#"):
            continue
        total += 1
        fields = line.strip().split("\t")
        chrom = fields[0]
        ref = fields[3]
        alt = fields[4]

        # Parse INFO for CLNSIG
        info = {}
        for item in fields[7].split(";"):
            if "=" in item:
                k, v = item.split("=", 1)
                info[k] = v

        clnsig = info.get("CLNSIG", "")
        if clnsig in ("Pathogenic", "Likely_pathogenic", "Pathogenic/Likely_pathogenic"):
            pathogenic.append((chrom, ref, alt))

chr1_path = [v for v in pathogenic if v[0] in ("1", "chr1")]
snps = sum(1 for _, ref, alt in pathogenic if len(ref) == 1 and len(alt) == 1)
indels = len(pathogenic) - snps

print(f"Total variants: {total}")
print(f"Pathogenic/Likely pathogenic: {len(pathogenic)}")
print(f"Pathogenic on chr1: {len(chr1_path)}")
print(f"Pathogenic SNPs: {snps}")
print(f"Pathogenic Indels: {indels}")
