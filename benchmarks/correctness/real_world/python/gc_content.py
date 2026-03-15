import json
from Bio import SeqIO
from Bio.SeqUtils import gc_fraction

result = {}
for record in SeqIO.parse("real_data/yeast_genome.fa", "fasta"):
    result[record.id] = gc_fraction(record.seq)
print(json.dumps({"gc_per_sequence": result}))
