#!/usr/bin/env python3
"""Day 24: Programmatic Database Access - Python equivalent."""

import csv
import json
import time
import os
from pathlib import Path
from Bio import Entrez
import requests

Entrez.email = os.environ.get("NCBI_EMAIL", "user@example.com")
api_key = os.environ.get("NCBI_API_KEY", None)
if api_key:
    Entrez.api_key = api_key

ENSEMBL_BASE = "https://rest.ensembl.org"
UNIPROT_BASE = "https://rest.uniprot.org"
REACTOME_BASE = "https://reactome.org/ContentService"
QUICKGO_BASE = "https://www.ebi.ac.uk/QuickGO/services"
STRING_BASE = "https://string-db.org/api"


def ncbi_gene(symbol):
    """Search NCBI Gene for a symbol."""
    try:
        handle = Entrez.esearch(db="gene", term=f"{symbol}[Gene Name] AND Homo sapiens[ORGN]")
        result = Entrez.read(handle)
        handle.close()
        if int(result["Count"]) > 0:
            return {"id": result["IdList"][0], "found": True}
        return None
    except Exception:
        return None


def ensembl_symbol(species, symbol):
    """Look up gene by symbol in Ensembl."""
    try:
        url = f"{ENSEMBL_BASE}/lookup/symbol/{species}/{symbol}"
        resp = requests.get(url, headers={"Content-Type": "application/json"}, timeout=10)
        if resp.status_code == 200:
            data = resp.json()
            return data.get("id", "N/A")
        return None
    except Exception:
        return None


def uniprot_search(query):
    """Search UniProt."""
    try:
        url = f"{UNIPROT_BASE}/uniprotkb/search"
        params = {"query": query, "format": "json", "size": 1}
        resp = requests.get(url, params=params, timeout=10)
        if resp.status_code == 200:
            data = resp.json()
            results = data.get("results", [])
            if results:
                return results[0].get("primaryAccession", "N/A")
        return None
    except Exception:
        return None


def reactome_pathways(symbol):
    """Get Reactome pathways for a gene symbol."""
    try:
        url = f"https://reactome.org/ContentService/search/query?query={symbol}&species=Homo%20sapiens&types=Pathway"
        resp = requests.get(url, headers={"Accept": "application/json"}, timeout=10)
        if resp.status_code == 200:
            data = resp.json()
            entries = data.get("results", [])
            pathways = []
            for group in entries:
                for entry in group.get("entries", []):
                    pathways.append(entry.get("name", ""))
            return pathways
        return []
    except Exception:
        return []


def go_annotations(symbol):
    """Get GO annotations for a gene."""
    try:
        url = f"{QUICKGO_BASE}/annotation/search"
        params = {"geneProductId": symbol, "taxonId": "9606", "limit": 50}
        resp = requests.get(url, params=params, timeout=10)
        if resp.status_code == 200:
            data = resp.json()
            return data.get("results", [])
        return []
    except Exception:
        return []


def string_network(identifiers):
    """Get STRING protein interaction network."""
    try:
        url = f"{STRING_BASE}/json/network"
        params = {
            "identifiers": "%0d".join(identifiers),
            "species": 9606,
        }
        resp = requests.get(url, params=params, timeout=10)
        if resp.status_code == 200:
            return resp.json()
        return None
    except Exception:
        return None


def annotate_gene(symbol):
    """Annotate a single gene from multiple databases."""
    ncbi = ncbi_gene(symbol)
    time.sleep(0.2)

    ensembl_id = ensembl_symbol("homo_sapiens", symbol)
    time.sleep(0.2)

    uniprot_acc = uniprot_search(f"gene:{symbol} AND organism_id:9606")
    time.sleep(0.2)

    pathways = reactome_pathways(symbol)
    time.sleep(0.2)

    go = go_annotations(symbol)
    time.sleep(0.2)

    return {
        "symbol": symbol,
        "ncbi_found": ncbi is not None,
        "ensembl_id": ensembl_id if ensembl_id else "N/A",
        "uniprot_found": uniprot_acc is not None,
        "pathway_count": len(pathways),
        "go_term_count": len(go),
    }


def main():
    with open("data/gene_list.csv", "r") as f:
        reader = csv.DictReader(f)
        genes = list(reader)

    symbols = [g["symbol"] for g in genes]

    annotations = []
    for s in symbols:
        ann = annotate_gene(s)
        annotations.append(ann)

    with open("data/annotations.tsv", "w", newline="") as f:
        fieldnames = ["symbol", "ncbi_found", "ensembl_id", "uniprot_found",
                       "pathway_count", "go_term_count"]
        writer = csv.DictWriter(f, fieldnames=fieldnames, delimiter="\t")
        writer.writeheader()
        for row in annotations:
            writer.writerow(row)

    up_genes = [g["symbol"] for g in genes if g["direction"] == "up"]
    down_genes = [g["symbol"] for g in genes if g["direction"] == "down"]

    up_pathways = []
    for g in up_genes:
        p = reactome_pathways(g)
        up_pathways.extend(p)
        time.sleep(0.2)

    down_pathways = []
    for g in down_genes:
        p = reactome_pathways(g)
        down_pathways.extend(p)
        time.sleep(0.2)

    sorted_symbols = sorted(symbols)
    top5 = sorted_symbols[:5]
    network = string_network(top5)

    summary = {
        "total_genes": len(symbols),
        "upregulated": len(up_genes),
        "downregulated": len(down_genes),
        "annotations_complete": len(annotations),
        "up_pathway_hits": len(up_pathways),
        "down_pathway_hits": len(down_pathways),
        "network_found": network is not None,
    }

    with open("data/summary.json", "w") as f:
        json.dump(summary, f, indent=2)


if __name__ == "__main__":
    main()
