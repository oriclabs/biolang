"""Reverse complement all sequences from FASTA. Output JSON."""
import json
import sys
from Bio import SeqIO
from Bio.Seq import Seq

results = {}
for record in SeqIO.parse("data/contigs.fa", "fasta"):
    rc = str(Seq(str(record.seq)).reverse_complement())
    results[record.id] = rc

json.dump({"sequences": results}, sys.stdout, indent=2)
print()
