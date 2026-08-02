"""Small 10x MEX directory shaped like the NSCLC tutorial data.

Four populations with distinct marker blocks, plus MT- genes so the
percent-mito QC step has something real to filter on, and some junk barcodes
that the min-features filter should drop.
"""
import gzip
import os
import random
import argparse

random.seed(5)

parser = argparse.ArgumentParser(description="Create a deterministic 10x MEX fixture")
parser.add_argument("--output", default="nsclc_like")
args = parser.parse_args()

OUT = args.output
os.makedirs(OUT, exist_ok=True)

N_POP, PER_POP = 4, 60
N_MARKER = 30          # marker genes per population
N_BG = 40              # shared background genes
N_MT = 8               # MT- genes
N_JUNK = 25            # empty droplets

genes = []
for p in range(N_POP):
    genes += [f"MARK{p}_{i:03d}" for i in range(N_MARKER)]
genes += [f"BG{i:03d}" for i in range(N_BG)]
genes += [f"MT-ND{i}" for i in range(N_MT)]
n_genes = len(genes)

barcodes, truth, cols = [], [], []
for p in range(N_POP):
    for c in range(PER_POP):
        bc = f"{p}{c:03d}AACCGGTT-1"
        barcodes.append(bc)
        truth.append(p)
        col = {}
        for gi, g in enumerate(genes):
            if g.startswith(f"MARK{p}_"):
                v = random.randint(4, 30)
            elif g.startswith("MARK"):
                v = 1 if random.random() < 0.10 else 0
            elif g.startswith("MT-"):
                # a slice of cells are high-mito and should be filtered out
                hi = (c % 12 == 0)
                v = random.randint(30, 60) if hi else random.randint(0, 3)
            else:
                v = random.randint(0, 6)
            if v:
                col[gi] = v
        cols.append(col)

# empty droplets: almost nothing, should fail min-features
for j in range(N_JUNK):
    barcodes.append(f"JUNK{j:04d}AACC-1")
    truth.append(-1)
    cols.append({random.randrange(n_genes): 1 for _ in range(random.randint(1, 4))})

with gzip.open(f"{OUT}/features.tsv.gz", "wt") as f:
    for i, g in enumerate(genes):
        f.write(f"ENSG{i:08d}\t{g}\tGene Expression\n")
with gzip.open(f"{OUT}/barcodes.tsv.gz", "wt") as f:
    for b in barcodes:
        f.write(b + "\n")

nnz = sum(len(c) for c in cols)
with gzip.open(f"{OUT}/matrix.mtx.gz", "wt") as f:
    f.write("%%MatrixMarket matrix coordinate integer general\n%\n")
    f.write(f"{n_genes} {len(barcodes)} {nnz}\n")
    for ci, col in enumerate(cols):
        for gi, v in sorted(col.items()):
            f.write(f"{gi + 1} {ci + 1} {v}\n")   # MEX is 1-indexed, genes x cells

with open(f"{OUT}/truth.csv", "w") as f:
    f.write("barcode,cluster\n")
    for b, t in zip(barcodes, truth):
        f.write(f"{b},{t}\n")

print(f"{n_genes} genes x {len(barcodes)} barcodes ({N_JUNK} junk), {nnz} nonzero")
print(f"output: {os.path.abspath(OUT)}")
