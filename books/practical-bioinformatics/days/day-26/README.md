# Day 26: AI-Assisted Analysis

| | |
|---|---|
| **Difficulty** | Intermediate |
| **Biology knowledge** | Intermediate (gene expression, variants, pathway analysis) |
| **Coding knowledge** | Intermediate (functions, records, pipes, tables, string operations) |
| **Time** | ~3 hours |
| **Prerequisites** | Days 1--25 completed, BioLang installed (see Appendix A) |
| **Data needed** | Generated locally via `init.bl` (simulated gene lists, variant data) |

## Quick Start

```bash
cd days/day-26
bl init.bl
bl scripts/analysis.bl    # requires LLM API key
```

## What You'll Learn

- Using BioLang's LLM builtins (`chat`, `chat_code`, `llm_models`) for biological interpretation
- Configuring LLM providers (Anthropic, OpenAI, Ollama, OpenAI-compatible)
- Prompt engineering techniques for biological questions
- Passing structured context (records, tables, lists) to AI
- Building human-in-the-loop analysis pipelines
- Verifying AI output against programmatic checks
- Cost management via caching and batching

## Files

| File | Description |
|------|-------------|
| `init.bl` | Generates example gene lists, variant data, and sequence files |
| `scripts/analysis.bl` | Full AI-assisted analysis pipeline (BioLang, requires LLM API key) |
| `scripts/analysis.py` | Python equivalent using openai/anthropic SDK |
| `scripts/analysis.R` | R equivalent using httr2 |
| `python/requirements.txt` | Python dependencies |
| `r/install.R` | R package installation |
| `compare.md` | Language comparison (BioLang vs Python vs R) |
| `expected/output.txt` | Expected output structure |

## LLM Provider Setup

Set one of these environment variables before running:

```bash
# Option 1: Anthropic (Claude)
export ANTHROPIC_API_KEY="sk-ant-..."

# Option 2: OpenAI (GPT)
export OPENAI_API_KEY="sk-..."

# Option 3: Ollama (local, free)
ollama pull llama3.1
export OLLAMA_MODEL="llama3.1"

# Option 4: Any OpenAI-compatible provider
export LLM_BASE_URL="https://api.together.xyz"
export LLM_API_KEY="..."
export LLM_MODEL="meta-llama/Llama-3-70b-chat-hf"
```

## Important Safety Note

All AI-generated interpretations must be verified against primary databases (NCBI, UniProt, OMIM, PubMed) before use in research or clinical settings. LLMs can hallucinate gene functions, fabricate citations, and produce confident but incorrect biological claims.
