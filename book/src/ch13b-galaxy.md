# Galaxy ToolShed & Workflow Parsing

The Galaxy ToolShed is the app store for the Galaxy bioinformatics platform.
It hosts thousands of community-contributed tools covering everything from
read alignment and variant calling to phylogenetics and metabolomics. Each
repository in the ToolShed carries metadata -- owner, description, download
counts, and revision history -- making it a rich discovery resource even if
you never run Galaxy itself.

BioLang provides read-only access to the Galaxy ToolShed through four built-in
functions. There is nothing to import and nothing to install. You can search
repositories, browse popular tools, list categories, and inspect individual
tools from the same scripts that process your sequence data. A fifth builtin,
`galaxy_to_bl`, takes a Galaxy workflow record and generates BioLang pipeline
code from it.

## Searching Repositories

`galaxy_search` queries the ToolShed by name or keyword. It returns a list of
records, each describing a matching repository.

```
# Find BWA-related tools in the Galaxy ToolShed
galaxy_search("bwa")
  |> each(|t| print(t.name + " by " + t.owner + " (" + str(t.downloads) + " downloads)"))
```

Each record in the result contains:

| Field         | Type   | Description                        |
|---------------|--------|------------------------------------|
| `name`        | String | Repository name                    |
| `owner`       | String | ToolShed username of the maintainer|
| `description` | String | Short summary                      |
| `downloads`   | Int    | Total download count               |
| `url`         | String | ToolShed repository URL            |

The default result set includes all matches. Pass a second argument to cap the
number of results, which is useful in exploratory sessions where you want a
quick overview.

```
# Top 5 matches for short-read aligners
galaxy_search("bwa", 5)
  |> each(|t| print(t.name + " (" + t.owner + "): " + t.description))
```

Search terms are matched against repository names and descriptions, so broader
queries work too.

```
# Find tools related to RNA-seq
galaxy_search("rna-seq", 10)
  |> filter(|t| t.downloads > 1000)
  |> each(|t| print(t.name + ": " + str(t.downloads) + " downloads"))
```

## Popular Tools

`galaxy_popular` returns the most-downloaded repositories in the ToolShed,
sorted by download count descending. This is useful for discovering which
tools the Galaxy community relies on most heavily.

```
galaxy_popular(10)
  |> map(|t| { name: t.name, owner: t.owner, downloads: t.downloads })
```

Without an argument, `galaxy_popular()` returns a default set. Pass a limit
to widen or narrow the list.

```
# Top 25 Galaxy tools by download count
galaxy_popular(25)
  |> each(|t| print(t.name + " by " + t.owner + " - " + str(t.downloads) + " downloads"))
```

## Categories

The ToolShed organizes repositories into categories such as "Assembly",
"Variant Analysis", "RNA", and "Visualization". `galaxy_categories` returns
them all as a list of records.

```
galaxy_categories()
  |> each(|c| print(c.name + ": " + c.description))
```

You can use this to explore what kinds of tools exist before drilling into
a specific area.

```
# Find categories related to sequencing
galaxy_categories()
  |> filter(|c| contains(lower(c.name), "sequenc") or contains(lower(c.description), "sequenc"))
  |> each(|c| print(c.name))
```

## Repository Details

`galaxy_tool` returns a detailed record for a single repository, identified
by owner and name.

```
let tool = galaxy_tool("devteam", "bwa")
print("Tool: " + tool.name)
print("Owner: " + tool.owner)
print("Downloads: " + str(tool.downloads))
print("Last updated: " + tool.last_updated)
print("Description: " + tool.description)
```

The returned record includes fields beyond what the search results provide,
such as `last_updated` and the full repository URL. Use this to verify that
a tool is actively maintained before building a workflow around it.

```
# Check maintenance status of a tool before adopting it
let tool = galaxy_tool("iuc", "samtools_sort")
print(tool.name + " last updated: " + tool.last_updated)
print("Downloads: " + str(tool.downloads))
```

## Cross-Registry Discovery

One of BioLang's strengths is that nf-core, BioContainers, and Galaxy ToolShed
are all accessible from the same script. When evaluating a tool, you can check
all three registries in a single pass to understand the full landscape --
whether curated pipelines exist, whether container images are available, and
whether Galaxy wrappers have been written.

```
# Search for BWA across all three registries
let tool = "bwa"

let nf_results = nfcore_search(tool)
let bc_results = biocontainers_search(tool, 5)
let gx_results = galaxy_search(tool, 5)

print("=== Cross-registry search for: " + tool + " ===")
print("nf-core pipelines mentioning " + tool + ": " + str(len(nf_results)))
print("BioContainers: " + str(len(bc_results)))
print("Galaxy tools: " + str(len(gx_results)))
```

This pattern scales to any tool or keyword. Here is a more thorough version
that prints details from each registry side by side.

```
# Compare tool availability across registries
let tools = ["samtools", "bcftools", "hisat2", "salmon"]

tools |> each(|name| {
  print("--- " + name + " ---")

  let gx = galaxy_search(name, 3)
  let bc = biocontainers_search(name, 3)

  if len(gx) > 0 then
    print("  Galaxy: " + gx[0].name + " by " + gx[0].owner
          + " (" + str(gx[0].downloads) + " downloads)")
  else
    print("  Galaxy: not found")

  if len(bc) > 0 then
    print("  BioContainers: " + bc[0].name
          + " (latest: " + bc[0].latest_version + ")")
  else
    print("  BioContainers: not found")

  print("")
})
```

## Workflow Code Generation

Galaxy workflows are JSON documents that describe a directed acyclic graph of
tool invocations. BioLang can parse these workflows and generate equivalent
BioLang pipeline code via the `galaxy_to_bl` builtin. This is useful when
migrating from Galaxy to a script-based workflow, or when you want to use a
Galaxy workflow as a starting point for a BioLang pipeline.

`galaxy_to_bl` takes a record that represents a Galaxy workflow and returns a
string of BioLang code. The expected input format mirrors the structure of a
Galaxy workflow export.

```
# Construct a Galaxy workflow record
let workflow = {
  name: "RNA-seq Analysis",
  annotation: "Basic RNA-seq pipeline",
  steps: [
    { name: "FastQC", tool_id: "fastqc/0.74", inputs: ["reads"], outputs: ["report"] },
    { name: "HISAT2", tool_id: "hisat2/2.2.1", inputs: ["reads"], outputs: ["aligned_bam"] },
    { name: "featureCounts", tool_id: "featurecounts/2.0.6", inputs: ["aligned_bam"], outputs: ["counts"] }
  ]
}

# Generate BioLang pipeline code
let bl_code = galaxy_to_bl(workflow)
print(bl_code)
```

The generated code maps each Galaxy step to a BioLang function call, preserving
the data flow between steps. You can write it directly to a file and use it as
a starting point for further customization.

```
# Save the generated code
let bl_code = galaxy_to_bl(workflow)
write_text("rnaseq.bl", bl_code)
print("Pipeline written to rnaseq.bl")
```

For more complex workflows with branching and multiple outputs, the generated
code includes comments marking each step and its connections.

```
# A branching QC + alignment workflow
let workflow = {
  name: "QC and Align",
  annotation: "Parallel QC with alignment",
  steps: [
    { name: "FastQC", tool_id: "fastqc/0.74", inputs: ["reads"], outputs: ["qc_report"] },
    { name: "Trimmomatic", tool_id: "trimmomatic/0.39", inputs: ["reads"], outputs: ["trimmed"] },
    { name: "BWA-MEM2", tool_id: "bwa_mem2/2.2.1", inputs: ["trimmed"], outputs: ["aligned"] },
    { name: "MultiQC", tool_id: "multiqc/1.14", inputs: ["qc_report"], outputs: ["summary"] }
  ]
}

let bl_code = galaxy_to_bl(workflow)
print(bl_code)
```

## Configuration

The Galaxy ToolShed URL defaults to the main public instance at
`https://toolshed.g2.bx.psu.edu`. You can override it for private or
institutional ToolShed installations.

Set the URL in `~/.biolang/apis.yaml`:

```
galaxy:
  toolshed_url: "https://toolshed.example.org"
```

Or via the environment variable `BIOLANG_GALAXY_TOOLSHED_URL`:

```
export BIOLANG_GALAXY_TOOLSHED_URL="https://toolshed.example.org"
```

The environment variable takes precedence over the config file. If neither is
set, the public ToolShed is used.

## Example: Building a Tool Inventory

A core facility managing Galaxy and non-Galaxy workflows needs to know which
tools are available in the ToolShed and whether container images exist for
running them outside Galaxy. This script searches Galaxy for tools matching a
keyword, then cross-references each one with BioContainers to check container
availability.

```
# Build a tool inventory for variant analysis
let keyword = "variant"

# Step 1: Search Galaxy for variant-related tools
let gx_tools = galaxy_search(keyword, 20)
print("Found " + str(len(gx_tools)) + " Galaxy tools for: " + keyword)
print("")

# Step 2: For each Galaxy tool, check BioContainers
let inventory = gx_tools |> map(|t| {
  # Search BioContainers using the tool name
  let bc = biocontainers_search(t.name, 1)

  let container_status = if len(bc) > 0 then
    "available (" + bc[0].latest_version + ")"
  else
    "not found"

  {
    name: t.name,
    owner: t.owner,
    galaxy_downloads: t.downloads,
    container: container_status
  }
})

# Step 3: Report
print("Tool Inventory: " + keyword)
print("========================================")
inventory
  |> sort_by(|a| -a.galaxy_downloads)
  |> each(|t| {
    print(t.name + " (" + t.owner + ")")
    print("  Galaxy downloads: " + str(t.galaxy_downloads))
    print("  BioContainers:    " + t.container)
  })

# Step 4: Summary statistics
let with_container = inventory |> filter(|t| not starts_with(t.container, "not"))
let without_container = inventory |> filter(|t| starts_with(t.container, "not"))

print("")
print("Summary:")
print("  Total tools:          " + str(len(inventory)))
print("  With container image: " + str(len(with_container)))
print("  Without container:    " + str(len(without_container)))
```

You can extend this pattern to cover additional registries or to export the
inventory as a structured file.

```
# Export as JSON for downstream use
inventory |> write_json("variant_tool_inventory.json")
```

## Example: Migrating a Galaxy Workflow

When moving a project from Galaxy to BioLang, you can combine the ToolShed
lookup with code generation in a single script. Look up each tool to verify
it exists, then generate the BioLang equivalent.

```
# Define the workflow steps from a Galaxy export
let workflow = {
  name: "Whole Genome Variant Calling",
  annotation: "BWA-MEM2 alignment with GATK HaplotypeCaller",
  steps: [
    { name: "FastQC", tool_id: "fastqc/0.74", inputs: ["reads_r1", "reads_r2"], outputs: ["qc_report"] },
    { name: "BWA-MEM2", tool_id: "bwa_mem2/2.2.1", inputs: ["reads_r1", "reads_r2"], outputs: ["aligned_bam"] },
    { name: "MarkDuplicates", tool_id: "picard_markduplicates/3.1.1", inputs: ["aligned_bam"], outputs: ["dedup_bam"] },
    { name: "HaplotypeCaller", tool_id: "gatk4_haplotypecaller/4.5", inputs: ["dedup_bam"], outputs: ["raw_vcf"] },
    { name: "FilterVcf", tool_id: "gatk4_filtermutectcalls/4.5", inputs: ["raw_vcf"], outputs: ["filtered_vcf"] }
  ]
}

# Verify each tool exists in the ToolShed
print("Verifying tools...")
workflow.steps |> each(|step| {
  # Extract base tool name from tool_id
  let parts = split(step.tool_id, "/")
  let tool_name = parts[0]
  let results = galaxy_search(tool_name, 1)
  if len(results) > 0 then
    print("  " + step.name + ": found (" + results[0].owner + ")")
  else
    print("  " + step.name + ": WARNING - not found in ToolShed")
})

# Generate the BioLang pipeline
print("")
let bl_code = galaxy_to_bl(workflow)
print("Generated pipeline:")
print(bl_code)

# Save to file
write_text("wgs_variant_calling.bl", bl_code)
print("Pipeline saved to wgs_variant_calling.bl")
```

## Summary

BioLang's Galaxy ToolShed builtins -- `galaxy_search`, `galaxy_popular`,
`galaxy_categories`, `galaxy_tool`, and `galaxy_to_bl` -- bring the Galaxy
ecosystem into your scripts as native data. Use them to discover tools,
evaluate popularity and maintenance status, cross-reference with BioContainers
and nf-core, and generate BioLang pipeline code from Galaxy workflows. The
ToolShed becomes one more queryable registry alongside the others, all
accessible from the same pipes, filters, and record operations you use for
everything else.
