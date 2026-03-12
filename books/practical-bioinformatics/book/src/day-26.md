# Day 26: AI-Assisted Analysis

| | |
|---|---|
| **Difficulty** | Intermediate |
| **Biology knowledge** | Intermediate (gene expression, variants, pathway analysis) |
| **Coding knowledge** | Intermediate (functions, records, pipes, tables, string operations) |
| **Time** | ~3 hours |
| **Prerequisites** | Days 1--25 completed, BioLang installed (see Appendix A) |
| **Data needed** | Generated locally via `init.bl` (simulated gene lists, variant data) |

## What You'll Learn

- How to use BioLang's built-in LLM functions (`chat`, `chat_code`, `llm_models`)
- How to configure LLM providers (Anthropic, OpenAI, Ollama, OpenAI-compatible)
- How to engineer effective prompts for biological questions
- How to pass structured data as context for AI-assisted interpretation
- How to build human-in-the-loop analysis pipelines
- Why AI outputs must always be verified before use in research or clinical settings
- How to combine LLM interpretation with programmatic validation

---

## The Problem

*"Can AI help me interpret these results or write analysis code?"*

You have just completed a differential expression analysis. In front of you is a table of 500 genes --- each with a fold change, a p-value, and a gene symbol. Some of these genes are well-characterized cancer drivers. Others are poorly annotated lncRNAs. A few are housekeeping genes that probably represent technical noise. You need to sort signal from noise, identify biologically meaningful patterns, connect your findings to known pathways, and write a paragraph for your manuscript's results section.

This is the kind of task where large language models can accelerate your work. An LLM can summarize what is known about a gene, suggest pathway connections, draft interpretive text, and even generate analysis code. But it can also hallucinate citations, fabricate gene functions, and confidently present incorrect biological claims. The challenge is using AI as a genuine accelerator while maintaining scientific rigor.

This chapter teaches you to integrate LLMs into your BioLang workflows --- not as a replacement for domain expertise, but as a tool that amplifies it.

![AI-Assisted Analysis Architecture: chat(), chat_code(), and llm_models() functions connecting to auto-detected LLM providers](images/day26-ai-architecture.svg)

> **Critical safety note.** Every AI-generated interpretation in this chapter must be treated as a hypothesis, not a fact. LLMs can hallucinate gene functions, fabricate citations, invent protein interactions, and produce plausible-sounding but incorrect biological claims. Never use LLM output directly in a clinical report, grant application, or publication without independent verification against primary sources (NCBI Gene, UniProt, PubMed, OMIM).

---

## Section 1: Setting Up LLM Access

Before using `chat()` or `chat_code()`, you need to configure an LLM provider. BioLang auto-detects your provider from environment variables in this priority order:

1. `ANTHROPIC_API_KEY` --- uses Claude (default model: `claude-sonnet-4-20250514`)
2. `OPENAI_API_KEY` --- uses GPT (default model: `gpt-4o`)
3. `OLLAMA_MODEL` --- uses a local Ollama instance (no API key needed)
4. `LLM_BASE_URL` + `LLM_API_KEY` --- any OpenAI-compatible endpoint

### Checking Your Configuration

<!-- requires: LLM API key (ANTHROPIC_API_KEY or OPENAI_API_KEY) -->

```bio
let config = llm_models()
println(f"Provider: {config.provider}")
println(f"Model: {config.model}")
println(f"Configured: {config.configured}")
```

If `configured` is `false`, no provider environment variable is set. The `env_vars` field lists all options:

```bio
let config = llm_models()
if config.configured == false {
    println("No LLM provider configured. Set one of:")
    config.env_vars |> each(|v| println(f"  {v}"))
}
```

### Provider Setup Examples

**Anthropic (Claude):**
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
# Optional: override model
export ANTHROPIC_MODEL="claude-sonnet-4-20250514"
```

**OpenAI (GPT):**
```bash
export OPENAI_API_KEY="sk-..."
# Optional: override model
export OPENAI_MODEL="gpt-4o"
```

**Ollama (local, free):**
```bash
# First install and run Ollama, then pull a model
ollama pull llama3.1
export OLLAMA_MODEL="llama3.1"
```

**OpenAI-compatible (Together, Groq, LM Studio):**
```bash
export LLM_BASE_URL="https://api.together.xyz"
export LLM_API_KEY="..."
export LLM_MODEL="meta-llama/Llama-3-70b-chat-hf"
```

For this chapter, any provider will work. Ollama is a good choice if you want to avoid API costs --- just note that smaller local models produce less accurate biological interpretations than large cloud models.

---

## Section 2: Basic Chat Interaction

The `chat()` function sends a prompt to your configured LLM and returns the response as a string. It accepts one or two arguments:

- `chat(prompt)` --- simple question
- `chat(prompt, context)` --- question with additional context (string, record, list, or table)

### Simple Questions

<!-- requires: LLM API key (ANTHROPIC_API_KEY or OPENAI_API_KEY) -->

```bio
let answer = chat("What is the function of the TP53 gene in cancer biology? Two sentences.")
println(answer)
```

The LLM behind `chat()` is configured with a bioinformatics system prompt, so it understands BioLang syntax and biological terminology.

### Providing Context

The second argument to `chat()` passes structured data as context. BioLang automatically formats records, lists, and tables into a readable text representation:

<!-- requires: LLM API key (ANTHROPIC_API_KEY or OPENAI_API_KEY) -->

```bio
let gene_info = {
    symbol: "BRCA1",
    fold_change: -2.3,
    pvalue: 0.0001,
    sample_type: "triple-negative breast cancer"
}

let interpretation = chat(
    "Interpret the significance of this differentially expressed gene in the given cancer type.",
    gene_info
)
println(interpretation)
```

When you pass a record, BioLang formats it as `key: value` lines. When you pass a table, it formats as tab-separated values. When you pass a list, each element appears on its own line. This means you can pipe analysis results directly into LLM interpretation.

### Code Generation with chat_code()

The `chat_code()` function is specialized for generating BioLang code. It returns only valid BioLang syntax --- no explanations, no markdown fences:

<!-- requires: LLM API key (ANTHROPIC_API_KEY or OPENAI_API_KEY) -->

```bio
let code = chat_code("Write a function that calculates the ratio of transition to transversion mutations from a list of variant records with ref and alt fields.")
println(code)
```

> **Caution.** Always review generated code before executing it. `chat_code()` output may contain syntax errors, call nonexistent functions, or implement incorrect logic. Treat it as a first draft.

---

## Section 3: Prompt Engineering for Biology

The quality of LLM responses depends heavily on how you construct your prompts. Biological prompts require particular precision because ambiguous terminology is common (e.g., "expression" means different things in molecular biology vs. clinical medicine vs. software engineering).

![LLM prompting decision tree: raw question through context, data, and format constraints to verified output](images/day26-prompt-decision-tree.svg)

### Principle 1: Be Specific About Biological Context

Bad prompt:
```bio
let vague = chat("What does EGFR do?")
```

Better prompt:
```bio
let specific = chat("What is the role of EGFR in non-small cell lung cancer, specifically regarding tyrosine kinase inhibitor resistance mechanisms? Limit to 3 key points.")
```

### Principle 2: Specify the Output Format

<!-- requires: LLM API key (ANTHROPIC_API_KEY or OPENAI_API_KEY) -->

```bio
let prompt = "Given these differentially expressed genes, categorize them into: (1) known oncogenes, (2) tumor suppressors, (3) metabolic genes, (4) unknown/uncharacterized. Return as a simple list with the category before each gene name."

let genes = ["TP53", "BRCA1", "MYC", "GAPDH", "LINC01234", "KRAS", "RB1", "PKM", "ALDOA", "MALAT1"]

let categorized = chat(prompt, genes)
println(categorized)
```

### Principle 3: Chain Prompts for Complex Analysis

Rather than asking one massive question, break complex analysis into steps:

<!-- requires: LLM API key (ANTHROPIC_API_KEY or OPENAI_API_KEY) -->

```bio
let genes = ["BRCA1", "TP53", "CDH1", "PTEN", "PIK3CA"]

let step1 = chat(
    "List the primary biological pathway for each of these genes. One line per gene, format: GENE - pathway name.",
    genes
)

let step2 = chat(
    "Based on these gene-pathway associations, what biological process is most likely disrupted? One paragraph.",
    step1
)

println("Pathway associations:")
println(step1)
println("")
println("Biological interpretation:")
println(step2)
```

### Principle 4: Request Uncertainty Acknowledgment

<!-- requires: LLM API key (ANTHROPIC_API_KEY or OPENAI_API_KEY) -->

```bio
let honest_prompt = "For each gene, state its known function and your confidence level (high/medium/low) based on how well-characterized it is. If a gene is poorly studied, say so explicitly rather than speculating."

let genes = ["TP53", "LINC01234", "LOC105377243"]
let result = chat(honest_prompt, genes)
println(result)
```

This prompt structure encourages the LLM to flag when it is uncertain rather than inventing plausible-sounding functions.

---

## Section 4: Variant Interpretation with AI

One of the most practical applications of LLM assistance is interpreting genetic variants. Clinicians and researchers routinely need to assess whether a variant is pathogenic, benign, or of uncertain significance. An LLM can summarize known information about a variant, but the final classification must always come from curated databases and expert review.

### Building a Variant Interpretation Pipeline

<!-- requires: LLM API key (ANTHROPIC_API_KEY or OPENAI_API_KEY) -->

```bio
let variants = read_tsv("data/variants.tsv")

let interpret_variant = |variant| {
    let prompt = f"Interpret this genetic variant for clinical significance. Include: (1) gene function, (2) known disease associations, (3) predicted functional impact, (4) whether this position is conserved. State clearly if you are uncertain about any point."

    let context = {
        gene: variant.gene,
        chromosome: variant.chrom,
        position: variant.pos,
        ref_allele: variant.ref,
        alt_allele: variant.alt,
        consequence: variant.consequence
    }

    try {
        chat(prompt, context)
    } catch err {
        f"Error interpreting {variant.gene}: {err}"
    }
}

let top_variants = variants
    |> filter(|v| v.consequence != "synonymous_variant")
    |> sort(|a, b| a.gene < b.gene)

top_variants |> each(|v| {
    println(f"=== {v.gene} {v.chrom}:{v.pos} {v.ref}>{v.alt} ===")
    let interp = interpret_variant(v)
    println(interp)
    println("")
})
```

> **Clinical warning.** This is an educational exercise. Real clinical variant interpretation requires validated pipelines, accredited laboratories, ACMG/AMP guideline compliance, and review by board-certified clinical geneticists. Never use raw LLM output for patient care decisions.

### Adding Programmatic Checks

LLM interpretation is more useful when combined with programmatic data. Here is a pattern that cross-references AI interpretation with structured variant annotations:

<!-- requires: LLM API key (ANTHROPIC_API_KEY or OPENAI_API_KEY) -->

```bio
let annotate_variant = |v| {
    let known_oncogenes = ["TP53", "BRCA1", "BRCA2", "KRAS", "EGFR", "PIK3CA", "BRAF", "MYC", "RB1", "PTEN"]
    let is_cancer_gene = known_oncogenes |> filter(|g| g == v.gene) |> len() > 0

    let is_missense = v.consequence == "missense_variant"
    let is_nonsense = v.consequence == "stop_gained"
    let is_frameshift = contains(v.consequence, "frameshift")

    let severity = if is_nonsense { "high" }
        else if is_frameshift { "high" }
        else if is_missense { "moderate" }
        else { "low" }

    let context = {
        gene: v.gene,
        position: f"{v.chrom}:{v.pos}",
        change: f"{v.ref}>{v.alt}",
        consequence: v.consequence,
        cancer_gene: is_cancer_gene,
        computed_severity: severity
    }

    let ai_interp = try {
        chat("Briefly interpret this variant's clinical significance in 2-3 sentences. Note if the gene is a known cancer driver.", context)
    } catch err {
        "AI interpretation unavailable"
    }

    {
        gene: v.gene,
        variant: f"{v.chrom}:{v.pos}{v.ref}>{v.alt}",
        consequence: v.consequence,
        severity: severity,
        cancer_gene: is_cancer_gene,
        ai_interpretation: ai_interp
    }
}

let variants = read_tsv("data/variants.tsv")
let annotated = variants |> map(annotate_variant)

annotated |> each(|a| {
    println(f"Gene: {a.gene}")
    println(f"  Variant: {a.variant}")
    println(f"  Consequence: {a.consequence}")
    println(f"  Severity: {a.severity}")
    println(f"  Cancer gene: {a.cancer_gene}")
    println(f"  AI: {a.ai_interpretation}")
    println("")
})
```

---

## Section 5: Gene List Analysis

Differential expression experiments produce gene lists that need biological interpretation. LLMs can help identify functional themes, but they work best when you provide structured summary data rather than raw lists of hundreds of genes.

### Summarizing a Gene List

<!-- requires: LLM API key (ANTHROPIC_API_KEY or OPENAI_API_KEY) -->

```bio
let de_genes = read_tsv("data/de_genes.tsv")

let upregulated = de_genes |> filter(|g| g.log2fc > 1.0 and g.padj < 0.05)
let downregulated = de_genes |> filter(|g| g.log2fc < -1.0 and g.padj < 0.05)

let up_names = upregulated |> map(|g| g.gene) |> sort(|a, b| a < b)
let down_names = downregulated |> map(|g| g.gene) |> sort(|a, b| a < b)

let summary = {
    total_tested: len(de_genes),
    significant_up: len(up_names),
    significant_down: len(down_names),
    top_up_genes: up_names |> map(|g| str(g)) |> join(", "),
    top_down_genes: down_names |> map(|g| str(g)) |> join(", "),
    experiment: "RNA-seq, tumor vs normal, breast tissue"
}

let interpretation = chat(
    "Analyze this differential expression result. Identify: (1) key biological themes among upregulated genes, (2) key themes among downregulated genes, (3) potential pathway disruptions, (4) any genes that warrant follow-up experiments. Be specific about which genes drive each conclusion.",
    summary
)

println(interpretation)
```

### Batch Gene Annotation

For larger gene lists, process in batches to avoid overwhelming the LLM context window:

<!-- requires: LLM API key (ANTHROPIC_API_KEY or OPENAI_API_KEY) -->

```bio
let batch_annotate = |genes, batch_size| {
    let results = []
    let n = len(genes)
    let i = 0
    let batches = []

    let current = []
    genes |> each(|g| {
        current = current + [g]
        if len(current) >= batch_size {
            batches = batches + [current]
            current = []
        }
    })
    if len(current) > 0 {
        batches = batches + [current]
    }

    batches |> map(|batch| {
        let gene_list = batch |> join(", ")
        let prompt = "For each gene, provide: gene symbol, primary function (one phrase), associated disease if any. Format as one line per gene."
        try {
            chat(prompt, gene_list)
        } catch err {
            f"Batch failed: {err}"
        }
    })
}

let genes = read_tsv("data/de_genes.tsv")
    |> filter(|g| g.padj < 0.01)
    |> map(|g| g.gene)

let annotations = batch_annotate(genes, 10)
annotations |> each(|batch_result| {
    println(batch_result)
    println("---")
})
```

---

## Section 6: Building AI-Augmented Pipelines

The real power of LLM integration appears when you combine it with BioLang's data processing capabilities in end-to-end pipelines.

### The Human-in-the-Loop Pattern

![Human-in-the-Loop Analysis Cycle: Load, Compute, AI Interprets, Human Reviews, Accept or Reject with re-prompt loop](images/day26-human-in-loop.svg)

The key principle: computation is programmatic, interpretation is AI-assisted, decisions are human. Never automate the human review step.

### Full Pipeline: DE Gene Report

This pipeline loads differential expression data, computes summary statistics, asks an LLM to interpret the biology, and writes both the raw data and interpretation to a report file:

<!-- requires: LLM API key (ANTHROPIC_API_KEY or OPENAI_API_KEY) -->

```bio
let de_genes = read_tsv("data/de_genes.tsv")

let sig = de_genes |> filter(|g| g.padj < 0.05)
let up = sig |> filter(|g| g.log2fc > 1.0)
let down = sig |> filter(|g| g.log2fc < -1.0)

let stats = {
    total_genes: len(de_genes),
    significant: len(sig),
    upregulated: len(up),
    downregulated: len(down),
    mean_abs_fc: sig |> map(|g| if g.log2fc > 0 { g.log2fc } else { -1.0 * g.log2fc }) |> mean(),
    top_up: up |> sort(|a, b| a.padj < b.padj) |> map(|g| g.gene) |> join(", "),
    top_down: down |> sort(|a, b| a.padj < b.padj) |> map(|g| g.gene) |> join(", ")
}

let interpretation = try {
    chat(
        "Write a results paragraph for a manuscript describing this differential expression analysis of breast cancer tumor vs normal tissue. Include: (1) overall summary, (2) notable upregulated pathways, (3) notable downregulated pathways, (4) suggested follow-up experiments. Be specific about gene names.",
        stats
    )
} catch err {
    f"[AI interpretation unavailable: {err}]"
}

let report_lines = [
    "# Differential Expression Report",
    "",
    "## Summary Statistics",
    f"Total genes tested: {stats.total_genes}",
    f"Significant (padj < 0.05): {stats.significant}",
    f"Upregulated (log2FC > 1): {stats.upregulated}",
    f"Downregulated (log2FC < -1): {stats.downregulated}",
    f"Mean absolute fold change: {stats.mean_abs_fc}",
    "",
    "## Top Upregulated Genes",
    stats.top_up,
    "",
    "## Top Downregulated Genes",
    stats.top_down,
    "",
    "## AI-Assisted Interpretation",
    "NOTE: The following was generated by an LLM and requires expert review.",
    "",
    interpretation
]

write_lines(report_lines, "data/output/de_report.txt")
println("Report written to data/output/de_report.txt")
```

### Pipeline: Sequence Feature Interpretation

<!-- requires: LLM API key (ANTHROPIC_API_KEY or OPENAI_API_KEY) -->

```bio
let sequences = read_fasta("data/sequences.fasta")

let features = sequences |> map(|seq| {
    let gc = gc_content(seq.sequence)
    let length = len(seq.sequence)
    let at_rich = gc < 0.4
    let gc_rich = gc > 0.6

    {
        id: seq.id,
        length: length,
        gc_content: gc,
        at_rich: at_rich,
        gc_rich: gc_rich
    }
})

let feature_table = features |> to_table()

let summary = {
    num_sequences: len(features),
    mean_gc: features |> map(|f| f.gc_content) |> mean(),
    mean_length: features |> map(|f| f.length) |> mean(),
    at_rich_count: features |> filter(|f| f.at_rich) |> len(),
    gc_rich_count: features |> filter(|f| f.gc_rich) |> len()
}

let interp = try {
    chat(
        "These are sequence composition statistics from a set of genomic regions. What biological significance might the GC content distribution suggest? Consider: promoter regions, coding vs non-coding, isochores, CpG islands. Be brief (3-4 sentences).",
        summary
    )
} catch err {
    "[Interpretation unavailable]"
}

println("Sequence features:")
println(feature_table)
println("")
println("AI interpretation:")
println(interp)
```

---

## Section 7: Limitations and Best Practices

### What LLMs Are Good At in Bioinformatics

| Task | Reliability | Notes |
|------|------------|-------|
| Summarizing known gene functions | High | For well-studied genes (TP53, BRCA1, etc.) |
| Suggesting pathway connections | Medium | Cross-reference with KEGG, Reactome |
| Drafting results text | Medium | Always edit for accuracy |
| Generating analysis code | Medium | Always review and test |
| Interpreting novel gene functions | Low | Tends to hallucinate |
| Clinical variant classification | **Very Low** | Never rely on LLM alone |
| Citing literature | **Very Low** | Frequently fabricates citations |

### What LLMs Cannot Do

1. **Access real-time data.** LLMs have a training cutoff date. They cannot check the current ClinVar entry for a variant or find papers published last month.

2. **Perform calculations.** If you ask an LLM to compute a p-value, it will guess. Use BioLang's `mean()`, `sum()`, and statistical functions for computation.

3. **Guarantee biological accuracy.** An LLM might state that "GENE_X is a known tumor suppressor involved in DNA repair" even when GENE_X is a fictional gene or has no such function.

4. **Replace peer review.** LLM-generated text sounds authoritative but may contain subtle errors that only a domain expert would catch.

### Best Practice: The Verification Pattern

<!-- requires: LLM API key (ANTHROPIC_API_KEY or OPENAI_API_KEY) -->

```bio
let verify_gene_claim = |gene, claim| {
    let check_prompt = f"Regarding the gene {gene}: Is the following claim accurate? Answer ONLY 'verified', 'uncertain', or 'likely incorrect', followed by a one-sentence justification.\n\nClaim: {claim}"

    let verification = try {
        chat(check_prompt)
    } catch err {
        "verification_failed"
    }

    {
        gene: gene,
        claim: claim,
        verification: verification,
        needs_manual_review: contains(verification, "uncertain") or contains(verification, "incorrect") or contains(verification, "failed")
    }
}

let result = verify_gene_claim("BRCA1", "BRCA1 is involved in homologous recombination DNA repair")
println(f"Verification: {result.verification}")
println(f"Needs manual review: {result.needs_manual_review}")
```

> **Important.** Using an LLM to verify another LLM's output is better than nothing, but it is not equivalent to checking a primary database. The verification pattern above is a triage step --- it flags claims that the LLM itself is uncertain about, but it can still miss confident-sounding errors.

### Best Practice: Always Wrap in try/catch

LLM API calls can fail for many reasons: network issues, rate limits, expired API keys, provider outages. Every `chat()` call in a pipeline should be wrapped in `try/catch`:

```bio
let safe_interpret = |data| {
    try {
        chat("Interpret this data briefly.", data)
    } catch err {
        f"[AI unavailable: {err}]"
    }
}
```

This ensures your pipeline continues even when the LLM is unreachable.

### Best Practice: Separate Computation from Interpretation

```bio
# GOOD: compute programmatically, interpret with AI
let gc = gc_content(seq)
let interp = chat(f"What might a GC content of {gc} suggest about this genomic region?")

# BAD: ask the AI to compute
let result = chat("What is the GC content of ATCGATCGATCG?")
# The LLM might give the wrong number!
```

Never ask an LLM to perform calculations that BioLang can do directly.

---

## Section 8: Cost and Rate Limiting

LLM API calls cost money (except Ollama) and are rate-limited. When processing large datasets, consider:

### Caching Responses

```bio
let cached_interpret = |gene, cache_path| {
    if file_exists(cache_path) {
        let lines = read_lines(cache_path)
        lines |> join("\n")
    } else {
        let result = chat(f"Summarize the function of {gene} in one paragraph.")
        write_lines([result], cache_path)
        result
    }
}

mkdir("data/cache")
let brca1_info = cached_interpret("BRCA1", "data/cache/BRCA1.txt")
println(brca1_info)
```

### Batching Genes

Rather than one API call per gene, batch multiple genes into a single prompt:

<!-- requires: LLM API key (ANTHROPIC_API_KEY or OPENAI_API_KEY) -->

```bio
let genes = ["TP53", "BRCA1", "KRAS", "EGFR", "PIK3CA"]

# One call instead of five
let batch_result = chat(
    "For each gene, provide a one-line summary of its role in cancer. Format: GENE: summary",
    genes
)
println(batch_result)
```

---

## Section 9: Generating Analysis Code

The `chat_code()` function generates BioLang code from natural language descriptions. This is useful for scaffolding new analyses, but the output should always be reviewed and tested before use.

### Example: Generating a Filter Pipeline

<!-- requires: LLM API key (ANTHROPIC_API_KEY or OPENAI_API_KEY) -->

```bio
let task = "Read a TSV file called 'results.tsv', filter rows where the column 'padj' is less than 0.05 and 'log2fc' is greater than 2, sort by padj ascending, and write the result to 'significant.tsv'"

let code = chat_code(task)
println("Generated code:")
println(code)
println("")
println("Review this code before running it!")
```

### Providing Context to chat_code()

You can pass existing code or data structures as context to help the LLM generate compatible code:

<!-- requires: LLM API key (ANTHROPIC_API_KEY or OPENAI_API_KEY) -->

```bio
let existing_code = "let data = read_tsv(\"samples.tsv\")\nlet filtered = data |> filter(|r| r.quality > 30)"

let code = chat_code(
    "Add a step that groups the filtered data by the 'tissue' column and computes the mean quality per group.",
    existing_code
)
println(code)
```

---

## Exercises

### Exercise 1: Gene Function Summarizer

Write a BioLang script that:
1. Reads the gene list from `data/de_genes.tsv`
2. Selects the top 5 most significantly upregulated genes (highest log2fc with padj < 0.05)
3. For each gene, uses `chat()` to get a one-sentence function summary
4. Writes the results (gene, fold change, p-value, AI summary) to `data/output/gene_summaries.txt`
5. Wraps every `chat()` call in `try/catch`

### Exercise 2: Prompt Comparison

Write a script that sends the same gene list to `chat()` with three different prompts:
1. A vague prompt ("Tell me about these genes")
2. A specific prompt with biological context
3. A prompt requesting structured output with confidence levels

Compare the responses. Which prompt produces the most useful output for a research context?

### Exercise 3: AI-Verified Variant Report

Extend the variant interpretation pipeline from Section 4:
1. For each variant, generate an AI interpretation
2. Then run a second `chat()` call asking the LLM to identify any claims in its own interpretation that it is less than 90% confident about
3. Flag variants where the AI self-reports uncertainty
4. Write a report with a "confidence" column

### Exercise 4: Code Generation Validator

Write a script that:
1. Uses `chat_code()` to generate a BioLang function
2. Uses `chat()` to review the generated code for potential bugs
3. Writes both the generated code and the review to a file
4. Adds a header warning that the code needs human review

---

## Key Takeaways

1. **Three LLM builtins.** `chat(prompt, context?)` for general questions, `chat_code(prompt, context?)` for code generation, `llm_models()` to check configuration.

2. **Auto-detection.** BioLang detects your LLM provider from environment variables: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `OLLAMA_MODEL`, or `LLM_BASE_URL`.

3. **Context is powerful.** Pass structured data (records, tables, lists) as the second argument to `chat()`. BioLang formats them automatically.

4. **Prompt engineering matters.** Specify organism, tissue, assay type, and desired output format. Chain multiple focused prompts rather than one massive question.

5. **Computation is programmatic.** Use BioLang functions for calculations (GC content, statistics, filtering). Use LLMs only for interpretation and text generation.

6. **Always verify.** LLMs hallucinate gene functions, fabricate citations, and invent protein interactions. Cross-reference every biological claim against NCBI, UniProt, OMIM, or PubMed.

7. **Always use try/catch.** LLM API calls can fail. Wrap every `chat()` call so your pipeline degrades gracefully.

8. **Never trust LLMs for clinical decisions.** AI-assisted interpretation is a research accelerator, not a substitute for clinical expertise, accredited laboratories, or ACMG/AMP guidelines.

9. **Cache and batch.** Save API costs by caching responses and batching multiple genes into single prompts.

10. **Human-in-the-loop.** The correct pattern is: compute programmatically, interpret with AI, review with human expertise. Never automate the review step.

---

## Next

In **Day 27**, we tackle **Pipeline Orchestration** --- chaining multi-step analyses into reproducible, resumable workflows that can handle sample sheets with hundreds of entries.
