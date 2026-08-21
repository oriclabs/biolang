# Real external-tool tests

Ordinary `cargo test` runs do not install software, pull images, or require
samtools. The real samtools workflow is deliberately marked `ignored`.

It covers command forms from the official samtools manuals: SAM-to-BAM
conversion, coordinate sorting, indexing, `quickcheck`, counts, indexed region
queries, JSON `flagstat`, `stats`, FASTA indexing, and a CRAM round trip. The
fixture directory and filenames contain spaces. The test also checks detailed
argument provenance, invalid options, malformed SAM input, and `bl convert`
delegation when the workspace `bl` executable has been built.

## Use samtools already on PATH

```text
cargo test -p bl-convert --test samtools_tutorial -- --ignored --nocapture
```

## Use an exact native executable

PowerShell:

```powershell
$env:BL_CONVERT_TEST_SAMTOOLS = "C:\tools\samtools.exe"
cargo test -p bl-convert --test samtools_tutorial -- --ignored --nocapture
```

Bash:

```bash
BL_CONVERT_TEST_SAMTOOLS=/opt/samtools/bin/samtools \
  cargo test -p bl-convert --test samtools_tutorial -- --ignored --nocapture
```

## Explicitly use a container runtime

This form may pull BL Convert's pinned samtools BioContainers image. The runtime
must already be installed and ready.

PowerShell:

```powershell
$env:BL_CONVERT_TEST_RUNTIME = "docker"
cargo test -p bl-convert --test samtools_tutorial -- --ignored --nocapture
```

Bash:

```bash
BL_CONVERT_TEST_RUNTIME=docker \
  cargo test -p bl-convert --test samtools_tutorial -- --ignored --nocapture
```

Accepted runtime names are `docker`, `podman`, `apptainer`, and `singularity`.
The test uses an isolated temporary `BL_CONVERT_HOME` and working directory. It
does not purge an image after the test because doing so could remove a cached
image used by another project.

Validate every pinned catalog tag without pulling the images:

```powershell
$env:BL_CONVERT_TEST_MANIFEST_RUNTIME = "docker"
cargo test -p bl-convert --test container_catalog -- --ignored --nocapture
```

The manifest test accepts Docker or Podman and requires registry access.

Build the BioLang CLI first to include the optional delegation assertion:

```text
cargo build -p bl-cli -p bl-convert
```
