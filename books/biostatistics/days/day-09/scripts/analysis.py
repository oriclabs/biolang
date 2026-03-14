# Day 9: Non-Parametric Tests
import pandas as pd
from scipy import stats

micro = pd.read_csv("microbiome.csv")
sites = pd.read_csv("body_sites.csv")

# Mann-Whitney U: IBD vs Healthy
ibd = micro[micro.group == "IBD"]["otu_count"]
healthy = micro[micro.group == "Healthy"]["otu_count"]
u_stat, p_val = stats.mannwhitneyu(ibd, healthy, alternative="two-sided")
print("=== Mann-Whitney U (IBD vs Healthy) ===")
print(f"IBD median:     {ibd.median()}")
print(f"Healthy median: {healthy.median()}")
print(f"U-statistic:    {u_stat:.1f}")
print(f"P-value:        {p_val:.4f}")

# Kruskal-Wallis: body sites
groups = [g["shannon_diversity"].values for _, g in sites.groupby("body_site")]
h_stat, p_kw = stats.kruskal(*groups)
print("\n=== Kruskal-Wallis (Body Sites) ===")
for site in sites.body_site.unique():
    vals = sites[sites.body_site == site]["shannon_diversity"]
    print(f"  {site:6s}: median={vals.median():.0f}")
print(f"H-statistic: {h_stat:.2f}")
print(f"P-value:     {p_kw:.4e}")
