#!/usr/bin/env python3
"""Day 17: Protein Analysis — Python equivalent.

Requires: pip install requests biopython
"""

import requests
from collections import Counter

# ── Step 1: Protein Sequence Basics ──────────────────────────────────

print("=== Step 1: Protein Sequence Basics ===\n")

p53_seq = "MEEPQSDPSVEPPLSQETFSDLWKLLPENNVLSPLPSQAMDDLMLSPDDIEQWFTEDPGPDEAPRMPEAAPPVAPAPAAPTPAAPAPAPSWPLSSSVPSQKTYPQGLNGTVNLPGRNSFEV"
print(f"Length: {len(p53_seq)} amino acids")
print(f"Type: str")

# ── Step 2: UniProt Lookup ───────────────────────────────────────────

print("\n=== Step 2: UniProt Lookup ===\n")

def uniprot_entry(accession):
    """Fetch protein info from UniProt REST API."""
    url = f"https://rest.uniprot.org/uniprotkb/{accession}.json"
    r = requests.get(url)
    r.raise_for_status()
    data = r.json()
    name = data.get("proteinDescription", {}).get("recommendedName", {}).get("fullName", {}).get("value", "")
    organism = data.get("organism", {}).get("scientificName", "")
    gene_names = [g.get("geneName", {}).get("value", "") for g in data.get("genes", [])]
    seq_len = data.get("sequence", {}).get("length", 0)
    function = ""
    for comment in data.get("comments", []):
        if comment.get("commentType") == "FUNCTION":
            texts = comment.get("texts", [])
            if texts:
                function = texts[0].get("value", "")
    return {
        "accession": accession,
        "name": name,
        "organism": organism,
        "gene_names": gene_names,
        "sequence_length": seq_len,
        "function": function,
    }

entry = uniprot_entry("P04637")
print(f"Protein: {entry['name']}")
print(f"Gene: {entry['gene_names']}")
print(f"Organism: {entry['organism']}")
print(f"Length: {entry['sequence_length']} aa")
print(f"Function: {entry['function'][:80]}...")

# Get FASTA sequence
def uniprot_fasta(accession):
    url = f"https://rest.uniprot.org/uniprotkb/{accession}.fasta"
    r = requests.get(url)
    r.raise_for_status()
    lines = r.text.strip().split("\n")
    return "".join(lines[1:])  # skip header

fasta = uniprot_fasta("P04637")
print(f"\nFirst 60 residues: {fasta[:60]}")
print(f"Full length: {len(fasta)} aa")

# ── Step 3: Protein Features and Domains ─────────────────────────────

print("\n=== Step 3: Protein Features and Domains ===\n")

def uniprot_features(accession):
    url = f"https://rest.uniprot.org/uniprotkb/{accession}.json"
    r = requests.get(url)
    r.raise_for_status()
    data = r.json()
    features = []
    for f in data.get("features", []):
        features.append({
            "type": f.get("type", ""),
            "description": f.get("description", ""),
            "location": f"{f.get('location', {}).get('start', {}).get('value', '')}..{f.get('location', {}).get('end', {}).get('value', '')}",
        })
    return features

features = uniprot_features("P04637")
print(f"Total features: {len(features)}")

type_counts = Counter(f["type"] for f in features)
print(f"Feature types: {dict(type_counts)}")

domains = [f for f in features if f["type"] == "Domain"]
for d in domains:
    print(f"  Domain: {d['description']} ({d['location']})")

binding = [f for f in features if f["type"] == "Binding site"]
print(f"\nBinding sites: {len(binding)}")

# ── Step 4: GO Terms ─────────────────────────────────────────────────

print("\n=== Step 4: GO Terms ===\n")

def uniprot_go(accession):
    url = f"https://rest.uniprot.org/uniprotkb/{accession}.json"
    r = requests.get(url)
    r.raise_for_status()
    data = r.json()
    terms = []
    for ref in data.get("uniProtKBCrossReferences", []):
        if ref.get("database") == "GO":
            go_id = ref.get("id", "")
            props = {p["key"]: p["value"] for p in ref.get("properties", [])}
            term = props.get("GoTerm", "")
            aspect_map = {"P": "biological_process", "F": "molecular_function", "C": "cellular_component"}
            aspect = aspect_map.get(props.get("GoEvidenceType", "")[:1], term[:1] if term else "")
            # Parse aspect from term prefix
            if term.startswith("P:"):
                aspect = "biological_process"
                term = term[2:]
            elif term.startswith("F:"):
                aspect = "molecular_function"
                term = term[2:]
            elif term.startswith("C:"):
                aspect = "cellular_component"
                term = term[2:]
            terms.append({"id": go_id, "term": term, "aspect": aspect})
    return terms

go_terms = uniprot_go("P04637")
print(f"GO annotations: {len(go_terms)}")

bp = sum(1 for t in go_terms if t["aspect"] == "biological_process")
mf = sum(1 for t in go_terms if t["aspect"] == "molecular_function")
cc = sum(1 for t in go_terms if t["aspect"] == "cellular_component")
print(f"Biological Process: {bp}")
print(f"Molecular Function: {mf}")
print(f"Cellular Component: {cc}")

# ── Step 5: PDB Structures ───────────────────────────────────────────

print("\n=== Step 5: PDB Structures ===\n")

def pdb_entry(pdb_id):
    url = f"https://data.rcsb.org/rest/v1/core/entry/{pdb_id}"
    r = requests.get(url)
    r.raise_for_status()
    data = r.json()
    return {
        "id": pdb_id,
        "title": data.get("struct", {}).get("title", ""),
        "method": data.get("exptl", [{}])[0].get("method", "") if data.get("exptl") else "",
        "resolution": data.get("rcsb_entry_info", {}).get("resolution_combined", [None])[0],
        "release_date": data.get("rcsb_accession_info", {}).get("initial_release_date", ""),
    }

structure = pdb_entry("1TUP")
print(f"Title: {structure['title']}")
print(f"Resolution: {structure['resolution']} angstrom")
print(f"Method: {structure['method']}")
print(f"Release date: {structure['release_date']}")

# ── Step 6: Amino Acid Composition ───────────────────────────────────

print("\n=== Step 6: Amino Acid Composition ===\n")

seq = "MEEPQSDPSVEPPLSQETFSDLWKLLPENNVLSPLPSQAMDDLMLSPDD"
counts = Counter(seq)
print(f"Amino acid counts: {dict(sorted(counts.items()))}")

HYDROPHOBIC = set("AVLIMFWP")
POLAR = set("STNQYC")
POSITIVE = set("KRH")
NEGATIVE = set("DE")

def classify_aa(aa):
    if aa in HYDROPHOBIC: return "hydrophobic"
    if aa in POLAR: return "polar"
    if aa in POSITIVE: return "positive"
    if aa in NEGATIVE: return "negative"
    return "other"

groups = Counter(classify_aa(aa) for aa in seq)
print(f"Property distribution: {dict(groups)}")

total = len(seq)
for group in ["hydrophobic", "polar", "negative", "positive"]:
    count = groups[group]
    pct = round(count / total * 100, 1)
    print(f"  {group}: {count}/{total} ({pct}%)")

# ── Step 7: K-mer Analysis ───────────────────────────────────────────

print("\n=== Step 7: K-mer Analysis ===\n")

seq = "MEEPQSDPSVEPPLSQETFSDLWKLL"
trimers = [seq[i:i+3] for i in range(len(seq) - 2)]
print(f"Protein 3-mers: {len(trimers)}")
print(f"First 5 trimers: {trimers[:5]}")

dipeptides = Counter(seq[i:i+2] for i in range(len(seq) - 1))
print(f"\nDipeptide counts (top 10):")
for dp, count in dipeptides.most_common(10):
    print(f"  {dp}: {count}")

print("\n=== Done ===")
