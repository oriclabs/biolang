# Real-data correctness

Generated 2026-08-04 21:47:07 on Microsoft Windows NT 10.0.26200.0

| | |
|---|---|
| BioLang | bl 1.1.0 |
| Python | Python 3.13.14 |
| R | Rscript (R) version 4.6.1 (2026-06-24) |
| Data | E. coli K-12, S. cerevisiae, ClinVar via download_real_data.py |
| Tolerance | floats 1e-6; integers and strings exact |

| Task | Reference | Result |
|---|---|---|
| gc_content | Python | PASS |
| kmer_count | Python | PASS |
| vcf_filter | Python | PASS |
| reverse_complement | Python | PASS |
| translate | Python | PASS |
| csv_groupby | Python | PASS |
| gff_features | Python | PASS |
| sequence_stats | Python | PASS |
| bed_intervals | Python | PASS |
| gc_content | R | PASS |
| vcf_filter | R | PASS |
| reverse_complement | R | PASS |
| translate | R | PASS |
| csv_groupby | R | PASS |
| gff_features | R | PASS |
| sequence_stats | R | PASS |
| bed_intervals | R | PASS |

vs Python: 9 passed, 0 failed, 0 skipped

vs R: 8 passed, 0 failed, 0 skipped

