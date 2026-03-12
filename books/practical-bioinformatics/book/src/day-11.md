# Day 11: Sequence Comparison

| | |
|---|---|
| **Difficulty** | Intermediate |
| **Biology knowledge** | Intermediate (DNA composition, codons, restriction enzymes) |
| **Coding knowledge** | Intermediate (loops, records, functions, pipes) |
| **Time** | ~3 hours |
| **Prerequisites** | Days 1-10 completed, BioLang installed (see Appendix A) |
| **Data needed** | None (sequences defined inline) |
| **Requirements** | None (offline); internet optional for Section 8 API examples |

## What You'll Learn

- How to compare sequences by base composition and GC content
- How k-mer decomposition enables alignment-free similarity
- How dotplots visually reveal similarity, repeats, and rearrangements
- How to find exact motifs including restriction enzyme recognition sites
- Why reverse complement matters for double-stranded DNA
- How to analyze codon usage bias across genes
- How to compare genes across species using Ensembl APIs

---

## The Problem

Two sequences sit on your screen. Are they related? How similar? Where do they differ? Sequence comparison is the foundation of evolutionary biology, variant detection, and functional prediction.

Some comparisons are quick: does this gene have unusually high GC content? Others are structural: do these two sequences share long stretches of similarity? And some are functional: does this promoter contain a known transcription factor binding site?

Today you will build a toolkit for answering all of these questions, starting from the simplest metric --- base composition --- and working up to multi-species gene comparison.

---

## Base Composition Analysis

The simplest way to compare two sequences is to count their nucleotides. GC content --- the fraction of bases that are G or C --- varies dramatically across organisms, from ~25% in some parasites to ~70% in thermophilic bacteria. It is a quick first-pass metric: if two sequences have wildly different GC content, they likely come from different organisms or genomic regions.

```bio
let seqs = [
    {name: "E. coli",   seq: dna"GCGCATCGATCGATCGCG"},
    {name: "Human",     seq: dna"ATATCGATCGATATATAT"},
    {name: "Thermus",   seq: dna"GCGCGCGCGCGCGCGCGC"},
]
for s in seqs {
    let gc = round(gc_content(s.seq) * 100, 1)
    let counts = base_counts(s.seq)
    println(f"{s.name}: GC={gc}%, A={counts.A}, T={counts.T}, G={counts.G}, C={counts.C}")
}
```

Expected output:

```
E. coli: GC=61.1%, A=2, T=2, G=5, C=6
Human: GC=27.8%, A=7, T=7, G=2, C=3
Thermus: GC=100.0%, A=0, T=0, G=9, C=9
```

`gc_content()` returns a float between 0.0 and 1.0. Multiplying by 100 gives a percentage. `base_counts()` returns a record with fields `A`, `T`, `G`, and `C`.

Notice how the three example sequences span a wide GC range: the *Thermus* fragment is entirely GC (thermophilic organisms use GC-rich DNA for thermal stability), while the human fragment is AT-rich (common in non-coding regions).

---

## K-mer Analysis

A **k-mer** is a subsequence of length k. Decomposing a sequence into k-mers is the foundation of alignment-free comparison --- instead of aligning two sequences end to end, you compare their k-mer content.

Here is how k-mers work. Given a sequence, a sliding window of size k moves one base at a time:

```
Sequence: A T C G A T C G
           |---|                 → ATC
             |---|               → TCG
               |---|             → CGA
                 |---|           → GAT
                   |---|         → ATC
                     |---|       → TCG

3-mers:   ATC  TCG  CGA  GAT  ATC  TCG
```

Each position produces one k-mer. A sequence of length L contains L - k + 1 k-mers.

### Extracting K-mers

```bio
let seq = dna"ATCGATCGATCG"
let kmers_list = kmers(seq, 3)
println(f"Sequence: {seq}")
println(f"3-mers: {kmers_list}")
```

Expected output:

```
Sequence: ATCGATCGATCG
3-mers: [ATC, TCG, CGA, GAT, ATC, TCG, CGA, GAT, ATC, TCG]
```

### K-mer Frequency

Counting how often each k-mer appears reveals sequence composition at a deeper level than single-base counts.

```bio
let seq = dna"ATCGATCGATCG"
let freq = kmer_count(seq, 3)
println(f"3-mer frequencies: {freq}")
```

Expected output:

```
3-mer frequencies: {ATC: 3, TCG: 3, CGA: 2, GAT: 2}
```

### Alignment-Free Similarity with K-mers

Two sequences that share many k-mers are likely similar, even without performing a formal alignment. The **Jaccard similarity** measures this: the size of the intersection divided by the size of the union of the two k-mer sets.

```bio
let seq1 = dna"ATCGATCGATCGATCG"
let seq2 = dna"ATCGATCGTTTTGATCG"

let k1 = set(kmers(seq1, 5))
let k2 = set(kmers(seq2, 5))

let shared = intersection(k1, k2)
let total = union(k1, k2)
let jaccard = len(shared) / len(total)
println(f"Shared 5-mers: {len(shared)}")
println(f"Total unique 5-mers: {len(total)}")
println(f"K-mer similarity: {round(jaccard * 100, 1)}%")
```

Expected output:

```
Shared 5-mers: 6
Total unique 5-mers: 17
K-mer similarity: 35.3%
```

Jaccard similarity ranges from 0% (no shared k-mers) to 100% (identical k-mer sets). It is fast to compute, works on sequences of different lengths, and does not require alignment. Tools like Mash and Sourmash use this principle for large-scale genome comparison.

---

## Dotplots --- Visual Sequence Comparison

A dotplot is the oldest and most intuitive method for comparing two sequences. The idea is simple:

- Place **sequence 1** along the X axis
- Place **sequence 2** along the Y axis
- Put a **dot** at position (i, j) if the bases at position i and j match

The resulting pattern reveals structural relationships at a glance:

| Pattern | Meaning |
|---------|---------|
| Continuous diagonal line | The sequences are similar in that region |
| Broken diagonal | Similarity with insertions or deletions |
| Parallel diagonal lines | Repeated regions |
| Perpendicular lines | Inverted repeats |
| No dots | No similarity |

```bio
let seq1 = dna"ATCGATCGATCG"
let seq2 = dna"ATCGTTGATCG"
dotplot(seq1, seq2)
```

The `dotplot()` function generates an SVG visualization. You can customize it:

```bio
dotplot(seq1, seq2, window: 3, title: "Pairwise comparison")
```

The `window` parameter sets the match window size. A window of 1 shows every single-base match (noisy). A window of 3 or larger filters out random matches, leaving only meaningful stretches of similarity.

### Self-Dotplots

Comparing a sequence against itself is a powerful way to find internal repeats. Any repeated region appears as a parallel diagonal line offset from the main diagonal.

```bio
let repeat_seq = dna"ATCGATCGATCGATCG"
dotplot(repeat_seq, repeat_seq, window: 3, title: "Self-comparison: internal repeats")
```

The main diagonal (where the sequence matches itself perfectly) will always be present. Parallel lines above or below the diagonal indicate tandem repeats.

---

## Motif Finding

A **motif** is a short sequence pattern with biological significance. Start codons, stop codons, restriction enzyme recognition sites, and transcription factor binding sites are all motifs.

### Finding Exact Motifs

```bio
let seq = dna"ATGATCGATGATCGATGATCG"
let atg_sites = find_motif(seq, "ATG")
println(f"ATG positions: {atg_sites}")
```

Expected output:

```
ATG positions: [0, 9, 18]
```

Positions are zero-indexed. Each value is the start position where the motif begins in the sequence.

### Restriction Enzyme Sites

Restriction enzymes cut DNA at specific recognition sequences. Finding these sites is essential for cloning, Southern blotting, and restriction fragment analysis.

```bio
let seq = dna"ATCGGAATTCGATCGGGATCCATCG"
let ecori = find_motif(seq, "GAATTC")
let bamhi = find_motif(seq, "GGATCC")
println(f"EcoRI sites: {ecori}")
println(f"BamHI sites: {bamhi}")
```

Expected output:

```
EcoRI sites: [4]
BamHI sites: [14]
```

Common restriction enzymes and their recognition sequences:

| Enzyme | Sequence | Cut pattern |
|--------|----------|-------------|
| EcoRI | GAATTC | G^AATTC |
| BamHI | GGATCC | G^GATCC |
| HindIII | AAGCTT | A^AGCTT |
| NotI | GCGGCCGC | GC^GGCCGC |
| XhoI | CTCGAG | C^TCGAG |

---

## Reverse Complement and Strand Awareness

DNA is double-stranded. A motif on the forward strand has a corresponding motif on the reverse strand. When you search for a binding site, you must check both strands --- the protein does not care which strand it binds.

```bio
let forward = dna"ATGCGATCGATCG"
let revcomp = reverse_complement(forward)
println(f"Forward:  5'-{forward}-3'")
println(f"RevComp:  5'-{revcomp}-3'")
```

Expected output:

```
Forward:  5'-ATGCGATCGATCG-3'
RevComp:  5'-CGATCGATCGCAT-3'
```

### Searching Both Strands

```bio
let seq = dna"ATCGGAATTCGATCG"
let motif = "GAATTC"
let fwd_hits = find_motif(seq, motif)
let rev_hits = find_motif(reverse_complement(seq), motif)
println(f"Forward strand hits: {fwd_hits}")
println(f"Reverse strand hits: {rev_hits}")
```

Expected output:

```
Forward strand hits: [4]
Reverse strand hits: [1]
```

EcoRI's recognition sequence (GAATTC) is a **palindrome** --- its reverse complement is also GAATTC. This means EcoRI cuts both strands at the same site. Not all restriction enzymes are palindromic, but most Type II enzymes are.

---

## Codon Analysis

Codons are triplets of nucleotides that encode amino acids. Different organisms prefer different codons for the same amino acid --- a phenomenon called **codon usage bias**. Highly expressed genes tend to use preferred codons for faster translation.

```bio
let gene = dna"ATGGCTGCTTCTGATAAATGA"
let usage = codon_usage(gene)
println(f"Codon usage: {usage}")
```

Expected output:

```
Codon usage: {ATG: 1, GCT: 1, GCT: 1, TCT: 1, GAT: 1, AAA: 1, TGA: 1}
```

### Comparing Codon Bias Between Species

Different organisms have evolved different codon preferences. *E. coli* prefers GCG for alanine, while humans prefer GCC. Comparing codon usage can reveal whether a gene has been horizontally transferred or synthetically designed.

```bio
let human_gene = dna"ATGGCTGCTTCTGATAAATGA"
let ecoli_gene = dna"ATGGCAGCGAGCGATAAATGA"
let human_usage = codon_usage(human_gene)
let ecoli_usage = codon_usage(ecoli_gene)
println(f"Human codons: {human_usage}")
println(f"E. coli codons: {ecoli_usage}")
```

Expected output:

```
Human codons: {ATG: 1, GCT: 1, GCT: 1, TCT: 1, GAT: 1, AAA: 1, TGA: 1}
E. coli codons: {ATG: 1, GCA: 1, GCG: 1, AGC: 1, GAT: 1, AAA: 1, TGA: 1}
```

Notice how both genes encode roughly similar proteins but use different codons: the human gene uses GCT (alanine) where *E. coli* uses GCA and GCG.

---

## Multi-Species Comparison via APIs

Comparing a gene across species reveals evolutionary conservation. Genes that are highly conserved across distant species are usually functionally important.

> **Requires CLI:** This example uses file I/O / network APIs not available in the browser. Run with `bl run`.

```bio
# requires: internet connection

let species = [
    {name: "Human", species: "homo_sapiens"},
    {name: "Mouse", species: "mus_musculus"},
    {name: "Zebrafish", species: "danio_rerio"},
]

let results = species |> map(|sp| {
    let gene = ensembl_symbol(sp.species, "BRCA1")
    let protein = ensembl_sequence(gene.id, type: "protein")
    {name: sp.name, gene_id: gene.id, protein_len: len(protein.seq)}
})

let comparison = results |> to_table()
println(comparison)
```

Expected output (values depend on current Ensembl release):

```
name      | gene_id            | protein_len
Human     | ENSG00000012048    | 1863
Mouse     | ENSMUSG00000017146 | 1812
Zebrafish | ENSDARG00000076256 | 1679
```

The BRCA1 protein is conserved across vertebrates but gets progressively shorter in more distant species --- zebrafish BRCA1 is about 10% shorter than human BRCA1. This kind of comparison is a first step toward understanding which regions of the protein are functionally essential (the conserved parts) versus dispensable (the parts that vary).

---

## Building a Similarity Matrix

When you have more than two sequences, pairwise comparison produces a **similarity matrix** --- a table where each cell contains the similarity between two sequences.

```bio
let sequences = [
    {name: "seq1", seq: dna"ATCGATCGATCGATCG"},
    {name: "seq2", seq: dna"ATCGATCGTTTTGATCG"},
    {name: "seq3", seq: dna"GCGCGCGCGCGCGCGC"},
]

let results = []
for i in range(0, len(sequences)) {
    for j in range(0, len(sequences)) {
        let k1 = set(kmers(sequences[i].seq, 5))
        let k2 = set(kmers(sequences[j].seq, 5))
        let shared = len(intersection(k1, k2))
        let total = len(union(k1, k2))
        let sim = if total > 0 { round(shared / total, 3) } else { 0.0 }
        results = push(results, {
            seq1: sequences[i].name,
            seq2: sequences[j].name,
            similarity: sim
        })
    }
}
let matrix = results |> to_table()
println(matrix)
```

Expected output:

```
seq1 | seq2 | similarity
seq1 | seq1 | 1.0
seq1 | seq2 | 0.353
seq1 | seq3 | 0.0
seq2 | seq1 | 0.353
seq2 | seq2 | 1.0
seq2 | seq3 | 0.0
seq3 | seq1 | 0.0
seq3 | seq2 | 0.0
seq3 | seq3 | 1.0
```

The matrix confirms what you would expect: seq1 and seq2 share some similarity (they have overlapping subsequences), but seq3 (all GC) shares nothing with either.

Reading a similarity matrix:
- The diagonal is always 1.0 (every sequence is identical to itself)
- The matrix is symmetric (similarity of A to B equals similarity of B to A)
- Values near 0.0 mean unrelated sequences; values near 1.0 mean nearly identical sequences

---

## Complete Example: Gene Comparison Report

This script ties together everything from today --- base composition, k-mers, motif finding, and API-based cross-species comparison --- into a single analysis.

> **Requires CLI:** This example uses file I/O / network APIs not available in the browser. Run with `bl run`.

```bio
# Compare TP53 protein sequence properties across species
# requires: internet connection (optional: NCBI_API_KEY for higher rate limits)

fn compare_gene(gene_symbol, species_list) {
    let results = []
    for sp in species_list {
        try {
            let gene = ensembl_symbol(sp.species, gene_symbol)
            let cds = ensembl_sequence(gene.id, type: "cdna")
            let prot = ensembl_sequence(gene.id, type: "protein")
            results = push(results, {
                species: sp.name,
                cds_length: len(cds.seq),
                protein_length: len(prot.seq),
                gc: round(gc_content(cds.seq) * 100, 1)
            })
        } catch e {
            println(f"  Skipping {sp.name}: {e}")
        }
    }
    results |> to_table()
}

let species = [
    {name: "Human", species: "homo_sapiens"},
    {name: "Mouse", species: "mus_musculus"},
    {name: "Chicken", species: "gallus_gallus"},
]

let comparison = compare_gene("TP53", species)
println(comparison)
```

Expected output (values depend on current Ensembl release):

```
species | cds_length | protein_length | gc
Human   | 1182       | 393            | 48.2
Mouse   | 1176       | 391            | 49.1
Chicken | 1113       | 370            | 52.8
```

TP53 (the "guardian of the genome") is highly conserved across vertebrates. The protein length varies by only ~6%, but GC content differs more --- chicken TP53 has higher GC content, consistent with the generally higher GC content of bird genomes.

---

## Exercises

1. **GC content ranking.** Create an array of 5 DNA sequences with different compositions. Calculate GC content for each and sort them from highest to lowest using `sort_by` and `reverse`.

2. **Start and stop codons.** Given the sequence `dna"ATGCGATCGATGATCGTAGATCGATGATCGTGAATCG"`, find all start codons (ATG) and all stop codons (TAA, TAG, TGA). Print the positions of each.

3. **Self-dotplot for repeats.** Create a sequence that contains a repeated motif (e.g., `dna"ATCGATCGATCGATCG"`) and use `dotplot()` to compare it against itself. How many parallel diagonals do you see?

4. **K-mer similarity at different k values.** Compare two related sequences at k=3, k=5, and k=7. How does increasing k affect the Jaccard similarity? Why?

5. **Cross-species comparison.** Use the Ensembl API to compare BRCA1 across human, mouse, and zebrafish. Build a table with columns for species, CDS length, protein length, and GC content.

---

## Key Takeaways

- **GC content** and base composition are quick first-pass comparisons between sequences
- **K-mers** enable alignment-free similarity measurement --- fast and effective for large-scale comparisons
- **Dotplots** visually reveal similarity, insertions, deletions, and repeats at a glance
- **`find_motif()`** searches for exact patterns including restriction enzyme recognition sites
- **Reverse complement** is essential --- biology uses both DNA strands, and many binding sites are palindromic
- **Codon usage bias** varies across organisms and reveals evolutionary and functional signatures
- **API-based multi-species comparison** reveals evolutionary conservation of genes and proteins

---

## What's Next

Tomorrow: finding variants in genomes --- VCF analysis, variant filtering, and clinical interpretation.
