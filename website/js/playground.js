// BioLang Playground — WASM-powered inline code execution for docs
// Auto-adds Run buttons to all language-biolang code blocks
// Lazy-loads WASM on first Run click (~4 MB), then cached for session

(function () {
  'use strict';

  // ── WASM module state ──
  var wasm = null;
  var wasmLoading = false;
  var wasmQueue = [];

  function getWasmBasePath() {
    // Detect path to /wasm/ from current page location
    var path = window.location.pathname;
    // Count depth from site root
    var segments = path.split('/').filter(function(s) { return s.length > 0; });
    // Remove the last segment (the HTML file itself)
    segments.pop();
    var prefix = segments.map(function() { return '..'; }).join('/');
    return prefix ? prefix + '/wasm' : './wasm';
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
      '  var mod = await import("' + basePath + '/br_wasm.js");',
      '  await mod.default();',
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
    // File I/O: FASTA, FASTQ, VCF, BED, GFF, SAM, BAM, CSV, generic open/save
    if (/\b(read_fasta|read_fastq|read_vcf|read_bed|read_gff|read_sam|read_bam|read_csv|write_csv|write_fasta|write_fastq|write_vcf|write_bed)\b/.test(text)) return true;
    if (/\b(csv|tsv|vcf|fastq|fasta|bam|sam|bed|gff)\s*\(/.test(text)) return true;
    if (/\b(open|save|write_file|read_file|read_lines)\s*\(/.test(text)) return true;
    // Network APIs: NCBI, Ensembl, UniProt, KEGG, PDB, etc.
    if (/\b(ncbi_gene|ncbi_search|ncbi_sequence|ensembl_gene|ensembl_vep|uniprot_search|uniprot_entry|kegg_get|kegg_find|pdb_entry|string_network|go_term|go_annotations|cosmic_gene|datasets_gene|reactome_pathways|ucsc_sequence|fetch|http_get|http_post)\b/.test(text)) return true;
    // LLM chat
    if (/\b(chat|chat_code|llm|ask_llm)\s*\(/.test(text)) return true;
    // Notebooks and pipelines
    if (/\b(notebook|pipeline|import\s+")\b/.test(text)) return true;
    // Saving plots to files (save_svg, save_png, save_plot write to disk)
    if (/\b(save_plot|save_svg|save_png)\s*\(/.test(text)) return true;
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
        div.style.cssText = 'position:relative';
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
    btn.innerHTML = '&#9654; Running...';
    outputEl.style.display = 'block';
    var pre = outputEl.previousElementSibling || outputEl.parentNode.querySelector('pre');
    if (pre) pre.style.borderRadius = '8px 8px 0 0';
    resultEl.innerHTML = '';

    var t0 = performance.now();
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
    resetButton(btn);
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
})();
