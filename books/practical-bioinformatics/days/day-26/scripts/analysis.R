library(httr2)
library(jsonlite)

detect_provider <- function() {
  if (nzchar(key <- Sys.getenv("ANTHROPIC_API_KEY", ""))) {
    model <- Sys.getenv("ANTHROPIC_MODEL", "claude-sonnet-4-20250514")
    return(list(provider = "anthropic", key = key, model = model,
                base_url = "https://api.anthropic.com"))
  }
  if (nzchar(key <- Sys.getenv("OPENAI_API_KEY", ""))) {
    model <- Sys.getenv("OPENAI_MODEL", "gpt-4o")
    base <- Sys.getenv("OPENAI_BASE_URL", "https://api.openai.com")
    return(list(provider = "openai", key = key, model = model, base_url = base))
  }
  if (nzchar(model <- Sys.getenv("OLLAMA_MODEL", ""))) {
    base <- Sys.getenv("OLLAMA_BASE_URL", "http://localhost:11434")
    return(list(provider = "ollama", key = "ollama", model = model, base_url = base))
  }
  if (nzchar(base <- Sys.getenv("LLM_BASE_URL", ""))) {
    key <- Sys.getenv("LLM_API_KEY", "")
    model <- Sys.getenv("LLM_MODEL", "default")
    return(list(provider = "compatible", key = key, model = model, base_url = base))
  }
  stop("No LLM provider configured. Set OPENAI_API_KEY, ANTHROPIC_API_KEY, or OLLAMA_MODEL.")
}

chat_llm <- function(config, prompt, context = NULL, system_msg = NULL) {
  if (is.null(system_msg)) {
    system_msg <- paste(
      "You are a bioinformatics assistant. Be concise and practical.",
      "When discussing genes or variants, be specific about evidence."
    )
  }

  user_msg <- prompt
  if (!is.null(context)) {
    if (is.list(context) && !is.null(names(context))) {
      ctx_str <- paste(names(context), context, sep = ": ", collapse = "\n")
    } else if (is.vector(context)) {
      ctx_str <- paste(context, collapse = "\n")
    } else {
      ctx_str <- as.character(context)
    }
    user_msg <- paste0(prompt, "\n\n--- Context ---\n", ctx_str)
  }

  if (config$provider == "anthropic") {
    body <- list(
      model = config$model,
      max_tokens = 4096L,
      system = system_msg,
      messages = list(list(role = "user", content = user_msg))
    )
    resp <- tryCatch({
      request(paste0(config$base_url, "/v1/messages")) |>
        req_headers(
          `x-api-key` = config$key,
          `anthropic-version` = "2023-06-01",
          `content-type` = "application/json"
        ) |>
        req_body_json(body) |>
        req_perform()
    }, error = function(e) {
      return(NULL)
    })
    if (is.null(resp)) return("[AI unavailable]")
    parsed <- resp_body_json(resp)
    if (!is.null(parsed$error)) return(paste("[AI error:", parsed$error$message, "]"))
    return(parsed$content[[1]]$text)
  }

  # OpenAI-compatible (OpenAI, Ollama, etc.)
  body <- list(
    model = config$model,
    max_tokens = 4096L,
    messages = list(
      list(role = "system", content = system_msg),
      list(role = "user", content = user_msg)
    )
  )

  url <- paste0(config$base_url, "/v1/chat/completions")

  resp <- tryCatch({
    req <- request(url) |>
      req_headers(`content-type` = "application/json") |>
      req_body_json(body)
    if (nzchar(config$key)) {
      req <- req |> req_headers(Authorization = paste("Bearer", config$key))
    }
    req |> req_perform()
  }, error = function(e) {
    return(NULL)
  })

  if (is.null(resp)) return("[AI unavailable]")
  parsed <- resp_body_json(resp)
  if (!is.null(parsed$error)) return(paste("[AI error:", parsed$error$message, "]"))
  return(parsed$choices[[1]]$message$content)
}

compute_gc <- function(seq) {
  chars <- strsplit(toupper(seq), "")[[1]]
  gc_count <- sum(chars %in% c("G", "C"))
  gc_count / length(chars)
}

main <- function() {
  config <- detect_provider()
  cat(sprintf("Using provider: %s, model: %s\n", config$provider, config$model))

  # Load DE genes
  de_genes <- read.delim("data/de_genes.tsv", stringsAsFactors = FALSE)
  sig <- de_genes[de_genes$padj < 0.05, ]
  up <- sig[sig$log2fc > 1.0, ]
  down <- sig[sig$log2fc < -1.0, ]

  up_names <- up$gene[order(up$padj)]
  down_names <- down$gene[order(down$padj)]

  stats <- list(
    total_genes = nrow(de_genes),
    significant = nrow(sig),
    upregulated = nrow(up),
    downregulated = nrow(down),
    mean_abs_fc = mean(abs(sig$log2fc)),
    top_up = paste(up_names, collapse = ", "),
    top_down = paste(down_names, collapse = ", ")
  )

  interpretation <- chat_llm(
    config,
    paste(
      "Write a results paragraph for a manuscript describing this differential",
      "expression analysis of breast cancer tumor vs normal tissue. Include:",
      "(1) overall summary, (2) notable upregulated pathways,",
      "(3) notable downregulated pathways, (4) suggested follow-up experiments."
    ),
    context = stats
  )

  # Variant annotation
  variants <- read.delim("data/variants.tsv", stringsAsFactors = FALSE)
  known_oncogenes <- c(
    "TP53", "BRCA1", "BRCA2", "KRAS", "EGFR",
    "PIK3CA", "BRAF", "MYC", "RB1", "PTEN"
  )

  annotated <- list()
  nonsyn <- variants[variants$consequence != "synonymous_variant", ]
  for (i in seq_len(nrow(nonsyn))) {
    v <- nonsyn[i, ]
    is_cancer <- v$gene %in% known_oncogenes
    cons <- v$consequence
    severity <- if (cons == "stop_gained" || grepl("frameshift", cons) || grepl("splice", cons)) {
      "high"
    } else if (cons == "missense_variant") {
      "moderate"
    } else {
      "low"
    }

    context <- list(
      gene = v$gene,
      position = paste0(v$chrom, ":", v$pos),
      change = paste0(v$ref, ">", v$alt),
      consequence = cons,
      cancer_gene = is_cancer,
      computed_severity = severity
    )

    ai_interp <- chat_llm(
      config,
      "Briefly interpret this variant's clinical significance in 2-3 sentences.",
      context = context
    )

    annotated[[length(annotated) + 1]] <- list(
      gene = v$gene,
      variant = paste0(v$chrom, ":", v$pos, v$ref, ">", v$alt),
      consequence = cons,
      severity = severity,
      cancer_gene = is_cancer,
      ai_interpretation = ai_interp
    )
  }

  # Sequence features
  fasta_lines <- readLines("data/sequences.fasta")
  seq_features <- list()
  current_id <- NULL
  current_seq <- ""
  for (line in fasta_lines) {
    if (startsWith(line, ">")) {
      if (!is.null(current_id) && nzchar(current_seq)) {
        gc <- compute_gc(current_seq)
        seq_features[[length(seq_features) + 1]] <- list(
          id = current_id, length = nchar(current_seq),
          gc_content = gc, at_rich = gc < 0.4, gc_rich = gc > 0.6
        )
      }
      current_id <- sub("^>", "", line)
      current_seq <- ""
    } else {
      current_seq <- paste0(current_seq, line)
    }
  }
  if (!is.null(current_id) && nzchar(current_seq)) {
    gc <- compute_gc(current_seq)
    seq_features[[length(seq_features) + 1]] <- list(
      id = current_id, length = nchar(current_seq),
      gc_content = gc, at_rich = gc < 0.4, gc_rich = gc > 0.6
    )
  }

  gc_vals <- sapply(seq_features, function(s) s$gc_content)
  seq_summary <- list(
    num_sequences = length(seq_features),
    mean_gc = mean(gc_vals),
    at_rich_count = sum(sapply(seq_features, function(s) s$at_rich)),
    gc_rich_count = sum(sapply(seq_features, function(s) s$gc_rich))
  )

  seq_interp <- chat_llm(
    config,
    paste(
      "These are sequence composition statistics from genomic regions.",
      "What biological significance might the GC content distribution suggest?",
      "Consider promoter regions, coding vs non-coding, CpG islands.",
      "Be brief (3-4 sentences)."
    ),
    context = seq_summary
  )

  # Write report
  dir.create("data/output", showWarnings = FALSE, recursive = TRUE)
  out <- file("data/output/ai_report.txt", "w")
  writeLines("# AI-Assisted Analysis Report", out)
  writeLines(sprintf("# Generated: %s", format(Sys.time(), "%Y-%m-%d %H:%M")), out)
  writeLines("# NOTE: AI interpretations require expert verification", out)
  writeLines("", out)
  writeLines("## Differential Expression Summary", out)
  writeLines(sprintf("Total genes tested: %d", stats$total_genes), out)
  writeLines(sprintf("Significant (padj < 0.05): %d", stats$significant), out)
  writeLines(sprintf("Upregulated (log2FC > 1): %d", stats$upregulated), out)
  writeLines(sprintf("Downregulated (log2FC < -1): %d", stats$downregulated), out)
  writeLines(sprintf("Mean absolute fold change: %.2f", stats$mean_abs_fc), out)
  writeLines("", out)
  writeLines("## Top Upregulated Genes", out)
  writeLines(stats$top_up, out)
  writeLines("", out)
  writeLines("## Top Downregulated Genes", out)
  writeLines(stats$top_down, out)
  writeLines("", out)
  writeLines("## AI Interpretation (DE Analysis)", out)
  writeLines(interpretation, out)
  writeLines("", out)
  writeLines("## Variant Annotations", out)
  for (v in annotated) {
    writeLines(sprintf("  %s (%s) - %s - severity:%s", v$gene, v$variant, v$consequence, v$severity), out)
    writeLines(sprintf("    AI: %s", v$ai_interpretation), out)
    writeLines("", out)
  }
  writeLines("## Sequence Composition Analysis", out)
  writeLines(sprintf("Sequences analyzed: %d", seq_summary$num_sequences), out)
  writeLines(sprintf("Mean GC content: %.4f", seq_summary$mean_gc), out)
  writeLines(sprintf("AT-rich regions: %d", seq_summary$at_rich_count), out)
  writeLines(sprintf("GC-rich regions: %d", seq_summary$gc_rich_count), out)
  writeLines("", out)
  writeLines("## AI Interpretation (Sequence Composition)", out)
  writeLines(seq_interp, out)
  writeLines("", out)
  writeLines("## Disclaimer", out)
  writeLines("All AI-generated interpretations in this report were produced by a large", out)
  writeLines("language model and have NOT been verified against primary databases.", out)
  writeLines("Do not use for clinical decision-making without expert review.", out)
  close(out)

  cat("Report written to data/output/ai_report.txt\n")
}

main()
