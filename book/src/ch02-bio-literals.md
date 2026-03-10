# Chapter 2: Bio Literals

BioLang has first-class types for biological sequences. These are not strings -- they
carry domain semantics, validate their contents, and support biological operations
directly. This chapter covers DNA, RNA, Protein, and Quality literals, along with
the operations that make them powerful.

## DNA Literals

A DNA literal is created with the `dna` prefix:

```
let seq = dna"ATCGATCGATCG"
```

DNA literals accept only valid IUPAC nucleotide codes: `A`, `T`, `C`, `G`, and
ambiguity codes `N`, `R`, `Y`, `S`, `W`, `K`, `M`, `B`, `D`, `H`, `V`.

```
# Ambiguous positions are valid
let probe = dna"ATCNNGATCG"

# Case is normalized to uppercase
let lower = dna"atcgatcg"
print(lower)   # => dna"ATCGATCG"
```

Invalid bases cause a compile-time error:

```
# This is a compile error -- U is not a DNA base
let bad = dna"AUCG"
# Error: invalid DNA base 'U' at position 1
```

## RNA Literals

RNA uses the `rna` prefix and contains `A`, `U`, `C`, `G`:

```
let mrna = rna"AUGCUUAAGGCUAG"
```

## Protein Literals

Protein sequences use standard single-letter amino acid codes:

```
let p53_fragment = protein"MEEPQSDPSVEPPLSQETFSDLWKLLPENNVLSPLPS"
```

Protein literals validate against the 20 standard amino acids plus `X` (unknown),
`*` (stop), `U` (selenocysteine), and `O` (pyrrolysine):

```
let with_stop = protein"MVLSPADKTNVK*"
```

## Quality Score Literals

Phred+33 quality scores are first-class values:

```
let quals = qual"IIIIIHHHGGFFEEDCBA"
```

Each character maps to a Phred quality score. You can work with them numerically:

```
let q = qual"IIIIIHHHGG"
print(mean(q))       # => 37.2
print(min(q))        # => 38  (G = 38)
print(max(q))        # => 40  (I = 40)

# Filter reads by mean quality
let reads = read_fastq("sample.fastq.gz")
let hq_reads = reads |> filter(|r| mean(r.quality) >= 30)
```

## Sequence Operations

### Complement and Reverse Complement

```
let forward = dna"ATCGATCG"

let comp = forward |> complement()
print(comp)   # => dna"TAGCTAGC"

let rc = forward |> reverse_complement()
print(rc)     # => dna"CGATCGAT"
```

Reverse complement is the workhorse of strand-aware bioinformatics:

```
# Check if a primer binds to either strand
let template = dna"ATCGATCGAATTCGCTAGC"
let primer = dna"GCTAGC"

let fwd_match = template |> find_motif(primer)
let rev_match = template |> find_motif(primer |> reverse_complement())

print(f"Forward hits: {len(fwd_match)}, Reverse hits: {len(rev_match)}")
```

### Transcription and Translation

```
let gene = dna"ATGAAAGCTTTTCGATAG"

# Transcribe DNA to RNA (T -> U)
let mrna = gene |> transcribe()
print(mrna)   # => rna"AUGAAAGCUUUUCGAUAG"

# Translate RNA (or DNA) to protein
let protein_seq = gene |> translate()
print(protein_seq)   # => protein"MKAFR*"

# Translate with a different codon table (e.g., mitochondrial)
let mito_protein = gene |> translate(table: 2)
```

### GC Content

```
let contig = dna"ATCGATCGGGCCCATATATGCGCGC"

let gc = contig |> gc_content()
print(f"GC: {gc:.2%}")   # => GC: 56.00%

# Sliding window GC content for detecting isochores
let gc_windows = contig |> window(10) |> map(|w| gc_content(w))
print(gc_windows)
```

### Subsequences with `slice` and `len`

```
let genome_fragment = dna"ATCGATCGATCGATCGATCG"

print(seq_len(genome_fragment))           # => 20
print(genome_fragment |> slice(0, 6)) # => dna"ATCGAT"

# Extract a coding region by coordinates
let cds_start = 3
let cds_end = 15
let cds = genome_fragment |> slice(cds_start, cds_end)
```

### Motif Searching

`find_motif` returns a list of match positions:

```
let seq = dna"ATCGAATTCGATCGAATTCG"

# Find EcoRI recognition site
let ecori_sites = seq |> find_motif(dna"GAATTC")
print(ecori_sites)   # => [3, 13]

# Regex search on sequences (returns match records)
let matches = seq |> regex_find("GA[AT]TTC")
matches |> each(|m| print(f"Match at {m.start}: {m.text}"))
```

## Example: Find All ORFs in a DNA Sequence

An open reading frame (ORF) starts with ATG and ends at the first in-frame stop codon
(TAA, TAG, TGA). This script finds all ORFs in all six reading frames.

```
# orf_finder.bl
# Find all open reading frames in a FASTA sequence.

let find_orfs = |seq, min_length| {
  let orfs = []
  let codons = seq |> chunk(3)
  let in_orf = false
  let orf_start = 0

  codons |> enumerate() |> each(|i, codon| {
    if !in_orf && codon == dna"ATG" then {
      in_orf = true
      orf_start = i * 3
    }
    if in_orf && (codon == dna"TAA" || codon == dna"TAG" || codon == dna"TGA") then {
      let orf_end = (i + 1) * 3
      let orf_len = orf_end - orf_start
      if orf_len >= min_length then {
        orfs = orfs ++ [{start: orf_start, end: orf_end, length: orf_len, seq: seq |> slice(orf_start, orf_end)}]
      }
      in_orf = false
    }
  })
  orfs
}

let sequences = read_fasta("contigs.fa")

# Search all 6 reading frames
let all_orfs = sequences |> flat_map(|entry| {
  let fwd = entry.seq
  let rev = entry.seq |> reverse_complement()

  0..3 |> flat_map(|frame| {
    let fwd_orfs = fwd |> slice(frame, seq_len(fwd)) |> find_orfs(300)
      |> map(|orf| {
        ...orf,
        seq_id: entry.id,
        strand: "+",
        frame: frame,
        start: orf.start + frame
      })
    let rev_orfs = rev |> slice(frame, seq_len(rev)) |> find_orfs(300)
      |> map(|orf| {
        ...orf,
        seq_id: entry.id,
        strand: "-",
        frame: frame
      })
    fwd_orfs ++ rev_orfs
  })
})

print(f"Found {len(all_orfs)} ORFs (>= 300 bp)")
all_orfs
  |> sort_by(|orf| -orf.length)
  |> take(20)
  |> each(|orf| print(f"  {orf.seq_id} {orf.strand}:{orf.start}-{orf.end} ({orf.length} bp)"))
```

## Example: Codon Usage Table

Given a coding sequence, calculate the frequency of each codon and display
the codon usage bias.

```
# codon_usage.bl
# Build a codon usage table from a coding sequence.

let cds_records = read_fasta("cds_sequences.fa")

# Aggregate codons across all coding sequences
let codon_list = cds_records
  |> flat_map(|rec| rec.seq |> chunk(3))
  |> filter(|codon| seq_len(codon) == 3)
  |> map(|codon| {codon: to_string(codon)})

let codon_counts = table(codon_list)
  |> group_by("codon")
  |> summarize(|key, group| {codon: key, count: nrow(group)})

let total = codon_counts |> map(|c| c.count) |> sum()

let usage_table = codon_counts
  |> map(|c| {
    ...c,
    amino_acid: into(c.codon, "DNA") |> translate() |> to_string(),
    frequency: c.count / total,
    per_thousand: (c.count / total) * 1000.0
  })
  |> sort_by(|c| c.amino_acid)

print(f"Codon usage from {len(cds_records)} sequences ({total} codons)\n")
print("Codon  AA  Count     Freq   /1000")
print("-----  --  --------  -----  -----")
usage_table |> each(|row|
  print(f"{row.codon}    {row.amino_acid}   {row.count:>8}  {row.frequency:.4f}  {row.per_thousand:.1f}")
)

# Identify rare codons (< 10 per thousand)
let rare = usage_table |> filter(|c| c.per_thousand < 10.0)
print(f"\nRare codons ({len(rare)}):")
rare |> each(|c| print(f"  {c.codon} ({c.amino_acid}): {c.per_thousand:.1f}/1000"))
```

## Example: Restriction Enzyme Site Finder

Find all restriction enzyme recognition sites in a sequence and predict
fragment sizes for a virtual digest.

```
# digest.bl
# Virtual restriction digest of a DNA sequence.

# Common restriction enzymes: name -> recognition site
let enzymes = [
  {name: "EcoRI",   site: dna"GAATTC",  cut_offset: 1},
  {name: "BamHI",   site: dna"GGATCC",  cut_offset: 1},
  {name: "HindIII", site: dna"AAGCTT",  cut_offset: 1},
  {name: "NotI",    site: dna"GCGGCCGC", cut_offset: 2},
  {name: "XhoI",    site: dna"CTCGAG",  cut_offset: 1},
  {name: "SalI",    site: dna"GTCGAC",  cut_offset: 1},
  {name: "NcoI",    site: dna"CCATGG",  cut_offset: 1},
  {name: "PstI",    site: dna"CTGCAG",  cut_offset: 5}
]

let entry = read_fasta("plasmid.fa") |> first()
let seq = entry.seq
print(f"Sequence length: {seq_len(seq)} bp\n")

# Find all cut sites for each enzyme
let results = enzymes |> map(|enzyme| {
  let sites = seq |> find_motif(enzyme.site)
  let cut_positions = sites |> map(|pos| pos + enzyme.cut_offset)
  {name: enzyme.name, site: to_string(enzyme.site), n_cuts: len(sites), positions: cut_positions}
})

# Report
results |> each(|r| {
  if r.n_cuts > 0 then {
    print(f"{r.name} ({r.site}): {r.n_cuts} site(s) at {r.positions}")
  } else {
    print(f"{r.name} ({r.site}): no sites")
  }
})

# Virtual double digest with EcoRI + BamHI
let ecori_result = results |> find(|r| r.name == "EcoRI")
let ecori_cuts = ecori_result.positions
let bamhi_result = results |> find(|r| r.name == "BamHI")
let bamhi_cuts = bamhi_result.positions
let all_cuts = (ecori_cuts ++ bamhi_cuts) |> sort() |> unique()

# Calculate fragment sizes (circular plasmid)
let fragments = if len(all_cuts) == 0 then [seq_len(seq)]
  else {
    let sorted_cuts = all_cuts |> sort()
    let frags = sorted_cuts
      |> window(2)
      |> map(|pair| pair[1] - pair[0])
    # Add the wrap-around fragment for circular DNA
    let wrap = seq_len(seq) - last(sorted_cuts) + first(sorted_cuts)
    frags ++ [wrap]
  }

print(f"\nEcoRI + BamHI double digest: {len(fragments)} fragments")
fragments
  |> sort() |> reverse()
  |> enumerate()
  |> each(|i, size| print(f"  Fragment {i + 1}: {size} bp"))
```

## Summary

Bio literals are the foundation of BioLang. They carry biological meaning, enforce
validity at parse time, and chain naturally with biological operations through pipes.
Instead of calling string manipulation functions on raw text, you work directly with
typed sequences that know about complements, codons, reading frames, and motifs.

In the next chapter, we will cover the full type system -- variables, records, type
coercion, and nil handling -- which builds on these bio types to represent complex
biological data structures.
