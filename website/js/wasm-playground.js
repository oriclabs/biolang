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
          appendOutput(result.value, 'result', result.type);
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
      'print("Sequence:", seq)\n' +
      'print(f"Length: {len(seq)} bp")\n\n' +
      '# DNA literals are first-class types\n' +
      'let prot = protein"MKAF"\n' +
      'print("Protein:", prot)\n' +
      'print(f"Protein length: {len(prot)} aa")\n\n' +
      '# K-mer analysis\n' +
      'let counts = kmer_count(seq, 3)\n' +
      'print(f"3-mer counts: {counts}")\n'
    },
    { name: 'Variables & Types', code:
      '# BioLang is dynamically typed\n' +
      'let name = "BRCA1"\n' +
      'let count = 42\n' +
      'let ratio = 0.95\n' +
      'let active = true\n' +
      'let genes = ["TP53", "EGFR", "BRCA1"]\n\n' +
      'print(f"Gene: {name}")\n' +
      'print(f"Count: {count}")\n' +
      'print(f"Genes: {genes}")\n' +
      'print(f"Length: {len(genes)}")\n'
    },
    { name: 'Pipes & Transforms', code:
      '# Pipe operator |> passes result as first argument\n' +
      'let nums = [3, 1, 4, 1, 5, 9, 2, 6]\n\n' +
      'let result = nums\n' +
      '  |> filter(|n| n > 3)\n' +
      '  |> sort()\n' +
      '  |> map(|n| n * 10)\n\n' +
      'print(f"Original: {nums}")\n' +
      'print(f"Filtered, sorted, scaled: {result}")\n' +
      'print(f"Sum: {sum(result)}")\n'
    },
    { cat: 'Bioinformatics' },
    { name: 'GC Content Analysis', code:
      '# Analyze GC content of multiple sequences\n' +
      'fn calc_gc(s) {\n' +
      '  let bases = split(s, "")\n' +
      '  let gc = bases |> filter(|b| b == "G" or b == "C") |> len()\n' +
      '  round(gc / len(bases) * 100, 1)\n' +
      '}\n\n' +
      'let sequences = [\n' +
      '  "ATCGATCGATCGATCG",\n' +
      '  "GCGCGCGCATATATAT",\n' +
      '  "AAATTTAAATTTAAAT",\n' +
      '  "GCGCGCGCGCGCGCGC"\n' +
      ']\n\n' +
      'let gc_values = sequences |> map(|s| calc_gc(s))\n' +
      'print(f"GC% values: {gc_values}")\n' +
      'print(f"Mean GC%: {round(mean(gc_values), 1)}")\n' +
      'print(f"Min GC%: {min(gc_values)}")\n' +
      'print(f"Max GC%: {max(gc_values)}")\n'
    },
    { name: 'K-mer Counting', code:
      '# Count k-mers in a DNA sequence\n' +
      'let seq = dna"ATCGATCGATCGATCG"\n' +
      'let k = 3\n\n' +
      '# kmer_count returns a table of k-mer frequencies\n' +
      'let counts = kmer_count(seq, k)\n' +
      'print(f"{k}-mer counts:")\n' +
      'print(counts)\n\n' +
      '# Try different k values\n' +
      'let di = kmer_count(seq, 2)\n' +
      'print(f"\\n2-mer counts:")\n' +
      'print(di)\n'
    },
    { name: 'Sequence Literals', code:
      '# BioLang has native DNA, RNA, and Protein literals\n' +
      'let dna_seq = dna"ATGAAAGCGTTCGAA"\n' +
      'let rna_seq = rna"AUGAAAGCGUUCGAA"\n' +
      'let prot = protein"MKAFEPG"\n\n' +
      'print("DNA:", dna_seq)\n' +
      'print(f"DNA length: {len(dna_seq)} bp")\n\n' +
      'print("RNA:", rna_seq)\n' +
      'print(f"RNA length: {len(rna_seq)} bases")\n\n' +
      'print("Protein:", prot)\n' +
      'print(f"Protein length: {len(prot)} aa")\n\n' +
      '# Type checking\n' +
      'print(f"DNA type: {type(dna_seq)}")\n' +
      'print(f"RNA type: {type(rna_seq)}")\n' +
      'print(f"Protein type: {type(prot)}")\n'
    },
    { cat: 'Data & Math' },
    { name: 'Statistics', code:
      '# Statistical functions\n' +
      'let data = [2.5, 3.1, 4.7, 5.2, 3.8, 6.1, 4.3, 5.5]\n\n' +
      'print(f"Mean:   {mean(data)}")\n' +
      'print(f"Median: {median(data)}")\n' +
      'print(f"Stdev:  {stdev(data)}")\n' +
      'print(f"Min:    {min(data)}")\n' +
      'print(f"Max:    {max(data)}")\n' +
      'print(f"Sum:    {sum(data)}")\n' +
      'print(f"Sorted: {sort(data)}")\n'
    },
    { name: 'Records & Maps', code:
      '# Records (like structs/objects)\n' +
      'let gene = {\n' +
      '  name: "BRCA1",\n' +
      '  chrom: "chr17",\n' +
      '  start: 43044295,\n' +
      '  end: 43125483\n' +
      '}\n\n' +
      'print(f"Gene: {gene.name}")\n' +
      'print(f"Location: {gene.chrom}:{gene.start}-{gene.end}")\n' +
      'print(f"Length: {gene.end - gene.start} bp")\n\n' +
      '# Maps\n' +
      'let codon_table = {"ATG": "Met", "TAA": "Stop", "GCG": "Ala"}\n' +
      'print(f"ATG codes for: {codon_table[\\"ATG\\"]}")\n'
    },
    { name: 'Functions & Closures', code:
      '# Define custom functions\n' +
      'fn gc_percent(s) {\n' +
      '  let bases = split(s, "")\n' +
      '  let gc = bases |> filter(|b| b == "G" or b == "C") |> len()\n' +
      '  round(gc / len(bases) * 100, 1)\n' +
      '}\n\n' +
      'fn classify(gc) {\n' +
      '  if gc > 60.0 {\n' +
      '    "GC-rich"\n' +
      '  } else if gc < 40.0 {\n' +
      '    "AT-rich"\n' +
      '  } else {\n' +
      '    "balanced"\n' +
      '  }\n' +
      '}\n\n' +
      'let seqs = ["GCGCGCGCGC", "ATATATATAT", "ATCGATCGAT"]\n\n' +
      'seqs |> each(|s| {\n' +
      '  let gc = gc_percent(s)\n' +
      '  print(f"{s}: GC={gc}% -> {classify(gc)}")\n' +
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
