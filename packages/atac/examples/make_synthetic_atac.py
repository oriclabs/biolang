"""Synthetic scATAC fragments with three known cell populations.

Each population has a set of genes it is preferentially accessible at, so a
correct gene_activity + clustering pipeline should recover the three groups.
Writes fragments.tsv.gz, genes.gtf and truth.csv.
"""
import gzip
import random

random.seed(11)

N_GENES = 120
N_PER_POP = 40
POPS = 3
GENE_LEN = 4000
SPACING = 40_000

# genes 0..39 mark pop0, 40..79 mark pop1, 80..119 mark pop2
genes = []
for g in range(N_GENES):
    chrom = f"chr{g % 4 + 1}"
    start = 100_000 + (g // 4) * SPACING
    strand = "+" if g % 2 == 0 else "-"
    genes.append((f"GENE{g:03d}", chrom, start, start + GENE_LEN, strand))

with open("genes.gtf", "w") as f:
    for name, chrom, start, end, strand in genes:
        attrs = f'gene_id "{name}"; gene_name "{name}";'
        f.write(f"{chrom}\tsynth\tgene\t{start}\t{end}\t.\t{strand}\t.\t{attrs}\n")

marker_block = N_GENES // POPS  # 40 genes per population

rows = []
truth = []
for pop in range(POPS):
    for c in range(N_PER_POP):
        bc = f"CELL{pop}_{c:03d}"
        truth.append((bc, pop))
        for gi, (name, chrom, start, end, strand) in enumerate(genes):
            in_marker_block = (gi // marker_block) == pop
            # marker genes get many more fragments than background
            lam = 9 if in_marker_block else 1
            n = sum(1 for _ in range(lam * 2) if random.random() < 0.5)
            for _ in range(n):
                # place inside the gene body, occasionally just upstream
                if random.random() < 0.2:
                    pos = start - random.randint(1, 1800)
                else:
                    pos = random.randint(start, end - 100)
                rows.append((chrom, pos, pos + 80, bc, 1))

random.shuffle(rows)
with gzip.open("fragments.tsv.gz", "wt") as f:
    f.write("# synthetic fragments\n")
    for chrom, s, e, bc, n in rows:
        f.write(f"{chrom}\t{s}\t{e}\t{bc}\t{n}\n")

with open("truth.csv", "w") as f:
    for bc, pop in truth:
        f.write(f"{bc},{pop}\n")

print(f"{len(rows)} fragments, {len(truth)} cells, {N_GENES} genes")
