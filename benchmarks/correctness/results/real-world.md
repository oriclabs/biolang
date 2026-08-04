# Real-data correctness

Generated 2026-08-04 22:11:47 on Microsoft Windows NT 10.0.26200.0

| | |
|---|---|
| BioLang | bl 1.1.0 |
| Python | Python 3.13.14 |
| R | Rscript (R) version 4.6.1 (2026-06-24) |
| Data | E. coli K-12, S. cerevisiae, ClinVar via download_real_data.py |
| Tolerance | floats 1e-6; integers and strings exact |

| Task | Reference | Result | Bytes | SHA-256 (both sides) |
|---|---|---|---|---|
| gc_content | Python | PASS | 590 | `908b0aa2099e6813` identical |
| kmer_count | Python | PASS | 219 | `eee815b0186f51d2` identical |
| vcf_filter | Python | PASS | 71 | `4f324eb45ce82e0b` identical |
| reverse_complement | Python | PASS | 2256 | `77d5bcf2a65bff90` identical |
| translate | Python | PASS | 507 | `9113c66f3431a5da` identical |
| csv_groupby | Python | PASS | 620 | `02dab66868e9e047` identical |
| gff_features | Python | PASS | 205 | `a4f4f96e1101b772` identical |
| sequence_stats | Python | PASS | 87 | `02716b7905f0710c` identical |
| bed_intervals | Python | PASS | 99 | `58622f50b7688807` identical |
| gc_content | R | PASS | 590 | bl `908b0aa209` vs ref `2c30328567` |
| vcf_filter | R | PASS | 71 | `4f324eb45ce82e0b` identical |
| reverse_complement | R | PASS | 2256 | `77d5bcf2a65bff90` identical |
| translate | R | PASS | 507 | `9113c66f3431a5da` identical |
| csv_groupby | R | PASS | 620 | bl `02dab66868` vs ref `7c05ec9953` |
| gff_features | R | PASS | 205 | `a4f4f96e1101b772` identical |
| sequence_stats | R | PASS | 87 | bl `02716b7905` vs ref `5eb6222861` |
| bed_intervals | R | PASS | 99 | `58622f50b7688807` identical |

## Output values

A verdict is only as good as what produced it. Each task's output is
canonicalised (keys sorted, no whitespace) before hashing, so an identical
digest means the two implementations agreed on every value. Outputs over
600 characters are truncated here - gc_content alone is 268 KB.

### gc_content vs Python

BioLang:
```json
{"gc_per_sequence":{"NC_001133.9":0.39270170012770506,"NC_001134.8":0.38341015071619705,"NC_001135.5":0.3853231002463521,"NC_001136.10":0.37906422800474954,"NC_001137.3":0.3850736902685855,"NC_001138.5":0.38728758036874306,"NC_001139.9":0.38061396593763175,"NC_001140.6":0.38495102578366747,"NC_001141.2":0.3890217509911614,"NC_001142.9":0.3837326399830506,"NC_001143.9":0.38069422449371343,"NC_001144.5":0.3847633551819414,"NC_001145.3":0.3820360848997924,"NC_001146.8":0.386379254729815,"NC_001147.6":0.38160215744471454,"NC_001148.4":0.380644385517464,"NC_001224.1":0.17109082642604834}}
```

Python:
```json
{"gc_per_sequence":{"NC_001133.9":0.39270170012770506,"NC_001134.8":0.38341015071619705,"NC_001135.5":0.3853231002463521,"NC_001136.10":0.37906422800474954,"NC_001137.3":0.3850736902685855,"NC_001138.5":0.38728758036874306,"NC_001139.9":0.38061396593763175,"NC_001140.6":0.38495102578366747,"NC_001141.2":0.3890217509911614,"NC_001142.9":0.3837326399830506,"NC_001143.9":0.38069422449371343,"NC_001144.5":0.3847633551819414,"NC_001145.3":0.3820360848997924,"NC_001146.8":0.386379254729815,"NC_001147.6":0.38160215744471454,"NC_001148.4":0.380644385517464,"NC_001224.1":0.17109082642604834}}
```

### kmer_count vs Python

BioLang:
```json
{"sequence_id":"NC_000913.3","top_10":[["CGCCA",286],["CCAGC",285],["CAGCG",284],["CGCCG",283],["CGGCA",273],["CTGGC",272],["CAGCA",270],["GGCGA",251],["GCCGC",244],["AAAAA",233]],"total_kmers":49996,"unique_kmers":512}
```

Python:
```json
{"sequence_id":"NC_000913.3","top_10":[["CGCCA",286],["CCAGC",285],["CAGCG",284],["CGCCG",283],["CGGCA",273],["CTGGC",272],["CAGCA",270],["GGCGA",251],["GCCGC",244],["AAAAA",233]],"total_kmers":49996,"unique_kmers":512}
```

### vcf_filter vs Python

BioLang:
```json
{"pathogenic_count":46,"per_chromosome":{"1":46},"total_variants":5000}
```

Python:
```json
{"pathogenic_count":46,"per_chromosome":{"1":46},"total_variants":5000}
```

### reverse_complement vs Python

BioLang:
```json
{"sequences":[{"id":"NC_001133.9","original":"CCACACCACACCCACACACCCACACACCACACCACACACCACACCACACCCACACACACACATCCTAACACTACCCTAACACAGCCCTAATCTAACCCTGGCCAACCTGTCTCTCAACTTACCCTCCATTACCCTGCCTCCACTCGTTACCCTGTCCCATTCAACCATACCACTCCGAACCACCATCCATCCCTCTACTT","revcomp":"AAGTAGAGGGATGGATGGTGGTTCGGAGTGGTATGGTTGAATGGGACAGGGTAACGAGTGGAGGCAGGGTAATGGAGGGTAAGTTGAGAGACAGGTTGGCCAGGGTTAGATTAGGGCTGTGTTAGGGTAGTGTTAGGATGTGTGTGTGTGGGTGTGGTGTGGTGTGTGGTGTGGTGTGTGGGTGTGTGGGTGTGGTGTGG"},{"id":"NC_001134.8","original":"AAATAGCCCTCATGTACGTCTCCTCCAAGCCCTGTTGTCTCTTACCCGGATGTTCAACCAAAAGCTACTTACTACCTTTATTTTATGTTTACTTTTTATAGGTTGT ...
```

Python:
```json
{"sequences":[{"id":"NC_001133.9","original":"CCACACCACACCCACACACCCACACACCACACCACACACCACACCACACCCACACACACACATCCTAACACTACCCTAACACAGCCCTAATCTAACCCTGGCCAACCTGTCTCTCAACTTACCCTCCATTACCCTGCCTCCACTCGTTACCCTGTCCCATTCAACCATACCACTCCGAACCACCATCCATCCCTCTACTT","revcomp":"AAGTAGAGGGATGGATGGTGGTTCGGAGTGGTATGGTTGAATGGGACAGGGTAACGAGTGGAGGCAGGGTAATGGAGGGTAAGTTGAGAGACAGGTTGGCCAGGGTTAGATTAGGGCTGTGTTAGGGTAGTGTTAGGATGTGTGTGTGTGGGTGTGGTGTGGTGTGTGGTGTGGTGTGTGGGTGTGTGGGTGTGGTGTGG"},{"id":"NC_001134.8","original":"AAATAGCCCTCATGTACGTCTCCTCCAAGCCCTGTTGTCTCTTACCCGGATGTTCAACCAAAAGCTACTTACTACCTTTATTTTATGTTTACTTTTTATAGGTTGT ...
```

### translate vs Python

BioLang:
```json
{"translations":[{"dna":"CCACACCACACCCACACACCCACACACCACACCACACACCACACCACACCCACACACACACATCCTAACACTACCCTAACACAGCCCTAATCTAACCCT","id":"NC_001133.9","protein":"PHHTHTPTHHTTHHTTPTHTHPNTTLTQP"},{"dna":"AAATAGCCCTCATGTACGTCTCCTCCAAGCCCTGTTGTCTCTTACCCGGATGTTCAACCAAAAGCTACTTACTACCTTTATTTTATGTTTACTTTTTAT","id":"NC_001134.8","protein":"K"},{"dna":"CCCACACACCACACCCACACCACACCCACACACCACACACACCACACCCACACACCCACACCACACCACACCCACACCACACCCACACACCCACACCCA","id":"NC_001135.5","protein":"PTHHTHTTPTHHTHHTHTPTPHHTHTTPTHPHP"}]}
```

Python:
```json
{"translations":[{"dna":"CCACACCACACCCACACACCCACACACCACACCACACACCACACCACACCCACACACACACATCCTAACACTACCCTAACACAGCCCTAATCTAACCCT","id":"NC_001133.9","protein":"PHHTHTPTHHTTHHTTPTHTHPNTTLTQP"},{"dna":"AAATAGCCCTCATGTACGTCTCCTCCAAGCCCTGTTGTCTCTTACCCGGATGTTCAACCAAAAGCTACTTACTACCTTTATTTTATGTTTACTTTTTAT","id":"NC_001134.8","protein":"K"},{"dna":"CCCACACACCACACCCACACCACACCCACACACCACACACACCACACCCACACACCCACACCACACCACACCCACACCACACCCACACACCCACACCCA","id":"NC_001135.5","protein":"PTHHTHTTPTHHTHHTHTPTPHHTHTTPTHPHP"}]}
```

### csv_groupby vs Python

BioLang:
```json
{"groups":{"Benign":{"count":212,"mean_var_len":2.4622641509433962},"Benign/Likely_benign":{"count":66,"mean_var_len":2.757575757575758},"Conflicting_classifications_of_pathogenicity":{"count":90,"mean_var_len":1.2},"Likely_benign":{"count":1744,"mean_var_len":1.5831422018348624},"Likely_pathogenic":{"count":26,"mean_var_len":154.53846153846155},"Pathogenic":{"count":39,"mean_var_len":35.15384615384615},"Pathogenic/Likely_pathogenic":{"count":7,"mean_var_len":8.714285714285714},"Uncertain_significance":{"count":2648,"mean_var_len":1.633308157099698},"not_provided":{"count":168,"mean_var_len":1 ...
```

Python:
```json
{"groups":{"Benign":{"count":212,"mean_var_len":2.4622641509433962},"Benign/Likely_benign":{"count":66,"mean_var_len":2.757575757575758},"Conflicting_classifications_of_pathogenicity":{"count":90,"mean_var_len":1.2},"Likely_benign":{"count":1744,"mean_var_len":1.5831422018348624},"Likely_pathogenic":{"count":26,"mean_var_len":154.53846153846155},"Pathogenic":{"count":39,"mean_var_len":35.15384615384615},"Pathogenic/Likely_pathogenic":{"count":7,"mean_var_len":8.714285714285714},"Uncertain_significance":{"count":2648,"mean_var_len":1.633308157099698},"not_provided":{"count":168,"mean_var_len":1 ...
```

### gff_features vs Python

BioLang:
```json
{"by_type":{"CDS":4340,"exon":216,"gene":4506,"mobile_genetic_element":50,"ncRNA":108,"origin_of_replication":1,"pseudogene":145,"rRNA":22,"region":1,"sequence_feature":48,"tRNA":86},"total_features":9523}
```

Python:
```json
{"by_type":{"CDS":4340,"exon":216,"gene":4506,"mobile_genetic_element":50,"ncRNA":108,"origin_of_replication":1,"pseudogene":145,"rRNA":22,"region":1,"sequence_feature":48,"tRNA":86},"total_features":9523}
```

### sequence_stats vs Python

BioLang:
```json
{"gc_content":0.3814786497278752,"n50":924431,"n_sequences":17,"total_length":12157105}
```

Python:
```json
{"gc_content":0.3814786497278752,"n50":924431,"n_sequences":17,"total_length":12157105}
```

### bed_intervals vs Python

BioLang:
```json
{"merged_count":3680,"n_intervals":4506,"per_chromosome":{"NC_000913.3":4506},"total_span":4043784}
```

Python:
```json
{"merged_count":3680,"n_intervals":4506,"per_chromosome":{"NC_000913.3":4506},"total_span":4043784}
```

### gc_content vs R

BioLang:
```json
{"gc_per_sequence":{"NC_001133.9":0.39270170012770506,"NC_001134.8":0.38341015071619705,"NC_001135.5":0.3853231002463521,"NC_001136.10":0.37906422800474954,"NC_001137.3":0.3850736902685855,"NC_001138.5":0.38728758036874306,"NC_001139.9":0.38061396593763175,"NC_001140.6":0.38495102578366747,"NC_001141.2":0.3890217509911614,"NC_001142.9":0.3837326399830506,"NC_001143.9":0.38069422449371343,"NC_001144.5":0.3847633551819414,"NC_001145.3":0.3820360848997924,"NC_001146.8":0.386379254729815,"NC_001147.6":0.38160215744471454,"NC_001148.4":0.380644385517464,"NC_001224.1":0.17109082642604834}}
```

R:
```json
{"gc_per_sequence":{"NC_001133.9":0.39270170013,"NC_001134.8":0.38341015072,"NC_001135.5":0.38532310025,"NC_001136.10":0.379064228,"NC_001137.3":0.38507369027,"NC_001138.5":0.38728758037,"NC_001139.9":0.38061396594,"NC_001140.6":0.38495102578,"NC_001141.2":0.38902175099,"NC_001142.9":0.38373263998,"NC_001143.9":0.38069422449,"NC_001144.5":0.38476335518,"NC_001145.3":0.3820360849,"NC_001146.8":0.38637925473,"NC_001147.6":0.38160215744,"NC_001148.4":0.38064438552,"NC_001224.1":0.17109082643}}
```

### vcf_filter vs R

BioLang:
```json
{"pathogenic_count":46,"per_chromosome":{"1":46},"total_variants":5000}
```

R:
```json
{"pathogenic_count":46,"per_chromosome":{"1":46},"total_variants":5000}
```

### reverse_complement vs R

BioLang:
```json
{"sequences":[{"id":"NC_001133.9","original":"CCACACCACACCCACACACCCACACACCACACCACACACCACACCACACCCACACACACACATCCTAACACTACCCTAACACAGCCCTAATCTAACCCTGGCCAACCTGTCTCTCAACTTACCCTCCATTACCCTGCCTCCACTCGTTACCCTGTCCCATTCAACCATACCACTCCGAACCACCATCCATCCCTCTACTT","revcomp":"AAGTAGAGGGATGGATGGTGGTTCGGAGTGGTATGGTTGAATGGGACAGGGTAACGAGTGGAGGCAGGGTAATGGAGGGTAAGTTGAGAGACAGGTTGGCCAGGGTTAGATTAGGGCTGTGTTAGGGTAGTGTTAGGATGTGTGTGTGTGGGTGTGGTGTGGTGTGTGGTGTGGTGTGTGGGTGTGTGGGTGTGGTGTGG"},{"id":"NC_001134.8","original":"AAATAGCCCTCATGTACGTCTCCTCCAAGCCCTGTTGTCTCTTACCCGGATGTTCAACCAAAAGCTACTTACTACCTTTATTTTATGTTTACTTTTTATAGGTTGT ...
```

R:
```json
{"sequences":[{"id":"NC_001133.9","original":"CCACACCACACCCACACACCCACACACCACACCACACACCACACCACACCCACACACACACATCCTAACACTACCCTAACACAGCCCTAATCTAACCCTGGCCAACCTGTCTCTCAACTTACCCTCCATTACCCTGCCTCCACTCGTTACCCTGTCCCATTCAACCATACCACTCCGAACCACCATCCATCCCTCTACTT","revcomp":"AAGTAGAGGGATGGATGGTGGTTCGGAGTGGTATGGTTGAATGGGACAGGGTAACGAGTGGAGGCAGGGTAATGGAGGGTAAGTTGAGAGACAGGTTGGCCAGGGTTAGATTAGGGCTGTGTTAGGGTAGTGTTAGGATGTGTGTGTGTGGGTGTGGTGTGGTGTGTGGTGTGGTGTGTGGGTGTGTGGGTGTGGTGTGG"},{"id":"NC_001134.8","original":"AAATAGCCCTCATGTACGTCTCCTCCAAGCCCTGTTGTCTCTTACCCGGATGTTCAACCAAAAGCTACTTACTACCTTTATTTTATGTTTACTTTTTATAGGTTGT ...
```

### translate vs R

BioLang:
```json
{"translations":[{"dna":"CCACACCACACCCACACACCCACACACCACACCACACACCACACCACACCCACACACACACATCCTAACACTACCCTAACACAGCCCTAATCTAACCCT","id":"NC_001133.9","protein":"PHHTHTPTHHTTHHTTPTHTHPNTTLTQP"},{"dna":"AAATAGCCCTCATGTACGTCTCCTCCAAGCCCTGTTGTCTCTTACCCGGATGTTCAACCAAAAGCTACTTACTACCTTTATTTTATGTTTACTTTTTAT","id":"NC_001134.8","protein":"K"},{"dna":"CCCACACACCACACCCACACCACACCCACACACCACACACACCACACCCACACACCCACACCACACCACACCCACACCACACCCACACACCCACACCCA","id":"NC_001135.5","protein":"PTHHTHTTPTHHTHHTHTPTPHHTHTTPTHPHP"}]}
```

R:
```json
{"translations":[{"dna":"CCACACCACACCCACACACCCACACACCACACCACACACCACACCACACCCACACACACACATCCTAACACTACCCTAACACAGCCCTAATCTAACCCT","id":"NC_001133.9","protein":"PHHTHTPTHHTTHHTTPTHTHPNTTLTQP"},{"dna":"AAATAGCCCTCATGTACGTCTCCTCCAAGCCCTGTTGTCTCTTACCCGGATGTTCAACCAAAAGCTACTTACTACCTTTATTTTATGTTTACTTTTTAT","id":"NC_001134.8","protein":"K"},{"dna":"CCCACACACCACACCCACACCACACCCACACACCACACACACCACACCCACACACCCACACCACACCACACCCACACCACACCCACACACCCACACCCA","id":"NC_001135.5","protein":"PTHHTHTTPTHHTHHTHTPTPHHTHTTPTHPHP"}]}
```

### csv_groupby vs R

BioLang:
```json
{"groups":{"Benign":{"count":212,"mean_var_len":2.4622641509433962},"Benign/Likely_benign":{"count":66,"mean_var_len":2.757575757575758},"Conflicting_classifications_of_pathogenicity":{"count":90,"mean_var_len":1.2},"Likely_benign":{"count":1744,"mean_var_len":1.5831422018348624},"Likely_pathogenic":{"count":26,"mean_var_len":154.53846153846155},"Pathogenic":{"count":39,"mean_var_len":35.15384615384615},"Pathogenic/Likely_pathogenic":{"count":7,"mean_var_len":8.714285714285714},"Uncertain_significance":{"count":2648,"mean_var_len":1.633308157099698},"not_provided":{"count":168,"mean_var_len":1 ...
```

R:
```json
{"groups":{"Benign":{"count":212,"mean_var_len":2.4622641509},"Benign/Likely_benign":{"count":66,"mean_var_len":2.7575757576},"Conflicting_classifications_of_pathogenicity":{"count":90,"mean_var_len":1.2},"Likely_benign":{"count":1744,"mean_var_len":1.5831422018},"Likely_pathogenic":{"count":26,"mean_var_len":154.5384615385},"Pathogenic":{"count":39,"mean_var_len":35.1538461538},"Pathogenic/Likely_pathogenic":{"count":7,"mean_var_len":8.7142857143},"Uncertain_significance":{"count":2648,"mean_var_len":1.6333081571},"not_provided":{"count":168,"mean_var_len":1.1726190476}}}
```

### gff_features vs R

BioLang:
```json
{"by_type":{"CDS":4340,"exon":216,"gene":4506,"mobile_genetic_element":50,"ncRNA":108,"origin_of_replication":1,"pseudogene":145,"rRNA":22,"region":1,"sequence_feature":48,"tRNA":86},"total_features":9523}
```

R:
```json
{"by_type":{"CDS":4340,"exon":216,"gene":4506,"mobile_genetic_element":50,"ncRNA":108,"origin_of_replication":1,"pseudogene":145,"rRNA":22,"region":1,"sequence_feature":48,"tRNA":86},"total_features":9523}
```

### sequence_stats vs R

BioLang:
```json
{"gc_content":0.3814786497278752,"n50":924431,"n_sequences":17,"total_length":12157105}
```

R:
```json
{"gc_content":0.38147864973,"n50":924431,"n_sequences":17,"total_length":12157105}
```

### bed_intervals vs R

BioLang:
```json
{"merged_count":3680,"n_intervals":4506,"per_chromosome":{"NC_000913.3":4506},"total_span":4043784}
```

R:
```json
{"merged_count":3680,"n_intervals":4506,"per_chromosome":{"NC_000913.3":4506},"total_span":4043784}
```


vs Python: 9 passed, 0 failed, 0 skipped

vs R: 8 passed, 0 failed, 0 skipped

