// BioLang Playground — WASM-powered inline code execution for docs
// Auto-adds Run buttons to all language-biolang code blocks
// Lazy-loads WASM on first Run click (~4 MB), then cached for session

(function () {
  'use strict';

  // ── Fetch bridge for WASM file I/O ──
  // Allows read_csv, read_fasta etc. to fetch data files from the server
  var _pageBasePath = (function() {
    var path = window.location.pathname;
    var idx = path.lastIndexOf("/");
    return idx >= 0 ? path.substring(0, idx + 1) : "/";
  })();

  window.__blFiles = window.__blFiles || {};

  if (!window.__blFetch) {
    window.__blFetch = {
      sync: function(url) {
        if (window.__blFiles && window.__blFiles[url]) {
          return window.__blFiles[url];
        }
        var fetchUrl = url;
        var isRelative = !/^https?:\/\//.test(url) && !/^\//.test(url);
        var tryPaths = isRelative
          ? [_pageBasePath + url, "/books/data/" + url.replace(/^data\//, "")]
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
  }

  // ── WASM module state ──
  var wasm = null;
  var wasmLoading = false;
  var wasmQueue = [];

  function getWasmBasePath() {
    // Use absolute path to /wasm/ from site root
    return '/wasm';
  }

  function loadWasm(callback) {
    if (wasm) { callback(null); return; }
    if (wasmLoading) { wasmQueue.push(callback); return; }
    wasmLoading = true;
    wasmQueue.push(callback);

    var basePath = getWasmBasePath();
    var script = document.createElement('script');
    script.type = 'module';
    script.textContent = [
      'try {',
      '  var mod = await import("' + basePath + '/bl_wasm.js");',
      '  await mod.default();',
      '  mod.init();',
      '  window.__blWasm = { evaluate: mod.evaluate, reset: mod.reset };',
      '  window.dispatchEvent(new Event("bl-wasm-ready"));',
      '} catch(e) {',
      '  window.__blWasmError = e;',
      '  window.dispatchEvent(new Event("bl-wasm-error"));',
      '}'
    ].join('\n');
    document.head.appendChild(script);

    window.addEventListener('bl-wasm-ready', function() {
      wasm = window.__blWasm;
      wasmLoading = false;
      var q = wasmQueue.slice();
      wasmQueue = [];
      q.forEach(function(cb) { cb(null); });
    }, { once: true });

    window.addEventListener('bl-wasm-error', function() {
      wasmLoading = false;
      var err = window.__blWasmError || new Error('WASM load failed');
      var q = wasmQueue.slice();
      wasmQueue = [];
      q.forEach(function(cb) { cb(err); });
    }, { once: true });
  }

  function escapeHtml(s) {
    var d = document.createElement('div');
    d.textContent = s;
    return d.innerHTML;
  }

  // ── CLI detection ──

  function isCLIRequired(pre) {
    // Check if the preceding element is a blockquote/note with "Requires CLI"
    var prev = pre.previousElementSibling;
    if (prev && prev.textContent && prev.textContent.indexOf('Requires CLI') !== -1) return true;
    // Check code content for CLI-only patterns
    var code = pre.querySelector('code');
    var text = code ? code.textContent : '';
    // Write operations are always CLI-only
    if (/\b(write_csv|write_fasta|write_fastq|write_vcf|write_bed)\b/.test(text)) return true;
    if (/\b(open|save|write_file|write_lines|mkdir)\s*\(/.test(text)) return true;
    if (/\b(save_plot|save_svg|save_png)\s*\(/.test(text)) return true;
    // Read operations are allowed — fetch bridge loads data from server
    // BAM/SAM are binary formats that can't be fetched as text
    if (/\b(read_sam|read_bam)\b/.test(text)) return true;
    // Network APIs: CLI-only (require API keys or lack CORS support)
    // Note: ncbi_search, ncbi_gene, ncbi_sequence, ncbi_summary, ncbi_fetch work in WASM
    // because NCBI E-utilities support CORS and are accessed via the fetch hook.
    if (/\b(ensembl_gene|ensembl_vep|uniprot_search|uniprot_entry|kegg_get|kegg_find|pdb_entry|string_network|go_term|go_annotations|cosmic_gene|datasets_gene|reactome_pathways|ucsc_sequence|fetch|http_get|http_post)\b/.test(text)) return true;
    // LLM chat
    if (/\b(chat|chat_code|llm|ask_llm)\s*\(/.test(text)) return true;
    // Notebooks and pipelines
    if (/\b(notebook|import\s+")\b/.test(text)) return true;
    // pipeline keyword — only match if it starts a line (not in comments)
    if (/^\s*pipeline\s+\w/m.test(text)) return true;
    return false;
  }

  // ── Detect runnable code blocks ──

  function shouldSkipBlock(code) {
    var text = code.textContent || '';
    // Skip REPL interaction (bl> prompt)
    if (text.trimStart().indexOf('bl>') === 0) return true;
    // Skip very short blocks (single-line comments or trivial)
    var lines = text.trim().split('\n');
    if (lines.length < 2 && !text.includes('let ') && !text.includes('print') && !text.includes('|>') && !text.includes('println')) return true;
    // Skip blocks that are just output (no code keywords)
    var hasCode = /\b(let|fn|if|for|while|print|println|dna"|rna"|protein"|import|\|>)\b/.test(text);
    if (!hasCode) return true;
    return false;
  }

  function autoMarkRunnable() {
    var blocks = document.querySelectorAll('code.language-biolang, code.language-bio');
    blocks.forEach(function(code) {
      if (shouldSkipBlock(code)) return;
      var pre = code.parentElement;
      if (pre && pre.tagName === 'PRE') {
        pre.setAttribute('data-runnable', '');
      }
    });
  }

  // ── UI: Add Run buttons ──

  function addPlaygroundButtons() {
    document.querySelectorAll('pre[data-runnable]').forEach(function (pre) {
      if (pre.querySelector('.bl-run-btn')) return;

      // Ensure wrapper has relative positioning for button placement
      var wrapper = pre.parentNode;
      if (!wrapper.style.position || wrapper.style.position === 'static') {
        var div = document.createElement('div');
        div.style.cssText = 'position:relative;overflow:hidden';
        pre.parentNode.insertBefore(div, pre);
        div.appendChild(pre);
        wrapper = div;
      }

      var cliRequired = isCLIRequired(pre);

      // Run button — positioned over the code block
      var btn = document.createElement('button');
      btn.className = 'bl-run-btn';
      if (cliRequired) {
        btn.style.cssText = 'position:absolute;top:8px;right:52px;padding:4px 12px;font-size:12px;border-radius:4px;background:rgba(71,85,105,0.85);color:#94a3b8;border:none;cursor:not-allowed;opacity:0.6;transition:opacity 0.2s;z-index:10;font-family:system-ui,sans-serif;font-weight:600;';
        btn.innerHTML = '&#9654; CLI Only';
        btn.title = 'This example requires file I/O or network APIs — run with: bl run';
        btn.disabled = true;
      } else {
        btn.style.cssText = 'position:absolute;top:8px;right:52px;padding:4px 12px;font-size:12px;border-radius:4px;background:rgba(124,58,237,0.85);color:#fff;border:none;cursor:pointer;opacity:0.6;transition:opacity 0.2s;z-index:10;font-family:system-ui,sans-serif;font-weight:600;';
        btn.innerHTML = '&#9654; Run';
        btn.title = 'Run this code (loads BioLang WASM on first click)';
      }
      wrapper.addEventListener('mouseenter', function(){ btn.style.opacity = '1'; });
      wrapper.addEventListener('mouseleave', function(){ if (!btn.disabled) btn.style.opacity = '0.6'; });
      wrapper.appendChild(btn);

      // Output panel (hidden initially)
      var outputEl = document.createElement('div');
      outputEl.className = 'bl-output';
      outputEl.style.cssText = 'display:none;border:1px solid #334155;border-top:none;border-radius:0 0 8px 8px;overflow:hidden;';
      outputEl.innerHTML =
        '<div style="padding:6px 12px;background:rgba(30,41,59,0.6);border-bottom:1px solid #334155;display:flex;align-items:center;justify-content:space-between">' +
          '<span style="font-size:11px;color:#64748b;font-family:system-ui,sans-serif">Output</span>' +
          '<div style="display:flex;align-items:center;gap:8px">' +
            '<span class="bl-timing" style="font-size:11px;color:#94a3b8;font-family:system-ui,sans-serif"></span>' +
            '<button class="bl-close" style="font-size:14px;color:#64748b;background:none;border:none;cursor:pointer;line-height:1">&times;</button>' +
          '</div>' +
        '</div>' +
        '<pre style="border:0;border-radius:0;margin:0;background:rgba(15,23,42,0.85);padding:8px 12px;max-height:300px;overflow-y:auto"><code class="bl-result" style="font-size:13px;line-height:1.5;white-space:pre-wrap"></code></pre>';
      wrapper.appendChild(outputEl);

      if (!cliRequired) {
        btn.addEventListener('click', function () {
          var code = pre.querySelector('code');
          var src = code ? code.textContent : pre.textContent;
          runCode(src, btn, outputEl);
        });
      }

      outputEl.querySelector('.bl-close').addEventListener('click', function () {
        outputEl.style.display = 'none';
        pre.style.borderRadius = '';
      });
    });
  }

  // ── Execute code via WASM ──

  function runCode(code, btn, outputEl) {
    btn.disabled = true;
    btn.style.opacity = '1';
    btn.innerHTML = '&#9203; Loading...';
    btn.style.background = '#475569';

    var timingEl = outputEl.querySelector('.bl-timing');
    var resultEl = outputEl.querySelector('.bl-result');
    timingEl.textContent = '';
    resultEl.innerHTML = '';
    outputEl.style.display = 'none';

    if (wasm) {
      executeCode(code, btn, timingEl, resultEl, outputEl);
      return;
    }

    // Lazy-load WASM
    timingEl.textContent = '';
    outputEl.style.display = 'block';
    resultEl.innerHTML = '<span style="color:#94a3b8">Downloading BioLang runtime (~4 MB)...</span>';
    var pre = outputEl.previousElementSibling || outputEl.parentNode.querySelector('pre');
    if (pre) pre.style.borderRadius = '8px 8px 0 0';

    loadWasm(function(err) {
      if (err) {
        resultEl.innerHTML = '<span style="color:#f87171">Failed to load WASM: ' + escapeHtml(String(err)) + '</span>';
        resetButton(btn);
        return;
      }
      executeCode(code, btn, timingEl, resultEl, outputEl);
    });
  }

  function executeCode(code, btn, timingEl, resultEl, outputEl) {
    var hasFileOps = /\bread_(csv|fasta|fastq|vcf|bed|gff|parquet)\s*\(/.test(code);
    btn.innerHTML = hasFileOps ? '&#9654; Loading data...' : '&#9654; Running...';
    outputEl.style.display = 'block';
    var pre = outputEl.previousElementSibling || outputEl.parentNode.querySelector('pre');
    if (pre) pre.style.borderRadius = '8px 8px 0 0';
    resultEl.innerHTML = hasFileOps ? '<span style="color:#94a3b8">Fetching data files...</span>' : '';

    // Use setTimeout to let the UI update before blocking on sync XHR
    setTimeout(function() {
    var t0 = performance.now();
    // Reset interpreter state so variables from previous runs don't shadow builtins
    if (wasm.reset) { try { wasm.reset(); } catch(_) {} }
    var resultJson;
    try {
      resultJson = wasm.evaluate(code);
    } catch (e) {
      resultEl.innerHTML = '<span style="color:#f87171">Runtime error: ' + escapeHtml(String(e)) + '</span>';
      timingEl.textContent = formatElapsed(t0);
      resetButton(btn);
      return;
    }

    var result;
    try {
      result = JSON.parse(resultJson);
    } catch (e) {
      resultEl.innerHTML = '<span style="color:#94a3b8">' + escapeHtml(resultJson) + '</span>';
      timingEl.textContent = formatElapsed(t0);
      resetButton(btn);
      return;
    }

    var lines = [];

    // Show stdout
    if (result.output && result.output.trim()) {
      var stdoutText = result.output.trimEnd();
      if (stdoutText.indexOf('<svg') !== -1) {
        var parts = stdoutText.split(/(<svg[\s\S]*?<\/svg>)/);
        for (var pi = 0; pi < parts.length; pi++) {
          if (parts[pi].trimStart().indexOf('<svg') === 0) {
            lines.push('<div class="bl-svg-output" style="background:#fff;border-radius:4px;padding:8px;margin:4px 0;overflow-x:auto;max-width:100%">' + parts[pi] + '</div>');
          } else if (parts[pi].trim()) {
            lines.push('<span style="color:#e2e8f0">' + escapeHtml(parts[pi]) + '</span>');
          }
        }
      } else {
        lines.push('<span style="color:#e2e8f0">' + escapeHtml(stdoutText) + '</span>');
      }
    }

    if (result.ok) {
      // Show return value (skip nil/null/empty)
      if (result.value && result.value !== 'null' && result.value !== 'nil' && result.value !== 'Nil' && result.value !== '()' && result.value !== 'None') {
        // Detect SVG output — render inline instead of escaping
        if (result.value.trimStart().indexOf('<svg') === 0) {
          lines.push('<div class="bl-svg-output" style="background:#fff;border-radius:4px;padding:8px;margin:4px 0;overflow-x:auto;max-width:100%">' + result.value + '</div>');
        } else {
          var typeLabel = result.type ? '<span style="color:#94a3b8;font-size:11px"> : ' + escapeHtml(result.type) + '</span>' : '';
          lines.push('<span style="color:#4ade80">\u2192 ' + escapeHtml(result.value) + typeLabel + '</span>');
        }
      }
    } else {
      lines.push('<span style="color:#f87171">\u2716 ' + escapeHtml(result.error || 'Unknown error') + '</span>');
    }

    if (lines.length === 0) {
      lines.push('<span style="color:#64748b">(no output)</span>');
    }

    resultEl.innerHTML = lines.join('\n');
    // Scale SVG to fit output container
    var svgContainer = outputEl.querySelector('.bl-svg-output');
    if (svgContainer) {
      var svgEl = svgContainer.querySelector('svg');
      if (svgEl) { svgEl.style.maxWidth = '100%'; svgEl.style.height = 'auto'; }
    }
    timingEl.textContent = formatElapsed(t0);
    saveToHistory(code);
    resetButton(btn);
    }, 50); // end setTimeout — allows UI to show "Loading data..." before sync XHR blocks
  }

  function saveToHistory(code) {
    try {
      var history = JSON.parse(localStorage.getItem("bl-playground-history") || "[]");
      history.unshift({ code: code.trim(), time: Date.now() });
      if (history.length > 50) history = history.slice(0, 50);
      localStorage.setItem("bl-playground-history", JSON.stringify(history));
    } catch (_) { /* localStorage unavailable */ }
  }

  function formatElapsed(t0) {
    var ms = performance.now() - t0;
    return ms < 1 ? '<1ms' : ms < 1000 ? Math.round(ms) + 'ms' : (ms / 1000).toFixed(3) + 's';
  }

  function resetButton(btn) {
    btn.innerHTML = '&#9654; Run';
    btn.disabled = false;
    btn.style.background = 'rgba(124,58,237,0.85)';
  }

  // ── Init ──

  function initPlayground() {
    autoMarkRunnable();
    addPlaygroundButtons();
  }

  // Wait for components to load (main.js sets components-loaded)
  if (document.body.classList.contains('components-loaded')) {
    setTimeout(initPlayground, 200);
  } else {
    var observer = new MutationObserver(function () {
      if (document.body.classList.contains('components-loaded')) {
        observer.disconnect();
        setTimeout(initPlayground, 200);
      }
    });
    observer.observe(document.body, { attributes: true, attributeFilter: ['class'] });
    // Fallback if components-loaded never fires
    setTimeout(initPlayground, 3000);
  }

  // ── Inline docs — show signature on hover over builtin names ──

  var BUILTIN_DOCS = {
    "println": "println(value) — Print value with newline",
    "print": "print(value) — Print value without newline",
    "len": "len(collection) — Length of list, string, or table",
    "map": "map(list, fn) — Transform each element",
    "filter": "filter(list, fn) — Keep elements where fn returns true",
    "reduce": "reduce(list, fn, init) — Fold list to single value",
    "each": "each(list, fn) — Execute fn for side effects, returns nil",
    "sort_by": "sort_by(list, fn) — Sort by comparison function",
    "mean": "mean(list) — Arithmetic mean",
    "median": "median(list) — Middle value",
    "stdev": "stdev(list) — Standard deviation",
    "sum": "sum(list) — Sum of numeric values",
    "min": "min(list) — Minimum value",
    "max": "max(list) — Maximum value",
    "gc_content": "gc_content(seq) — GC fraction (0.0-1.0)",
    "reverse_complement": "reverse_complement(seq) — Reverse complement of DNA/RNA",
    "transcribe": "transcribe(dna) — DNA → RNA (T→U)",
    "translate": "translate(seq) — DNA/RNA → protein",
    "kmers": "kmers(seq, k) — All k-mers as list",
    "find_motif": "find_motif(seq, motif) — Find motif positions",
    "seq_len": "seq_len(seq) — Sequence length",
    "tm": "tm(seq) — Melting temperature",
    "read_csv": "read_csv(path) — Read CSV file as table",
    "read_fasta": "read_fasta(path) — Read FASTA file as list of records",
    "read_fastq": "read_fastq(path) — Read FASTQ file as list of records",
    "read_vcf": "read_vcf(path) — Read VCF file as list of records",
    "read_bed": "read_bed(path) — Read BED file as list of records",
    "read_gff": "read_gff(path) — Read GFF file as list of records",
    "collect": "collect(stream) — Materialize stream into list",
    "typeof": "typeof(value) — Get type name as string",
    "range": "range(start, end, [step]) — Generate number sequence",
    "sort": "sort(list) — Sort ascending",
    "reverse": "reverse(list) — Reverse list order",
    "unique": "unique(list) — Remove duplicates",
    "flatten": "flatten(list) — Flatten nested lists",
    "zip": "zip(list1, list2) — Pair elements",
    "enumerate": "enumerate(list) — Add index to each element",
    "take": "take(list, n) — First n elements",
    "skip": "skip(list, n) — Skip first n elements",
    "head": "head(list, n) — First n elements (alias for take)",
    "join": "join(list, sep) — Join list to string",
    "split": "split(str, sep) — Split string to list",
    "contains": "contains(collection, item) — Check membership",
    "keys": "keys(record) — Get record field names",
    "values": "values(record) — Get record field values",
    "round": "round(num, [decimals]) — Round number",
    "abs": "abs(num) — Absolute value",
    "str": "str(value) — Convert to string",
    "int": "int(value) — Convert to integer",
    "float": "float(value) — Convert to float",
    "assert": "assert(condition, message) — Assert truth, error if false",
    "from_records": "from_records(list) — List of records → Table",
    "to_table": "to_table(list) — List of records → Table",
    "select": "select(table, cols...) — Pick columns",
    "group_by": "group_by(table, col) — Group rows by column",
    "summarize": "summarize(grouped, fn) — Aggregate groups",
    "mutate": "mutate(table, col, fn) — Add/modify column",
    "arrange": "arrange(table, col) — Sort table by column",
    "bio_join": "bio_join(left, right, [key], [type]) — Multi-omics join",
    "resolve": "resolve(id, [db]) — Cross-database ID resolver",
    "heatmap": "heatmap(data, [opts]) — SVG heatmap visualization",
    "volcano_plot": "volcano_plot(data, [opts]) — DE volcano plot",
    "pca_plot": "pca_plot(data, [opts]) — PCA scatter plot",
    "bar_chart": "bar_chart(data, [title]) — SVG bar chart",
    "scatter": "scatter(x, y, [opts]) — SVG scatter plot",
    "histogram": "histogram(data, [bins]) — SVG histogram",
    "qc_report": "qc_report(reads) — FASTQ quality metrics",
    "primer_design": "primer_design(seq, start, end) — PCR primer design",
    "blast": "blast(query, targets) — Local k-mer similarity search",
    "diff_table": "diff_table(a, b, [key]) — Compare two tables",
    "liftover": "liftover(chr, start, end) — Genome coordinate conversion",
    "ncbi_search": "ncbi_search(db, query) — Search NCBI database",
    "ncbi_gene": "ncbi_gene(query) — Get NCBI gene info",
    "ttest": "ttest(group1, group2) — Two-sample t-test",
    "cor": "cor(x, y) — Pearson correlation",
    "quantile": "quantile(list, p) — p-th quantile",
    "variance": "variance(list) — Sample variance",
    "mean_phred": "mean_phred(quality_str) — Mean Phred quality score"
  };

  // Create tooltip element
  var docTooltip = document.createElement("div");
  docTooltip.id = "bl-doc-tooltip";
  docTooltip.style.cssText = "display:none;position:fixed;z-index:9999;background:#1e293b;color:#e2e8f0;border:1px solid #334155;border-radius:6px;padding:4px 10px;font-size:12px;font-family:system-ui,sans-serif;pointer-events:none;max-width:350px;box-shadow:0 4px 12px rgba(0,0,0,0.3);white-space:nowrap;";
  document.body.appendChild(docTooltip);

  // Scan code blocks and wrap builtin names in hoverable spans
  function addDocHints() {
    document.querySelectorAll('code.language-biolang, code.language-bio').forEach(function(code) {
      if (code.dataset.hinted) return;
      code.dataset.hinted = "1";

      var html = code.innerHTML;
      Object.keys(BUILTIN_DOCS).forEach(function(name) {
        // Only match whole words that look like function calls or pipe targets
        var re = new RegExp("\\b(" + name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + ")\\b(?=\\s*[\\(|])", "g");
        html = html.replace(re, '<span class="bl-hint" data-doc="' + name + '">$1</span>');
      });
      code.innerHTML = html;
    });

    document.addEventListener("mouseover", function(e) {
      var hint = e.target.closest ? e.target.closest(".bl-hint") : null;
      if (hint) {
        var doc = BUILTIN_DOCS[hint.dataset.doc];
        if (doc) {
          docTooltip.textContent = doc;
          docTooltip.style.display = "block";
          var r = hint.getBoundingClientRect();
          docTooltip.style.left = r.left + "px";
          docTooltip.style.top = (r.top - 30) + "px";
        }
      } else {
        docTooltip.style.display = "none";
      }
    });
  }

  // Run after playground init
  setTimeout(addDocHints, 2000);
})();
