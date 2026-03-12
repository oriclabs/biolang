// BioLang WASM Playground — loads br-wasm module for in-browser execution
(function () {
  'use strict';

  var wasm = null;
  var ready = false;

  var editor   = document.getElementById('pg-editor');
  var output   = document.getElementById('pg-output');
  var lineNums = document.getElementById('pg-line-nums');
  var runBtn   = document.getElementById('pg-run');
  var clearBtn = document.getElementById('pg-clear');
  var resetBtn = document.getElementById('pg-reset');
  var statusDot  = document.getElementById('pg-status-dot');
  var statusText = document.getElementById('pg-status-text');
  var timingEl   = document.getElementById('pg-timing');
  var varsBar    = document.getElementById('pg-vars');
  var exToggle   = document.getElementById('pg-examples-toggle');
  var exMenu     = document.getElementById('pg-examples-menu');

  if (!editor) return;

  // ── Line numbers ──
  function updateLineNumbers() {
    var lines = editor.value.split('\n').length;
    var html = '';
    for (var i = 1; i <= lines; i++) {
      html += i + '\n';
    }
    lineNums.textContent = html;
    // Sync scroll
    lineNums.style.paddingTop = (16 - editor.scrollTop) + 'px';
  }

  editor.addEventListener('input', updateLineNumbers);
  editor.addEventListener('scroll', function () {
    lineNums.style.paddingTop = (16 - editor.scrollTop) + 'px';
  });

  // Strip leading newline from initial textarea content
  editor.value = editor.value.replace(/^\n/, '');
  updateLineNumbers();

  // ── Tab key support ──
  editor.addEventListener('keydown', function (e) {
    if (e.key === 'Tab') {
      e.preventDefault();
      var start = this.selectionStart;
      var end = this.selectionEnd;
      this.value = this.value.substring(0, start) + '  ' + this.value.substring(end);
      this.selectionStart = this.selectionEnd = start + 2;
      updateLineNumbers();
    }
    // Ctrl+Enter to run
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault();
      runCode();
    }
  });

  // ── WASM loading ──
  async function loadWasm() {
    try {
      var module = await import('../wasm/br_wasm.js');
      await module.default();
      module.init();
      wasm = module;
      ready = true;
      statusDot.classList.remove('loading');
      statusText.textContent = 'Ready';
    } catch (err) {
      statusDot.classList.remove('loading');
      statusDot.classList.add('error');
      statusText.textContent = 'Failed to load';
      appendOutput('Failed to load BioLang WASM module: ' + err.message, 'error');
      console.error('WASM load error:', err);
    }
  }

  loadWasm();

  // ── Run code ──
  function runCode() {
    if (!ready) {
      appendOutput('WASM module not loaded yet. Please wait...', 'error');
      return;
    }

    var source = editor.value;
    if (!source.trim()) return;

    runBtn.classList.add('running');
    runBtn.innerHTML = '<svg width="12" height="14" viewBox="0 0 12 14" fill="currentColor" class="animate-spin"><circle cx="6" cy="7" r="5" fill="none" stroke="currentColor" stroke-width="2" stroke-dasharray="20 10"/></svg> Running...';

    // Clear previous output
    output.innerHTML = '';
    timingEl.textContent = '';

    // Use setTimeout to let the UI update before blocking on WASM
    setTimeout(function () {
      var t0 = performance.now();
      var resultJson;
      try {
        resultJson = wasm.evaluate(source);
      } catch (err) {
        appendOutput('Runtime error: ' + err.message, 'error');
        finishRun(t0);
        return;
      }

      var result;
      try {
        result = JSON.parse(resultJson);
      } catch (e) {
        appendOutput('Could not parse result', 'error');
        finishRun(t0);
        return;
      }

      // Show stdout first
      if (result.output && result.output.trim()) {
        var lines = result.output.split('\n');
        for (var i = 0; i < lines.length; i++) {
          if (lines[i] !== '' || i < lines.length - 1) {
            appendOutput(lines[i], 'stdout');
          }
        }
      }

      // Show result or error
      if (result.ok) {
        if (result.value && result.value !== 'nil' && result.value !== '()') {
          // Detect SVG output — render inline
          if (result.value.trimStart().indexOf('<svg') === 0) {
            appendSvg(result.value);
          } else {
            appendOutput(result.value, 'result', result.type);
          }
        }
      } else if (result.error) {
        appendOutput(result.error, 'error');
      }

      finishRun(t0);
      updateVarsBar();
    }, 10);
  }

  function finishRun(t0) {
    var elapsed = performance.now() - t0;
    timingEl.textContent = elapsed < 1 ? '<1ms' : Math.round(elapsed) + 'ms';
    runBtn.classList.remove('running');
    runBtn.innerHTML = '<svg width="12" height="14" viewBox="0 0 12 14" fill="currentColor"><path d="M0 0l12 7-12 7z"/></svg> Run';
  }

  function appendOutput(text, kind, typeName) {
    var line = document.createElement('div');
    line.className = 'pg-output-line';
    if (kind === 'error') line.className += ' pg-output-error';
    else if (kind === 'result') line.className += ' pg-output-result';
    else line.className += ' pg-output-stdout';
    line.textContent = text;

    if (typeName && kind === 'result') {
      var typeSpan = document.createElement('span');
      typeSpan.className = 'pg-output-type';
      typeSpan.textContent = '  : ' + typeName;
      line.appendChild(typeSpan);
    }

    output.appendChild(line);
    output.scrollTop = output.scrollHeight;
  }

  function appendSvg(svgStr) {
    var wrapper = document.createElement('div');
    wrapper.style.cssText = 'background:#fff;border-radius:4px;padding:8px;margin:4px 0;overflow-x:auto;max-width:100%';
    wrapper.innerHTML = svgStr;
    var svg = wrapper.querySelector('svg');
    if (svg) { svg.style.maxWidth = '100%'; svg.style.height = 'auto'; }
    output.appendChild(wrapper);
    output.scrollTop = output.scrollHeight;
  }

  // ── Variables bar ──
  function updateVarsBar() {
    if (!ready) return;
    try {
      var varsJson = wasm.list_variables();
      var vars = JSON.parse(varsJson);
      if (vars.length === 0) {
        varsBar.innerHTML = '<span style="color:var(--pg-text-dim)">No variables</span>';
        return;
      }
      var html = '';
      for (var i = 0; i < vars.length; i++) {
        var v = vars[i];
        html += '<span class="pg-var-tag">';
        html += '<span class="pg-var-name">' + escapeHtml(v.name) + '</span>';
        html += '<span class="pg-var-type">' + escapeHtml(v.type) + '</span>';
        html += '</span>';
      }
      varsBar.innerHTML = html;
    } catch (e) {
      // Ignore
    }
  }

  // ── Buttons ──
  runBtn.addEventListener('click', runCode);

  clearBtn.addEventListener('click', function () {
    output.innerHTML = '';
    timingEl.textContent = '';
  });

  resetBtn.addEventListener('click', function () {
    if (!ready) return;
    wasm.reset();
    output.innerHTML = '';
    timingEl.textContent = '';
    varsBar.innerHTML = '<span style="color:var(--pg-text-dim)">No variables</span>';
    appendOutput('Interpreter state reset.', 'stdout');
  });

  // ── Examples dropdown ──
  var EXAMPLES = [
    { cat: 'Basics' },
    { name: 'Hello DNA', code:
      '# DNA sequence basics\n' +
      'let seq = dna"ATCGATCGATCG"\n' +
      'println(f"Sequence: {seq}")\n' +
      'println(f"Length:   {seq_len(seq)} bp")\n' +
      'println(f"GC:      {round(gc_content(seq) * 100, 1)}%")\n\n' +
      '# DNA literals are first-class types\n' +
      'let prot = protein"MKAF"\n' +
      'println(f"Protein: {prot}")\n' +
      'println(f"Protein length: {len(prot)} aa")\n'
    },
    { name: 'Variables & Types', code:
      '# BioLang is dynamically typed\n' +
      'let name = "BRCA1"\n' +
      'let count = 42\n' +
      'let ratio = 0.95\n' +
      'let active = true\n' +
      'let genes = ["TP53", "EGFR", "BRCA1"]\n\n' +
      'println(f"Gene: {name}")\n' +
      'println(f"Count: {count}")\n' +
      'println(f"Genes: {genes}")\n' +
      'println(f"Length: {len(genes)}")\n'
    },
    { name: 'Pipes & Transforms', code:
      '# Pipe operator |> passes result as first argument\n' +
      'let nums = [3, 1, 4, 1, 5, 9, 2, 6]\n\n' +
      'let result = nums\n' +
      '  |> filter(|n| n > 3)\n' +
      '  |> sort()\n' +
      '  |> map(|n| n * 10)\n\n' +
      'println(f"Original: {nums}")\n' +
      'println(f"Filtered, sorted, scaled: {result}")\n' +
      'println(f"Sum: {sum(result)}")\n'
    },
    { cat: 'Bioinformatics' },
    { name: 'DNA Operations', code:
      '# Full DNA analysis pipeline\n' +
      'let seq = dna"ATCGATCGATCG"\n' +
      'println(f"Sequence:    {seq}")\n' +
      'println(f"Complement:  {complement(seq)}")\n' +
      'println(f"Rev-comp:    {reverse_complement(seq)}")\n' +
      'println(f"Transcribed: {transcribe(seq)}")\n\n' +
      '# Translate a coding sequence\n' +
      'let coding = dna"ATGAAAGCTTTTGACTGA"\n' +
      'println(f"DNA:     {coding}")\n' +
      'println(f"Protein: {translate(coding)}")\n'
    },
    { name: 'GC Content Analysis', code:
      '# Analyze GC content using built-in functions\n' +
      'let sequences = [\n' +
      '  dna"ATCGATCGATCGATCG",\n' +
      '  dna"GCGCGCGCATATATAT",\n' +
      '  dna"AAATTTAAATTTAAAT",\n' +
      '  dna"GCGCGCGCGCGCGCGC"\n' +
      ']\n\n' +
      'let gc_values = sequences |> map(|s| round(gc_content(s) * 100, 1))\n' +
      'println(f"GC% values: {gc_values}")\n' +
      'println(f"Mean GC%:   {round(mean(gc_values), 1)}")\n' +
      'println(f"Min GC%:    {min(gc_values)}")\n' +
      'println(f"Max GC%:    {max(gc_values)}")\n'
    },
    { name: 'K-mer Counting', code:
      '# Count k-mers in a DNA sequence\n' +
      'let seq = dna"ATCGATCGATCGATCG"\n\n' +
      '# kmer_count returns a table of k-mer frequencies\n' +
      'let tri = kmer_count(seq, 3)\n' +
      'println("3-mer counts:")\n' +
      'println(tri)\n\n' +
      '# Try different k values\n' +
      'let di = kmer_count(seq, 2)\n' +
      'println("\\n2-mer counts:")\n' +
      'println(di)\n'
    },
    { name: 'Sequence Literals', code:
      '# BioLang has native DNA, RNA, and Protein literals\n' +
      'let dna_seq = dna"ATGAAAGCGTTCGAA"\n' +
      'let rna_seq = rna"AUGAAAGCGUUCGAA"\n' +
      'let prot = protein"MKAFEPG"\n\n' +
      'println(f"DNA:     {dna_seq} ({seq_len(dna_seq)} bp)")\n' +
      'println(f"RNA:     {rna_seq} ({seq_len(rna_seq)} bases)")\n' +
      'println(f"Protein: {prot} ({len(prot)} aa)")\n\n' +
      '# Type checking\n' +
      'println(f"DNA type:     {type(dna_seq)}")\n' +
      'println(f"RNA type:     {type(rna_seq)}")\n' +
      'println(f"Protein type: {type(prot)}")\n'
    },
    { name: 'ORF Finding', code:
      '# Find open reading frames in a sequence\n' +
      'let seq = dna"AATGATGAAAGCTTTTGACTGAATCGATG"\n' +
      'let orfs = find_orfs(seq)\n' +
      'println(f"Sequence: {seq}")\n' +
      'println(f"Found {len(orfs)} ORFs:")\n' +
      'orfs |> each(|orf| println(f"  {orf}"))\n\n' +
      '# Codon usage analysis\n' +
      'let coding = dna"ATGAAAGCTTTTGACTGA"\n' +
      'let usage = codon_usage(coding)\n' +
      'println(f"\\nCodon usage: {usage}")\n'
    },
    { cat: 'Statistics' },
    { name: 'Descriptive Stats', code:
      '# Statistical functions\n' +
      'let data = [2.5, 3.1, 4.7, 5.2, 3.8, 6.1, 4.3, 5.5]\n\n' +
      'println(f"Mean:   {round(mean(data), 2)}")\n' +
      'println(f"Median: {median(data)}")\n' +
      'println(f"Stdev:  {round(stdev(data), 2)}")\n' +
      'println(f"Min:    {min(data)}")\n' +
      'println(f"Max:    {max(data)}")\n' +
      'println(f"Sum:    {sum(data)}")\n' +
      'println(f"Sorted: {sort(data)}")\n'
    },
    { name: 'Hypothesis Testing', code:
      '# T-test: are tumor and normal different?\n' +
      'let normal = [5.2, 4.8, 5.1, 4.9, 5.3]\n' +
      'let tumor = [8.1, 7.9, 8.5, 7.6, 8.3]\n\n' +
      'let result = ttest(normal, tumor)\n' +
      'println(f"t-statistic: {round(result.statistic, 3)}")\n' +
      'println(f"p-value:     {result.p_value}")\n' +
      'println(f"Significant: {result.p_value < 0.05}")\n\n' +
      '# Correlation\n' +
      'let x = [1.0, 2.0, 3.0, 4.0, 5.0]\n' +
      'let y = [2.1, 4.0, 5.8, 8.1, 9.9]\n' +
      'let r = cor(x, y)\n' +
      'println(f"\\nCorrelation: {round(r, 4)}")\n'
    },
    { name: 'Linear Regression', code:
      '# Simple linear regression\n' +
      'let x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]\n' +
      'let y = [2.1, 3.9, 6.2, 7.8, 10.1, 12.3]\n\n' +
      'let model = lm(x, y)\n' +
      'println(f"Slope:     {round(model.slope, 3)}")\n' +
      'println(f"Intercept: {round(model.intercept, 3)}")\n' +
      'println(f"R-squared: {round(model.r_squared, 3)}")\n' +
      'println(f"p-value:   {model.p_value}")\n'
    },
    { cat: 'Data & Functions' },
    { name: 'Records', code:
      '# Records (like structs/objects)\n' +
      'let gene = {\n' +
      '  name: "BRCA1",\n' +
      '  chrom: "chr17",\n' +
      '  start: 43044295,\n' +
      '  end: 43125483\n' +
      '}\n\n' +
      'println(f"Gene: {gene.name}")\n' +
      'println(f"Location: {gene.chrom}:{gene.start}-{gene.end}")\n' +
      'println(f"Length: {gene.end - gene.start} bp")\n'
    },
    { name: 'Functions & Closures', code:
      '# Define custom functions\n' +
      'fn classify_gc(seq) {\n' +
      '  let gc = gc_content(seq) * 100\n' +
      '  if gc > 60.0 { "GC-rich" } else if gc < 40.0 { "AT-rich" } else { "balanced" }\n' +
      '}\n\n' +
      'let seqs = [\n' +
      '  dna"GCGCGCGCGC",\n' +
      '  dna"ATATATATAT",\n' +
      '  dna"ATCGATCGAT"\n' +
      ']\n\n' +
      'seqs |> each(|s| {\n' +
      '  let gc = round(gc_content(s) * 100, 1)\n' +
      '  println(f"{s}: GC={gc}% -> {classify_gc(s)}")\n' +
      '})\n'
    }
  ];

  function buildExamplesMenu() {
    var html = '';
    for (var i = 0; i < EXAMPLES.length; i++) {
      var ex = EXAMPLES[i];
      if (ex.cat) {
        html += '<button class="pg-ex-cat" disabled>' + escapeHtml(ex.cat) + '</button>';
      } else {
        html += '<button data-ex="' + i + '">' + escapeHtml(ex.name) + '</button>';
      }
    }
    exMenu.innerHTML = html;

    exMenu.addEventListener('click', function (e) {
      var btn = e.target.closest('[data-ex]');
      if (!btn) return;
      var idx = parseInt(btn.dataset.ex, 10);
      var ex = EXAMPLES[idx];
      if (ex && ex.code) {
        editor.value = ex.code;
        updateLineNumbers();
        exMenu.classList.remove('open');
        // Auto-reset and run
        if (ready) {
          wasm.reset();
          varsBar.innerHTML = '';
        }
        runCode();
      }
    });
  }
  buildExamplesMenu();

  exToggle.addEventListener('click', function (e) {
    e.stopPropagation();
    exMenu.classList.toggle('open');
  });
  document.addEventListener('click', function () {
    exMenu.classList.remove('open');
  });
  exMenu.addEventListener('click', function (e) {
    e.stopPropagation();
  });

  // ── Utility ──
  function escapeHtml(str) {
    var div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }

})();
