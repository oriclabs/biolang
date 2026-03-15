#!/usr/bin/env python3
"""Generate large test files for BLViewer stress testing."""
import os, random, string

OUT = os.path.join(os.path.dirname(__file__), "data_large")
os.makedirs(OUT, exist_ok=True)
random.seed(42)

CHROMS = [f"chr{i}" for i in range(1, 23)] + ["chrX", "chrY"]
BASES = "ACGT"
QUALS = "".join(chr(i) for i in range(33, 74))  # phred 0-40
GENES = [f"GENE{i}" for i in range(1, 5001)]

def rand_seq(n):
    return "".join(random.choices(BASES, k=n))

def rand_qual(n):
    return "".join(random.choices(QUALS, k=n))

# ─── 1. Large VCF — 500K variants (~200MB) ───────────────────────
print("Generating large VCF (500K variants)...")
with open(os.path.join(OUT, "large_500k.vcf"), "w") as f:
    f.write("##fileformat=VCFv4.2\n")
    f.write('##INFO=<ID=DP,Number=1,Type=Integer,Description="Read Depth">\n')
    f.write('##INFO=<ID=AF,Number=1,Type=Float,Description="Allele Frequency">\n')
    f.write('##INFO=<ID=GENE,Number=1,Type=String,Description="Gene">\n')
    f.write('##FORMAT=<ID=GT,Number=1,Type=String,Description="Genotype">\n')
    f.write('##FORMAT=<ID=DP,Number=1,Type=Integer,Description="Read Depth">\n')
    f.write("#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tSAMPLE1\tSAMPLE2\tSAMPLE3\n")
    for i in range(500000):
        chrom = random.choice(CHROMS)
        pos = random.randint(1, 250000000)
        ref = random.choice(BASES)
        alt = random.choice([b for b in BASES if b != ref])
        qual = round(random.uniform(10, 99), 1)
        filt = random.choice(["PASS", "PASS", "PASS", "LowQual", "LowDP"])
        dp = random.randint(5, 500)
        af = round(random.uniform(0.001, 0.5), 4)
        gene = random.choice(GENES)
        info = f"DP={dp};AF={af};GENE={gene}"
        gt1 = random.choice(["0/0", "0/1", "1/1"])
        gt2 = random.choice(["0/0", "0/1", "1/1"])
        gt3 = random.choice(["0/0", "0/1", "1/1"])
        dp1, dp2, dp3 = random.randint(5, 200), random.randint(5, 200), random.randint(5, 200)
        f.write(f"{chrom}\t{pos}\trs{100000+i}\t{ref}\t{alt}\t{qual}\t{filt}\t{info}\tGT:DP\t{gt1}:{dp1}\t{gt2}:{dp2}\t{gt3}:{dp3}\n")

# ─── 2. Large BED — 1M regions (~60MB) ───────────────────────────
print("Generating large BED (1M regions)...")
with open(os.path.join(OUT, "large_1m.bed"), "w") as f:
    for i in range(1000000):
        chrom = random.choice(CHROMS)
        start = random.randint(0, 250000000)
        end = start + random.randint(100, 50000)
        name = f"peak_{i}"
        score = random.randint(0, 1000)
        strand = random.choice(["+", "-"])
        f.write(f"{chrom}\t{start}\t{end}\t{name}\t{score}\t{strand}\n")

# ─── 3. Large FASTQ — 500K reads (~250MB) ────────────────────────
print("Generating large FASTQ (500K reads)...")
with open(os.path.join(OUT, "large_500k.fq"), "w") as f:
    for i in range(500000):
        rlen = random.randint(100, 150)
        f.write(f"@read_{i} length={rlen}\n")
        f.write(rand_seq(rlen) + "\n")
        f.write("+\n")
        f.write(rand_qual(rlen) + "\n")

# ─── 4. Large CSV — 1M rows expression matrix (~150MB) ───────────
print("Generating large CSV (1M rows)...")
samples = [f"sample_{j}" for j in range(1, 21)]
with open(os.path.join(OUT, "large_1m.csv"), "w") as f:
    f.write("gene_id,gene_name,chrom,start,end,biotype," + ",".join(samples) + "\n")
    biotypes = ["protein_coding", "lncRNA", "miRNA", "pseudogene", "snRNA"]
    for i in range(1000000):
        gid = f"ENSG{i:011d}"
        gname = f"GENE{i}"
        chrom = random.choice(CHROMS)
        start = random.randint(1, 250000000)
        end = start + random.randint(500, 100000)
        bt = random.choice(biotypes)
        vals = ",".join(str(round(random.expovariate(0.01), 2)) for _ in range(20))
        f.write(f"{gid},{gname},{chrom},{start},{end},{bt},{vals}\n")

# ─── 5. Large GFF — 500K features (~120MB) ───────────────────────
print("Generating large GFF (500K features)...")
with open(os.path.join(OUT, "large_500k.gff"), "w") as f:
    f.write("##gff-version 3\n")
    ftypes = ["gene", "mRNA", "exon", "CDS", "five_prime_UTR", "three_prime_UTR"]
    sources = ["ensembl", "havana", "ensembl_havana"]
    for i in range(500000):
        chrom = random.choice(CHROMS)
        src = random.choice(sources)
        ftype = random.choice(ftypes)
        start = random.randint(1, 250000000)
        end = start + random.randint(50, 50000)
        score = "."
        strand = random.choice(["+", "-"])
        phase = random.choice([".", "0", "1", "2"]) if ftype == "CDS" else "."
        attrs = f"ID=feature_{i};Name={random.choice(GENES)};biotype=protein_coding"
        f.write(f"{chrom}\t{src}\t{ftype}\t{start}\t{end}\t{score}\t{strand}\t{phase}\t{attrs}\n")

# ─── 6. Large FASTA — 10K sequences, long (~100MB) ───────────────
print("Generating large FASTA (10K seqs, avg 10kb)...")
with open(os.path.join(OUT, "large_10k.fa"), "w") as f:
    for i in range(10000):
        slen = random.randint(2000, 20000)
        f.write(f">seq_{i} length={slen} organism=test\n")
        seq = rand_seq(slen)
        for j in range(0, len(seq), 80):
            f.write(seq[j:j+80] + "\n")

# ─── 7. Large TSV — 2M rows simple table (~200MB) ────────────────
print("Generating large TSV (2M rows)...")
with open(os.path.join(OUT, "large_2m.tsv"), "w") as f:
    f.write("id\tchrom\tposition\tref\talt\tquality\tdepth\tfreq\tgene\teffect\n")
    effects = ["missense", "synonymous", "nonsense", "frameshift", "splice_donor", "splice_acceptor", "intron", "intergenic", "UTR_3", "UTR_5"]
    for i in range(2000000):
        chrom = random.choice(CHROMS)
        pos = random.randint(1, 250000000)
        ref = random.choice(BASES)
        alt = random.choice([b for b in BASES if b != ref])
        qual = round(random.uniform(0, 99.9), 1)
        depth = random.randint(1, 1000)
        freq = round(random.uniform(0, 1), 4)
        gene = random.choice(GENES)
        effect = random.choice(effects)
        f.write(f"var_{i}\t{chrom}\t{pos}\t{ref}\t{alt}\t{qual}\t{depth}\t{freq}\t{gene}\t{effect}\n")

print("\nDone! Files generated in:", OUT)
for fn in sorted(os.listdir(OUT)):
    sz = os.path.getsize(os.path.join(OUT, fn))
    if sz > 1024*1024:
        print(f"  {fn}: {sz / (1024*1024):.0f} MB")
    else:
        print(f"  {fn}: {sz / 1024:.0f} KB")
