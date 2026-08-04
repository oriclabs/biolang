"""Levenshtein distance between the first five sequences, pairwise. Output JSON."""
import json
import sys
from Bio import SeqIO

def levenshtein(a, b):
    # Straightforward DP. BioLang uses Myers' bit-parallel algorithm, so this
    # is a genuinely different implementation of the same definition rather
    # than the same code twice.
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        cur = [i]
        for j, cb in enumerate(b, 1):
            cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (ca != cb)))
        prev = cur
    return prev[-1]

records = []
for i, rec in enumerate(SeqIO.parse("data/sequences.fa", "fasta")):
    if i >= 5:
        break
    records.append((rec.id, str(rec.seq)[:300]))

pairs = []
for i in range(len(records)):
    for j in range(i + 1, len(records)):
        pairs.append({"a": records[i][0], "b": records[j][0],
                      "distance": levenshtein(records[i][1], records[j][1])})

json.dump({"pairs": pairs}, sys.stdout, indent=2)
print()
