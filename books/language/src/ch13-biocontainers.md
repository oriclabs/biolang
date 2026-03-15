# Chapter 13: BioContainers Integration

Reproducible bioinformatics demands pinned software versions. A variant calling
pipeline that works today must produce identical results next year, on a
different machine, with the same tool versions. The BioContainers project
addresses this by packaging over 9,000 bioinformatics tools as container images
hosted on registries like quay.io and Docker Hub.

BioLang gives you native access to the BioContainers registry. Four built-in
functions let you search for tools, discover popular packages, inspect version
histories, and retrieve exact container image URIs -- all without leaving your
script. No imports are needed.

## Searching for Tools

`biocontainers_search` queries the BioContainers registry by name or keyword.
It returns a list of records, each describing a matching tool.

```biolang
# requires: internet connection (all biocontainers_* functions query the BioContainers API)
# Find all samtools-related containers
let results = biocontainers_search("samtools")
# results => [
#   {name: "samtools", description: "Tools for manipulating NGS alignments...",
#    organization: "biocontainers", version_count: 42,
#    latest_version: "1.19--h50ea8bc_0",
#    latest_image: "quay.io/biocontainers/samtools:1.19--h50ea8bc_0"},
#   {name: "htslib", description: "C library for high-throughput sequencing...", ...},
#   ...
# ]

results |> each(|tool| {
  print(tool.name + " (" + str(tool.version_count) + " versions)")
  print("  Latest: " + tool.latest_version)
})
```

The default limit is 25 results. Pass a second argument to narrow or widen the
search.

```biolang
# Top 5 matches for short-read aligners
let aligners = biocontainers_search("bwa", 5)

aligners |> each(|a| print(a.name + " => " + a.latest_image))
```

Search terms are matched against tool names and descriptions, so broader
queries work too.

```biolang
# Find tools related to RNA-seq quantification
let quant_tools = biocontainers_search("salmon rna-seq")

quant_tools
  |> filter(|t| t.version_count > 5)
  |> each(|t| print(t.name + ": " + t.description))
```

## Popular Tools

`biocontainers_popular` returns the most-pulled tools in the registry. This is
useful for discovering which tools the community relies on and for auditing
whether your pipeline uses well-maintained software.

```biolang
let top20 = biocontainers_popular()

print("Top 20 BioContainers tools:")
top20 |> each(|t| print("  " + t.name + " - " + t.latest_version))
```

Pass a limit to retrieve more.

```biolang
# Top 50 most popular bioinformatics containers
let top50 = biocontainers_popular(50)

# Which of our pipeline tools are in the top 50?
let our_tools = ["samtools", "bcftools", "bwa-mem2", "gatk4", "picard"]
let popular_names = top50 |> map(|t| t.name)

our_tools |> each(|tool| {
  let found = popular_names |> find(|n| n == tool)
  if found != nil then
    print(tool + " is in the top 50")
  else
    print(tool + " is NOT in the top 50")
})
```

## Tool Details

`biocontainers_info` returns a detailed record for a single tool, including its
full version history with per-version container images.

```biolang
let info = biocontainers_info("samtools")
# info => {
#   name: "samtools",
#   description: "Tools for manipulating NGS alignments...",
#   organization: "biocontainers",
#   aliases: ["samtools"],
#   versions: [
#     {version: "1.19--h50ea8bc_0",
#      images: [{registry: "quay.io", image: "quay.io/biocontainers/samtools:1.19--h50ea8bc_0",
#                type: "Docker", size: 14250000}]},
#     {version: "1.18--h50ea8bc_0", images: [...]},
#     ...
#   ]
# }

print(info.name + " by " + info.organization)
print(str(len(info.versions)) + " versions available")

# List the 5 most recent versions
info.versions
  |> take(5)
  |> each(|v| {
    let image = v.images |> first()
    print("  " + v.version + " (" + image.registry + ", "
          + str(image.size / 1000000) + " MB)")
  })
```

The `images` list for each version may contain entries from multiple registries
or image types (Docker, Singularity). Filter by registry or type if your
infrastructure requires a specific format.

```biolang
# Find Singularity images for deepvariant
let dv = biocontainers_info("deepvariant")

dv.versions |> each(|v| {
  let singularity = v.images |> filter(|img| img.type == "Singularity")
  if len(singularity) > 0 then
    print(v.version + " has Singularity image")
})
```

## Version Management

`biocontainers_versions` returns a flat list of all versions for a tool, each
with a list of full image URI strings. This is the function to use when you
need to pin a specific version in a pipeline manifest.

```biolang
let versions = biocontainers_versions("gatk4")
# versions => [
#   {version: "4.5.0.0--py310hdfd78af_0",
#    images: ["quay.io/biocontainers/gatk4:4.5.0.0--py310hdfd78af_0"]},
#   {version: "4.4.0.0--py310hdfd78af_0",
#    images: ["quay.io/biocontainers/gatk4:4.4.0.0--py310hdfd78af_0"]},
#   ...
# ]

# Find the latest GATK 4.4.x release
let gatk44 = versions
  |> filter(|v| starts_with(v.version, "4.4"))
  |> first()

print("Pinning GATK to: " + gatk44.images[0])
```

You can use this to check whether a specific version exists before committing
to it in a pipeline definition.

```biolang
# Verify that bcftools 1.18 is available
let bc_versions = biocontainers_versions("bcftools")
let target = bc_versions |> find(|v| starts_with(v.version, "1.18"))

if target != nil then
  print("bcftools 1.18 available: " + target.images[0])
else
  print("bcftools 1.18 not found in BioContainers")
```

## Example: Building a Reproducible Tool Manifest

A variant calling pipeline needs exact container images for every tool. Use the
BioContainers builtins to resolve each tool to a pinned image URI and export
the manifest.

```biolang
# Tools required for a germline variant calling pipeline
let required = [
  {name: "bwa-mem2",  min_version: "2.2"},
  {name: "samtools",  min_version: "1.18"},
  {name: "gatk4",     min_version: "4.4"},
  {name: "bcftools",  min_version: "1.18"},
]

let manifest = required |> map(|req| {
  let versions = biocontainers_versions(req.name)

  # Find the newest version that satisfies the minimum
  let matching = versions
    |> filter(|v| starts_with(v.version, req.min_version))

  if len(matching) == 0 then {
    print("WARNING: no " + req.name + " >= " + req.min_version + " found")
    {tool: req.name, version: "MISSING", image: "MISSING"}
  } else {
    let chosen = matching |> first()
    {tool: req.name, version: chosen.version, image: chosen.images[0]}
  }
})

# Print the resolved manifest
print("Variant Calling Pipeline - Tool Manifest")
print("=========================================")
manifest |> each(|m| {
  print(m.tool + ":")
  print("  version: " + m.version)
  print("  image:   " + m.image)
})

# Export as structured data
manifest |> write_json("pipeline_manifest.json")
```

This script produces a lockfile-style manifest that can be checked into version
control alongside the pipeline definition.

## Example: Tool Discovery for a New Analysis

When starting a new analysis type, you need to survey what tools are available.
Here we explore the methylation analysis landscape.

```biolang
# What methylation tools exist in BioContainers?
let methyl_tools = biocontainers_search("methylation", 50)

print(str(len(methyl_tools)) + " methylation-related tools found")
print("")

# Group by version count to find well-maintained tools
let mature = methyl_tools
  |> filter(|t| t.version_count >= 5)
  |> sort_by(|t| -t.version_count)

let new_tools = methyl_tools
  |> filter(|t| t.version_count < 3)

print("Mature tools (" + str(len(mature)) + "):")
mature |> each(|t| {
  print("  " + t.name + " - " + str(t.version_count) + " versions"
        + " (latest: " + t.latest_version + ")")
  print("    " + t.description)
})

print("")
print("Newer tools (" + str(len(new_tools)) + "):")
new_tools |> take(10) |> each(|t| {
  print("  " + t.name + " - " + t.latest_version)
})

# Deep dive into the top candidate
let bismark = biocontainers_info("bismark")
print("")
print("Bismark detail:")
print("  " + bismark.description)
print("  " + str(len(bismark.versions)) + " releases")
print("  Aliases: " + join(bismark.aliases, ", "))

# Check image sizes across versions
bismark.versions |> take(5) |> each(|v| {
  let docker = v.images |> filter(|img| img.type == "Docker") |> first()
  if docker != nil then
    print("  " + v.version + ": " + str(docker.size / 1000000) + " MB")
})
```

## Example: Container Image Audit

For an existing pipeline, verify that every tool has a valid BioContainers
image and flag any that are outdated.

```biolang
# Current pipeline tools and their pinned versions
let pinned = [
  {tool: "bwa-mem2",  version: "2.2.1--hd03093a_2"},
  {tool: "samtools",  version: "1.17--h50ea8bc_0"},
  {tool: "gatk4",     version: "4.3.0.0--py310hdfd78af_0"},
  {tool: "bcftools",  version: "1.17--h3cc50cf_1"},
  {tool: "multiqc",   version: "1.14--pyhdfd78af_0"},
  {tool: "fastp",     version: "0.23.2--hb7a2d85_2"},
]

let audit = pinned |> map(|entry| {
  let info = biocontainers_info(entry.tool)
  let all_versions = info.versions |> map(|v| v.version)

  # Check if pinned version still exists
  let exists = all_versions |> find(|v| v == entry.version) != nil

  # Check if there is a newer version
  let latest = info.versions |> first()
  let is_latest = latest.version == entry.version

  # Count how many versions behind
  let versions_behind = if is_latest then
    0
  else {
    let idx = all_versions
      |> enumerate()
      |> find(|pair| pair.value == entry.version)
    if idx != nil then idx.index else -1
  }

  {
    tool: entry.tool,
    pinned: entry.version,
    latest: latest.version,
    exists: exists,
    is_latest: is_latest,
    versions_behind: versions_behind,
  }
})

# Report
print("Pipeline Container Audit")
print("========================")

let missing = audit |> filter(|a| not a.exists)
let outdated = audit |> filter(|a| a.exists and not a.is_latest)
let current = audit |> filter(|a| a.is_latest)

if len(missing) > 0 then {
  print("")
  print("MISSING (pinned version no longer in registry):")
  missing |> each(|a| print("  " + a.tool + " " + a.pinned
                             + " => latest: " + a.latest))
}

if len(outdated) > 0 then {
  print("")
  print("OUTDATED:")
  outdated |> each(|a| print("  " + a.tool + " " + a.pinned
                              + " => " + a.latest
                              + " (" + str(a.versions_behind) + " versions behind)"))
}

if len(current) > 0 then {
  print("")
  print("CURRENT:")
  current |> each(|a| print("  " + a.tool + " " + a.pinned))
}

print("")
print(str(len(current)) + " current, "
      + str(len(outdated)) + " outdated, "
      + str(len(missing)) + " missing")

audit |> write_json("container_audit.json")
```

## Summary

BioLang's four BioContainers builtins -- `biocontainers_search`,
`biocontainers_popular`, `biocontainers_info`, and `biocontainers_versions` --
bring the full BioContainers registry into your scripts as native data. Use
them to discover tools, pin container images for reproducibility, audit
existing pipelines, and explore new analysis domains. Combined with BioLang's
pipes and collection operations, a few lines of code replace manual registry
browsing and ad hoc version tracking.
