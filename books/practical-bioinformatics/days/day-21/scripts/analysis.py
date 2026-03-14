#!/usr/bin/env python3
"""Day 21: Performance and Parallel Processing — Python equivalent."""

import time
import os
from concurrent.futures import ProcessPoolExecutor, as_completed
from multiprocessing import Pool, cpu_count
from Bio import SeqIO
from Bio.SeqUtils import gc_fraction


def gc_content(seq):
    seq = str(seq).upper()
    if len(seq) == 0:
        return 0.0
    gc = sum(1 for c in seq if c in "GC")
    return gc / len(seq)


def count_kmers(args):
    seq, k = args
    seq = str(seq)
    if len(seq) < k:
        return 0
    return len([seq[i:i+k] for i in range(len(seq) - k + 1)])


def analyze_read_gc(seq):
    return gc_content(seq)


def main():
    print("=" * 60)
    print("Day 21: Performance Benchmark (Python)")
    print("=" * 60)

    # --- Load FASTQ ---
    print("\nLoading FASTQ reads...")
    reads = list(SeqIO.parse("data/sample.fastq", "fastq"))
    seqs = [str(r.seq) for r in reads]
    print(f"Loaded {len(reads)} reads")

    # --- Serial GC analysis ---
    print("\n-- Serial GC Analysis --")
    t0 = time.time()
    gc_values = [gc_content(s) for s in seqs]
    avg_gc = sum(gc_values) / len(gc_values)
    high_gc = sum(1 for g in gc_values if g > 0.5)
    lengths = [len(s) for s in seqs]
    mean_len = sum(lengths) / len(lengths)
    min_len = min(lengths)
    max_len = max(lengths)
    serial_time = time.time() - t0

    print(f"  Mean GC:    {avg_gc * 100:.1f}%")
    print(f"  High GC:    {high_gc} reads")
    print(f"  Mean len:   {mean_len:.0f}")
    print(f"  Length range: {min_len}-{max_len}")
    print(f"  Time:       {serial_time:.3f}s")

    # --- Parallel GC analysis (ProcessPoolExecutor) ---
    print("\n-- Parallel GC Analysis --")
    t0 = time.time()
    with ProcessPoolExecutor(max_workers=cpu_count()) as pool:
        gc_values_par = list(pool.map(analyze_read_gc, seqs, chunksize=1000))
    avg_gc_par = sum(gc_values_par) / len(gc_values_par)
    high_gc_par = sum(1 for g in gc_values_par if g > 0.5)
    lengths_par = [len(s) for s in seqs]
    mean_len_par = sum(lengths_par) / len(lengths_par)
    min_len_par = min(lengths_par)
    max_len_par = max(lengths_par)
    parallel_time = time.time() - t0

    print(f"  Mean GC:    {avg_gc_par * 100:.1f}%")
    print(f"  High GC:    {high_gc_par} reads")
    print(f"  Mean len:   {mean_len_par:.0f}")
    print(f"  Length range: {min_len_par}-{max_len_par}")
    print(f"  Time:       {parallel_time:.3f}s")
    if parallel_time > 0:
        print(f"  Speedup:    {serial_time / parallel_time:.1f}x")

    # --- Streaming GC analysis ---
    print("\n-- Streaming GC Analysis --")
    t0 = time.time()
    stream_gc_sum = 0.0
    stream_len_sum = 0
    stream_high = 0
    stream_count = 0
    stream_min = float("inf")
    stream_max = 0

    for record in SeqIO.parse("data/sample.fastq", "fastq"):
        seq = str(record.seq)
        gc = gc_content(seq)
        l = len(seq)
        stream_gc_sum += gc
        stream_len_sum += l
        if gc > 0.5:
            stream_high += 1
        if l < stream_min:
            stream_min = l
        if l > stream_max:
            stream_max = l
        stream_count += 1

    stream_time = time.time() - t0

    print(f"  Mean GC:    {stream_gc_sum / stream_count * 100:.1f}%")
    print(f"  High GC:    {stream_high} reads")
    print(f"  Mean len:   {stream_len_sum / stream_count:.0f}")
    print(f"  Length range: {stream_min}-{stream_max}")
    print(f"  Time:       {stream_time:.3f}s")
    print(f"  Memory:     constant (streaming)")

    # --- K-mer counting on FASTA ---
    print("\n-- K-mer Counting (FASTA, k=6) --")
    fasta_seqs = [str(r.seq) for r in SeqIO.parse("data/sequences.fasta", "fasta")]
    print(f"Loaded {len(fasta_seqs)} sequences")

    print("\nSerial k-mer counting:")
    t0 = time.time()
    kmer_counts_serial = [count_kmers((s, 6)) for s in fasta_seqs]
    kmer_serial_time = time.time() - t0
    print(f"  Time: {kmer_serial_time:.3f}s")
    print(f"  Total k-mers: {sum(kmer_counts_serial)}")

    print("\nParallel k-mer counting:")
    t0 = time.time()
    with ProcessPoolExecutor(max_workers=cpu_count()) as pool:
        kmer_counts_par = list(pool.map(count_kmers, [(s, 6) for s in fasta_seqs], chunksize=500))
    kmer_par_time = time.time() - t0
    print(f"  Time: {kmer_par_time:.3f}s")
    print(f"  Total k-mers: {sum(kmer_counts_par)}")
    if kmer_par_time > 0:
        print(f"  Speedup: {kmer_serial_time / kmer_par_time:.1f}x")

    # --- Summary ---
    print("\n" + "=" * 60)
    print("Summary")
    print("=" * 60)
    print(f"{'Approach':<15} {'GC Time':>10} {'K-mer Time':>12}")
    print("-" * 40)
    print(f"{'Serial':<15} {serial_time:>9.3f}s {kmer_serial_time:>11.3f}s")
    print(f"{'Parallel':<15} {parallel_time:>9.3f}s {kmer_par_time:>11.3f}s")
    print(f"{'Streaming':<15} {stream_time:>9.3f}s {'N/A':>12}")


if __name__ == "__main__":
    main()
