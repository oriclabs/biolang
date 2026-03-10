"""Benchmark 8: Real-World Protein Sequence Analysis
UniProt reviewed E. coli K-12 proteome."""
from Bio import SeqIO
from statistics import mean, median

records = list(SeqIO.parse("data_real/ecoli_proteome.fa", "fasta"))
lengths = [len(r.seq) for r in records]
total_residues = sum(lengths)

# N50
sorted_desc = sorted(lengths, reverse=True)
cumsum = 0
n50 = 0
for l in sorted_desc:
    cumsum += l
    if cumsum >= total_residues / 2:
        n50 = l
        break

print(f"Proteins: {len(records)}")
print(f"Total residues: {total_residues}")
print(f"Mean length: {mean(lengths):.1f}")
print(f"Min length: {min(lengths)}")
print(f"Max length: {max(lengths)}")
print(f"N50: {n50}")

# Length distribution
short = sum(1 for l in lengths if l < 200)
medium_count = sum(1 for l in lengths if 200 <= l < 500)
long_count = sum(1 for l in lengths if l >= 500)

print(f"Short (<200 aa): {short}")
print(f"Medium (200-499 aa): {medium_count}")
print(f"Long (>=500 aa): {long_count}")
