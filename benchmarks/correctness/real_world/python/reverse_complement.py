import json
from Bio import SeqIO
from Bio.Seq import Seq

results = []
for i, record in enumerate(SeqIO.parse("real_data/yeast_genome.fa", "fasta")):
    if i >= 5:
        break
    sub = str(record.seq)[:200].upper()
    rc = str(Seq(sub).reverse_complement())
    results.append({"id": record.id, "original": sub, "revcomp": rc})

print(json.dumps({"sequences": results}))
