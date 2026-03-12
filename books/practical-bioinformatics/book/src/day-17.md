# Day 17: Protein Analysis

| | |
|---|---|
| **Difficulty** | Intermediate |
| **Biology knowledge** | Intermediate (amino acids, protein structure, domains) |
| **Coding knowledge** | Intermediate (records, pipes, lambda functions, maps) |
| **Time** | ~3 hours |
| **Prerequisites** | Days 1-16 completed, BioLang installed (see Appendix A) |
| **Data needed** | None (all examples use API calls or inline sequences) |
| **Requirements** | Internet connection for API sections (UniProt, PDB, Ensembl) |

## What You'll Learn

- How to work with protein sequences and understand amino acid properties
- How to query UniProt for protein information, features, domains, and GO terms
- How to access 3D structure data from the PDB
- How to analyze amino acid composition and k-mer profiles
- How to compare orthologs across species and assess mutation impact

---

## The Problem

You found a missense mutation in EGFR. Does it affect the protein? Is it in a critical domain? What does the structure look like? Protein analysis connects genetic variants to functional consequences. DNA tells you *what changed*; protein analysis tells you *why it matters*.

Every gene encodes a protein (or several), and the protein is what actually does the work in the cell. A single amino acid change can destroy enzyme activity, disrupt a binding interface, or destabilize the entire fold. To understand the impact of a variant, you need to know the protein: its domains, its structure, its function, and the properties of the amino acids involved.

---

## Protein Sequence Basics

Proteins are chains of amino acids. Where DNA uses a 4-letter alphabet (A, T, G, C), proteins use a 20-letter alphabet. Each amino acid has distinct chemical properties that determine how the protein folds and functions.

```
Amino Acid Properties
=====================

Hydrophobic: A, V, L, I, M, F, W, P    (pack in the protein interior)
Polar:       S, T, N, Q, Y, C          (surface, form hydrogen bonds)
Positive:    K, R, H                    (basic, often bind DNA/RNA)
Negative:    D, E                       (acidic, often in catalytic sites)
Special:     G (flexible), P (rigid)
```

Protein structure has four levels:

```
Levels of Protein Structure
============================

Primary    →  Amino acid sequence (MEEPQSD...)
Secondary  →  Local folding: alpha helices, beta sheets
Tertiary   →  Complete 3D fold of one chain
Quaternary →  Multiple chains assembled together
```

Each level builds on the previous one. The primary sequence determines everything else --- change one amino acid, and the entire fold can be disrupted.

BioLang has a native protein literal type, just like DNA and RNA:

```bio
let p53 = protein"MEEPQSDPSVEPPLSQETFSDLWKLLPENNVLSPLPSQAMDDLMLSPDDIEQWFTEDPGPDEAPRMPEAAPPVAPAPAAPTPAAPAPAPSWPLSSSVPSQKTYPQGLNGTVNLPGRNSFEV"
println(f"Length: {len(p53)} amino acids")
println(f"Type: {type(p53)}")
```

Expected output:

```
Length: 120 amino acids
Type: Protein
```

The `protein"..."` literal validates that every character is a valid amino acid code. Just as `dna"ATCG"` ensures valid nucleotides, `protein"MEEP..."` ensures valid residues.

---

## UniProt: The Protein Knowledge Base

UniProt is the single most important protein database. It assigns each protein a stable accession number (like P04637 for human TP53) and aggregates information from hundreds of sources: sequence, function, domains, GO annotations, disease associations, post-translational modifications, and cross-references to every other major database.

### Looking Up a Protein

```bio
# requires: internet connection

# Look up a protein by accession
let entry = uniprot_entry("P04637")  # TP53
println(f"Protein: {entry.name}")
println(f"Gene: {entry.gene_names}")
println(f"Organism: {entry.organism}")
println(f"Length: {entry.sequence_length} aa")
println(f"Function: {substr(entry.function, 0, 80)}...")
```

> **Requires CLI:** This example uses file I/O / network APIs not available in the browser. Run with `bl run`.

Expected output:

```
Protein: Cellular tumor antigen p53
Gene: [TP53, P53]
Organism: Homo sapiens (Human)
Length: 393 aa
Function: Acts as a tumor suppressor in many tumor types; induces growth arrest or apop...
```

The `uniprot_entry()` function returns a record with fields: `accession`, `name`, `organism`, `sequence_length`, `gene_names` (a list), and `function`.

### Getting the Protein Sequence

```bio
# requires: internet connection

# Get the FASTA sequence as a string
let fasta = uniprot_fasta("P04637")
println(f"First 60 residues: {substr(fasta, 0, 60)}")
println(f"Full length: {len(fasta)} aa")
```

> **Requires CLI:** This example uses file I/O / network APIs not available in the browser. Run with `bl run`.

Expected output:

```
First 60 residues: MEEPQSDPSVEPPLSQETFSDLWKLLPENNVLSPLPSQAMDDLMLSPDDIEQWFTEDPGP
Full length: 393 aa
```

The `uniprot_fasta()` function returns the raw amino acid sequence as a string.

### Searching UniProt

```bio
# requires: internet connection

# Search UniProt for human kinases in the reviewed (SwissProt) database
let results = uniprot_search("kinase AND organism_id:9606 AND reviewed:true")
println(f"Human kinases in SwissProt: {len(results)}")
println(f"First 3: {results |> take(3)}")
```

> **Requires CLI:** This example uses file I/O / network APIs not available in the browser. Run with `bl run`.

Expected output:

```
Human kinases in SwissProt: 518
First 3: [{accession: P00533, name: Epidermal growth factor receptor, ...}, ...]
```

---

## Protein Features and Domains

Proteins are not uniform chains --- they contain distinct regions (domains) that perform specific functions. A kinase domain phosphorylates substrates. A DNA-binding domain recognizes specific sequences. A transmembrane domain anchors the protein in the membrane.

UniProt annotates these features with precise locations. The `uniprot_features()` function returns a list of records, each with `type`, `description`, and `location` fields.

```bio
# requires: internet connection

let features = uniprot_features("P04637")
println(f"Total features: {len(features)}")

# Count by type
let types = features |> map(|f| f.type) |> frequencies()
println(f"Feature types: {types}")

# Find domains
let domains = features |> filter(|f| f.type == "Domain")
for d in domains {
    println(f"  Domain: {d.description} ({d.location})")
}

# Find binding sites
let binding = features |> filter(|f| f.type == "Binding site")
println(f"\nBinding sites: {len(binding)}")
for b in binding {
    println(f"  {b.description} ({b.location})")
}
```

> **Requires CLI:** This example uses file I/O / network APIs not available in the browser. Run with `bl run`.

Expected output:

```
Total features: 68
Feature types: {Chain: 1, Domain: 3, DNA binding: 1, Region: 4, ...}
  Domain: Transactivation domain 1 (1..43)
  Domain: Proline-rich region (63..97)
  Domain: Tetramerization domain (323..356)

Binding sites: 4
  Zinc (176)
  Zinc (179)
  Zinc (238)
  Zinc (242)
```

### Why Features Matter for Variant Interpretation

When you find a missense mutation, the first question is: *where in the protein is it?* A mutation in a flexible loop might be tolerated. A mutation in the DNA-binding domain that disrupts a zinc-coordinating residue is almost certainly pathogenic. Features give you this context.

```bio
# requires: internet connection

# Check if a mutation position falls in a domain
let features = uniprot_features("P04637")
let domains = features |> filter(|f| f.type == "Domain")

# TP53 R248W is one of the most common cancer mutations
let mutation_pos = 248
println(f"Mutation at position {mutation_pos}")
println(f"Domains in TP53:")
for d in domains {
    println(f"  {d.description}: {d.location}")
}
println("Position 248 falls in the DNA-binding domain (102-292)")
println("This is a hotspot mutation that disrupts DNA contact")
```

> **Requires CLI:** This example uses file I/O / network APIs not available in the browser. Run with `bl run`.

Expected output:

```
Mutation at position 248
Domains in TP53:
  Transactivation domain 1: 1..43
  Proline-rich region: 63..97
  Tetramerization domain: 323..356
Position 248 falls in the DNA-binding domain (102-292)
This is a hotspot mutation that disrupts DNA contact
```

---

## GO Terms for Protein Function

Gene Ontology (GO) terms classify what a protein does at three levels: Biological Process (what it participates in), Molecular Function (what biochemical activity it has), and Cellular Component (where in the cell it acts). You encountered GO briefly in Day 16. Here we focus on protein-level annotation.

```bio
# requires: internet connection

let go_terms = uniprot_go("P04637")
println(f"GO annotations: {len(go_terms)}")

# Group by aspect
let bp = go_terms |> filter(|t| t.aspect == "biological_process") |> len()
let mf = go_terms |> filter(|t| t.aspect == "molecular_function") |> len()
let cc = go_terms |> filter(|t| t.aspect == "cellular_component") |> len()
println(f"Biological Process: {bp}")
println(f"Molecular Function: {mf}")
println(f"Cellular Component: {cc}")

# Show some specific terms
let functions = go_terms |> filter(|t| t.aspect == "molecular_function")
println(f"\nMolecular functions:")
for f in functions |> take(5) {
    println(f"  {f.id}: {f.term}")
}
```

> **Requires CLI:** This example uses file I/O / network APIs not available in the browser. Run with `bl run`.

Expected output:

```
GO annotations: 142
Biological Process: 98
Molecular Function: 24
Cellular Component: 20

Molecular functions:
  GO:0003700: DNA-binding transcription factor activity
  GO:0003677: DNA binding
  GO:0005515: protein binding
  GO:0046982: protein heterodimerization activity
  GO:0042802: identical protein binding
```

GO terms tell you the functional context. If a protein has "kinase activity" (MF), participates in "signal transduction" (BP), and localizes to the "plasma membrane" (CC), you have a clear picture of a membrane-associated signaling kinase.

---

## PDB: 3D Protein Structures

The Protein Data Bank (PDB) contains experimentally determined 3D structures of proteins, solved by X-ray crystallography, cryo-EM, or NMR. Resolution matters: lower numbers mean sharper detail. A 1.5 Angstrom structure shows individual atoms; a 4.0 Angstrom structure shows overall shape but not side-chain detail.

```bio
# requires: internet connection

let structure = pdb_entry("1TUP")  # TP53 DNA-binding domain
println(f"Title: {structure.title}")
println(f"Resolution: {structure.resolution} angstrom")
println(f"Method: {structure.method}")
println(f"Release date: {structure.release_date}")
println(f"Organism: {structure.organism}")
```

> **Requires CLI:** This example uses file I/O / network APIs not available in the browser. Run with `bl run`.

Expected output:

```
Title: CRYSTAL STRUCTURE OF THE TETRAMERIZATION DOMAIN OF THE TUMOR SUPPRESSOR P53
Resolution: 1.7 angstrom
Method: X-RAY DIFFRACTION
Release date: 1995-10-15
Organism: Homo sapiens
```

### Searching for Structures

```bio
# requires: internet connection

# Search for all structures of a protein
let p53_structures = pdb_search("TP53")
println(f"TP53 structures in PDB: {len(p53_structures)}")
println(f"First 5 IDs: {p53_structures |> take(5)}")

# Look up a specific structure for more detail
let best = pdb_entry(first(p53_structures))
println(f"\nFirst hit: {best.id}")
println(f"  Title: {best.title}")
println(f"  Method: {best.method}")
println(f"  Resolution: {best.resolution}")
```

> **Requires CLI:** This example uses file I/O / network APIs not available in the browser. Run with `bl run`.

Expected output:

```
TP53 structures in PDB: 385
First 5 IDs: [1TUP, 1TSR, 1UOL, 2AC0, 2AHI]

First hit: 1TUP
  Title: CRYSTAL STRUCTURE OF THE TETRAMERIZATION DOMAIN OF THE TUMOR SUPPRESSOR P53
  Method: X-RAY DIFFRACTION
  Resolution: 1.7
```

### Getting the Protein Sequence from PDB

```bio
# requires: internet connection

# Get the amino acid sequence from a PDB entry (entity 1)
let seq = pdb_sequence("1TUP", 1)
println(f"Type: {type(seq)}")
println(f"Length: {len(seq)} residues")
println(f"Sequence: {substr(str(seq), 0, 50)}...")
```

> **Requires CLI:** This example uses file I/O / network APIs not available in the browser. Run with `bl run`.

Expected output:

```
Type: Protein
Length: 60 residues
Sequence: PQHLRVEGNLHAEYLDDKQTKFISLHGNVQLGDSSVKFKSNEDLRNEEGF...
```

The `pdb_sequence()` function takes a PDB ID and an entity number (typically 1 for the main protein chain) and returns a `Protein` value.

---

## Amino Acid Composition Analysis

The amino acid composition of a protein tells you a lot about its character. Membrane proteins are enriched in hydrophobic residues. DNA-binding proteins are enriched in positively charged residues (K, R). Intrinsically disordered regions tend to be enriched in charged and polar residues and depleted in hydrophobic ones.

```bio
let seq = protein"MEEPQSDPSVEPPLSQETFSDLWKLLPENNVLSPLPSQAMDDLMLSPDD"
let counts = base_counts(seq)
println(f"Amino acid counts: {counts}")
```

Expected output:

```
Amino acid counts: {A: 2, D: 5, E: 4, F: 1, K: 1, L: 7, M: 2, N: 2, P: 7, Q: 3, S: 4, T: 1, V: 2, W: 1}
```

Despite its name, `base_counts()` works on all BioLang sequence types --- DNA, RNA, and Protein. It returns a map of character frequencies.

### Classifying by Chemical Properties

```bio
let seq = protein"MEEPQSDPSVEPPLSQETFSDLWKLLPENNVLSPLPSQAMDDLMLSPDD"
let counts = base_counts(seq)

# Classify each amino acid by chemical property
fn classify_aa(aa) {
    match aa {
        "A" | "V" | "L" | "I" | "M" | "F" | "W" | "P" => "hydrophobic",
        "S" | "T" | "N" | "Q" | "Y" | "C" => "polar",
        "K" | "R" | "H" => "positive",
        "D" | "E" => "negative",
        _ => "other"
    }
}

# Count by property group
let residues = split(str(seq), "")
let groups = residues |> map(|aa| classify_aa(aa)) |> frequencies()
println(f"Property distribution: {groups}")

# Calculate percentages
let total = len(residues)
for group in ["hydrophobic", "polar", "negative", "positive"] {
    let count = groups[group]
    let pct = round(count / total * 100, 1)
    println(f"  {group}: {count}/{total} ({pct}%)")
}
```

Expected output:

```
Property distribution: {hydrophobic: 20, polar: 10, negative: 9, positive: 1}
  hydrophobic: 20/48 (41.7%)
  polar: 10/48 (20.8%)
  negative: 9/48 (18.8%)
  positive: 1/48 (2.1%)
```

A high fraction of hydrophobic residues is expected in globular proteins (they form the core). The very low positive charge here reflects this fragment of TP53 being the transactivation domain, which is acidic (lots of D and E).

---

## K-mer Analysis of Proteins

Just as DNA k-mers reveal motifs and repeat patterns (Day 5), protein k-mers can identify sequence motifs and conserved patterns. Dipeptide and tripeptide frequencies are used in machine learning models that predict protein localization, solubility, and function.

```bio
# Protein k-mers reveal motifs and domain signatures
let seq = protein"MEEPQSDPSVEPPLSQETFSDLWKLL"
let trimers = kmers(seq, 3)
println(f"Protein 3-mers: {len(trimers)}")
println(f"First 5 trimers: {trimers |> take(5)}")

# Count dipeptide frequencies
let dipeptides = kmer_count(seq, 2)
println(f"\nDipeptide counts (top 10):")
println(dipeptides |> head(10))
```

Expected output:

```
Protein 3-mers: 24
First 5 trimers: [MEE, EEP, EPQ, PQS, QSD]

Dipeptide counts (top 10):
EP: 2
PL: 2
PS: 2
SD: 2
SQ: 1
...
```

Certain dipeptides are over-represented in specific structural contexts. For example, "PP" is common in proline-rich regions that resist folding, while "LV" and "IL" clusters are typical of hydrophobic cores.

---

## Comparing Proteins Across Species

Orthologous proteins --- the same gene in different species --- reveal what evolution has preserved. Highly conserved positions are functionally critical. Variable positions are tolerant of change. Comparing orthologs is one of the most powerful ways to predict whether a mutation is damaging.

```bio
# requires: internet connection

# Compare TP53 across species
let accessions = ["P04637", "Q00366", "O09185"]  # Human, Chicken, Mouse
let names = ["Human", "Chicken", "Mouse"]

let proteins = []
for i in range(0, len(accessions)) {
    let entry = uniprot_entry(accessions[i])
    proteins = proteins + [{
        species: names[i],
        accession: entry.accession,
        name: entry.name,
        organism: entry.organism,
        length: entry.sequence_length
    }]
}

let comparison = proteins |> to_table()
println("TP53 Orthologs:")
println(comparison)
```

> **Requires CLI:** This example uses file I/O / network APIs not available in the browser. Run with `bl run`.

Expected output:

```
TP53 Orthologs:
species   accession  name                       organism                length
Human     P04637     Cellular tumor antigen p53  Homo sapiens (Human)    393
Chicken   Q00366     Cellular tumor antigen p53  Gallus gallus (Chicken) 367
Mouse     O09185     Cellular tumor antigen p53  Mus musculus (Mouse)    387
```

The lengths differ slightly between species, but the core structure is conserved. The DNA-binding domain (roughly residues 100-290 in human) is the most highly conserved region, reflecting its critical function.

---

## Protein Mutation Impact

When you find a missense variant, the question is: does this amino acid change matter? The answer depends on several factors:

1. **Where** in the protein is the mutation? (domain, active site, surface?)
2. **What** property changed? (charge, size, hydrophobicity?)
3. **How conserved** is this position? (conserved = important)

### Assessing Property Changes

```bio
# Assess the impact of a point mutation
let normal = protein"MEEPQSDPSVEPPLSQE"
let mutant = protein"MEEPQSDPSVEPPLSRE"  # Q16R: glutamine → arginine

# Compare the changed residue
let normal_aa = substr(str(normal), 15, 1)
let mutant_aa = substr(str(mutant), 15, 1)
println(f"Position 16: {normal_aa} -> {mutant_aa}")

fn classify_aa(aa) {
    match aa {
        "A" | "V" | "L" | "I" | "M" | "F" | "W" | "P" => "hydrophobic",
        "S" | "T" | "N" | "Q" | "Y" | "C" => "polar",
        "K" | "R" | "H" => "positive",
        "D" | "E" => "negative",
        _ => "other"
    }
}

let normal_class = classify_aa(normal_aa)
let mutant_class = classify_aa(mutant_aa)
println(f"Property: {normal_class} -> {mutant_class}")

if normal_class != mutant_class {
    println("WARNING: Property change detected --- likely functional impact")
} else {
    println("Same property class --- may be tolerated")
}
```

Expected output:

```
Position 16: Q -> R
Property: polar -> positive
WARNING: Property change detected --- likely functional impact
```

A polar-to-positive change introduces a new charge. This is the kind of change most likely to disrupt protein function, especially if it occurs at a conserved position in a functional domain.

### Using Ensembl VEP for Variant Assessment

For real variant assessment, the Variant Effect Predictor (VEP) integrates multiple lines of evidence: conservation, structural data, and known disease associations.

```bio
# requires: internet connection

# Assess a known pathogenic EGFR mutation
let vep = ensembl_vep("7:55249071:C:T")  # EGFR variant
println(f"Consequence: {vep.consequence}")
println(f"Gene: {vep.gene}")
println(f"Impact: {vep.impact}")
```

> **Requires CLI:** This example uses file I/O / network APIs not available in the browser. Run with `bl run`.

Expected output:

```
Consequence: missense_variant
Gene: EGFR
Impact: MODERATE
```

---

## Complete Protein Analysis Pipeline

This pipeline brings together everything from this chapter: UniProt lookup, feature extraction, GO annotation, and PDB structure search. It produces a comprehensive report for any protein given its UniProt accession.

```bio
# Complete Protein Analysis Report
# requires: internet connection

fn protein_report(accession) {
    println(f"\n{'=' * 50}")
    println(f"Protein Report: {accession}")
    println(f"{'=' * 50}\n")

    # Basic info
    let entry = uniprot_entry(accession)
    println(f"Name: {entry.name}")
    println(f"Gene: {entry.gene_names}")
    println(f"Organism: {entry.organism}")
    println(f"Length: {entry.sequence_length} aa")

    # Get sequence and analyze composition
    let fasta = uniprot_fasta(accession)
    let residues = split(fasta, "")
    let total = len(residues)

    fn classify_aa(aa) {
        match aa {
            "A" | "V" | "L" | "I" | "M" | "F" | "W" | "P" => "hydrophobic",
            "S" | "T" | "N" | "Q" | "Y" | "C" => "polar",
            "K" | "R" | "H" => "positive",
            "D" | "E" => "negative",
            _ => "other"
        }
    }

    let groups = residues |> map(|aa| classify_aa(aa)) |> frequencies()
    println(f"\nComposition:")
    for group in ["hydrophobic", "polar", "negative", "positive"] {
        let count = groups[group]
        let pct = round(count / total * 100, 1)
        println(f"  {group}: {pct}%")
    }

    # Domains
    let features = uniprot_features(accession)
    let domains = features |> filter(|f| f.type == "Domain")
    println(f"\nDomains ({len(domains)}):")
    for d in domains {
        println(f"  {d.description}: {d.location}")
    }

    # GO terms
    let go = uniprot_go(accession)
    let bp = go |> filter(|t| t.aspect == "biological_process") |> len()
    let mf = go |> filter(|t| t.aspect == "molecular_function") |> len()
    let cc = go |> filter(|t| t.aspect == "cellular_component") |> len()
    println(f"\nGO annotations: {len(go)} total")
    println(f"  Biological Process: {bp}")
    println(f"  Molecular Function: {mf}")
    println(f"  Cellular Component: {cc}")

    # PDB structures
    let structures = pdb_search(first(entry.gene_names))
    println(f"\nPDB structures: {len(structures)}")
    if len(structures) > 0 {
        let top = pdb_entry(first(structures))
        println(f"  Best: {top.id} - {top.method}, {top.resolution} angstrom")
    }
}

# Generate reports for key cancer proteins
let targets = ["P04637", "P00533", "P01116"]  # TP53, EGFR, KRAS
for acc in targets {
    protein_report(acc)
}
```

> **Requires CLI:** This example uses file I/O / network APIs not available in the browser. Run with `bl run`.

Expected output:

```
==================================================
Protein Report: P04637
==================================================

Name: Cellular tumor antigen p53
Gene: [TP53, P53]
Organism: Homo sapiens (Human)
Length: 393 aa

Composition:
  hydrophobic: 35.4%
  polar: 28.2%
  negative: 10.7%
  positive: 14.2%

Domains (3):
  Transactivation domain 1: 1..43
  Proline-rich region: 63..97
  Tetramerization domain: 323..356

GO annotations: 142 total
  Biological Process: 98
  Molecular Function: 24
  Cellular Component: 20

PDB structures: 385
  Best: 1TUP - X-RAY DIFFRACTION, 1.7 angstrom

==================================================
Protein Report: P00533
==================================================

Name: Epidermal growth factor receptor
Gene: [EGFR, ERBB1, HER1]
Organism: Homo sapiens (Human)
Length: 1210 aa

Composition:
  hydrophobic: 38.1%
  polar: 24.5%
  negative: 11.3%
  positive: 13.8%

Domains (4):
  Furin-like cysteine rich domain: 177..338
  Furin-like cysteine rich domain: 481..621
  Protein kinase domain: 712..979
  Receptor L domain: 57..167

GO annotations: 96 total
  Biological Process: 62
  Molecular Function: 18
  Cellular Component: 16

PDB structures: 290
  Best: 1NQL - X-RAY DIFFRACTION, 2.5 angstrom

==================================================
Protein Report: P01116
==================================================

Name: GTPase KRas
Gene: [KRAS]
Organism: Homo sapiens (Human)
Length: 189 aa

Composition:
  hydrophobic: 34.9%
  polar: 25.9%
  negative: 14.8%
  positive: 13.2%

Domains (0):

GO annotations: 78 total
  Biological Process: 52
  Molecular Function: 14
  Cellular Component: 12

PDB structures: 620
  Best: 4OBE - X-RAY DIFFRACTION, 1.2 angstrom
```

---

## Exercises

1. **Insulin deep dive.** Look up insulin (P01308) in UniProt and list its domains, features, and GO terms. How many PDB structures exist for it?

2. **Composition comparison.** Get the amino acid sequences for a membrane protein (e.g., EGFR, P00533) and a nuclear protein (e.g., TP53, P04637). Compare their hydrophobic/polar/charged ratios. Which has more hydrophobic residues, and why?

3. **Structure search.** Find all PDB structures for EGFR using `pdb_search()`. Pick the first result and look up its resolution and method. How does cryo-EM resolution compare to X-ray crystallography?

4. **K-mer motifs.** Use `kmers()` and `kmer_count()` to analyze protein 3-mers in the first 100 residues of TP53 (get the sequence with `uniprot_fasta("P04637")`). Are there any repeated tripeptides?

5. **Ortholog comparison.** Build a protein comparison table for BRCA1 across three species: human (P38398), mouse (P48754), and chicken (F1NLG5). Compare their lengths and domain counts.

---

## Key Takeaways

- **UniProt is the primary protein knowledge base** --- accession numbers are stable identifiers that never change, even as annotation improves.
- **Protein features map function to sequence** --- domains, binding sites, and active sites explain what each region of the protein does.
- **GO terms classify function at three levels** --- biological process, molecular function, and cellular component give complementary views.
- **PDB structures show the 3D shape** --- resolution matters; lower numbers mean more reliable atomic detail.
- **Amino acid properties determine protein behavior** --- hydrophobicity, charge, and size all affect folding, binding, and catalysis.
- **Mutations in critical domains have the highest impact** --- a change in an active site or binding interface is far more damaging than one in a flexible loop.

---

## What's Next

Tomorrow: **Day 18 --- Genomic Coordinates and Intervals.** BED operations, overlap queries, coordinate systems (0-based vs 1-based), and the interval arithmetic that underlies every genome browser and variant annotation tool.
