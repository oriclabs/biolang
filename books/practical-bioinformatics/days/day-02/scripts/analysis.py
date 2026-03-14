"""
Day 2: Your First Language — BioLang (Python equivalent)
Complete analysis script covering all chapter concepts
"""

from Bio.Seq import Seq
from Bio.SeqUtils import gc_fraction
from functools import reduce

# --- Variables and Types ---
print("=== Variables and Types ===")
name = "BRCA1"
length = 81189
gc = 0.423
is_oncogene = False
seq = Seq("ATGCGATCG")

print(f"Gene: {name}, Length: {length}, GC: {gc}")
print(f"Types: name={type(name).__name__}, length={type(length).__name__}, "
      f"gc={type(gc).__name__}, seq={type(seq).__name__}")

# --- The Pipe Operator ---
print()
print("=== Pipe Operator ===")

# Python has no pipes — must nest or use temp variables
gc_nested = round(gc_fraction(Seq("ATCGATCGATCG")), 3)
print(f"Nested: {gc_nested}")

gc_piped = round(gc_fraction(Seq("ATCGATCGATCG")), 3)
print(f"Piped:  {gc_piped}")

# Central dogma
print("Central dogma:")
dna_seq = Seq("ATGAAACCCGGG")
rna_seq = dna_seq.transcribe()
protein = rna_seq.translate()
print(protein)

# Motif search — requires manual implementation for overlapping matches
import re
def find_motif(seq_str, motif):
    return [m.start() for m in re.finditer(f"(?={motif})", str(seq_str))]

positions = find_motif("ATGATGCCGATG", "ATG")
print(f"Start codon positions: {positions}")
print(f"Found {len(positions)} start codons")

# --- Lists and Records ---
print()
print("=== Lists and Records ===")

genes = ["BRCA1", "TP53", "EGFR", "KRAS"]
print(f"Genes: {genes}")
print(f"Count: {len(genes)}")
print(f"First: {genes[0]}")
print(f"Last:  {genes[-1]}")

gene = {
    "name": "TP53",
    "chromosome": "17",
    "length": 19149,
    "is_tumor_suppressor": True
}
print(f"Gene record: {gene['name']} on chr{gene['chromosome']}")

# --- Functions ---
print()
print("=== Functions ===")

def gc_rich(seq):
    return gc_fraction(seq) > 0.6

print(f"GCGCGCGCATGC is GC-rich: {gc_rich(Seq('GCGCGCGCATGC'))}")
print(f"AAAATTTT is GC-rich: {gc_rich(Seq('AAAATTTT'))}")

# Lambdas
double = lambda x: x * 2
print(f"double(5) = {double(5)}")

# --- Control Flow ---
print()
print("=== Control Flow ===")

gc_val = 0.65
if gc_val > 0.6:
    print("GC-rich region")
elif gc_val < 0.4:
    print("AT-rich region")
else:
    print("Balanced composition")

codons = ["ATG", "GCT", "TAA"]
for codon in codons:
    print(f"Codon: {codon}")

base = "A"
if base in ("A", "G"):
    print(f"{base} is a Purine")
elif base in ("C", "T"):
    print(f"{base} is a Pyrimidine")
else:
    print("Unknown base")

# --- Higher-Order Functions ---
print()
print("=== Higher-Order Functions ===")

sequences = [Seq("ATCG"), Seq("GCGCGC"), Seq("ATATAT")]

# map
gc_values = list(map(lambda s: gc_fraction(s), sequences))
print(f"GC values: {gc_values}")

# filter
gc_rich_seqs = list(filter(lambda s: gc_fraction(s) > 0.4, sequences))
print(f"GC-rich count: {len(gc_rich_seqs)}")

# each (Python uses for loop)
print("All genes:")
for g in ["BRCA1", "TP53", "EGFR"]:
    print(f"  Gene: {g}")

# reduce
total_length = reduce(lambda a, b: a + b, map(len, sequences))
print(f"Total bases: {total_length}")

# --- Putting It All Together ---
print()
print("=== Mini-Analysis ===")

fragments = [
    {"name": "exon1", "seq": Seq("ATGCGATCGATCG")},
    {"name": "exon2", "seq": Seq("GCGCGCATATAT")},
    {"name": "exon3", "seq": Seq("TTTTAAAACCCC")},
]

# Find GC-rich exons
gc_rich_exons = [f["name"] for f in fragments if gc_fraction(f["seq"]) > 0.5]
print(f"GC-rich exons: {gc_rich_exons}")

# Summary statistics
import statistics
frag_gc = [round(gc_fraction(f["seq"]), 3) for f in fragments]
print(f"GC contents: {frag_gc}")
print(f"Mean GC: {round(statistics.mean(frag_gc), 3)}")

# Classify each fragment
def classify_gc(gc_val):
    if gc_val > 0.6:
        return "GC-rich"
    elif gc_val < 0.4:
        return "AT-rich"
    else:
        return "balanced"

for f in fragments:
    gc_f = round(gc_fraction(f["seq"]), 3)
    print(f"{f['name']}: GC={gc_f} ({classify_gc(gc_f)})")

# --- Language Comparison Task ---
print()
print("=== BioLang vs Python vs R (same task) ===")
seqs = [Seq("ATCGATCG"), Seq("GCGCGCGC"), Seq("ATATATAT")]
for s in seqs:
    gc = gc_fraction(s)
    if gc > 0.5:
        print(f"{s}: {round(gc, 3)}")
