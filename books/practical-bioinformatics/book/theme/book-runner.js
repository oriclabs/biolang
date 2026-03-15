(function() {
  "use strict";

  // Synchronous fetch bridge for WASM read_csv/read_fasta/etc.
  // Resolves relative paths against the book's data directory on the server.
  // Uses synchronous XMLHttpRequest (works in main thread, same-origin or CORS-enabled).

  // Detect book data base path from current page URL
  // e.g., /books/practical-bioinformatics/html/day-02.html → /books/practical-bioinformatics/html/
  var _bookBasePath = (function() {
    var path = window.location.pathname;
    var idx = path.lastIndexOf("/");
    return idx >= 0 ? path.substring(0, idx + 1) : "/";
  })();

  // In-memory file registry — pre-populated or cached from fetches
  window.__blFiles = {};

  window.__blFetch = {
    sync: function(url) {
      // 1. Check in-memory registry first
      if (window.__blFiles && window.__blFiles[url]) {
        return window.__blFiles[url];
      }

      // 2. Resolve relative paths — try multiple locations
      var fetchUrl = url;
      var isRelative = !/^https?:\/\//.test(url) && !/^\//.test(url);

      // Try locations in order: page-relative, then shared /books/data/
      var tryPaths = isRelative
        ? [_bookBasePath + url, "/books/data/" + url.replace(/^data\//, "")]
        : [fetchUrl];

      for (var pi = 0; pi < tryPaths.length; pi++) {
        try {
          var xhr = new XMLHttpRequest();
          xhr.open("GET", tryPaths[pi], false);
          xhr.send(null);
          if (xhr.status >= 200 && xhr.status < 300) {
            window.__blFiles[url] = xhr.responseText;
            return xhr.responseText;
          }
        } catch (e) {
          // Try next path
        }
      }
      return "ERROR:404 File not found (" + url + ")";
    }
  };

  // WASM module state
  var wasm = null;
  var wasmLoading = false;
  var wasmQueue = [];
  var wasmBasePath = "../../../wasm";

  // Ordered list of all runnable code blocks on this page
  var allBlocks = [];
  // Track which blocks have been executed (by index)
  var executedBlocks = {};

  function loadWasm(callback) {
    if (wasm) { callback(null); return; }
    if (wasmLoading) { wasmQueue.push(callback); return; }
    wasmLoading = true;
    wasmQueue.push(callback);

    var script = document.createElement("script");
    script.type = "module";
    script.textContent = [
      'try {',
      '  var mod = await import("' + wasmBasePath + '/bl_wasm.js");',
      '  await mod.default();',
      '  mod.init();',
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
    var prev = pre.previousElementSibling;
    if (prev && prev.tagName === "BLOCKQUOTE" && prev.textContent.indexOf("Requires CLI") !== -1) return true;
    var code = pre.querySelector("code");
    var text = code ? code.textContent : "";
    // Write operations are always CLI-only
    if (/\b(write_csv|write_fasta|write_fastq|write_vcf|write_bed)\b/.test(text)) return true;
    if (/\b(open|save|write_file|write_lines|mkdir)\s*\(/.test(text)) return true;
    if (/\b(save_plot|save_svg|save_png)\s*\(/.test(text)) return true;
    // Read operations are allowed — they use the fetch bridge to load data from the server
    // BAM/SAM are binary formats that can't be fetched as text
    if (/\b(read_sam|read_bam)\b/.test(text)) return true;
    // Network APIs: CLI-only (require API keys or lack CORS support)
    // Note: ncbi_search, ncbi_gene, ncbi_sequence, ncbi_summary, ncbi_fetch work in WASM
    // because NCBI E-utilities support CORS and are accessed via the fetch hook.
    if (/\b(ensembl_gene|ensembl_vep|uniprot_search|uniprot_entry|kegg_get|kegg_find|pdb_entry|string_network|go_term|go_annotations|cosmic_gene|datasets_gene|reactome_pathways|ucsc_sequence|fetch|http_get|http_post)\b/.test(text)) return true;
    if (/\b(chat|chat_code|llm|ask_llm)\s*\(/.test(text)) return true;
    if (/\b(notebook|pipeline|import\s+")\b/.test(text)) return true;
    return false;
  }

  function createRunButton(codeBlock, blockIndex) {
    var pre = codeBlock.parentElement;
    if (!pre || pre.querySelector(".bl-run-btn")) return;

    var cliRequired = isCLIRequired(pre);

    // Wrapper for button bar
    var bar = document.createElement("div");
    bar.className = "bl-run-bar";
    bar.style.cssText = "display:flex;align-items:center;gap:6px;padding:4px 8px;background:#1e293b;border-radius:6px 6px 0 0;border:1px solid #334155;border-bottom:none;margin-top:8px;flex-wrap:wrap;";

    // Block number badge
    var badge = document.createElement("span");
    badge.className = "bl-block-num";
    badge.textContent = "#" + (blockIndex + 1);
    badge.style.cssText = "font-size:10px;color:#64748b;font-family:system-ui,sans-serif;min-width:20px;";
    bar.appendChild(badge);

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
      btn.title = "Run this code block (state persists from previous blocks)";
      btn.style.cssText = "background:#7c3aed;color:#fff;border:none;padding:4px 14px;border-radius:4px;font-size:12px;font-weight:600;cursor:pointer;font-family:system-ui,sans-serif;transition:background 0.15s;";
      btn.onmouseenter = function() { if (!btn.disabled) btn.style.background = "#6d28d9"; };
      btn.onmouseleave = function() { if (!btn.disabled) btn.style.background = "#7c3aed"; };
    }
    bar.appendChild(btn);

    // "Run All Above" button (only for blocks after the first)
    if (blockIndex > 0 && !cliRequired) {
      var runAllBtn = document.createElement("button");
      runAllBtn.className = "bl-run-all-btn";
      runAllBtn.textContent = "\u25B6\u25B6 Run All Above + This";
      runAllBtn.title = "Run all previous code blocks first, then this one";
      runAllBtn.style.cssText = "background:#1e40af;color:#93c5fd;border:none;padding:4px 10px;border-radius:4px;font-size:11px;font-weight:500;cursor:pointer;font-family:system-ui,sans-serif;transition:background 0.15s;";
      runAllBtn.onmouseenter = function() { runAllBtn.style.background = "#1e3a8a"; };
      runAllBtn.onmouseleave = function() { runAllBtn.style.background = "#1e40af"; };
      bar.appendChild(runAllBtn);
    }

    // Status text
    var status = document.createElement("span");
    status.className = "bl-run-status";
    status.style.cssText = "font-size:11px;color:#94a3b8;font-family:system-ui,sans-serif;margin-left:auto;";
    if (cliRequired) {
      status.textContent = "Requires CLI \u2014 run with: bl run script.bl";
    }
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
    if (pre.nextSibling) {
      pre.parentNode.insertBefore(output, pre.nextSibling);
    } else {
      pre.parentNode.appendChild(output);
    }

    if (!cliRequired) {
      btn.addEventListener("click", function() {
        var code = codeBlock.textContent;
        runCode(code, btn, status, output, blockIndex);
      });

      // "Run All Above + This" handler
      var runAllBtnEl = bar.querySelector(".bl-run-all-btn");
      if (runAllBtnEl) {
        runAllBtnEl.addEventListener("click", function() {
          runAllAbove(blockIndex, runAllBtnEl, status, output);
        });
      }
    }
  }

  function runAllAbove(targetIndex, btn, status, outputEl) {
    btn.disabled = true;
    btn.textContent = "\u23F3 Running all...";
    btn.style.background = "#475569";
    status.textContent = "";

    function doLoad(cb) {
      if (wasm) { cb(null); return; }
      status.textContent = "Downloading BioLang runtime (~4 MB)...";
      loadWasm(function(err) {
        status.textContent = "";
        cb(err);
      });
    }

    doLoad(function(err) {
      if (err) {
        btn.textContent = "\u25B6\u25B6 Run All Above + This";
        btn.disabled = false;
        btn.style.background = "#1e40af";
        outputEl.style.display = "block";
        outputEl.innerHTML = '<span style="color:#f87171;">Error loading WASM: ' + escapeHtml(String(err)) + '</span>';
        return;
      }

      // Reset interpreter to start fresh
      wasm.reset();
      executedBlocks = {};

      // Run all blocks from 0 to targetIndex (inclusive), skipping CLI-only
      var errors = [];
      for (var i = 0; i <= targetIndex; i++) {
        var block = allBlocks[i];
        if (!block || block.cliRequired) continue;

        var code = block.codeBlock.textContent;
        var t0 = performance.now();
        var resultJson;
        try {
          resultJson = wasm.evaluate(code);
        } catch (e) {
          errors.push("Block #" + (i + 1) + ": " + String(e));
          continue;
        }
        var result = JSON.parse(resultJson);
        executedBlocks[i] = true;

        // Update the run badge for intermediate blocks
        if (i < targetIndex && block.badge) {
          block.badge.textContent = "#" + (i + 1) + " \u2713";
          block.badge.style.color = "#4ade80";
        }

        if (!result.ok && i < targetIndex) {
          errors.push("Block #" + (i + 1) + ": " + (result.error || "Unknown error"));
        }

        // For the target block, show its output
        if (i === targetIndex) {
          var elapsed = ((performance.now() - t0) / 1000).toFixed(3);
          showResult(result, outputEl);
          status.textContent = (targetIndex + 1) + " blocks run \u2022 " + elapsed + "s";
          status.style.color = "#94a3b8";
        }
      }

      if (errors.length > 0 && targetIndex > 0) {
        status.textContent += " (" + errors.length + " error" + (errors.length > 1 ? "s" : "") + " above)";
      }

      btn.textContent = "\u25B6\u25B6 Run All Above + This";
      btn.disabled = false;
      btn.style.background = "#1e40af";
    });
  }

  function runCode(code, btn, status, outputEl, blockIndex) {
    btn.disabled = true;
    btn.textContent = "\u23F3 Loading...";
    btn.style.background = "#475569";
    status.textContent = "";
    outputEl.style.display = "none";
    outputEl.innerHTML = "";

    if (wasm) {
      executeCode(code, btn, status, outputEl, blockIndex);
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
      executeCode(code, btn, status, outputEl, blockIndex);
    });
  }

  function executeCode(code, btn, status, outputEl, blockIndex) {
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

    // Mark this block as executed
    if (blockIndex !== undefined) {
      executedBlocks[blockIndex] = true;
      var block = allBlocks[blockIndex];
      if (block && block.badge) {
        block.badge.textContent = "#" + (blockIndex + 1) + " \u2713";
        block.badge.style.color = "#4ade80";
      }
    }

    showResult(result, outputEl);

    // Show state info
    var stateInfo = "";
    if (blockIndex !== undefined && blockIndex > 0) {
      var prevRun = 0;
      for (var k in executedBlocks) { if (parseInt(k) < blockIndex) prevRun++; }
      if (prevRun > 0) {
        stateInfo = " \u2022 state from " + prevRun + " prev block" + (prevRun > 1 ? "s" : "");
      }
    }
    status.textContent = elapsed + "s" + stateInfo;
    status.style.color = "#94a3b8";
    resetButton(btn);
  }

  function showResult(result, outputEl) {
    outputEl.style.display = "block";
    var lines = [];

    if (result.output && result.output.trim()) {
      var stdoutText = result.output.trimEnd();
      if (stdoutText.indexOf("<svg") !== -1) {
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
      if (result.value && result.value !== "null" && result.value !== "nil" && result.value !== "()" && result.value !== "None" && result.value !== "Nil") {
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
    var svgs = outputEl.querySelectorAll(".bl-svg-output svg");
    for (var si = 0; si < svgs.length; si++) {
      svgs[si].style.maxWidth = "100%";
      svgs[si].style.height = "auto";
    }
  }

  function resetButton(btn) {
    btn.textContent = "\u25B6 Run";
    btn.disabled = false;
    btn.style.background = "#7c3aed";
  }

  // Find all BioLang code blocks and add Run buttons
  function init() {
    var blocks = document.querySelectorAll('code.language-bio, code.language-biolang, code.language-biorun');
    var idx = 0;
    blocks.forEach(function(block) {
      var text = block.textContent;
      if (text.indexOf("bl>") === 0) return;
      if (text.trim().split("\n").length < 2 && !text.includes("let ") && !text.includes("print") && !text.includes("|>")) return;

      var pre = block.parentElement;
      var cliReq = isCLIRequired(pre);

      allBlocks.push({
        codeBlock: block,
        cliRequired: cliReq,
        badge: null
      });
      var currentIdx = allBlocks.length - 1;
      createRunButton(block, currentIdx);

      // Store badge reference
      var bar = pre.previousElementSibling;
      if (bar && bar.className === "bl-run-bar") {
        var badgeEl = bar.querySelector(".bl-block-num");
        allBlocks[currentIdx].badge = badgeEl;
      }
    });

    // Add a "Reset All" button at the top if there are runnable blocks
    if (allBlocks.length > 1) {
      var content = document.querySelector(".content, main, article, #content");
      if (content) {
        var resetBar = document.createElement("div");
        resetBar.style.cssText = "display:flex;align-items:center;gap:8px;padding:6px 12px;background:#1e293b;border:1px solid #334155;border-radius:6px;margin-bottom:16px;position:sticky;top:0;z-index:10;";
        var resetBtn = document.createElement("button");
        resetBtn.textContent = "\u21BB Reset Interpreter";
        resetBtn.title = "Clear all variables and start fresh";
        resetBtn.style.cssText = "background:#dc2626;color:#fff;border:none;padding:4px 12px;border-radius:4px;font-size:12px;font-weight:600;cursor:pointer;font-family:system-ui,sans-serif;";
        resetBtn.addEventListener("click", function() {
          if (wasm) {
            wasm.reset();
            executedBlocks = {};
            allBlocks.forEach(function(b) {
              if (b.badge) {
                b.badge.textContent = "#" + (allBlocks.indexOf(b) + 1);
                b.badge.style.color = "#64748b";
              }
            });
            // Clear all outputs
            document.querySelectorAll(".bl-output").forEach(function(el) {
              el.style.display = "none";
              el.innerHTML = "";
            });
          }
        });
        var stateLabel = document.createElement("span");
        stateLabel.style.cssText = "font-size:11px;color:#94a3b8;font-family:system-ui,sans-serif;";
        stateLabel.textContent = allBlocks.length + " code blocks \u2022 state persists between runs \u2022 use \u25B6\u25B6 to auto-run dependencies";
        resetBar.appendChild(resetBtn);
        resetBar.appendChild(stateLabel);
        var firstChild = content.firstChild;
        // Insert after the first h1/h2 if it exists
        var heading = content.querySelector("h1, h2");
        if (heading && heading.nextSibling) {
          heading.parentNode.insertBefore(resetBar, heading.nextSibling);
        } else if (firstChild) {
          content.insertBefore(resetBar, firstChild);
        }
      }
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
