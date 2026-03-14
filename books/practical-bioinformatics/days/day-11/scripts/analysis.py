# Day 11: Sequence Comparison — Python equivalent
# Uses Biopython for sequence operations

from Bio.Seq import Seq
from Bio.SeqUtils import gc_fraction
from collections import Counter
import itertools

# ── Step 1: Base Composition Analysis ─────────────────────────────────

print("=== Base Composition Analysis ===")

seqs = [
    {"name": "E. coli",  "seq": "GCGCATCGATCGATCGCG"},
    {"name": "Human",    "seq": "ATATCGATCGATATATAT"},
    {"name": "Thermus",  "seq": "GCGCGCGCGCGCGCGCGC"},
]

for s in seqs:
    seq = Seq(s["seq"])
    gc = round(gc_fraction(seq) * 100, 1)
    counts = {base: s["seq"].count(base) for base in "ATGC"}
    print(f"{s['name']}: GC={gc}%, A={counts['A']}, T={counts['T']}, G={counts['G']}, C={counts['C']}")

print()

# ── Step 2: K-mer Extraction ──────────────────────────────────────────

print("=== K-mer Analysis ===")

seq_str = "ATCGATCGATCG"
k = 3
kmers_list = [seq_str[i:i+k] for i in range(len(seq_str) - k + 1)]
print(f"Sequence: {seq_str}")
print(f"3-mers: {kmers_list}")
print()

# K-mer frequency
freq = Counter(kmers_list)
print(f"3-mer frequencies: {dict(freq.most_common(5))}")
print()

# ── Step 3: Alignment-Free Similarity ─────────────────────────────────

print("=== K-mer Similarity (Jaccard) ===")

seq1 = "ATCGATCGATCGATCG"
seq2 = "ATCGATCGTTTTGATCG"
k = 5

k1 = set(seq1[i:i+k] for i in range(len(seq1) - k + 1))
k2 = set(seq2[i:i+k] for i in range(len(seq2) - k + 1))

shared = k1 & k2
total = k1 | k2
jaccard = len(shared) / len(total)

print(f"Seq1: {seq1}")
print(f"Seq2: {seq2}")
print(f"Shared 5-mers: {len(shared)}")
print(f"Total unique 5-mers: {len(total)}")
print(f"K-mer similarity: {round(jaccard * 100, 1)}%")
print()

# ── Step 4: Motif Finding ─────────────────────────────────────────────

print("=== Motif Finding ===")

motif_seq = "ATGATCGATGATCGATGATCG"
motif = "ATG"
positions = []
start = 0
while True:
    pos = motif_seq.find(motif, start)
    if pos == -1:
        break
    positions.append(pos)
    start = pos + 1

print(f"Sequence: {motif_seq}")
print(f"ATG positions: {positions}")

re_seq = "ATCGGAATTCGATCGGGATCCATCG"
ecori_pos = []
start = 0
while True:
    pos = re_seq.find("GAATTC", start)
    if pos == -1:
        break
    ecori_pos.append(pos)
    start = pos + 1

bamhi_pos = []
start = 0
while True:
    pos = re_seq.find("GGATCC", start)
    if pos == -1:
        break
    bamhi_pos.append(pos)
    start = pos + 1

print(f"EcoRI sites: {ecori_pos}")
print(f"BamHI sites: {bamhi_pos}")
print()

# ── Step 5: Reverse Complement ────────────────────────────────────────

print("=== Reverse Complement ===")

forward = Seq("ATGCGATCGATCG")
revcomp = forward.reverse_complement()
print(f"Forward:  5'-{forward}-3'")
print(f"RevComp:  5'-{revcomp}-3'")

strand_seq = Seq("ATCGGAATTCGATCG")
motif = "GAATTC"
fwd_hits = [i for i in range(len(strand_seq) - len(motif) + 1) if str(strand_seq[i:i+len(motif)]) == motif]
rev_seq = str(strand_seq.reverse_complement())
rev_hits = [i for i in range(len(rev_seq) - len(motif) + 1) if rev_seq[i:i+len(motif)] == motif]
print(f"Forward strand GAATTC hits: {fwd_hits}")
print(f"Reverse strand GAATTC hits: {rev_hits}")
print()

# ── Step 6: Codon Analysis ────────────────────────────────────────────

print("=== Codon Usage ===")

def codon_usage(seq_str):
    codons = [seq_str[i:i+3] for i in range(0, len(seq_str) - 2, 3)]
    return dict(Counter(codons))

human_gene = "ATGGCTGCTTCTGATAAATGA"
ecoli_gene = "ATGGCAGCGAGCGATAAATGA"
print(f"Human codons: {codon_usage(human_gene)}")
print(f"E. coli codons: {codon_usage(ecoli_gene)}")
print()

# ── Step 7: Similarity Matrix ─────────────────────────────────────────

print("=== Pairwise Similarity Matrix ===")

sequences = [
    {"name": "seq1", "seq": "ATCGATCGATCGATCG"},
    {"name": "seq2", "seq": "ATCGATCGTTTTGATCG"},
    {"name": "seq3", "seq": "GCGCGCGCGCGCGCGC"},
]

k = 5
print(f"{'seq1':>5} | {'seq2':>5} | similarity")
for s1 in sequences:
    for s2 in sequences:
        k1 = set(s1["seq"][i:i+k] for i in range(len(s1["seq"]) - k + 1))
        k2 = set(s2["seq"][i:i+k] for i in range(len(s2["seq"]) - k + 1))
        shared = len(k1 & k2)
        total = len(k1 | k2)
        sim = round(shared / total, 3) if total > 0 else 0.0
        print(f"{s1['name']:>5} | {s2['name']:>5} | {sim}")

print()
print("=== Analysis Complete ===")
