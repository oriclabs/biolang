import json
from Bio import SeqIO
from Bio.Seq import Seq

results = []
for i, record in enumerate(SeqIO.parse("real_data/yeast_genome.fa", "fasta")):
    if i >= 3:
        break
    sub = str(record.seq)[:99].upper()
    protein = str(Seq(sub).translate())
    results.append({"id": record.id, "dna": sub, "protein": protein})

print(json.dumps({"translations": results}))
