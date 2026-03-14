"""Day 1: What Is Bioinformatics? — Python (Biopython) equivalent."""

from Bio.Seq import Seq
from Bio.SeqUtils import gc_fraction
import re
from collections import Counter

# 1. Sequence basics
seq = Seq("ATGCGATCGATCGATCG")
print(f"Sequence: {seq}")
print(f"Length: {len(seq)} bases")
print(f"Type: {type(seq).__name__}")

# 2. Central dogma
gene = Seq("ATGAAACCCGGGTTTTAA")
print(f"DNA:     {gene}")
mrna = gene.transcribe()
print(f"RNA:     {mrna}")
protein = gene.translate(to_stop=True)
print(f"Protein: {protein}")

# 3. Base composition
fragment = Seq("ATGCGATCGATCGAATTCGATCG")
counts = Counter(str(fragment))
print(f"Base composition: {dict(counts)}")
gc = gc_fraction(fragment)
print(f"GC content: {gc}")

# 4. Motif search (restriction site)
target = "ATCGATCGAATTCGATCGATCG"
motif = "GAATTC"
positions = [m.start() for m in re.finditer(f"(?={motif})", target)]
print(f"EcoRI sites: {positions}")

# 5. Complement and reverse complement
seq2 = Seq("ATGCGATCGATCG")
comp = seq2.complement()
revcomp = comp.reverse_complement()
rna = revcomp.transcribe()
print(f"Piped result: {rna}")

# 6. Exercises
print(f"Ex2: {Seq('ATGGATCCCTAA').translate(to_stop=True)}")
ex3 = Counter(str(Seq("AAAAATTTTTCCCCCGGGGG")))
print(f"Ex3: {dict(ex3)}")
ex4 = [m.start() for m in re.finditer("(?=ATG)", "ATGATGATGATG")]
print(f"Ex4: {ex4}")
