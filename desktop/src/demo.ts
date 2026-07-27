import type { FileEntry, PackageInfo, WorkspaceSnapshot } from "./types";

export const demoFiles: Record<string, string> = {
  "analysis.bl": `# BRCA1 sequence quality snapshot
let sequence = dna"TAAACGTGAGAGAAACGTGCTGATTACACTTGTTCGTGTGGTAT"

let gc = gc_content(sequence)
let reverse = reverse_complement(sequence)
let motifs = kmer_count(sequence, 4)

println("GC content: " + str(gc))
println("Reverse complement: " + str(reverse))
println(motifs)
`,
  "pipelines/qc.bl": `# Stream reads without loading the full FASTQ file
read_fastq("data/reads.fastq")
  |> filter(|read| mean_phred(read.quality) >= 30)
  |> take(10)
  |> each(|read| println(read.id))
`,
  "pipelines/variants.bl": `let variants = read_vcf("data/cohort.vcf")

variants
  |> filter(|variant| variant.quality >= 30)
  |> summarize()
  |> println()
`,
  "biolang.toml": `[package]
name = "genome-workbench"
version = "0.1.0"
description = "Local BioLang analysis workspace"

[dependencies]
oric = { path = "../packages/oric" }
`,
  "README.md": `# Genome workbench

BioLang analyses and reproducible inputs for the current project.
`,
  "reports/origin-analysis.bl.md": `# Origin candidate analysis

This notebook keeps prose, BioLang cells, and results in one reproducible file.

\`\`\`biolang
let sequence = dna"TAAACGTGAGAGAAACGTGCTGATTACACTTGTTCGTGTGGTAT"
println("GC content: " + str(gc_content(sequence)))
\`\`\`

## Candidate motifs

\`\`\`biolang
let motifs = kmer_count(sequence, 4)
println(motifs)
\`\`\`
`,
  "pipelines/sequence-qc.blflow": `{
  "schemaVersion": 1,
  "name": "Sequence QC",
  "nodes": [
    {
      "id": "step_1",
      "operation": "read_fasta",
      "arguments": ["\\"data/sequences.fasta\\""],
      "x": 70,
      "y": 110
    },
    {
      "id": "step_2",
      "operation": "take",
      "arguments": ["10"],
      "x": 280,
      "y": 110
    },
    {
      "id": "step_3",
      "operation": "collect",
      "arguments": [],
      "x": 490,
      "y": 110
    }
  ],
  "edges": [
    { "from": "step_1", "to": "step_2" },
    { "from": "step_2", "to": "step_3" }
  ]
}
`,
  "data/sequences.fasta": `>ori_candidate
TAAACGTGAGAGAAACGTGCTGATTACACTTGTTCGTGTGGTAT
>control
CCAGATCGCGATACGTTACATACATGATAGAC
`,
  "data/expression.csv": `gene,sample_a,sample_b
BRCA1,12.4,14.8
TP53,8.1,7.9
MYC,21.0,19.6
`,
  "data/cohort.vcf": `##fileformat=VCFv4.3
#CHROM	POS	ID	REF	ALT	QUAL	FILTER
17	43044295	rs80357713	G	A	99	PASS
17	43045700	.	C	T	42	PASS
`,
  "data/genes.gff3": `##gff-version 3
chr17	RefSeq	gene	43044295	43170245	.	-	.	ID=gene-BRCA1;Name=BRCA1
chr17	RefSeq	mRNA	43044295	43170245	.	-	.	ID=rna-BRCA1;Parent=gene-BRCA1
chr17	RefSeq	exon	43106456	43106533	.	-	.	ID=exon-1;Parent=rna-BRCA1
`,
  "data/species.nwk": `((Human:0.01,Chimpanzee:0.012)Primates:0.02,(Mouse:0.08,Rat:0.07)Rodents:0.03)Mammals;`,
  "data/helix.pdb": `HEADER    DEMO HELIX
ATOM      1  N   ALA A   1      11.104  13.207   8.100  1.00 20.00           N
ATOM      2  CA  ALA A   1      12.560  13.255   8.200  1.00 20.00           C
ATOM      3  C   ALA A   1      13.020  14.690   8.500  1.00 20.00           C
ATOM      4  O   ALA A   1      12.340  15.650   8.160  1.00 20.00           O
ATOM      5  N   GLY A   2      14.180  14.820   9.130  1.00 20.00           N
`,
};

const file = (name: string, path: string, size: number): FileEntry => ({
  name,
  path,
  kind: "file",
  size,
  children: [],
});

export const demoWorkspace: WorkspaceSnapshot = {
  name: "genome-workbench",
  root: "browser://genome-workbench",
  truncated: false,
  entries: [
    {
      name: "data",
      path: "data",
      kind: "directory",
      size: 0,
      children: [
        file("sequences.fasta", "data/sequences.fasta", demoFiles["data/sequences.fasta"].length),
        file("expression.csv", "data/expression.csv", demoFiles["data/expression.csv"].length),
        file("cohort.vcf", "data/cohort.vcf", demoFiles["data/cohort.vcf"].length),
        file("genes.gff3", "data/genes.gff3", demoFiles["data/genes.gff3"].length),
        file("species.nwk", "data/species.nwk", demoFiles["data/species.nwk"].length),
        file("helix.pdb", "data/helix.pdb", demoFiles["data/helix.pdb"].length),
      ],
    },
    {
      name: "pipelines",
      path: "pipelines",
      kind: "directory",
      size: 0,
      children: [
        file("qc.bl", "pipelines/qc.bl", demoFiles["pipelines/qc.bl"].length),
        file("variants.bl", "pipelines/variants.bl", demoFiles["pipelines/variants.bl"].length),
        file("sequence-qc.blflow", "pipelines/sequence-qc.blflow", demoFiles["pipelines/sequence-qc.blflow"].length),
      ],
    },
    {
      name: "reports",
      path: "reports",
      kind: "directory",
      size: 0,
      children: [
        file("origin-analysis.bl.md", "reports/origin-analysis.bl.md", demoFiles["reports/origin-analysis.bl.md"].length),
      ],
    },
    file("analysis.bl", "analysis.bl", demoFiles["analysis.bl"].length),
    file("biolang.toml", "biolang.toml", demoFiles["biolang.toml"].length),
    file("README.md", "README.md", demoFiles["README.md"].length),
  ],
};

export const demoPackages: PackageInfo[] = [
  { name: "oric", version: "0.1.0", source: "path: ../packages/oric", installed: true },
];
