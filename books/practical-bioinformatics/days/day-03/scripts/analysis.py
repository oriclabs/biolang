#!/usr/bin/env python3
"""Day 3: Biology Crash Course for Developers — Python equivalent.

Uses Biopython for sequence operations.
Install: pip install biopython
"""

from Bio.Seq import Seq
from Bio.Data.CodonTable import standard_dna_table
from collections import Counter

print("=" * 60)
print("Day 3: Biology Crash Course for Developers")
print("=" * 60)

# ── Section 1: DNA — The Source Code ─────────────────────────

print()
print("--- DNA: The Source Code ---")

coding = Seq("ATGCGATCG")
comp = coding.complement()
rc = coding.reverse_complement()
print(f"Coding:     5'-{coding}-3'")
print(f"Complement: 3'-{comp}-5'")
print(f"RevComp:    5'-{rc}-3'")

# ── Section 2: The Central Dogma ─────────────────────────────

print()
print("--- The Central Dogma: DNA -> RNA -> Protein ---")

seq = Seq("ATGGCTAACTGA")
rna = seq.transcribe()
protein = seq.translate(to_stop=True)
print(f"DNA:     {seq}")
print(f"RNA:     {rna}")
print(f"Protein: {protein}")
print("M = Methionine (start), A = Alanine, N = Asparagine")

# ── Section 3: Codon Usage ───────────────────────────────────

print()
print("--- Codon Usage ---")

gene = "ATGGCTGCTTCTGATTGA"
codons = [gene[i:i+3] for i in range(0, len(gene), 3)]
usage = Counter(codons)
print(f"Sequence: {gene}")
print(f"Usage:    {dict(usage)}")

# ── Section 4: Mutations ────────────────────────────────────

print()
print("--- Mutations: Bugs in the Code ---")

normal = Seq("ATGGCTAACTGA")
mutant = Seq("ATGGCTGACTGA")  # A->G at position 7

normal_protein = normal.translate(to_stop=True)
mutant_protein = mutant.translate(to_stop=True)

print(f"Normal DNA:     {normal}")
print(f"Mutant DNA:     {mutant}")
print(f"Normal protein: {normal_protein}")
print(f"Mutant protein: {mutant_protein}")
print(f"Changed:        {normal_protein != mutant_protein}")
print("One base change (A->G) changed Asparagine (N) to Aspartate (D)")

# ── Section 5: Wobble Position Experiment ────────────────────

print()
print("--- Wobble Position Experiment ---")
print("Mutating each position of codon GCT (Alanine):")

original = Seq("ATGGCTTGA")
mut_pos1 = Seq("ATGTCTTGA")  # G->T at codon position 1
mut_pos2 = Seq("ATGGATTGA")  # C->A at codon position 2
mut_pos3 = Seq("ATGGCATGA")  # T->A at codon position 3

print(f"Original (GCT): {original.translate(to_stop=True)}")
print(f"Pos1 mut (TCT): {mut_pos1.translate(to_stop=True)}")
print(f"Pos2 mut (GAT): {mut_pos2.translate(to_stop=True)}")
print(f"Pos3 mut (GCA): {mut_pos3.translate(to_stop=True)}")
print("Position 3 (wobble) is most tolerant — GCA still encodes Alanine")

# ── Section 6: Genomic Intervals ────────────────────────────

print()
print("--- Genomic Intervals ---")

# Python has no built-in interval type; we use tuples or dataclasses
brca1 = ("chr17", 43044295, 43125483)
tp53 = ("chr17", 7668402, 7687550)

print(f"BRCA1: {brca1[0]}:{brca1[1]}-{brca1[2]}")
print(f"TP53:  {tp53[0]}:{tp53[1]}-{tp53[2]}")
print(f"Same chromosome: {brca1[0] == tp53[0]}")

egfr = ("chr7", 55019017, 55211628)
braf = ("chr7", 140719327, 140924929)

same_chrom = egfr[0] == braf[0]
overlaps = same_chrom and egfr[1] < braf[2] and braf[1] < egfr[2]

print(f"EGFR: {egfr[0]}:{egfr[1]}-{egfr[2]}")
print(f"BRAF: {braf[0]}:{braf[1]}-{braf[2]}")
print(f"Same chromosome: {same_chrom}")
print(f"Overlap: {overlaps}")

# ── Section 7: TP53 — The Guardian ──────────────────────────

print()
print("--- TP53: The Guardian of the Genome ---")

normal_tp53 = Seq("ATGGAGGAGCCGCAGTCAGATCCTAGC")
tp53_protein = normal_tp53.translate(to_stop=True)
gc = (normal_tp53.count("G") + normal_tp53.count("C")) / len(normal_tp53)
print(f"TP53 coding start: {normal_tp53}")
print(f"Protein begins:    {tp53_protein}")
print(f"GC content:        {gc}")
print("TP53 is mutated in >50% of all human cancers")

# ── Section 8: Exercise 1 — Hand Translation ────────────────

print()
print("--- Exercise 1: Hand Translation ---")

ex1 = Seq("ATGAAAGCTTGA")
print(f"Sequence: {ex1}")
print(f"Codons:   ATG | AAA | GCT | TGA")
print(f"Expected: M     K     A    Stop")
print(f"Result:   {ex1.translate(to_stop=True)}")

print()
print("=" * 60)
print("Day 3 complete!")
print("=" * 60)
