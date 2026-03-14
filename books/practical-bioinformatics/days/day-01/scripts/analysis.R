# Day 1: What Is Bioinformatics? — R (Biostrings) equivalent

library(Biostrings)

# 1. Sequence basics
seq <- DNAString("ATGCGATCGATCGATCG")
cat(sprintf("Sequence: %s\n", as.character(seq)))
cat(sprintf("Length: %d bases\n", length(seq)))
cat(sprintf("Type: %s\n", class(seq)))

# 2. Central dogma
gene <- DNAString("ATGAAACCCGGGTTTTAA")
cat(sprintf("DNA:     %s\n", as.character(gene)))
mrna <- RNAString(gsub("T", "U", as.character(gene)))
cat(sprintf("RNA:     %s\n", as.character(mrna)))
protein <- translate(gene)
# Remove trailing stop
prot_str <- sub("\\*$", "", as.character(protein))
cat(sprintf("Protein: %s\n", prot_str))

# 3. Base composition
fragment <- DNAString("ATGCGATCGATCGAATTCGATCG")
counts <- alphabetFrequency(fragment)[c("A", "C", "G", "T")]
cat(sprintf("Base composition: A=%d T=%d G=%d C=%d\n",
            counts["A"], counts["T"], counts["G"], counts["C"]))
gc <- letterFrequency(fragment, letters = "GC", as.prob = TRUE)
cat(sprintf("GC content: %f\n", gc))

# 4. Motif search
target <- DNAString("ATCGATCGAATTCGATCGATCG")
motif <- DNAString("GAATTC")
matches <- matchPattern(motif, target)
positions <- start(matches) - 1  # 0-based to match BioLang
cat(sprintf("EcoRI sites: [%s]\n", paste(positions, collapse = ", ")))

# 5. Complement and reverse complement
seq2 <- DNAString("ATGCGATCGATCG")
comp <- complement(seq2)
revcomp <- reverseComplement(comp)
rna_result <- RNAString(gsub("T", "U", as.character(revcomp)))
cat(sprintf("Piped result: %s\n", as.character(rna_result)))

# 6. Exercises
ex2 <- translate(DNAString("ATGGATCCCTAA"))
cat(sprintf("Ex2: %s\n", sub("\\*$", "", as.character(ex2))))
ex3 <- alphabetFrequency(DNAString("AAAAATTTTTCCCCCGGGGG"))[c("A", "T", "G", "C")]
cat(sprintf("Ex3: A=%d T=%d G=%d C=%d\n", ex3["A"], ex3["T"], ex3["G"], ex3["C"]))
ex4_seq <- DNAString("ATGATGATGATG")
ex4_matches <- matchPattern(DNAString("ATG"), ex4_seq)
cat(sprintf("Ex4: [%s]\n", paste(start(ex4_matches) - 1, collapse = ", ")))
