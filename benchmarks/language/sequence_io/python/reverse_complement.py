"""Reverse Complement: reverse complement every sequence in a FASTA"""
from Bio import SeqIO

records = list(SeqIO.parse("data/sequences.fa", "fasta"))
rc_lengths = [len(r.seq.reverse_complement()) for r in records]
print(f"Sequences: {len(records)}")
print(f"Total bp (reverse complemented): {sum(rc_lengths)}")
print(f"Mean length: {round(sum(rc_lengths) / len(rc_lengths), 1)}")
