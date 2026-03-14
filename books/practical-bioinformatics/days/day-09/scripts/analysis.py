#!/usr/bin/env python3
"""Day 9: Biological Databases and APIs — Python equivalent.

Requires: pip install biopython requests
Optional: Set NCBI_API_KEY and NCBI_EMAIL environment variables.

Note: This script demonstrates the same queries as analysis.bl
using Python's Biopython, requests, and standard libraries.
"""

import os
import time
import json
import requests
from Bio import Entrez, SeqIO

# Configure NCBI
Entrez.email = os.environ.get("NCBI_EMAIL", "user@example.com")
api_key = os.environ.get("NCBI_API_KEY")
if api_key:
    Entrez.api_key = api_key

print("=" * 60)
print("Day 9: Biological Databases and APIs (Python)")
print("=" * 60)

# ----------------------------------------------------------
# 1. NCBI — Gene Lookup
# ----------------------------------------------------------
print("\n--- 1. NCBI Gene Lookup ---")

handle = Entrez.esearch(db="gene", term="BRCA1[Gene Name] AND Homo sapiens[Organism]", retmax=1)
record = Entrez.read(handle)
handle.close()
gene_id = record["IdList"][0]

handle = Entrez.esummary(db="gene", id=gene_id)
summary = Entrez.read(handle)
handle.close()
doc = summary["DocumentSummarySet"]["DocumentSummary"][0]
print(f"Symbol: {doc['NomenclatureSymbol']}")
print(f"Name: {doc['NomenclatureName']}")
print(f"Description: {doc['Description']}")
chrom = doc.get("Chromosome", "?")
print(f"Chromosome: {chrom}")
map_loc = doc.get("MapLocation", "?")
print(f"Location: {map_loc}")

time.sleep(0.5)

# ----------------------------------------------------------
# 2. NCBI — PubMed Search
# ----------------------------------------------------------
print("\n--- 2. NCBI PubMed Search ---")

handle = Entrez.esearch(db="pubmed", term="BRCA1 breast cancer", retmax=5)
record = Entrez.read(handle)
handle.close()
print(f"PubMed articles found: {len(record['IdList'])}")
for pmid in record["IdList"]:
    print(f"  PMID: {pmid}")

time.sleep(0.5)

# ----------------------------------------------------------
# 3. Ensembl — Gene Model
# ----------------------------------------------------------
print("\n--- 3. Ensembl Gene Model ---")

resp = requests.get(
    "https://rest.ensembl.org/lookup/symbol/homo_sapiens/BRCA1",
    headers={"Content-Type": "application/json"},
)
resp.raise_for_status()
ens = resp.json()
print(f"Ensembl ID: {ens['id']}")
print(f"Biotype: {ens['biotype']}")
print(f"Position: chr{ens.get('seq_region_name', '?')}:{ens.get('start', '?')}-{ens.get('end', '?')}")
print(f"Strand: {ens.get('strand', '?')}")

time.sleep(0.5)

# ----------------------------------------------------------
# 4. Ensembl — Protein Sequence
# ----------------------------------------------------------
print("\n--- 4. Ensembl Protein Sequence ---")

resp = requests.get(
    f"https://rest.ensembl.org/sequence/id/{ens['id']}?type=protein",
    headers={"Content-Type": "application/json"},
)
resp.raise_for_status()
prot = resp.json()
seq = prot.get("seq", "")
print(f"Protein length: {len(seq)} amino acids")
print(f"First 60 aa: {seq[:60]}")
est_mw = len(seq) * 110
print(f"Estimated MW: ~{est_mw} Da ({est_mw / 1000:.0f} kDa)")

time.sleep(0.5)

# ----------------------------------------------------------
# 5. UniProt — Protein Function
# ----------------------------------------------------------
print("\n--- 5. UniProt Protein Function ---")

resp = requests.get(
    "https://rest.uniprot.org/uniprotkb/P38398.json",
    headers={"Accept": "application/json"},
)
resp.raise_for_status()
up = resp.json()
protein_name = up.get("proteinDescription", {}).get("recommendedName", {}).get("fullName", {}).get("value", "?")
organism = up.get("organism", {}).get("scientificName", "?")
seq_len = up.get("sequence", {}).get("length", 0)
gene_names = [g.get("geneName", {}).get("value", "") for g in up.get("genes", [])]
# Extract function from comments
function_text = ""
for comment in up.get("comments", []):
    if comment.get("commentType") == "FUNCTION":
        texts = comment.get("texts", [])
        if texts:
            function_text = texts[0].get("value", "")
            break
print(f"Name: {protein_name}")
print(f"Organism: {organism}")
print(f"Length: {seq_len} aa")
print(f"Gene names: {gene_names}")
print(f"Function: {function_text[:120]}...")

time.sleep(0.5)

# ----------------------------------------------------------
# 6. UniProt — Features and Domains
# ----------------------------------------------------------
print("\n--- 6. UniProt Features ---")

features = up.get("features", [])
print(f"Total features: {len(features)}")
domains = [f for f in features if f.get("type") == "Domain"]
print(f"Domains: {len(domains)}")
for d in domains:
    desc = d.get("description", "?")
    loc_start = d.get("location", {}).get("start", {}).get("value", "?")
    loc_end = d.get("location", {}).get("end", {}).get("value", "?")
    print(f"  {desc} ({loc_start}..{loc_end})")

time.sleep(0.5)

# ----------------------------------------------------------
# 7. KEGG — Pathway Links
# ----------------------------------------------------------
print("\n--- 7. KEGG Pathways ---")

resp = requests.get("https://rest.kegg.jp/find/genes/BRCA1")
resp.raise_for_status()
lines = [l for l in resp.text.strip().split("\n") if l]
print(f"KEGG gene hits: {len(lines)}")

resp = requests.get("https://rest.kegg.jp/link/pathway/hsa:672")
resp.raise_for_status()
link_lines = [l for l in resp.text.strip().split("\n") if l]
print(f"Pathways involving BRCA1: {len(link_lines)}")
for line in link_lines[:5]:
    parts = line.split("\t")
    if len(parts) >= 2:
        print(f"  {parts[1]}")

time.sleep(0.5)

# ----------------------------------------------------------
# 8. PDB — 3D Structures
# ----------------------------------------------------------
print("\n--- 8. PDB Structures ---")

resp = requests.get("https://data.rcsb.org/rest/v1/core/entry/1JM7")
resp.raise_for_status()
pdb = resp.json()
struct_info = pdb.get("struct", {})
print(f"Title: {struct_info.get('title', '?')}")
exptl = pdb.get("exptl", [{}])[0]
print(f"Method: {exptl.get('method', '?')}")
refine = pdb.get("refine", [{}])
resolution = refine[0].get("ls_d_res_high", "N/A") if refine else "N/A"
print(f"Resolution: {resolution}")

# Search for BRCA1 structures
search_payload = {
    "query": {
        "type": "terminal",
        "service": "full_text",
        "parameters": {"value": "BRCA1"},
    },
    "return_type": "entry",
}
resp = requests.post(
    "https://search.rcsb.org/rcsbsearch/v2/query",
    json=search_payload,
    headers={"Content-Type": "application/json"},
)
if resp.status_code == 200:
    results = resp.json()
    total = results.get("total_count", 0)
    print(f"Total BRCA1 structures in PDB: {total}")
else:
    print("PDB search failed")

time.sleep(0.5)

# ----------------------------------------------------------
# 9. STRING — Protein Interactions
# ----------------------------------------------------------
print("\n--- 9. STRING Interactions ---")

resp = requests.get(
    "https://string-db.org/api/json/network",
    params={"identifiers": "BRCA1", "species": 9606},
)
resp.raise_for_status()
interactions = resp.json()
print(f"Interaction partners: {len(interactions)}")

sorted_interactions = sorted(interactions, key=lambda x: x.get("score", 0), reverse=True)
print("Top 5 interactors:")
for i in sorted_interactions[:5]:
    print(f"  {i.get('preferredName_A', '?')} <-> {i.get('preferredName_B', '?')}: score={i.get('score', 0)}")

time.sleep(0.5)

# ----------------------------------------------------------
# 10. Gene Ontology
# ----------------------------------------------------------
print("\n--- 10. Gene Ontology ---")

resp = requests.get("https://www.ebi.ac.uk/QuickGO/services/ontology/go/terms/GO:0006281")
resp.raise_for_status()
go_data = resp.json()
go_results = go_data.get("results", [])
if go_results:
    term = go_results[0]
    print(f"GO term: {term.get('name', '?')} ({term.get('id', '?')})")
    print(f"Aspect: {term.get('aspect', '?')}")

resp = requests.get(
    "https://www.ebi.ac.uk/QuickGO/services/annotation/search",
    params={"geneProductId": "P38398", "limit": 10},
    headers={"Accept": "application/json"},
)
resp.raise_for_status()
ann_data = resp.json()
annotations = ann_data.get("results", [])
print(f"GO annotations for BRCA1: {len(annotations)}")
for a in annotations[:5]:
    print(f"  {a.get('goId', '?')}: {a.get('goName', '?')} ({a.get('goAspect', '?')})")

time.sleep(0.5)

# ----------------------------------------------------------
# 11. Reactome Pathways
# ----------------------------------------------------------
print("\n--- 11. Reactome Pathways ---")

resp = requests.get(
    f"https://reactome.org/ContentService/data/mapping/UniProt/P38398/pathways",
    headers={"Accept": "application/json"},
)
if resp.status_code == 200:
    pathways = resp.json()
    # Filter to human pathways
    human_pathways = [p for p in pathways if p.get("species", {}).get("displayName") == "Homo sapiens"]
    print(f"Reactome pathways: {len(human_pathways)}")
    for p in human_pathways[:5]:
        print(f"  {p.get('stId', '?')}: {p.get('displayName', '?')}")
else:
    print("Reactome query failed")

time.sleep(0.5)

# ----------------------------------------------------------
# 12. Batch Gene Table
# ----------------------------------------------------------
print("\n--- 12. Batch Gene Table ---")

genes = ["BRCA1", "TP53", "EGFR", "KRAS", "MYC"]
rows = []
for symbol in genes:
    handle = Entrez.esearch(db="gene", term=f"{symbol}[Gene Name] AND Homo sapiens[Organism]", retmax=1)
    record = Entrez.read(handle)
    handle.close()
    if record["IdList"]:
        gid = record["IdList"][0]
        handle = Entrez.esummary(db="gene", id=gid)
        summary = Entrez.read(handle)
        handle.close()
        doc = summary["DocumentSummarySet"]["DocumentSummary"][0]
        rows.append({
            "gene": symbol,
            "chrom": doc.get("Chromosome", "?"),
            "desc": doc.get("Description", "?"),
        })
    time.sleep(0.5)

print(f"{'gene':<8} {'chrom':<8} {'desc'}")
print("-" * 60)
for row in rows:
    print(f"{row['gene']:<8} {row['chrom']:<8} {row['desc']}")

print()
print("=" * 60)
print("Day 9 complete! (Python)")
print("=" * 60)
