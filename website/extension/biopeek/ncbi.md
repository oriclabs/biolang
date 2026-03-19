# BioPeek — NCBI Test Accessions

## Single Fetch (quick test)
```
NM_007294
```
BRCA1 mRNA, ~5.8kb

## Batch Fetch (5 cancer genes)
```
NM_007294, NM_000546, NM_005228, NM_000059, NM_004304
```
BRCA1, TP53, EGFR, BRCA2, ALK

## Batch Fetch (10 mixed types)
```
NM_007294, NM_000546, NC_000017.11, NP_009225, NM_005228, NM_000059, NM_004304, NM_002524, NM_000314, NM_001126112
```
mRNA + chromosome region + protein + genes

## Protein Accessions (UniProt)
```
P38398, P04637, P00533
```
BRCA1, TP53, EGFR proteins — fetched from UniProt REST

## Large Sequence (stress test)
```
U00096
```
E. coli K-12 genome, ~4.6MB — good for streaming threshold test

## Very Large (expect timeout)
```
NC_000017.11
```
Full human chromosome 17, ~83MB — will likely timeout (15s limit)

## Viral Genomes (small, fast)
```
NC_045512.2, NC_001802.1, NC_001477.1
```
SARS-CoV-2 (~30kb), HIV-1 (~9.7kb), Dengue (~10.7kb)

## Mitochondrial
```
NC_012920.1
```
Human mitochondrial genome, ~16.5kb

## Multiple Organisms
```
NM_007294, NM_011577, NM_001001303
```
Human BRCA1, Mouse Tgfb1, Zebrafish tp53
