"""Integer peptide mass, cyclopeptide-sequencing convention. Output JSON."""
import json
import sys

# BioLang's peptide_mass() sums the *integer* amino acid masses used throughout
# the cyclopeptide sequencing literature (Compeau & Pevzner), not monoisotopic
# masses. The two differ: SKADYEK is 821 on this table and 821.392
# monoisotopic, and the gap widens with length because every residue is a whole
# number here by definition rather than by rounding.
#
# BioPython is deliberately not used as the reference for this task. Its
# molecular_weight() implements the monoisotopic convention, so comparing
# against it would test whether two different questions happen to give the same
# answer. This is an independent transcription of the same published table,
# which checks the table and the summation.
INTEGER_MASSES = {
    "G": 57, "A": 71, "S": 87, "P": 97, "V": 99, "T": 101, "C": 103,
    "I": 113, "L": 113, "N": 114, "D": 115, "K": 128, "Q": 128, "E": 129,
    "M": 131, "H": 137, "F": 147, "R": 156, "Y": 163, "W": 186,
}

peptides = ["SKADYEK", "MTEITAAMVKELRESTGAGMMDCK", "WYVGA", "PEPTIDEK"]

results = [{"peptide": p, "mass": sum(INTEGER_MASSES[c] for c in p)}
           for p in peptides]

json.dump({"peptides": results}, sys.stdout, indent=2)
print()
