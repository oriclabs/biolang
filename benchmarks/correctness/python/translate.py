"""Translate first 3 FASTA sequences (trimmed to multiple of 3). Output JSON."""
import json
import sys
from Bio import SeqIO
from Bio.Seq import Seq

results = {}
for i, record in enumerate(SeqIO.parse("data/sequences.fa", "fasta")):
    if i >= 3:
        break
    seq = str(record.seq).upper()
    # Trim to multiple of 3
    trim_len = (len(seq) // 3) * 3
    seq = seq[:trim_len]
    # to_stop matches BioLang's translate(), which ends at the first stop
    # codon rather than emitting '*' and continuing.
    protein = str(Seq(seq).translate(to_stop=True))
    results[record.id] = protein

json.dump({"translations": results}, sys.stdout, indent=2)
print()
