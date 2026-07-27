/// Notebook format converters: Jupyter (.ipynb) and R Markdown (.Rmd) → BioLang (.bln).
///
/// BioLang notebook (.bln) format: markdown prose interleaved with ```biolang code blocks.
use serde::Deserialize;

// ── .ipynb (Jupyter) ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct JupyterNotebook {
    cells: Vec<JupyterCell>,
    #[serde(default)]
    metadata: serde_json::Value,
}

#[derive(Deserialize)]
struct JupyterCell {
    cell_type: String,
    source: SourceField,
}

/// Jupyter source fields can be a string or array of strings.
#[derive(Deserialize)]
#[serde(untagged)]
enum SourceField {
    Lines(Vec<String>),
    Single(String),
}

impl SourceField {
    fn join(&self) -> String {
        match self {
            SourceField::Lines(v) => v.join(""),
            SourceField::Single(s) => s.clone(),
        }
    }
}

/// Convert a Jupyter notebook (.ipynb) to a BioLang notebook (.bln).
///
/// Code cells are converted from Python to BioLang; markdown cells are kept as prose.
pub fn ipynb_to_bln(source: &str, filename: &str) -> String {
    let nb: JupyterNotebook = match serde_json::from_str(source) {
        Ok(n) => n,
        Err(e) => {
            return format!(
                "# ERROR: could not parse Jupyter notebook: {e}\n# Source: {filename}\n"
            )
        }
    };

    let kernel = nb
        .metadata
        .get("kernelspec")
        .and_then(|k| k.get("language"))
        .and_then(|l| l.as_str())
        .unwrap_or("python");

    let lang = if kernel.eq_ignore_ascii_case("r") { "r" } else { "python" };

    let mut out = format!(
        "# Converted from: {filename}\n# Original kernel: {kernel}\n\n"
    );

    for (i, cell) in nb.cells.iter().enumerate() {
        let raw = cell.source.join();
        if raw.trim().is_empty() {
            continue;
        }

        match cell.cell_type.as_str() {
            "markdown" => {
                out.push_str(&raw);
                if !raw.ends_with('\n') {
                    out.push('\n');
                }
                out.push('\n');
            }
            "code" | "raw" => {
                let converted =
                    super::convert_cell(&raw, lang, &format!("{filename}[cell {i}]"));
                let trimmed = converted.trim_end();
                if !trimmed.is_empty() {
                    out.push_str("```biolang\n");
                    out.push_str(trimmed);
                    out.push_str("\n```\n\n");
                }
            }
            _ => {}
        }
    }

    out
}

// ── .Rmd (R Markdown) ────────────────────────────────────────────────────────

/// State machine for parsing R Markdown documents.
enum RmdState {
    Prose,
    YamlFrontmatter,
    RChunk { options: String },
    OtherChunk { lang: String },
}

/// Convert an R Markdown (.Rmd) file to a BioLang notebook (.bln).
///
/// R code chunks are converted to BioLang; markdown prose and YAML frontmatter
/// are kept as-is. Non-R code chunks (python, bash, etc.) are preserved verbatim.
pub fn rmd_to_bln(source: &str, filename: &str) -> String {
    let mut out = format!("# Converted from: {filename}\n\n");
    let mut state = RmdState::Prose;
    let mut prose_buf = String::new();
    let mut code_buf = String::new();
    let mut frontmatter_done = false;
    let mut line_num = 0usize;

    for line in source.lines() {
        line_num += 1;

        match &state {
            RmdState::Prose => {
                // YAML frontmatter: first line is `---`
                if line_num == 1 && line.trim() == "---" && !frontmatter_done {
                    state = RmdState::YamlFrontmatter;
                    prose_buf.push_str(line);
                    prose_buf.push('\n');
                    continue;
                }

                if let Some(rest) = line.trim_start().strip_prefix("```") {
                    let opts = rest.trim().to_string();
                    if opts.starts_with('{') {
                        // R Markdown chunk: ```{r [label, options...]}
                        let inner = opts.trim_start_matches('{').trim_end_matches('}');
                        let (chunk_lang, chunk_opts) = parse_chunk_header(inner);

                        // Flush accumulated prose
                        flush_prose(&mut out, &mut prose_buf);

                        if chunk_lang.eq_ignore_ascii_case("r") {
                            state = RmdState::RChunk {
                                options: chunk_opts,
                            };
                        } else {
                            state = RmdState::OtherChunk { lang: chunk_lang };
                        }
                        code_buf.clear();
                    } else if opts.is_empty() {
                        // Bare closing fence — shouldn't appear here but handle gracefully
                        prose_buf.push_str(line);
                        prose_buf.push('\n');
                    } else {
                        // Non-brace chunk: ```python, ```bash, etc.
                        flush_prose(&mut out, &mut prose_buf);
                        state = RmdState::OtherChunk { lang: opts };
                        code_buf.clear();
                    }
                } else {
                    prose_buf.push_str(line);
                    prose_buf.push('\n');
                }
            }

            RmdState::YamlFrontmatter => {
                prose_buf.push_str(line);
                prose_buf.push('\n');
                if line.trim() == "---" || line.trim() == "..." {
                    frontmatter_done = true;
                    state = RmdState::Prose;
                }
            }

            RmdState::RChunk { options } => {
                if line.trim() == "```" {
                    // End of R chunk — convert and emit
                    let opts = options.clone();
                    let converted =
                        super::convert_cell(&code_buf, "r", &format!("{filename}[chunk]"));
                    let trimmed = converted.trim_end();

                    if should_skip_chunk(&opts) {
                        // @skip directive
                        out.push_str("```biolang\n# @skip\n");
                    } else {
                        out.push_str("```biolang\n");
                    }

                    if !trimmed.is_empty() {
                        out.push_str(trimmed);
                        out.push('\n');
                    }
                    out.push_str("```\n\n");
                    code_buf.clear();
                    state = RmdState::Prose;
                } else {
                    code_buf.push_str(line);
                    code_buf.push('\n');
                }
            }

            RmdState::OtherChunk { lang } => {
                if line.trim() == "```" {
                    // Preserve non-R chunks verbatim
                    let lang_label = lang.clone();
                    out.push_str(&format!("```{lang_label}\n"));
                    out.push_str(&code_buf);
                    out.push_str("```\n\n");
                    code_buf.clear();
                    state = RmdState::Prose;
                } else {
                    code_buf.push_str(line);
                    code_buf.push('\n');
                }
            }
        }
    }

    // Flush any trailing prose
    flush_prose(&mut out, &mut prose_buf);

    // Handle unclosed chunk (malformed input)
    if !code_buf.is_empty() {
        out.push_str("```biolang\n");
        out.push_str(code_buf.trim_end());
        out.push_str("\n```\n");
    }

    out
}

fn flush_prose(out: &mut String, prose_buf: &mut String) {
    let trimmed = prose_buf.trim_end();
    if !trimmed.is_empty() {
        out.push_str(trimmed);
        out.push_str("\n\n");
    }
    prose_buf.clear();
}

/// Parse `r label, echo=FALSE, eval=TRUE` → ("r", "label, echo=FALSE, eval=TRUE")
/// The language is the first token delimited by whitespace or comma.
fn parse_chunk_header(inner: &str) -> (String, String) {
    let lang_end = inner
        .find(|c: char| c == ',' || c.is_whitespace())
        .unwrap_or(inner.len());
    let lang = inner[..lang_end].trim().to_string();
    let rest = inner[lang_end..].trim_start_matches(',').trim().to_string();
    (lang, rest)
}

/// Check for eval=FALSE which means the chunk should be marked @skip.
fn should_skip_chunk(opts: &str) -> bool {
    opts.split(',')
        .any(|o| o.trim().eq_ignore_ascii_case("eval=false"))
}
