# Appendix B: Glossary

This glossary covers the biology, programming, and bioinformatics terms used throughout this book. Each entry references the day(s) where the concept is introduced or used most heavily.

---

**Alignment** — The process of arranging two or more sequences to identify regions of similarity. Alignment reveals evolutionary relationships, functional regions, and mutations. *Days 11, 12, 20*

**Allele** — One of two or more versions of a gene or genetic variant at a particular position in the genome. For example, a SNP might have a reference allele "A" and an alternate allele "G". *Days 12, 28*

**Amino acid** — The building blocks of proteins. There are 20 standard amino acids, each encoded by one or more three-letter codons in the genetic code. Represented by single-letter codes (e.g., M for methionine, A for alanine). *Days 1, 3, 17*

**Annotation** — Metadata attached to a genomic feature — what a region of DNA does, what gene it belongs to, what protein it encodes. Stored in GFF/GTF files. *Days 7, 18*

**API (Application Programming Interface)** — A structured way for programs to request data from a service. In bioinformatics, APIs provide programmatic access to databases like NCBI, Ensembl, and UniProt. *Days 9, 24*

**BAM (Binary Alignment Map)** — A compressed binary format for storing sequence alignment data. The binary counterpart of SAM. Requires an index (.bai) for random access. *Days 7, 12*

**Base pair (bp)** — A single unit of DNA consisting of two complementary nucleotides bonded together (A-T or C-G). Genome sizes are measured in base pairs: the human genome is approximately 3.2 billion bp. *Days 1, 3*

**BED (Browser Extensible Data)** — A tab-delimited file format for defining genomic regions. Each line specifies a chromosome, start position, and end position. Uses zero-based, half-open coordinates. *Days 7, 18*

**Bioinformatics** — The interdisciplinary field that develops methods and software for understanding biological data, particularly molecular biology data like DNA, RNA, and protein sequences. *Day 1*

**BLAST (Basic Local Alignment Search Tool)** — An algorithm for comparing sequences against a database to find similar sequences. One of the most widely used tools in bioinformatics. *Day 11*

**Builtin** — A function that is available in BioLang without importing anything. Examples include `gc_content`, `read_fasta`, and `println`. *Day 2*

**Categorical variable** — A variable that takes on a limited number of discrete values, such as tissue type or experimental condition. Contrast with continuous variables like expression levels or quality scores. *Days 10, 14*

**Chromosome** — A long, continuous piece of DNA containing many genes. Humans have 23 pairs of chromosomes (22 autosomes plus X/Y sex chromosomes). *Days 1, 3, 18*

**Closure** — A function that captures variables from its surrounding scope. In BioLang, closures are written as `|params| expression`. Also called a lambda. *Days 4, 6*

**Codon** — A sequence of three nucleotides that encodes a single amino acid (or a stop signal) during translation. For example, ATG encodes methionine and also serves as the start codon. *Days 1, 3, 17*

**Complement** — The matching strand of a DNA sequence, determined by base pairing rules: A pairs with T, C pairs with G. The complement of ATGC is TACG. *Days 3, 5*

**Contig** — A contiguous sequence of DNA assembled from overlapping reads. Genome assemblies consist of many contigs ordered into scaffolds and chromosomes. *Days 11, 20*

**Control flow** — Programming constructs that determine the order of execution: `if`/`else`, `for` loops, `while` loops. *Day 4*

**Coverage (Depth)** — The average number of times each base in the genome is read by sequencing. Higher coverage means higher confidence. Whole-genome sequencing typically targets 30x coverage. *Days 6, 12*

**CRAM** — A highly compressed file format for sequence alignments, more space-efficient than BAM. Uses reference-based compression. *Day 7*

**CSV (Comma-Separated Values)** — A plain-text tabular file format where columns are separated by commas. Widely used for sharing data between tools and languages. Read in BioLang with `read_csv`. *Days 10, 22*

**DE (Differential Expression)** — The statistical identification of genes that are expressed at significantly different levels between two or more conditions (e.g., tumor vs. normal tissue). *Days 13, 29*

**DNA (Deoxyribonucleic Acid)** — The molecule that carries genetic information in all living organisms. Composed of four nucleotide bases: adenine (A), thymine (T), cytosine (C), and guanine (G). *Days 1, 3*

**Enrichment analysis** — A statistical method for determining whether a predefined set of genes (e.g., a Gene Ontology category or KEGG pathway) is overrepresented in a list of genes of interest. *Day 16*

**Exome** — The portion of the genome that codes for proteins, comprising roughly 1-2% of the total genome. Whole-exome sequencing (WES) targets only these regions. *Days 12, 28*

**Exon** — A segment of a gene that is represented in the mature RNA after splicing. Exons contain the coding sequence that is translated into protein. *Days 3, 7, 18*

**False discovery rate (FDR)** — A method of correcting for multiple hypothesis testing. When thousands of genes are tested simultaneously, some will appear significant by chance. FDR controls the expected proportion of false positives among the rejected hypotheses. The Benjamini-Hochberg method is the most common FDR correction. *Days 14, 16*

**FASTA** — A text-based file format for representing nucleotide or protein sequences. Each entry has a header line starting with `>` followed by sequence lines. *Days 5, 6, 7*

**FASTQ** — An extension of FASTA that includes quality scores for each base. The standard output format of most sequencing instruments. Each record has four lines: header, sequence, separator, and quality string. *Days 6, 7, 8*

**Feature** — A defined region of a biological sequence with a specific function or annotation. Features include genes, exons, introns, promoters, and regulatory elements. Stored in GFF/GTF format. *Days 7, 18*

**Fold change** — The ratio of expression levels between two conditions. A fold change of 2 means a gene is expressed twice as much in one condition vs. the other. Often reported as log2 fold change. *Days 13, 14, 29*

**Frameshift** — A mutation caused by an insertion or deletion of nucleotides that is not a multiple of three, disrupting the reading frame. Frameshifts typically produce a truncated or nonfunctional protein. *Days 12, 28*

**Function** — A named, reusable block of code that takes inputs (parameters) and returns an output. In BioLang, defined with `let name = fn(params) { body }`. *Day 4*

**GC content** — The proportion of bases in a DNA sequence that are guanine (G) or cytosine (C). GC content affects DNA stability, gene density, and sequencing bias. *Days 1, 2, 5, 6*

**Gene** — A segment of DNA that encodes a functional product, typically a protein or RNA molecule. The human genome contains approximately 20,000 protein-coding genes. *Days 1, 3*

**Gene Ontology (GO)** — A standardized vocabulary for describing gene functions across three categories: molecular function, biological process, and cellular component. Used in enrichment analysis. *Days 16, 24*

**Genome** — The complete set of DNA in an organism. The human genome is approximately 3.2 billion base pairs. Reference genomes (like GRCh38) serve as the coordinate system for genomic analyses. *Days 1, 3*

**GFF/GTF (General Feature Format / Gene Transfer Format)** — File formats for describing genomic features (genes, exons, transcripts) with their coordinates and attributes. GFF3 is the current standard; GTF is a specialized variant used for gene annotations. *Days 7, 18*

**GWAS (Genome-Wide Association Study)** — A study that scans the entire genome for statistical associations between genetic variants and traits or diseases. Typically involves thousands to millions of participants. *Day 12*

**Haplotype** — A set of genetic variants that are inherited together on the same chromosome. Important for understanding genetic linkage and population structure. *Day 12*

**Higher-Order Function (HOF)** — A function that takes another function as an argument or returns a function. `map`, `filter`, and `reduce` are the most common HOFs in BioLang. *Days 4, 5, 8*

**Homolog** — A gene related to another gene by shared ancestry. Homologs can be orthologs (separated by speciation) or paralogs (separated by duplication). *Day 20*

**Illumina** — The dominant next-generation sequencing technology, producing short reads (typically 100-300 bp) with high accuracy (>99.9%). Most FASTQ files encountered in bioinformatics come from Illumina instruments. *Days 1, 6*

**Indel** — An insertion or deletion of one or more bases in a DNA sequence relative to a reference. Indels can cause frameshifts if they are not multiples of three bases. *Days 12, 28*

**Index** — A pre-computed data structure that enables fast random access to records within a large file. BAM files use .bai indexes; tabix creates .tbi indexes for VCF and BED files. Without an index, accessing a specific region requires reading the entire file. *Days 7, 8*

**Interval** — A genomic region defined by a chromosome, start position, and end position. In BioLang, intervals are a native type created with `interval("chr1", 100, 200)`. Interval arithmetic (intersection, union, subtraction) is fundamental to genomic analysis. *Day 18*

**Intron** — A segment of a gene that is removed (spliced out) from the RNA transcript before translation. Introns do not code for protein. *Days 3, 7*

**Isoform** — One of several variant forms of a protein, produced by alternative splicing of the same gene. Different isoforms can have distinct functions, tissue distributions, and disease associations. *Days 3, 13*

**k-mer** — A subsequence of length k from a larger sequence. k-mer analysis is used for genome assembly, error correction, and sequence comparison without alignment. *Days 5, 11*

**Lambda** — See *Closure*. A shorthand term for an anonymous function. In BioLang: `|x| x * 2`. *Days 4, 5*

**List** — An ordered collection of values. In BioLang, written as `[1, 2, 3]` or `["A", "B", "C"]`. Lists support indexing, slicing, and higher-order functions. *Days 4, 5*

**Locus (plural: Loci)** — A specific position or region on a chromosome. Can refer to a single base position (a SNP locus) or a larger region (a gene locus). *Days 12, 18*

**MAF (Minor Allele Frequency)** — The frequency of the second most common allele at a given locus in a population. Used to distinguish common variants (MAF > 1%) from rare variants. *Days 12, 28*

**Mapping quality** — A score indicating the confidence that a read has been aligned to the correct position in the reference genome. Higher scores indicate more unique mappings. Often on a Phred scale. *Days 7, 12*

**Motif** — A short, conserved sequence pattern that has biological significance. Examples include transcription factor binding sites, splice sites, and the Kozak consensus sequence. *Days 5, 11, 17*

**Mutation** — A change in the DNA sequence. Mutations include single-base substitutions (SNPs), insertions, deletions, and larger structural changes. *Days 1, 12*

**Normalization** — The process of adjusting raw data to account for systematic biases. In RNA-seq, normalization corrects for differences in sequencing depth and gene length. Common methods include TPM, FPKM, and DESeq2's median-of-ratios. *Days 13, 14*

**Nucleotide** — The basic building block of DNA and RNA. DNA nucleotides contain one of four bases (A, T, C, G) plus a sugar and phosphate group. RNA uses uracil (U) instead of thymine (T). *Days 1, 3*

**Null hypothesis** — The default assumption in a statistical test — typically that there is no difference between groups or no association between variables. Statistical tests compute the probability (p-value) of the data under this assumption. *Day 14*

**Open Reading Frame (ORF)** — A stretch of DNA that begins with a start codon (ATG) and ends with a stop codon (TAA, TAG, or TGA), potentially encoding a protein. *Days 5, 17*

**Ortholog** — Genes in different species that evolved from a common ancestral gene through speciation. Orthologs typically retain the same function. *Day 20*

**p-value** — The probability of observing a result at least as extreme as the one obtained, assuming the null hypothesis is true. In bioinformatics, p-values are typically adjusted for multiple testing (see FDR). *Days 14, 16*

**Paralog** — Genes within the same species that arose from gene duplication. Paralogs may diverge in function over time. *Day 20*

**Pathway** — A series of molecular interactions and reactions that lead to a biological outcome. Pathways connect genes, proteins, and metabolites into functional networks. *Day 16*

**PCR (Polymerase Chain Reaction)** — A laboratory technique for amplifying specific DNA sequences. Important for bioinformatics because PCR duplicates can bias sequencing results and must be identified and removed. *Days 1, 6*

**Phred score** — A logarithmic quality score indicating the probability of a base call being wrong. Phred 20 = 1% error; Phred 30 = 0.1% error; Phred 40 = 0.01% error. Encoded as ASCII characters in FASTQ files. *Days 6, 7*

**Phylogeny** — The evolutionary history and relationships among organisms or genes, typically represented as a tree. Phylogenetic analysis uses sequence similarity to infer these relationships. *Day 20*

**Pipe** — The `|>` operator in BioLang that passes the result of one expression as the first argument to the next function. `a |> f(b)` is equivalent to `f(a, b)`. *Days 2, 4*

**Polymorphism** — A variation in the DNA sequence that occurs at a frequency of 1% or greater in a population. Polymorphisms that change a single base are called SNPs. *Day 12*

**Promoter** — A region of DNA upstream of a gene where transcription factors bind to initiate gene expression. Promoter analysis can reveal gene regulation patterns. *Days 3, 11*

**Protein** — A large molecule made of amino acids, folded into a specific three-dimensional structure. Proteins perform most of the work in cells: catalysis, signaling, transport, and structure. *Days 1, 3, 17*

**Protein domain** — A conserved, independently folding structural unit within a protein. Domains often correspond to specific functions (e.g., kinase domains, DNA-binding domains). Databases like Pfam and InterPro catalog known protein domains. *Day 17*

**Quality control (QC)** — The process of evaluating raw data for errors, biases, and artifacts before analysis. In sequencing, QC includes checking read quality, adapter contamination, GC bias, and duplication rates. *Days 6, 8*

**Quality score** — A numerical value indicating confidence in a measurement. In sequencing, quality scores are Phred-scaled probabilities of error. In variant calling, quality scores indicate confidence in the variant call. *Days 6, 12*

**Read** — A single DNA sequence produced by a sequencing instrument. Modern sequencers produce millions to billions of short reads (100-300 bp for Illumina) or longer reads (10,000+ bp for PacBio/Nanopore). *Days 6, 7*

**Record** — A data structure with named fields. In BioLang, written as `{name: "BRCA1", length: 7088}`. Records are used to represent structured data like gene annotations and variant calls. *Days 4, 5*

**Reproducibility** — The ability for independent researchers to obtain the same results from the same data using the same analysis methods. Reproducible pipelines record software versions, parameters, and random seeds. *Day 22*

**Reverse complement** — The complement of a DNA sequence read in the reverse direction. The reverse complement of 5'-ATGC-3' is 5'-GCAT-3'. Essential because DNA is double-stranded and sequencing reads can come from either strand. *Days 3, 5*

**Reference genome** — A standard representative genome sequence for a species, used as a coordinate system for mapping reads and identifying variants. GRCh38 (hg38) is the current human reference. *Days 11, 12*

**RNA (Ribonucleic Acid)** — A single-stranded molecule transcribed from DNA. Messenger RNA (mRNA) carries genetic information from DNA to the ribosome for protein synthesis. Uses uracil (U) instead of thymine (T). *Days 1, 3, 13*

**RNA-seq** — A sequencing technology that measures gene expression by sequencing all RNA molecules in a sample. Produces millions of reads that are mapped to a reference genome and counted per gene. *Days 13, 29*

**SAM (Sequence Alignment Map)** — A text-based file format for storing sequence alignments. Each line represents a read and its alignment to a reference genome. BAM is the compressed binary equivalent. *Days 7, 12*

**Sequence** — An ordered series of nucleotides (DNA/RNA) or amino acids (protein). Sequences are the fundamental data type in bioinformatics. *Days 1, 2, 3*

**SNP (Single Nucleotide Polymorphism)** — A variation at a single position in the DNA sequence. SNPs are the most common type of genetic variation, with roughly 4-5 million per human genome. *Days 12, 28*

**Splice site** — The boundary between an exon and an intron. Splice sites are recognized by the spliceosome, which removes introns from the pre-mRNA. Mutations at splice sites can disrupt gene expression. *Days 3, 7*

**Strand** — The directionality of a DNA or RNA molecule. Double-stranded DNA has a forward (plus/sense) strand and a reverse (minus/antisense) strand. Genes can be located on either strand. Represented as `+`, `-`, or `.` (unknown) in genomic file formats. *Days 3, 18*

**Streaming** — Processing data record by record without loading the entire file into memory. Essential for files that exceed available RAM. In BioLang, `stream_fastq` and `stream_fasta` return lazy iterators. *Days 8, 21*

**Structural variant (SV)** — A genomic variant involving 50 or more base pairs. Includes large insertions, deletions, inversions, duplications, and translocations. Detected by specialized tools that analyze split reads, discordant read pairs, or long reads. *Day 12*

**Table** — A two-dimensional data structure with named columns and rows. In BioLang, tables are created with `to_table` and manipulated with `select`, `where`, `mutate`, `summarize`, and `group_by`. *Days 5, 10*

**Transcript** — The RNA molecule produced from a gene. A single gene can produce multiple transcripts through alternative splicing, each encoding a different protein isoform. *Days 3, 13*

**Transcriptome** — The complete set of RNA transcripts produced by an organism or cell type at a given time. RNA-seq measures the transcriptome to determine which genes are active and at what levels. *Day 13*

**Translation** — The process of converting an mRNA sequence into a protein sequence, reading three nucleotides (one codon) at a time. In BioLang, the `translate` function performs this conversion computationally. *Days 1, 3, 17*

**UTR (Untranslated Region)** — Portions of an mRNA molecule that are not translated into protein. The 5' UTR precedes the start codon; the 3' UTR follows the stop codon. UTRs regulate mRNA stability, localization, and translation efficiency. *Days 3, 18*

**Variant** — A difference between an individual's genome and the reference genome. Variants include SNPs, indels, structural variants, and copy number variants. *Days 12, 28*

**Variable** — A named storage location for a value. In BioLang, variables are declared with `let x = value` and reassigned with `x = new_value`. *Day 2*

**VCF (Variant Call Format)** — A text-based file format for storing genetic variants. Each line represents a variant with its position, reference allele, alternate allele, quality, and sample-specific genotype information. *Days 7, 12, 28*

**Volcano plot** — A scatter plot used to visualize differential expression results, plotting statistical significance (-log10 p-value) against magnitude of change (log2 fold change). Points in the upper-left and upper-right corners represent significantly differentially expressed genes. *Days 15, 19, 29*

**WES (Whole-Exome Sequencing)** — Sequencing of only the protein-coding regions (exons) of the genome, representing roughly 1-2% of the total genome. More cost-effective than WGS for finding coding mutations. *Days 12, 28*

**WGS (Whole-Genome Sequencing)** — Sequencing of the entire genome, including both coding and non-coding regions. Provides a complete picture but generates much more data than WES. *Days 12, 28*

**Zero-based coordinates** — A coordinate system where the first position is numbered 0. BED files use zero-based, half-open coordinates: a region from position 100 to 200 includes base 100 but not base 200. Contrast with one-based coordinates used in GFF and VCF. *Days 7, 18*
