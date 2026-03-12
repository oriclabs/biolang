# nf-core Integration

## Why nf-core in a DSL

nf-core is the largest curated collection of bioinformatics pipelines, maintained
by a global community and covering over 100 workflows -- from bulk RNA-seq to
rare-disease variant calling. Every pipeline follows strict standards: peer review,
continuous integration, container packaging, and a machine-readable parameter schema.

BioLang exposes nf-core as a set of built-in functions. There is nothing to import
and nothing to install. You can browse, search, inspect, and compare nf-core
pipelines from the same scripts that process your FASTQ files and call variants.
The five builtins -- `nfcore_list`, `nfcore_search`, `nfcore_info`, `nfcore_releases`,
and `nfcore_params` -- turn the nf-core catalog into a queryable data source that
fits naturally into pipes, filters, and record operations.

## Browsing the Catalog

`nfcore_list()` returns every pipeline in the nf-core organization. Each entry is
a record with fields like `name`, `description`, `stars`, and `topics`.

```
# requires: internet connection (all nfcore_* functions query the nf-core GitHub API)
# List all nf-core pipelines
nfcore_list()
```

The result is a list of records. You can pipe it through any BioLang operation:

```
# Show names and star counts, sorted by popularity
nfcore_list(sort_by: "stars")
  |> each |p| { name: p.name, stars: p.stars }
```

To limit the result set, pass `limit`. This is useful in exploratory sessions
where you want a quick overview rather than the full catalog:

```
# Top 10 pipelines by most recent release date
nfcore_list(sort_by: "release", limit: 10)
  |> each |p| { name: p.name, updated: p.updated }
```

Sorting accepts three values:

- `"stars"` -- community popularity (default)
- `"name"` -- alphabetical
- `"release"` -- most recently updated first

## Searching Pipelines

When you know what kind of analysis you need but not which pipeline implements it,
use `nfcore_search`. It matches against pipeline names, descriptions, and topic tags.

```
# Find RNA-seq related pipelines
nfcore_search("rnaseq")
```

Search accepts an optional `limit` to cap results:

```
# Find variant-calling pipelines, show top 5
nfcore_search("variant", 5)
```

The returned records have the same shape as `nfcore_list` entries, so you can
chain the same downstream operations:

```
# Search for methylation pipelines and extract their topics
nfcore_search("methylation")
  |> each |p| { name: p.name, topics: p.topics }
```

## Pipeline Details

`nfcore_info` returns a single record with comprehensive metadata for a named
pipeline. The returned record contains:

| Field         | Type   | Description                          |
|---------------|--------|--------------------------------------|
| `name`        | String | Short pipeline name                  |
| `full_name`   | String | GitHub-qualified name (nf-core/...)  |
| `description` | String | One-line summary                     |
| `stars`       | Int    | GitHub star count                    |
| `url`         | String | Repository URL                       |
| `license`     | String | License identifier (usually MIT)     |
| `topics`      | List   | GitHub topic tags                    |
| `open_issues` | Int    | Current open issue count             |
| `created`     | String | Repository creation date             |
| `updated`     | String | Last push date                       |

```
# Inspect the Sarek germline/somatic variant calling pipeline
let sarek = nfcore_info("sarek")
print("Pipeline: " + sarek.full_name)
print("License:  " + sarek.license)
print("Stars:    " + to_string(sarek.stars))
print("Topics:   " + join(sarek.topics, ", "))
```

You can use the metadata to make decisions in scripts. For example, checking
whether a pipeline is actively maintained before committing to it:

```
# Only proceed if the pipeline has been updated recently
let info = nfcore_info("taxprofiler")
if info.open_issues < 50 then
  print(info.name + " looks actively maintained")
```

## Releases and Versions

Production workflows should pin to a specific release rather than tracking the
development branch. `nfcore_releases` returns the full release history as a list
of records, each with `tag` and `published_at` fields.

```
# List all rnaseq releases
nfcore_releases("rnaseq")
```

The list is ordered newest-first, so the head element is the latest stable version:

```
# Get the latest stable release tag for rnaseq
let latest = nfcore_releases("rnaseq")
  |> first
print("Latest release: " + latest.tag + " (" + latest.published_at + ")")
```

You can also search for a specific version range or find how many releases a
pipeline has had -- a rough proxy for maturity:

```
# Count total releases for sarek
let release_count = nfcore_releases("sarek") |> len
print("sarek has " + to_string(release_count) + " releases")
```

```
# Find all 3.x releases of rnaseq
nfcore_releases("rnaseq")
  |> filter |r| starts_with(r.tag, "3.")
  |> each |r| r.tag
```

## Parameter Schemas

Every nf-core pipeline publishes a JSON schema describing its configurable
parameters -- input paths, reference genomes, trimming options, resource limits,
and more. `nfcore_params` fetches and parses this schema, returning a record
whose keys are parameter group names and whose values are records of individual
parameters.

```
# Fetch the full parameter schema for rnaseq
let params = nfcore_params("rnaseq")
```

The top-level keys are group names such as `"input_output_options"`,
`"reference_genome_options"`, or `"trimming_options"`. Each group is a record
of parameter entries:

```
# List all parameter groups
nfcore_params("rnaseq")
  |> keys
```

```
# Inspect the reference genome options
nfcore_params("rnaseq").reference_genome_options
  |> keys
```

This is particularly useful for validating that your configuration covers the
required parameters before submitting a long-running pipeline:

```
# Check whether a specific parameter exists
let params = nfcore_params("sarek")
let genome_opts = params.reference_genome_options
print(keys(genome_opts))
```

## Example: Finding the Right Pipeline

A common task: you need to process single-cell RNA-seq data but are unsure which
nf-core pipeline to use. Search the catalog, compare candidates, and pick the
best fit.

```
# Step 1: Search for single-cell pipelines
let candidates = nfcore_search("single-cell")

# Step 2: Show names, stars, and descriptions side by side
candidates
  |> each |p| {
    name: p.name,
    stars: p.stars,
    description: p.description
  }
  |> sort_by |a, b| b.stars - a.stars
  |>> each |p| print(p.name + " (" + to_string(p.stars) + " stars): " + p.description)

# Step 3: Get detailed info on the top candidate
let top = candidates
  |> sort_by |a, b| b.stars - a.stars
  |> first

let info = nfcore_info(top.name)
print("Selected: " + info.full_name)
print("License:  " + info.license)
print("Topics:   " + join(info.topics, ", "))

# Step 4: Check the latest release
let latest = nfcore_releases(top.name) |> first
print("Latest release: " + latest.tag + " published " + latest.published_at)

# Step 5: Preview the parameter groups
nfcore_params(top.name)
  |> keys
  |>> each |group| print("  - " + group)
```

This entire exploration runs in a single script -- no switching between a browser,
the nf-core CLI tool, and your pipeline configuration files.

## Example: Auditing Pipeline Parameters

Before launching a whole-genome sequencing analysis with Sarek, you want to
confirm that your reference genome configuration covers every required parameter
and that no deprecated options have crept into your config.

```
# Fetch the Sarek parameter schema
let params = nfcore_params("sarek")

# Extract all reference genome parameters
let ref_params = params.reference_genome_options

# Your current configuration
let my_config = {
  genome: "GRCh38",
  igenomes_base: "s3://ngi-igenomes/igenomes",
  fasta: nil,
  fasta_fai: nil
}

# Check for parameters in the schema that you have not configured
let schema_keys = keys(ref_params)
let config_keys = keys(my_config)

let missing = schema_keys
  |> filter |k| not(contains(config_keys, k))

if len(missing) > 0 then
  print("WARNING: unset reference genome parameters:")
  missing |>> each |k| print("  - " + k)
else
  print("All reference genome parameters are covered.")

# Check for keys in your config that are not in the schema (possibly deprecated)
let extra = config_keys
  |> filter |k| not(contains(schema_keys, k))

if len(extra) > 0 then
  print("WARNING: config keys not in current schema (possibly deprecated):")
  extra |>> each |k| print("  - " + k)
```

This catches configuration drift early -- before you discover it four hours into
a whole-genome analysis run.

## Example: Building a Pipeline Registry

For a core facility managing dozens of active projects, it helps to maintain a
catalog of available pipelines organized by topic. This script builds a topic
index from the full nf-core catalog.

```
# Fetch all pipelines
let all_pipelines = nfcore_list(sort_by: "stars")

# Build a topic-to-pipeline mapping
# Flatten: for each pipeline, emit one record per topic tag
let entries = all_pipelines
  |> flat_map |p| p.topics |> each |t| { topic: t, name: p.name, stars: p.stars }

# Group by topic
let topics = entries
  |> group_by |e| e.topic

# Print a summary: topics with 3 or more pipelines
keys(topics)
  |> filter |t| len(topics[t]) >= 3
  |> sort
  |>> each |t| {
    let pipelines = topics[t]
      |> sort_by |a, b| b.stars - a.stars
      |> each |p| p.name
    print(t + " (" + to_string(len(pipelines)) + " pipelines): " + join(pipelines, ", "))
  }
```

You can extend this to generate a Markdown report, push to a shared wiki, or
feed into a lab notebook:

```
# Export the top 20 pipelines as a tab-separated table
let header = "name\tstars\ttopics\tlatest_release"
print(header)

nfcore_list(sort_by: "stars", limit: 20)
  |>> each |p| {
    let latest = nfcore_releases(p.name) |> first
    let tag = if latest != nil then latest.tag else "unreleased"
    let topic_str = join(p.topics, ";")
    print(p.name + "\t" + to_string(p.stars) + "\t" + topic_str + "\t" + tag)
  }
```

Redirect stdout to a file and you have a pipeline inventory that updates every
time you run the script -- no manual curation needed.

## Parsing Nextflow Files

BioLang can parse Nextflow `.nf` files directly with `nf_parse()`. This function
reads the file and extracts its structure without requiring a Nextflow runtime.

```
# Parse a Nextflow pipeline file
let parsed = nf_parse("main.nf")
```

The result is a record with five fields:

| Field       | Type         | Content                                    |
|-------------|-------------|---------------------------------------------|
| `params`    | Record       | Parameter name-value pairs                  |
| `processes` | List[Record] | Process name, inputs, outputs, script, container, cpus, memory |
| `includes`  | List[Record] | Include name, alias, source path            |
| `workflow`  | List[String] | Workflow block lines                        |
| `dsl`       | String       | DSL version ("DSL1" or "DSL2")              |

You can pipe the parsed structure through any BioLang operation:

```
# List all processes and their containers
let parsed = nf_parse("variant_calling.nf")

parsed.processes
  |> each |p| {
    print(p.name + ": " + if p.container != nil then p.container else "no container")
  }
```

```
# Extract all parameters with their default values
let parsed = nf_parse("rnaseq.nf")

keys(parsed.params)
  |> sort
  |>> each |k| print(k + " = " + to_string(parsed.params[k]))
```

## Generating BioLang Code

The `nf_to_bl` function takes a parsed Nextflow record and generates BioLang
pipeline code as a string:

```
# Parse and convert a Nextflow pipeline
let parsed = nf_parse("rnaseq.nf")
let bl_code = nf_to_bl(parsed)

# Print the generated code
print(bl_code)

# Save to a file
write_file("rnaseq.bl", bl_code)
```

The generated code maps Nextflow constructs to BioLang equivalents:

- `params.*` become top-level variables
- `process` blocks become function definitions with `shell()` calls
- Container, CPU, and memory directives are preserved as comments
- Workflow call ordering maps to sequential pipe chains

This is a starting point -- edit the generated skeleton to add BioLang-specific
features like pipe chains, error handling, and parallel execution.

```
# Full workflow: parse, generate, and review
let parsed = nf_parse("sarek_main.nf")

# Show what was extracted
print("Found " + to_string(len(parsed.processes)) + " processes")
print("Found " + to_string(len(keys(parsed.params))) + " parameters")

# Generate BioLang code
let bl_code = nf_to_bl(parsed)
write_file("sarek.bl", bl_code)
print("Generated sarek.bl")
```
