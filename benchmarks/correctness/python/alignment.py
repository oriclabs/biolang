"""Global alignment of sequence pairs. Output JSON."""
import json
import sys
from Bio import SeqIO
from Bio import Align

records = []
for i, rec in enumerate(SeqIO.parse("data/sequences.fa", "fasta")):
    if i >= 4:
        break
    records.append((rec.id, str(rec.seq)[:120]))

aligner = Align.PairwiseAligner()
aligner.mode = "global"
aligner.match_score = 1
aligner.mismatch_score = -1
# BioLang applies a flat -2 per gap position; a linear gap penalty is the same
# thing expressed as open = extend = -2.
aligner.open_gap_score = -2
aligner.extend_gap_score = -2

results = []
for i in range(len(records) - 1):
    score = aligner.score(records[i][1], records[i + 1][1])
    results.append({"a": records[i][0], "b": records[i + 1][0], "score": int(score)})

json.dump({"alignments": results}, sys.stdout, indent=2)
print()
