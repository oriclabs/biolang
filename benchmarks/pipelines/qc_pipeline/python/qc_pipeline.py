"""QC Pipeline: Read -> Filter -> Stats -> Report
Multi-stage pipeline equivalent in Python."""
from Bio import SeqIO
import statistics

# Stage 1: Read FASTQ
records = list(SeqIO.parse("data/reads.fq", "fastq"))

# Stage 2: Filter by quality
filtered = [r for r in records if statistics.mean(r.letter_annotations["phred_quality"]) >= 25]

# Stage 3: Compute stats
n = len(filtered)
quals = [statistics.mean(r.letter_annotations["phred_quality"]) for r in filtered]
lengths = [len(r.seq) for r in filtered]

# Stage 4: Report
print("QC Pipeline Results:")
print(f"  Reads passing filter: {n}")
print(f"  Mean quality: {round(statistics.mean(quals), 1)}")
print(f"  Mean length: {round(statistics.mean(lengths), 1)}")
print(f"  Min length: {min(lengths)}")
print(f"  Max length: {max(lengths)}")
