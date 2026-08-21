# BL Convert

`bl-convert.exe` is BioLang's separate, MIT-licensed file-conversion CLI. It
does not become part of `bl.exe`, so installing it does not increase the
language executable's size.

```text
bl-convert convert input.csv output.tsv
bl-convert convert variants.vcf.gz variants.bed.gz
bl-convert convert genes.gff3 genes.bed --feature gene
bl-convert convert reads.fastq reads.fasta
bl-convert inspect variants.bed
bl-convert formats
```

Every conversion parses the complete input, writes to a temporary file,
validates the generated records, and only then moves the output into place.
Delimited, genomic and sequence records are streamed with bounded buffers.
JSON is currently materialized in memory so the converter can discover the
union of object keys before writing CSV or TSV.
Existing files require `--force`. `--dry-run` performs the parsing and
conversion without creating output. `--json` prints a machine-readable report;
`--report report.json` saves one.

## Supported conversions

| Source | Targets | Notes |
|---|---|---|
| CSV/TSV | CSV, TSV, JSON | CSV quoting is preserved semantically. JSON fields remain strings to avoid changing identifiers such as `001`. |
| JSON array of objects | CSV, TSV, JSON | Nested values become JSON text inside a cell and are reported as lossy tabular typing. |
| BED | BED | Validates coordinates and normalizes line endings. `track`/`browser` header lines pass through; a contig merely *named* `track...` is a record and is validated as one. |
| VCF | BED6 | Uses `POS - 1` for BED start and INFO/END when present. BED score is `0`; VCF QUAL, genotypes and most metadata are reported as lost. |
| GFF3/GTF | BED6 | Converts 1-based inclusive coordinates to 0-based half-open coordinates. BED score is `0`; `--feature` and `--name-attribute` control selection. |
| FASTA | FASTA | Validates records and applies `--line-width`. |
| FASTQ | FASTQ, FASTA | FASTQ is normalized to four lines, keeping any description on the `+` line so the pair is genuinely lossless. FASTQ to FASTA explicitly reports discarded qualities. |

Gzip input and output use `.gz`. BGZF input is readable as concatenated gzip,
but `.bgz` output is rejected until a true BGZF writer is implemented.

## Build or install

`bl-convert` ships in every release archive beside `bl`, so unpacking a release
is enough. From a source checkout:

```text
cargo build --release -p bl-convert
cargo install --path crates/bl-convert
```

The executable is named `bl-convert` and remains independent of `bl.exe`: the
archive carries two executables rather than linking the converter into the
language runtime. When both sit beside each other or are both on `PATH`,
`bl convert ...` delegates to `bl-convert`, and `bl convert INPUT OUTPUT`
supplies the `convert` subcommand automatically.

## Optional BioContainers tools

BL Convert has a curated catalog for samtools, bcftools, HTSlib, bedtools,
seqkit, fastp, FastQC, MultiQC, cutadapt, minimap2, Bowtie2 and STAR. Images use
explicit BioContainers tags rather than `latest`.

```text
bl-convert doctor
bl-convert tool catalog
bl-convert tool install samtools
bl-convert tool install bcftools --runtime podman
bl-convert tool register samtools --local
bl-convert tool register samtools --path C:\tools\samtools.exe
bl-convert tool register samtools --wsl --distribution Ubuntu
bl-convert tool list
bl-convert tool status samtools
```

Installation is always explicit: conversions never pull an image. Docker and
Podman are supported on Linux, macOS and Windows; Apptainer and Singularity are
supported primarily for Linux/HPC systems. These are Linux containers, so a
runtime may use architecture emulation on an ARM host when an image lacks a
native ARM manifest.

`tool register` never installs software. `--local` finds an existing executable
on `PATH`, `--path` records an exact native executable, and `--wsl` verifies the
tool inside an existing WSL distribution. BL Convert records the resolved path,
backend and version. Local and WSL registrations are removed from BL Convert's
manifest without uninstalling the external software.

Installed tools receive one mounted working directory at `/data`, container
networking is disabled for Docker/Podman runs, and arguments are passed directly
without a shell:

```text
bl-convert tool run samtools --workdir C:\analysis -- view -b -o reads.bam reads.sam
bl-convert tool run samtools --workdir C:\analysis --cpus 4 --memory 8g --report samtools-run.json -- view -@ 4 -b -o reads.bam reads.sam
bl-convert tool run htslib --workdir C:\analysis -- bgzip-input.vcf
bl-convert tool run htslib --executable tabix --workdir C:\analysis -- -p vcf variants.vcf.gz
```

Everything after `--` is passed as a separate argument without shell parsing,
so upstream tool parameters remain available. For Docker/Podman, networking is
off unless `--allow-network` is supplied. `--cpus` and `--memory` set runtime
limits. `--report` saves the backend, exact image/path, tool version, selected
executable, arguments, mounts, limits, network policy, exit code and timing.
Extra reference directories can be mounted explicitly and are
read-only unless `:rw` is requested:

```text
bl-convert tool run samtools --workdir C:\analysis --mount C:\refs=/refs:ro -- view -T /refs/hg38.fa reads.cram
```

Use `--read-only` for container inspection commands that do not create files.
Local and WSL tools are not sandboxes, so BL Convert rejects container-only
mount/resource controls for those backends rather than pretending to enforce
them. Removing a
tool only unregisters it by default; image deletion requires the additional
explicit `--purge` flag. Tool state is stored beneath
`~/.biolang/convert/tools.json`, or under `BL_CONVERT_HOME` when set.

The external tools retain their own licences. They run across a process and
container boundary and are not linked into the MIT-licensed BioLang binaries.

## Real samtools integration test

The ignored integration suite runs tutorial-style SAM-to-BAM conversion,
sorting, indexing, region queries, statistics, and a CRAM round trip through a
real samtools backend. Ordinary test runs never pull an image. See
[`tests/README.md`](tests/README.md) for native and container invocations.

## Deliberately unsupported in this first release

- FASTA to FASTQ: quality scores would have to be invented.
- BED to VCF or GFF: BED usually lacks mandatory biological fields.
- SAM/BAM and Parquet: these require heavier typed backends and will be added
  without weakening the validation and loss-reporting contract.
