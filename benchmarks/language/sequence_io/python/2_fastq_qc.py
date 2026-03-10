"""Benchmark 2: FASTQ Quality Control"""
from Bio import SeqIO
from statistics import mean, median

records = list(SeqIO.parse("data/reads.fq", "fastq"))
total = len(records)
lengths = [len(r.seq) for r in records]
qualities = [mean(r.letter_annotations["phred_quality"]) for r in records]
q30_count = sum(1 for q in qualities if q >= 30.0)

print(f"Total reads: {total}")
print(f"Q30 rate: {q30_count / total * 100.0:.3f}%")
print(f"Mean length: {mean(lengths):.1f}")
print(f"Min length: {min(lengths)}")
print(f"Max length: {max(lengths)}")
print(f"Mean quality: {mean(qualities):.2f}")
print(f"Median quality: {median(qualities):.2f}")
