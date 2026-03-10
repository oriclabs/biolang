"""File I/O: Read FASTA, filter by length, write filtered output"""
from Bio import SeqIO

records = list(SeqIO.parse("data/sequences.fa", "fasta"))
filtered = [r for r in records if len(r.seq) >= 2000]
count = SeqIO.write(filtered, "data/filtered_output.fa", "fasta")
print(f"Input records: {len(records)}")
print(f"Filtered records: {len(filtered)}")
print(f"Written: {count}")
