# BL Convert: Files and External Tools

BL Convert is BioLang's optional command-line companion for two related jobs:

1. safely converting common biological and tabular files; and
2. running selected command-line bioinformatics tools through a local, WSL,
   Docker, Podman, Apptainer, or Singularity installation.

It is a separate executable named `bl-convert`. The BioLang interpreter stays
small and independent: installing BL Convert does not link samtools, bcftools,
or a container runtime into `bl`.

## Install

`bl-convert` is included in every release archive next to `bl`, so unpacking a
release installs both. To build it from a source checkout instead:

```bash
cargo install --path crates/bl-convert
```

For a checkout-only build, use:

```bash
cargo build --release -p bl-convert
```

Confirm the installation:

```bash
bl-convert --version
bl-convert formats
```

When `bl` and `bl-convert` are installed beside each other, or both are on
`PATH`, this shorter form delegates to the companion executable:

```bash
bl convert input.csv output.tsv
```

The direct and delegated forms behave the same. `bl convert formats` and
`bl convert tool ...` also work.

## Built-in Conversions

These conversions run natively. They do **not** need Docker or another external
program.

| Input | Output | Important behaviour |
|---|---|---|
| CSV or TSV | CSV, TSV, JSON | Quoting is parsed correctly; tabular values remain strings, preserving identifiers such as `001`. |
| JSON array of objects | CSV, TSV, JSON | Nested values become JSON text inside a cell. JSON input is held in memory to discover all keys. |
| BED | BED | Coordinates are checked and line endings are normalized. `track` and `browser` header lines pass through unchanged; a contig whose name merely starts with those words is treated as data and validated. |
| VCF | BED6 | Converts 1-based VCF positions to 0-based BED starts and uses `INFO/END` when present. |
| GFF3 or GTF | BED6 | Converts 1-based inclusive coordinates to 0-based half-open coordinates. |
| FASTA | FASTA | Validates records and wraps sequence lines. |
| FASTQ | FASTQ, FASTA | Normalizes FASTQ records to four lines and preserves any description on the `+` line, so FASTQ to FASTQ loses nothing. FASTQ to FASTA reports the discarded quality scores. |

Gzip input and output are selected with `.gz`. A `.bgz` input can be read as
concatenated gzip, but BL Convert refuses `.bgz` output because ordinary gzip
is not a substitute for indexed BGZF.

Formats that require inventing biological information are intentionally not
offered. For example, FASTA cannot be converted faithfully to FASTQ because it
does not contain quality scores.

## Convert and Inspect

The usual command needs only input and output paths:

```bash
bl-convert convert samples.csv samples.tsv
bl-convert convert variants.vcf.gz variants.bed.gz
bl-convert convert genes.gff3 genes.bed --feature gene
bl-convert convert reads.fastq reads.fasta
```

Use `--from` or `--to` when a filename has no useful extension:

```bash
bl-convert convert incoming.data normalized.tsv --from csv --to tsv
```

Useful conversion options are:

| Option | Purpose |
|---|---|
| `--force` | Replace an existing output file. |
| `--dry-run` | Parse and convert without creating output. |
| `--json` | Print the result and loss information as JSON. |
| `--report FILE` | Save the conversion report as JSON. |
| `--feature TYPE` | Retain one GFF/GTF feature type, such as `gene` or `exon`. |
| `--name-attribute NAME` | Choose the GFF/GTF attribute used as the BED name. |
| `--line-width N` | Set FASTA wrapping; the default is 80. |

Before committing to a conversion, inspect the input:

```bash
bl-convert inspect variants.vcf.gz
bl-convert inspect unknown.data --from bed --json
```

BL Convert parses the complete input, writes to a temporary file, validates the
generated records, and only then moves the file to its destination. Existing
outputs are never silently replaced. Delimited, interval, and sequence records
stream with bounded buffers; JSON is the current exception because its union of
object keys must be found before a table header can be written.

## Choose an External-Tool Backend

Use the least complicated backend already available on the machine:

| Situation | Recommended backend |
|---|---|
| The tool is already installed and on `PATH` | Register it with `--local`. |
| You know the exact native executable | Register it with `--path`. |
| The tool is installed inside Windows Subsystem for Linux | Register it with `--wsl`. |
| You want a pinned, isolated installation | Explicitly install its BioContainers image. |
| You are on an HPC system | Prefer Apptainer or Singularity if supplied by the cluster. |

Check what is usable first:

```bash
bl-convert doctor
bl-convert doctor --json
bl-convert tool catalog
bl-convert tool list
```

`doctor` checks runtime commands and, for Docker and Podman, whether the daemon
is responding. Finding `docker.exe` alone is not enough if Docker Desktop is
stopped.

## Register an Existing Tool

Registration verifies the executable and records its resolved path and version.
It does not install or copy the external software.

```bash
# Find samtools on the native PATH
bl-convert tool register samtools --local

# Use one exact native executable
bl-convert tool register samtools --path C:\tools\samtools.exe

# Find samtools in the default WSL distribution
bl-convert tool register samtools --wsl

# Select a particular WSL distribution
bl-convert tool register samtools --wsl --distribution Ubuntu
```

Only one of `--local`, `--path`, or `--wsl` may be used at a time.

## Install a Pinned Container Tool

Container installation is always explicit. A conversion or tool run never
pulls an image behind your back.

```bash
bl-convert tool install samtools
bl-convert tool install bcftools --runtime podman
bl-convert tool status samtools
```

Supported container runtimes are Docker, Podman, Apptainer, and Singularity.
Docker and Podman are common on Windows, macOS, and Linux. Apptainer and
Singularity are most common on Linux and HPC systems. The curated images are
Linux images; an ARM machine may need runtime emulation when an image has no
native ARM manifest.

The curated catalog currently contains samtools, bcftools, HTSlib
(`bgzip`/`tabix`), bedtools, seqkit, fastp, FastQC, MultiQC, cutadapt, minimap2,
Bowtie2, and STAR. Each entry uses an explicit image tag rather than `latest`.

## Pass Detailed Tool Parameters

BL Convert is not a reduced wrapper around each program. Everything after `--`
is passed to the selected executable as separate arguments, without a shell:

```bash
bl-convert tool run samtools --workdir C:\analysis -- view -@ 4 -b -o reads.bam reads.sam
```

Here `--workdir`, `--cpus`, and `--memory` belong to BL Convert. `view`, `-@`,
and the remaining arguments belong to samtools:

```bash
bl-convert tool run samtools --workdir C:\analysis --cpus 4 --memory 8g --report samtools-run.json -- view -@ 4 -b -o reads.bam reads.sam
```

For an image that contains more than one allowed program, select it explicitly:

```bash
bl-convert tool run htslib --executable tabix --workdir C:\analysis -- -p vcf variants.vcf.gz
```

Arbitrary executable names are rejected. This prevents an image from becoming
an unrestricted shell escape through the BL Convert interface.

### A Tested Samtools Tutorial Sequence

The integration suite runs this workflow against a tiny real SAM file through
the pinned samtools container:

```bash
bl-convert tool run samtools --workdir ./tutorial -- view -bo aln.bam tutorial.sam
bl-convert tool run samtools --workdir ./tutorial -- sort -@ 2 -m 1M -o aln.sorted.bam aln.bam
bl-convert tool run samtools --workdir ./tutorial -- index aln.sorted.bam
bl-convert tool run samtools --workdir ./tutorial -- quickcheck -v aln.sorted.bam
bl-convert tool run samtools --workdir ./tutorial -- view -c aln.sorted.bam
bl-convert tool run samtools --workdir ./tutorial -- view aln.sorted.bam chr1:10-24
bl-convert tool run samtools --workdir ./tutorial -- flagstat -O json aln.sorted.bam
bl-convert tool run samtools --workdir ./tutorial -- stats aln.sorted.bam
```

It also creates a FASTA index, converts BAM to CRAM with a local reference,
decodes the CRAM, checks paths containing spaces, and confirms that malformed
SAM and invalid samtools options fail. These are semantic checks: the expected
read count and region records are asserted, not merely the process exit code.

Shell operators are not samtools arguments. Keep redirection outside the
separator:

```bash
bl-convert tool run samtools --workdir ./tutorial -- view -h aln.sorted.bam > alignments.sam
```

Prefer samtools' own `-o FILE` option for binary output, especially on Windows,
where shell pipeline behaviour varies between PowerShell versions.

## Working Directories, References, and Limits

For containers, `--workdir` is mounted at `/data`, which is also the process
working directory. Relative tool paths therefore refer to files under the host
working directory:

```text
Host:       C:\analysis\reads.sam
Container:  /data/reads.sam
Argument:   reads.sam
```

Mount reference data separately when needed:

```bash
bl-convert tool run samtools --workdir C:\analysis --mount C:\refs=/refs:ro -- view -T /refs/hg38.fa reads.cram
```

`--mount` may be repeated. It accepts
`HOST=/container/path[:ro|:rw]`; read-only is the safe default. Reserved system
and BL Convert mount targets cannot be replaced.

Docker and Podman networking is disabled by default. Enable it only for a tool
that genuinely needs a network connection:

```bash
bl-convert tool run multiqc --allow-network --workdir C:\analysis -- .
```

Other run controls are:

| Option | Meaning |
|---|---|
| `--read-only` | Mount `/data` read-only for inspection commands. |
| `--cpus N` | Docker/Podman CPU limit, such as `4` or `1.5`. |
| `--memory SIZE` | Docker/Podman memory limit, such as `8g` or `512m`. |
| `--allow-network` | Allow Docker/Podman networking for this run. |
| `--report FILE` | Save exact execution provenance as JSON. |
| `--force-report` | Replace an existing run report. |

Native and WSL programs are not sandboxes. BL Convert rejects container-only
mount and resource controls on those backends instead of suggesting limits it
cannot enforce.

## Reproducibility Reports

A tool-run report records the backend, tool version, exact executable path or
image, arguments, working directory, mounts, limits, network policy, exit code,
and elapsed time:

```bash
bl-convert tool run samtools --workdir C:\analysis --report samtools-view.json -- view -b -o reads.bam reads.sam
```

Keep this JSON beside the generated data and analysis script. It answers the
practical question, "Which samtools, with which settings, produced this file?"

## Remove or Change a Backend

```bash
# Forget the registration, but keep the external program or image
bl-convert tool remove samtools

# Also ask the container runtime to delete the managed image
bl-convert tool remove samtools --purge
```

Removing a native or WSL registration never uninstalls the external software.
Tool state is stored under `~/.biolang/convert/tools.json`, or beneath the
directory selected by `BL_CONVERT_HOME`.

## Common Problems

**"No supported container runtime is ready"** means no responsive runtime was
found. Start Docker/Podman, install an appropriate runtime, or register a local
or WSL executable instead.

**"Tool is not installed"** means the catalog knows the name, but no backend
has been selected. Run `tool register` or the explicit `tool install` command.

**"Output already exists"** is a safety check. Review the path, then repeat a
conversion with `--force`, or a run report with `--force-report`, if replacement
is intended.

**A file inside a container cannot be found** usually means the argument uses a
host path. Put working files under `--workdir` and refer to them relatively, or
add a `--mount` and use its container path.

**A parameter is rejected by BL Convert** usually means the separator is
missing. Put external-tool options after `--`.

## Licence Boundary

BL Convert and BioLang are MIT licensed. External tools and container images
retain their own licences. BL Convert starts them as separate processes; it
does not copy or link their code into the BioLang executables. Check the
external tool's licence and citation requirements before distributing an image
or publishing results.

## Current Scope

BL Convert provides a safe generic runner, so the complete documented command
line of a registered tool remains available. It does not yet translate typed
commands such as `bl-convert sort` into samtools calls, and built-in BAM,
Parquet, or indexed BGZF conversion is not yet implemented. Use a registered
external tool for those jobs and retain the provenance report.
