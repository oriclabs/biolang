#!/usr/bin/env python3
"""Generate synthetic benchmark data files.

Creates reproducible test data for all 5 benchmark tasks.
Uses a fixed random seed for reproducibility.
"""
import random
import os

random.seed(42)
os.makedirs("data", exist_ok=True)

BASES = "ACGT"

def random_seq(length):
    return "".join(random.choice(BASES) for _ in range(length))

def random_qual(length, min_q=15, max_q=40):
    return "".join(chr(random.randint(min_q, max_q) + 33) for _ in range(length))

# ── 1. FASTA: 10,000 sequences, 500-5000 bp ──
print("Generating sequences.fa...")
with open("data/sequences.fa", "w") as f:
    for i in range(10_000):
        length = random.randint(500, 5000)
        seq = random_seq(length)
        f.write(f">seq_{i:05d} length={length}\n")
        for j in range(0, len(seq), 80):
            f.write(seq[j:j+80] + "\n")

# ── 2. FASTQ: 100,000 reads, 100-150 bp ──
print("Generating reads.fq...")
with open("data/reads.fq", "w") as f:
    for i in range(100_000):
        length = random.randint(100, 150)
        seq = random_seq(length)
        qual = random_qual(length)
        f.write(f"@read_{i:06d}\n{seq}\n+\n{qual}\n")

# ── 3. VCF: 50,000 variants ──
print("Generating variants.vcf...")
chroms = [f"chr{c}" for c in list(range(1, 23)) + ["X", "Y"]]
with open("data/variants.vcf", "w") as f:
    f.write("##fileformat=VCFv4.2\n")
    f.write('##INFO=<ID=DP,Number=1,Type=Integer,Description="Depth">\n')
    f.write('##INFO=<ID=AF,Number=1,Type=Float,Description="Allele Frequency">\n')
    f.write("#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n")
    for i in range(50_000):
        chrom = random.choice(chroms)
        pos = random.randint(1, 250_000_000)
        ref = random.choice(BASES)
        # 80% SNPs, 20% indels
        if random.random() < 0.8:
            alt = random.choice([b for b in BASES if b != ref])
        else:
            if random.random() < 0.5:
                alt = ref + random_seq(random.randint(1, 5))
            else:
                ref = ref + random_seq(random.randint(1, 5))
                alt = ref[0]
        qual = round(random.uniform(5, 60), 1)
        dp = random.randint(1, 100)
        af = round(random.uniform(0.01, 1.0), 3)
        filt = "PASS" if qual >= 20 else "LowQual"
        f.write(f"{chrom}\t{pos}\t.\t{ref}\t{alt}\t{qual}\t{filt}\tDP={dp};AF={af}\n")

# ── 4. CSV: 5,000 samples with metadata ──
print("Generating samples.csv and metadata.csv...")
cohorts = ["control", "treatment_A", "treatment_B", "treatment_C"]
sites = ["site_1", "site_2", "site_3"]

with open("data/samples.csv", "w") as f:
    f.write("sample_id,depth,quality,read_count\n")
    for i in range(5_000):
        sid = f"S{i:04d}"
        depth = round(random.uniform(5, 80), 1)
        quality = round(random.uniform(15, 40), 1)
        read_count = random.randint(100_000, 10_000_000)
        f.write(f"{sid},{depth},{quality},{read_count}\n")

with open("data/metadata.csv", "w") as f:
    f.write("sample_id,cohort,site,age,sex\n")
    for i in range(5_000):
        sid = f"S{i:04d}"
        cohort = random.choice(cohorts)
        site = random.choice(sites)
        age = random.randint(20, 85)
        sex = random.choice(["M", "F"])
        f.write(f"{sid},{cohort},{site},{age},{sex}\n")

# ── 5. BED: 10,000 genomic regions ──
print("Generating regions.bed...")
chroms_bed = [f"chr{c}" for c in range(1, 23)]
with open("data/regions.bed", "w") as f:
    for i in range(10_000):
        chrom = random.choice(chroms_bed)
        start = random.randint(1, 250_000_000)
        end = start + random.randint(100, 10_000)
        name = f"region_{i:05d}"
        score = random.randint(0, 1000)
        strand = random.choice(["+", "-"])
        f.write(f"{chrom}\t{start}\t{end}\t{name}\t{score}\t{strand}\n")

# ── 6. BED: 1,000 query regions ──
print("Generating queries.bed...")
with open("data/queries.bed", "w") as f:
    for i in range(1_000):
        chrom = random.choice(chroms_bed)
        start = random.randint(1, 250_000_000)
        end = start + random.randint(500, 5_000)
        f.write(f"{chrom}\t{start}\t{end}\tquery_{i:04d}\t0\t+\n")

# ── 7. GFF3: 5,000 gene annotations ──
print("Generating annotations.gff3...")
feature_types = ["gene", "mRNA", "exon", "CDS"]
with open("data/annotations.gff3", "w") as f:
    f.write("##gff-version 3\n")
    gene_id = 0
    for i in range(5_000):
        chrom = random.choice(chroms_bed)
        start = random.randint(1, 250_000_000)
        gene_len = random.randint(1_000, 50_000)
        end = start + gene_len
        strand = random.choice(["+", "-"])
        gene_name = f"GENE{gene_id:05d}"
        f.write(f"{chrom}\t.\tgene\t{start}\t{end}\t.\t{strand}\t.\tID={gene_name};Name={gene_name}\n")
        # Add 2-5 exons per gene
        n_exons = random.randint(2, 5)
        exon_starts = sorted(random.sample(range(start, end - 100), min(n_exons, (end - start) // 100)))
        for j, es in enumerate(exon_starts):
            ee = es + random.randint(50, 500)
            if ee > end:
                ee = end
            f.write(f"{chrom}\t.\texon\t{es}\t{ee}\t.\t{strand}\t.\tParent={gene_name};ID={gene_name}.exon{j+1}\n")
        gene_id += 1

# ── 8. CSV: Gene expression count matrix (100 genes x 20 samples) ──
print("Generating gene_counts.csv...")
genes = [f"GENE{i:04d}" for i in range(100)]
samples_expr = [f"sample_{i:02d}" for i in range(20)]
conditions = ["control"] * 10 + ["treatment"] * 10
with open("data/gene_counts.csv", "w") as f:
    f.write("gene," + ",".join(samples_expr) + "\n")
    for gene in genes:
        base_expr = random.randint(10, 10_000)
        counts = []
        for j, sample in enumerate(samples_expr):
            # Treatment samples get 1.5x fold change for ~30% of genes
            fc = 1.5 if conditions[j] == "treatment" and random.random() < 0.3 else 1.0
            count = max(0, int(base_expr * fc * random.uniform(0.5, 1.5)))
            counts.append(str(count))
        f.write(gene + "," + ",".join(counts) + "\n")

# Also write conditions file
with open("data/conditions.csv", "w") as f:
    f.write("sample,condition\n")
    for s, c in zip(samples_expr, conditions):
        f.write(f"{s},{c}\n")

# ── 9. CSV: Gene annotation table (gene -> chromosome, pathway) ──
print("Generating gene_annotations.csv...")
pathways = ["cell_cycle", "apoptosis", "dna_repair", "metabolism", "signaling",
            "immune_response", "transcription", "translation", "transport", "unknown"]
with open("data/gene_annotations.csv", "w") as f:
    f.write("gene,chromosome,pathway,description\n")
    for i in range(500):
        gene = f"GENE{i:04d}"
        chrom = random.choice(chroms_bed)
        pathway = random.choice(pathways)
        desc = f"{pathway.replace('_', ' ')} related gene {gene}"
        f.write(f"{gene},{chrom},{pathway},{desc}\n")

print("Done. Files in data/:")
for fname in sorted(os.listdir("data")):
    size = os.path.getsize(f"data/{fname}")
    if size > 1_000_000:
        print(f"  {fname}: {size / 1_000_000:.1f} MB")
    else:
        print(f"  {fname}: {size / 1_000:.1f} KB")
