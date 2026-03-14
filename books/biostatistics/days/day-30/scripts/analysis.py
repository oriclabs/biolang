# Day 30: Capstone — GWAS Analysis
import pandas as pd
import numpy as np
from scipy import stats
from statsmodels.stats.multitest import multipletests

gwas = pd.read_csv("gwas_summary.csv")
print(f"Total SNPs: {len(gwas):,}")

# 1. Genome-wide significance
gws = gwas[gwas.pvalue < 5e-8]
print(f"\n=== Genome-Wide Significant (p < 5e-8) ===")
print(f"N hits: {len(gws)}")
print(f"True positives: {gws.true_assoc.sum()}")
print(f"False positives: {(gws.true_assoc == 0).sum()}")

# 2. Suggestive significance
sug = gwas[(gwas.pvalue < 1e-5) & (gwas.pvalue >= 5e-8)]
print(f"\nSuggestive (1e-5 < p < 5e-8): {len(sug)}")

# 3. Lambda (genomic inflation)
chisq_vals = stats.norm.ppf(1 - gwas.pvalue/2)**2
lambda_gc = np.median(chisq_vals) / 0.4549
print(f"\nGenomic inflation factor (lambda): {lambda_gc:.3f}")

# 4. FDR approach
_, padj, _, _ = multipletests(gwas.pvalue, method="fdr_bh")
gwas["padj"] = padj
fdr_sig = gwas[gwas.padj < 0.05]
print(f"\nFDR < 0.05: {len(fdr_sig)} SNPs")

# 5. Top hits
print("\n=== Top 10 Hits ===")
top = gwas.nsmallest(10, "pvalue")
for _, row in top.iterrows():
    print(f"  {row.snp_id} chr{row.chr}:{row.pos} beta={row.beta:.4f} p={row.pvalue:.2e} {'*' if row.true_assoc else ''}")

# 6. Power assessment
true_snps = gwas[gwas.true_assoc == 1]
detected = (true_snps.pvalue < 5e-8).sum()
print(f"\n=== Power ===")
print(f"True associations: {len(true_snps)}")
print(f"Detected at GWS: {detected}")
print(f"Power: {detected/len(true_snps)*100:.1f}%")

# 7. Manhattan-style summary by chromosome
print("\n=== Hits per Chromosome ===")
for chr_num in sorted(gws.chr.unique()):
    n_chr = (gws.chr == chr_num).sum()
    print(f"  chr{chr_num}: {n_chr} hits")
