"""File I/O: Parse large gzipped FASTA (Human chr22, ~10 MB compressed)"""
import gzip
from Bio import SeqIO

with gzip.open("data_real/human_chr22.fa.gz", "rt") as f:
    records = list(SeqIO.parse(f, "fasta"))
total_bp = sum(len(r.seq) for r in records)
print(f"Records: {len(records)}")
print(f"Total bp: {total_bp}")
