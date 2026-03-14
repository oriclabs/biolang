# Day 26: Language Comparison --- AI-Assisted Analysis

## Line Counts

| Operation | BioLang | Python (openai SDK) | R (httr2 + jsonlite) |
|-----------|---------|---------------------|----------------------|
| Provider detection | 5 (built-in) | 22 | 22 |
| Chat wrapper | 1 (built-in) | 20 | 35 |
| Load + filter DE genes | 4 | 10 | 6 |
| Compute statistics | 10 | 12 | 12 |
| AI interpretation call | 8 | 8 | 8 |
| Variant annotation loop | 30 | 35 | 42 |
| Sequence feature extraction | 10 | 22 | 26 |
| Report writing | 25 | 30 | 35 |
| Error handling | 3 (try/catch) | 5 (try/except) | 4 (tryCatch) |
| **Total script** | **~95** | **~175** | **~200** |

## Key Differences

### LLM Integration

```
# BioLang --- built-in, auto-detects provider
let answer = chat("What does TP53 do?")
let code = chat_code("Write a filter pipeline")
let config = llm_models()

# Python --- requires SDK install, manual client setup
from openai import OpenAI
client = OpenAI(api_key=os.environ["OPENAI_API_KEY"])
response = client.chat.completions.create(
    model="gpt-4o",
    messages=[{"role": "user", "content": "What does TP53 do?"}]
)
answer = response.choices[0].message.content

# R --- requires httr2, manual HTTP construction
library(httr2)
resp <- request("https://api.openai.com/v1/chat/completions") |>
  req_headers(Authorization = paste("Bearer", Sys.getenv("OPENAI_API_KEY"))) |>
  req_body_json(list(
    model = "gpt-4o",
    messages = list(list(role = "user", content = "What does TP53 do?"))
  )) |>
  req_perform()
answer <- resp_body_json(resp)$choices[[1]]$message$content
```

### Context Passing

```
# BioLang --- pass any value type as second arg, auto-formatted
let result = chat("Interpret this data", my_table)
let result = chat("Explain this gene", {symbol: "BRCA1", fc: -2.3})
let result = chat("Categorize these", ["TP53", "MYC", "KRAS"])

# Python --- must manually format context into string
if isinstance(context, dict):
    ctx_str = "\n".join(f"{k}: {v}" for k, v in context.items())
user_msg = f"{prompt}\n\n--- Context ---\n{ctx_str}"

# R --- must manually format context into string
ctx_str <- paste(names(context), context, sep = ": ", collapse = "\n")
user_msg <- paste0(prompt, "\n\n--- Context ---\n", ctx_str)
```

### Error Handling for API Calls

```
# BioLang --- expression-based try/catch
let answer = try { chat("Question") } catch err { "fallback" }

# Python --- statement-based try/except
try:
    answer = client.chat.completions.create(...)
    answer = answer.choices[0].message.content
except Exception as e:
    answer = "fallback"

# R --- function-based tryCatch
answer <- tryCatch(
  { resp <- req_perform(req); resp_body_json(resp)$choices[[1]]$message$content },
  error = function(e) "fallback"
)
```

### Multi-Provider Support

```
# BioLang --- automatic, just set env vars
# Supports: Anthropic, OpenAI, Ollama, any OpenAI-compatible
# Zero code changes needed to switch providers

# Python --- openai SDK works for OpenAI + compatible providers
# For Anthropic, need separate anthropic SDK or use openai with base_url
client = OpenAI(api_key=key, base_url="https://api.anthropic.com/v1/")

# R --- must implement each provider's API format manually
# Anthropic uses different headers and response format than OpenAI
```

## Why BioLang Is More Concise

1. **Built-in LLM functions.** `chat()`, `chat_code()`, and `llm_models()` are first-class builtins. No SDK installation, no client initialization, no import statements.

2. **Automatic context formatting.** Pass records, tables, or lists directly --- BioLang converts them to readable text. Python and R require manual serialization.

3. **Auto-detection.** Provider switching requires only changing an environment variable. No code changes.

4. **Bioinformatics system prompt.** `chat()` includes a built-in system prompt tuned for biological analysis. Python and R require writing your own.

5. **Expression-based try/catch.** Error handling is inline, returning a value. Python and R require multi-line try blocks.
