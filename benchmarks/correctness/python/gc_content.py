"""Compute GC content per sequence from a FASTA file. Output JSON."""
import json
import sys
from Bio import SeqIO

results = {}
for record in SeqIO.parse("data/contigs.fa", "fasta"):
    seq = str(record.seq).upper()
    gc = (seq.count("G") + seq.count("C")) / len(seq) if len(seq) > 0 else 0.0
    results[record.id] = round(gc, 6)

json.dump({"gc_per_sequence": results, "n_sequences": len(results)}, sys.stdout, indent=2)
print()
