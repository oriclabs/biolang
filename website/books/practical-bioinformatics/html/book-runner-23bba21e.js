(function() {
  "use strict";

  // WASM module state
  var wasm = null;
  var wasmLoading = false;
  var wasmQueue = [];
  var wasmBasePath = "../../../wasm";

  function loadWasm(callback) {
    if (wasm) { callback(null); return; }
    if (wasmLoading) { wasmQueue.push(callback); return; }
    wasmLoading = true;
    wasmQueue.push(callback);

    var script = document.createElement("script");
    script.type = "module";
    script.textContent = [
      'try {',
      '  var mod = await import("' + wasmBasePath + '/br_wasm.js");',
      '  await mod.default();',
      '  window.__blWasm = { evaluate: mod.evaluate, reset: mod.reset };',
      '  window.dispatchEvent(new Event("bl-wasm-ready"));',
      '} catch(e) {',
      '  window.__blWasmError = e;',
      '  window.dispatchEvent(new Event("bl-wasm-error"));',
      '}'
    ].join("\n");
    document.head.appendChild(script);

    window.addEventListener("bl-wasm-ready", function() {
      wasm = window.__blWasm;
      wasmLoading = false;
      var q = wasmQueue.slice();
      wasmQueue = [];
      q.forEach(function(cb) { cb(null); });
    }, { once: true });

    window.addEventListener("bl-wasm-error", function() {
      wasmLoading = false;
      var err = window.__blWasmError || new Error("WASM load failed");
      var q = wasmQueue.slice();
      wasmQueue = [];
      q.forEach(function(cb) { cb(err); });
    }, { once: true });
  }

  function escapeHtml(s) {
    var d = document.createElement("div");
    d.textContent = s;
    return d.innerHTML;
  }

  function isCLIRequired(pre) {
    // Check if the preceding element is a blockquote containing "Requires CLI"
    var prev = pre.previousElementSibling;
    if (prev && prev.tagName === "BLOCKQUOTE" && prev.textContent.indexOf("Requires CLI") !== -1) return true;
    // Check code content for CLI-only patterns
    var code = pre.querySelector("code");
    var text = code ? code.textContent : "";
    // File I/O
    if (/\b(read_fasta|read_fastq|read_vcf|read_bed|read_gff|read_sam|read_bam|read_csv|write_csv|write_fasta|write_fastq|write_vcf|write_bed)\b/.test(text)) return true;
    if (/\b(csv|tsv|vcf|fastq|fasta|bam|sam|bed|gff)\s*\(/.test(text)) return true;
    if (/\b(open|save|write_file|read_file|read_lines)\s*\(/.test(text)) return true;
    // Network APIs
    if (/\b(ncbi_gene|ncbi_search|ncbi_sequence|ensembl_gene|ensembl_vep|uniprot_search|uniprot_entry|kegg_get|kegg_find|pdb_entry|string_network|go_term|go_annotations|cosmic_gene|datasets_gene|reactome_pathways|ucsc_sequence|fetch|http_get|http_post)\b/.test(text)) return true;
    // LLM chat
    if (/\b(chat|chat_code|llm|ask_llm)\s*\(/.test(text)) return true;
    // Notebooks and pipelines
    if (/\b(notebook|pipeline|import\s+")\b/.test(text)) return true;
    // Saving plots to files (save_svg, save_png, save_plot write to disk)
    if (/\b(save_plot|save_svg|save_png)\s*\(/.test(text)) return true;
    return false;
  }

  function createRunButton(codeBlock) {
    var pre = codeBlock.parentElement;
    if (!pre || pre.querySelector(".bl-run-btn")) return;

    var cliRequired = isCLIRequired(pre);

    // Wrapper for button bar
    var bar = document.createElement("div");
    bar.className = "bl-run-bar";
    bar.style.cssText = "display:flex;align-items:center;gap:8px;padding:4px 8px;background:#1e293b;border-radius:6px 6px 0 0;border:1px solid #334155;border-bottom:none;margin-top:8px;";

    // Run button
    var btn = document.createElement("button");
    btn.className = "bl-run-btn";
    if (cliRequired) {
      btn.textContent = "\u25B6 CLI Only";
      btn.title = "This example requires file I/O or network APIs. Run with: bl run";
      btn.style.cssText = "background:#475569;color:#94a3b8;border:none;padding:4px 14px;border-radius:4px;font-size:12px;font-weight:600;cursor:not-allowed;font-family:system-ui,sans-serif;opacity:0.7;";
      btn.disabled = true;
    } else {
      btn.textContent = "\u25B6 Run";
      btn.title = "Run this code (loads BioLang WASM on first click)";
      btn.style.cssText = "background:#7c3aed;color:#fff;border:none;padding:4px 14px;border-radius:4px;font-size:12px;font-weight:600;cursor:pointer;font-family:system-ui,sans-serif;transition:background 0.15s;";
      btn.onmouseenter = function() { btn.style.background = "#6d28d9"; };
      btn.onmouseleave = function() { btn.style.background = "#7c3aed"; };
    }

    // Status text
    var status = document.createElement("span");
    status.className = "bl-run-status";
    status.style.cssText = "font-size:11px;color:#94a3b8;font-family:system-ui,sans-serif;";
    if (cliRequired) {
      status.textContent = "Requires CLI \u2014 run with: bl run script.bl";
    }

    bar.appendChild(btn);
    bar.appendChild(status);

    // Output area (hidden initially)
    var output = document.createElement("div");
    output.className = "bl-output";
    output.style.cssText = "display:none;background:#0f172a;border:1px solid #334155;border-top:none;border-radius:0 0 6px 6px;padding:8px 12px;font-family:'JetBrains Mono',ui-monospace,monospace;font-size:13px;line-height:1.5;white-space:pre-wrap;max-height:300px;overflow-y:auto;margin-bottom:8px;";

    // Adjust pre styling to connect with bar
    pre.style.borderRadius = "0";
    pre.style.marginTop = "0";
    pre.style.borderTop = "none";

    pre.parentNode.insertBefore(bar, pre);
    // Insert output after pre
    if (pre.nextSibling) {
      pre.parentNode.insertBefore(output, pre.nextSibling);
    } else {
      pre.parentNode.appendChild(output);
    }

    if (!cliRequired) {
      btn.addEventListener("click", function() {
        var code = codeBlock.textContent;
        runCode(code, btn, status, output);
      });
    }
  }

  function runCode(code, btn, status, outputEl) {
    btn.disabled = true;
    btn.textContent = "\u23F3 Loading...";
    btn.style.background = "#475569";
    status.textContent = "";
    outputEl.style.display = "none";
    outputEl.innerHTML = "";

    if (wasm) {
      executeCode(code, btn, status, outputEl);
      return;
    }

    status.textContent = "Downloading BioLang runtime (~4 MB)...";
    loadWasm(function(err) {
      if (err) {
        btn.textContent = "\u25B6 Run";
        btn.disabled = false;
        btn.style.background = "#7c3aed";
        status.textContent = "";
        outputEl.style.display = "block";
        outputEl.innerHTML = '<span style="color:#f87171;">Error loading WASM: ' + escapeHtml(String(err)) + '</span>';
        return;
      }
      status.textContent = "";
      executeCode(code, btn, status, outputEl);
    });
  }

  function executeCode(code, btn, status, outputEl) {
    btn.textContent = "\u25B6 Running...";
    outputEl.style.display = "block";
    outputEl.innerHTML = "";

    var t0 = performance.now();
    var resultJson;
    try {
      resultJson = wasm.evaluate(code);
    } catch (e) {
      outputEl.innerHTML = '<span style="color:#f87171;">Runtime error: ' + escapeHtml(String(e)) + '</span>';
      resetButton(btn);
      return;
    }
    var elapsed = ((performance.now() - t0) / 1000).toFixed(3);

    var result;
    try {
      result = JSON.parse(resultJson);
    } catch (e) {
      outputEl.innerHTML = '<span style="color:#94a3b8;">' + escapeHtml(resultJson) + '</span>';
      resetButton(btn);
      status.textContent = elapsed + "s";
      return;
    }

    var lines = [];

    // Show stdout output first
    if (result.output && result.output.trim()) {
      // Check if stdout contains SVG (e.g. from println(volcano(...)))
      var stdoutText = result.output.trimEnd();
      if (stdoutText.indexOf("<svg") !== -1) {
        // Split by SVG boundaries, render SVG inline and text as escaped
        var parts = stdoutText.split(/(<svg[\s\S]*?<\/svg>)/);
        for (var pi = 0; pi < parts.length; pi++) {
          if (parts[pi].trimStart().indexOf("<svg") === 0) {
            lines.push('<div class="bl-svg-output" style="background:#fff;border-radius:4px;padding:8px;margin:4px 0;overflow-x:auto;max-width:100%">' + parts[pi] + '</div>');
          } else if (parts[pi].trim()) {
            lines.push('<span style="color:#e2e8f0;">' + escapeHtml(parts[pi]) + '</span>');
          }
        }
      } else {
        lines.push('<span style="color:#e2e8f0;">' + escapeHtml(stdoutText) + '</span>');
      }
    }

    if (result.ok) {
      // Show return value if it's not empty/null and wasn't already printed
      if (result.value && result.value !== "null" && result.value !== "nil" && result.value !== "()" && result.value !== "None" && result.value !== "Nil") {
        // Detect SVG output — render inline instead of escaping
        if (result.value.trimStart().indexOf("<svg") === 0) {
          lines.push('<div class="bl-svg-output" style="background:#fff;border-radius:4px;padding:8px;margin:4px 0;overflow-x:auto;max-width:100%">' + result.value + '</div>');
        } else {
          var typeLabel = result.type ? '<span style="color:#94a3b8;font-size:11px;"> : ' + escapeHtml(result.type) + '</span>' : '';
          lines.push('<span style="color:#4ade80;">\u2192 ' + escapeHtml(result.value) + typeLabel + '</span>');
        }
      }
    } else {
      lines.push('<span style="color:#f87171;">\u2716 ' + escapeHtml(result.error || "Unknown error") + '</span>');
    }

    if (lines.length === 0) {
      lines.push('<span style="color:#64748b;">(no output)</span>');
    }

    outputEl.innerHTML = lines.join("\n");
    // Scale SVG to fit output container
    var svgs = outputEl.querySelectorAll(".bl-svg-output svg");
    for (var si = 0; si < svgs.length; si++) {
      svgs[si].style.maxWidth = "100%";
      svgs[si].style.height = "auto";
    }
    status.textContent = elapsed + "s";
    status.style.color = "#94a3b8";
    resetButton(btn);
  }

  function resetButton(btn) {
    btn.textContent = "\u25B6 Run";
    btn.disabled = false;
    btn.style.background = "#7c3aed";
  }

  // Find all BioLang code blocks and add Run buttons
  function init() {
    var blocks = document.querySelectorAll('code.language-bio, code.language-biolang, code.language-biorun');
    blocks.forEach(function(block) {
      // Skip blocks inside output examples or that are just showing REPL interaction
      var text = block.textContent;
      if (text.indexOf("bl>") === 0) return;
      // Skip very short blocks (single-line comments etc)
      if (text.trim().split("\n").length < 2 && !text.includes("let ") && !text.includes("print") && !text.includes("|>")) return;

      createRunButton(block);
    });
  }

  // Run on page load
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
