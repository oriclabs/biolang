# One-liner correctness

48 cases. Values are compared as JSON: floats to 1e-9, integers and strings exactly.

`DIFFER` marks a known convention difference, recorded on purpose; such a case fails if it ever starts agreeing.

| Case | Category | BioLang | Python | R | vs Py | vs R |
|---|---|---|---|---|---|---|
| gc_content_even | bio | `0.5` | `0.5` | `0.5` | PASS | PASS |
| gc_content_gc_rich | bio | `1.0` | `1.0` | `1` | PASS | PASS |
| gc_content_at_only | bio | `0.0` | `0.0` | `0` | PASS | PASS |
| reverse_complement | bio | `"ACGTACGT"` | `"ACGTACGT"` | `"ACGTACGT"` | PASS | PASS |
| reverse_complement_palindrome | bio | `"GAATTC"` | `"GAATTC"` | `"GAATTC"` | PASS | PASS |
| complement | bio | `"TGCATGCA"` | `"TGCATGCA"` | `"TGCATGCA"` | PASS | PASS |
| transcribe | bio | `"ACGUACGU"` | `"ACGUACGU"` | `"ACGUACGU"` | PASS | PASS |
| translate_simple | bio | `"MAIV"` | `"MAIV"` | `"MAIV"` | PASS | PASS |
| translate_start | bio | `"MF"` | `"MF"` | `"MF"` | PASS | PASS |
| seq_len | bio | `10` | `10` | `10` | PASS | PASS |
| hamming_zero | bio | `0` | `0` | `0` | PASS | PASS |
| hamming_three | bio | `7` | `7` | `7` | PASS | PASS |
| edit_distance_classic | bio | `3` | `3` | `3` | PASS | PASS |
| edit_distance_identical | bio | `0` | `0` | `0` | PASS | PASS |
| edit_distance_empty | bio | `3` | `3` | `3` | PASS | PASS |
| melting_temp_wallace | bio | `36.0` | `36` | - | PASS | SKIP |
| mean_ints | stats | `2.5` | `2.5` | `2.5` | PASS | PASS |
| mean_negative | stats | `0.0` | `0` | `0` | PASS | PASS |
| median_odd | stats | `2.0` | `2` | `2` | PASS | PASS |
| median_even | stats | `2.5` | `2.5` | `2.5` | PASS | PASS |
| stdev_sample | stats | `2.138089935` | `2.138089935` | `2.138089935` | PASS | PASS |
| variance_sample | stats | `4.571428571` | `4.571428571` | `4.571428571` | PASS | PASS |
| sum_floats | math | `7.0` | `7.0` | `7` | PASS | PASS |
| min_list | math | `2.0` | `2` | `2` | PASS | PASS |
| max_list | math | `9.0` | `9` | `9` | PASS | PASS |
| abs_negative | math | `7.5` | `7.5` | `7.5` | PASS | PASS |
| sqrt_two | math | `1.414213562` | `1.414213562` | `1.414213562` | PASS | PASS |
| log_natural | math | `2.302585093` | `2.302585093` | `2.302585093` | PASS | PASS |
| pow_int | math | `1024.0` | `1024` | `1024` | PASS | PASS |
| round_half_tie | math | `3.0` | `2` | `2` | DIFFER | DIFFER |
| round_binary_repr | math | `2.68` | `2.67` | `2.67` | DIFFER | DIFFER |
| floor_value | math | `3` | `3` | `3` | PASS | PASS |
| ceil_value | math | `4` | `4` | `4` | PASS | PASS |
| upper_case | string | `"ACGT"` | `"ACGT"` | `"ACGT"` | PASS | PASS |
| lower_case | string | `"acgt"` | `"acgt"` | `"acgt"` | PASS | PASS |
| trim_spaces | string | `"hello"` | `"hello"` | `"hello"` | PASS | PASS |
| substr_mid | string | `"CDE"` | `"CDE"` | `"CDE"` | PASS | PASS |
| starts_with_true | string | `true` | `true` | `true` | PASS | PASS |
| contains_substr | string | `true` | `true` | `true` | PASS | PASS |
| split_count | string | `4` | `4` | `4` | PASS | PASS |
| join_strings | string | `"a-b-c"` | `"a-b-c"` | `"a-b-c"` | PASS | PASS |
| replace_all | string | `"bonono"` | `"bonono"` | `"bonono"` | PASS | PASS |
| list_len | list | `5` | `5` | `5` | PASS | PASS |
| list_sorted | list | `[1, 2, 3]` | `[1, 2, 3]` | `[1, 2, 3]` | PASS | PASS |
| list_reversed | list | `[3, 2, 1]` | `[3, 2, 1]` | `[3, 2, 1]` | PASS | PASS |
| list_unique | list | `3` | `3` | `3` | PASS | PASS |
| kmers_count | kmer | `6` | `6` | `6` | PASS | PASS |
| kmer_distinct_count | kmer | `1` | `1` | `1` | PASS | PASS |

vs Python: 48 passed, 0 failed

vs R: 47 passed, 0 failed, 1 skipped
