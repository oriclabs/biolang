// BioLang Playground — Simulated inline interpreter for docs
// Handles ~40 core builtins, pipes, DNA/RNA/protein literals, collections, math

(function () {
  'use strict';

  // ── Mini Evaluator ──

  function BioLangEval() {
    this.env = {};
  }

  // DNA/RNA/Protein helpers
  function complementBase(b) {
    return { A: 'T', T: 'A', C: 'G', G: 'C', a: 't', t: 'a', c: 'g', g: 'c' }[b] || b;
  }
  function reverseComplement(seq) {
    return seq.split('').reverse().map(complementBase).join('');
  }
  function gcContent(seq) {
    var s = seq.toUpperCase();
    var gc = 0;
    for (var i = 0; i < s.length; i++) { if (s[i] === 'G' || s[i] === 'C') gc++; }
    return s.length ? gc / s.length : 0;
  }
  function transcribe(seq) {
    return seq.replace(/T/g, 'U').replace(/t/g, 'u');
  }
  var codonTable = {
    'AUG':'M','UUU':'F','UUC':'F','UUA':'L','UUG':'L','CUU':'L','CUC':'L','CUA':'L','CUG':'L',
    'AUU':'I','AUC':'I','AUA':'I','GUU':'V','GUC':'V','GUA':'V','GUG':'V','UCU':'S','UCC':'S',
    'UCA':'S','UCG':'S','CCU':'P','CCC':'P','CCA':'P','CCG':'P','ACU':'T','ACC':'T','ACA':'T',
    'ACG':'T','GCU':'A','GCC':'A','GCA':'A','GCG':'A','UAU':'Y','UAC':'Y','CAU':'H','CAC':'H',
    'CAA':'Q','CAG':'Q','AAU':'N','AAC':'N','AAA':'K','AAG':'K','GAU':'D','GAC':'D','GAA':'E',
    'GAG':'E','UGU':'C','UGC':'C','UGG':'W','CGU':'R','CGC':'R','CGA':'R','CGG':'R','AGU':'S',
    'AGC':'S','AGA':'R','AGG':'R','GGU':'G','GGC':'G','GGA':'G','GGG':'G','UAA':'*','UAG':'*','UGA':'*'
  };
  function translate(seq) {
    var rna = seq.toUpperCase().replace(/T/g, 'U');
    var protein = '';
    for (var i = 0; i + 2 < rna.length; i += 3) {
      var codon = rna.substring(i, i + 3);
      var aa = codonTable[codon];
      if (!aa || aa === '*') break;
      protein += aa;
    }
    return protein;
  }

  // Sparkline
  var sparkChars = '▁▂▃▄▅▆▇█';
  function sparkline(arr) {
    if (!arr.length) return '';
    var mn = Math.min.apply(null, arr), mx = Math.max.apply(null, arr);
    var range = mx - mn || 1;
    return arr.map(function (v) {
      var idx = Math.round(((v - mn) / range) * 7);
      return sparkChars[idx];
    }).join('');
  }

  // K-mer count
  function kmerCount(seq, k) {
    var counts = {};
    var s = seq.toUpperCase();
    for (var i = 0; i <= s.length - k; i++) {
      var kmer = s.substring(i, i + k);
      counts[kmer] = (counts[kmer] || 0) + 1;
    }
    return counts;
  }

  // ── Expression Parser & Evaluator ──

  BioLangEval.prototype.eval = function (code) {
    var lines = code.split('\n');
    var output = [];
    var self = this;
    this.env = {};

    lines.forEach(function (rawLine) {
      var line = rawLine.replace(/#.*$/, '').trim();
      if (!line) return;

      // let bindings
      var letMatch = line.match(/^let\s+(\w+)\s*=\s*(.+)$/);
      if (letMatch) {
        self.env[letMatch[1]] = self.evalExpr(letMatch[2].trim());
        return;
      }

      // print/println — find matching closing paren
      if (line.match(/^(?:print|println)\(/)) {
        var fnM = line.match(/^(print|println)\((.*)\)$/s);
        if (fnM) {
          var printArgs = self.splitTopLevel(fnM[2], ',');
          var parts = printArgs.map(function(a) { return self.formatPrint(self.evalExpr(a.trim())); });
          output.push(parts.join(''));
          return;
        }
      }

      // Bare expression
      var result = self.evalExpr(line);
      if (result !== undefined && result !== null) {
        output.push(self.formatValue(result));
      }
    });

    return output.join('\n');
  };

  BioLangEval.prototype.evalExpr = function (expr) {
    expr = expr.trim();

    // Handle pipe chains: split on |> and fold
    if (expr.indexOf('|>') !== -1) {
      return this.evalPipe(expr);
    }

    return this.evalAtom(expr);
  };

  BioLangEval.prototype.evalPipe = function (expr) {
    // Split on |> not inside parens/quotes
    var parts = this.splitPipe(expr);
    var val = this.evalAtom(parts[0].trim());
    for (var i = 1; i < parts.length; i++) {
      val = this.applyPipeStep(val, parts[i].trim());
    }
    return val;
  };

  BioLangEval.prototype.splitPipe = function (expr) {
    var parts = [];
    var depth = 0;
    var inStr = false;
    var strChar = '';
    var current = '';
    for (var i = 0; i < expr.length; i++) {
      var c = expr[i];
      if (inStr) {
        current += c;
        if (c === strChar && expr[i - 1] !== '\\') inStr = false;
        continue;
      }
      if (c === '"' || c === "'") { inStr = true; strChar = c; current += c; continue; }
      if (c === '(' || c === '[' || c === '{') { depth++; current += c; continue; }
      if (c === ')' || c === ']' || c === '}') { depth--; current += c; continue; }
      if (depth === 0 && c === '|' && expr[i + 1] === '>') {
        parts.push(current);
        current = '';
        i++; // skip >
        continue;
      }
      current += c;
    }
    parts.push(current);
    return parts;
  };

  BioLangEval.prototype.applyPipeStep = function (val, step) {
    // step is like "gc_content()" or "filter(fn(x) x > 2)" or "len()" or "sort()" etc.
    var fnMatch = step.match(/^(\w+)\((.*)\)$/s);
    if (!fnMatch) {
      // Bare function name
      return this.callBuiltin(step, [val]);
    }
    var fnName = fnMatch[1];
    var argsStr = fnMatch[2].trim();
    var args = [val];
    if (argsStr) {
      // Simple arg parsing for basic cases
      args.push(this.evalSimpleArg(argsStr));
    }
    return this.callBuiltin(fnName, args);
  };

  BioLangEval.prototype.evalSimpleArg = function (s) {
    s = s.trim();
    // Number
    if (/^-?\d+(\.\d+)?$/.test(s)) return parseFloat(s);
    // String
    if ((s[0] === '"' && s[s.length - 1] === '"') || (s[0] === "'" && s[s.length - 1] === "'")) {
      return s.slice(1, -1);
    }
    // Variable ref
    if (this.env[s] !== undefined) return this.env[s];
    return s;
  };

  BioLangEval.prototype.evalAtom = function (expr) {
    expr = expr.trim();
    if (!expr) return null;

    // DNA literal
    var dnaMatch = expr.match(/^dna"([^"]*)"$/);
    if (dnaMatch) return { _type: 'DNA', seq: dnaMatch[1].toUpperCase() };

    // RNA literal
    var rnaMatch = expr.match(/^rna"([^"]*)"$/);
    if (rnaMatch) return { _type: 'RNA', seq: rnaMatch[1].toUpperCase() };

    // Protein literal
    var protMatch = expr.match(/^protein"([^"]*)"$/);
    if (protMatch) return { _type: 'Protein', seq: protMatch[1].toUpperCase() };

    // Number
    if (/^-?\d+(\.\d+)?$/.test(expr)) return parseFloat(expr);

    // String
    if ((expr[0] === '"' && expr[expr.length - 1] === '"') || (expr[0] === "'" && expr[expr.length - 1] === "'")) {
      return expr.slice(1, -1);
    }

    // Boolean / nil
    if (expr === 'true') return true;
    if (expr === 'false') return false;
    if (expr === 'nil') return null;

    // List literal [a, b, c]
    if (expr[0] === '[' && expr[expr.length - 1] === ']') {
      var inner = expr.slice(1, -1).trim();
      if (!inner) return [];
      return inner.split(',').map(function (x) { return this.evalExpr(x.trim()); }.bind(this));
    }

    // Map literal {"k": v, ...}
    if (expr[0] === '{' && expr[expr.length - 1] === '}') {
      var mapInner = expr.slice(1, -1).trim();
      if (!mapInner) return {};
      var obj = {};
      var pairs = this.splitTopLevel(mapInner, ',');
      for (var i = 0; i < pairs.length; i++) {
        var kv = pairs[i].split(':');
        if (kv.length >= 2) {
          var key = kv[0].trim().replace(/^["']|["']$/g, '');
          obj[key] = this.evalExpr(kv.slice(1).join(':').trim());
        }
      }
      return obj;
    }

    // Function call: name(args)
    var fnMatch = expr.match(/^(\w+)\((.*)\)$/s);
    if (fnMatch) {
      var name = fnMatch[1];
      var argsStr = fnMatch[2].trim();
      var args = argsStr ? this.splitTopLevel(argsStr, ',').map(function (a) { return this.evalExpr(a.trim()); }.bind(this)) : [];
      return this.callBuiltin(name, args);
    }

    // Variable
    if (this.env[expr] !== undefined) return this.env[expr];

    // Arithmetic
    var arithMatch = expr.match(/^(.+?)\s*([+\-*\/%])\s*(.+)$/);
    if (arithMatch) {
      var left = this.evalExpr(arithMatch[1]);
      var right = this.evalExpr(arithMatch[3]);
      switch (arithMatch[2]) {
        case '+': return (typeof left === 'string') ? left + right : left + right;
        case '-': return left - right;
        case '*': return left * right;
        case '/': return right !== 0 ? left / right : NaN;
        case '%': return left % right;
      }
    }

    return expr;
  };

  BioLangEval.prototype.splitTopLevel = function (s, delim) {
    var parts = [];
    var depth = 0;
    var inStr = false;
    var strChar = '';
    var current = '';
    for (var i = 0; i < s.length; i++) {
      var c = s[i];
      if (inStr) {
        current += c;
        if (c === strChar && s[i - 1] !== '\\') inStr = false;
        continue;
      }
      if (c === '"' || c === "'") { inStr = true; strChar = c; current += c; continue; }
      if (c === '(' || c === '[' || c === '{') { depth++; current += c; continue; }
      if (c === ')' || c === ']' || c === '}') { depth--; current += c; continue; }
      if (depth === 0 && c === delim) {
        parts.push(current);
        current = '';
        continue;
      }
      current += c;
    }
    parts.push(current);
    return parts;
  };

  // ── Builtins ──

  BioLangEval.prototype.callBuiltin = function (name, args) {
    var a = args[0], b = args[1], c = args[2];
    var seq = (a && a._type) ? a.seq : (typeof a === 'string' ? a : '');

    switch (name) {
      // Core
      case 'len':
        if (a && a._type) return a.seq.length;
        if (Array.isArray(a)) return a.length;
        if (typeof a === 'string') return a.length;
        if (a && typeof a === 'object') return Object.keys(a).length;
        return 0;
      case 'type':
      case 'typeof':
        if (a && a._type) return a._type;
        if (Array.isArray(a)) return 'List';
        return typeof a === 'number' ? (Number.isInteger(a) ? 'Int' : 'Float') : typeof a === 'string' ? 'String' : typeof a === 'boolean' ? 'Bool' : a === null ? 'Nil' : 'Map';
      case 'print': case 'println': return this.formatPrint(a);
      case 'str': case 'to_string': return this.formatValue(a);
      case 'int': return typeof a === 'number' ? Math.floor(a) : parseInt(a) || 0;
      case 'float': return typeof a === 'number' ? a : parseFloat(a) || 0.0;
      case 'bool': return !!a;
      case 'abs': return Math.abs(a);
      case 'range': return Array.from({length: (b||0) - (a||0)}, function(_, i) { return a + i; });
      case 'assert': if (!a) throw new Error('Assertion failed'); return true;

      // Math
      case 'min': return Array.isArray(a) ? Math.min.apply(null, a) : Math.min(a, b);
      case 'max': return Array.isArray(a) ? Math.max.apply(null, a) : Math.max(a, b);
      case 'sum': return Array.isArray(a) ? a.reduce(function(s,v){return s+v;},0) : a;
      case 'mean': return Array.isArray(a) ? a.reduce(function(s,v){return s+v;},0)/a.length : a;
      case 'median':
        if (!Array.isArray(a)) return a;
        var sorted = a.slice().sort(function(x,y){return x-y;});
        var mid = Math.floor(sorted.length / 2);
        return sorted.length % 2 ? sorted[mid] : (sorted[mid-1] + sorted[mid]) / 2;
      case 'stdev':
        if (!Array.isArray(a) || a.length < 2) return 0;
        var m = a.reduce(function(s,v){return s+v;},0)/a.length;
        var variance = a.reduce(function(s,v){return s+(v-m)*(v-m);},0)/(a.length-1);
        return Math.sqrt(variance);
      case 'sqrt': return Math.sqrt(a);
      case 'pow': return Math.pow(a, b);
      case 'log': return Math.log(a);
      case 'log2': return Math.log2(a);
      case 'log10': return Math.log10(a);
      case 'round': return typeof b === 'number' ? parseFloat(a.toFixed(b)) : Math.round(a);
      case 'ceil': return Math.ceil(a);
      case 'floor': return Math.floor(a);
      case 'sin': return Math.sin(a);
      case 'cos': return Math.cos(a);
      case 'count': return Array.isArray(a) ? a.length : 0;

      // String
      case 'upper': return (typeof a === 'string' ? a : seq).toUpperCase();
      case 'lower': return (typeof a === 'string' ? a : seq).toLowerCase();
      case 'trim': return (typeof a === 'string' ? a : '').trim();
      case 'split': return (typeof a === 'string' ? a : '').split(b || '');
      case 'join': return Array.isArray(a) ? a.join(b || '') : '';
      case 'replace': return typeof a === 'string' ? a.replace(new RegExp(b, 'g'), c) : '';
      case 'starts_with': return typeof a === 'string' && a.startsWith(b);
      case 'ends_with': return typeof a === 'string' && a.endsWith(b);
      case 'contains': return Array.isArray(a) ? a.indexOf(b) !== -1 : typeof a === 'string' ? a.includes(b) : false;
      case 'repeat': return typeof a === 'string' ? a.repeat(b || 1) : '';
      case 'substring': return typeof a === 'string' ? a.substring(b || 0, c) : '';

      // Collections
      case 'first': return Array.isArray(a) ? a[0] : a;
      case 'last': return Array.isArray(a) ? a[a.length - 1] : a;
      case 'take': return Array.isArray(a) ? a.slice(0, b || 5) : a;
      case 'skip': return Array.isArray(a) ? a.slice(b || 0) : a;
      case 'reverse': if (a && a._type) return {_type: a._type, seq: a.seq.split('').reverse().join('')}; return Array.isArray(a) ? a.slice().reverse() : a;
      case 'sort':
        if (!Array.isArray(a)) return a;
        var s = a.slice();
        if (b === 'desc') s.sort(function(x,y){return y-x;});
        else s.sort(function(x,y){return x<y?-1:x>y?1:0;});
        return s;
      case 'unique': return Array.isArray(a) ? a.filter(function(v,i,arr){return arr.indexOf(v)===i;}) : a;
      case 'flatten': return Array.isArray(a) ? a.reduce(function(acc,v){return acc.concat(v);},[]) : a;
      case 'zip':
        if (!Array.isArray(a) || !Array.isArray(b)) return [];
        return a.map(function(v,i){return [v, b[i]];});
      case 'enumerate':
        if (!Array.isArray(a)) return [];
        return a.map(function(v,i){return [i, v];});
      case 'push':
        if (Array.isArray(a)) { var copy = a.slice(); copy.push(b); return copy; }
        return a;
      case 'keys': return (a && typeof a === 'object' && !Array.isArray(a)) ? Object.keys(a) : [];
      case 'values': return (a && typeof a === 'object' && !Array.isArray(a)) ? Object.values(a) : [];
      case 'chunk':
        if (!Array.isArray(a) || !b) return [a];
        var chunks = [];
        for (var i = 0; i < a.length; i += b) chunks.push(a.slice(i, i + b));
        return chunks;

      // Bio
      case 'gc_content':
        return parseFloat(gcContent(seq).toFixed(4));
      case 'reverse_complement':
        return a && a._type === 'DNA' ? {_type: 'DNA', seq: reverseComplement(a.seq)} : reverseComplement(seq);
      case 'complement':
        return a && a._type === 'DNA' ? {_type: 'DNA', seq: a.seq.split('').map(complementBase).join('')} : seq.split('').map(complementBase).join('');
      case 'transcribe':
        var tseq = transcribe(a && a._type ? a.seq : seq);
        return {_type: 'RNA', seq: tseq};
      case 'translate':
        var pseq = translate(a && a._type ? a.seq : seq);
        return {_type: 'Protein', seq: pseq};
      case 'kmer_count':
        return kmerCount(seq, b || 3);
      case 'validate':
        if (a && a._type === 'DNA') return /^[ATCGN]*$/i.test(a.seq);
        if (a && a._type === 'RNA') return /^[AUCGN]*$/i.test(a.seq);
        return true;

      // Visualization
      case 'sparkline':
        return sparkline(Array.isArray(a) ? a : []);

      // Hash
      case 'base64_encode': return typeof btoa !== 'undefined' ? btoa(a || '') : '';
      case 'base64_decode': return typeof atob !== 'undefined' ? atob(a || '') : '';

      default:
        return '[' + name + ': not available in playground]';
    }
  };

  // ── Formatting ──

  BioLangEval.prototype.formatPrint = function (val) {
    // Like formatValue but strings print without quotes
    if (typeof val === 'string') return val;
    return this.formatValue(val);
  };

  BioLangEval.prototype.formatValue = function (val) {
    if (val === null || val === undefined) return 'nil';
    if (val === true) return 'true';
    if (val === false) return 'false';
    if (typeof val === 'number') {
      if (Number.isInteger(val)) return val.toString();
      // Show up to 4 decimal places, trim trailing zeros
      var s = val.toFixed(6).replace(/0+$/, '').replace(/\.$/, '.0');
      return s;
    }
    if (typeof val === 'string') return '"' + val + '"';
    if (val && val._type) {
      return val._type.toLowerCase() + '"' + val.seq + '"';
    }
    if (Array.isArray(val)) {
      return '[' + val.map(this.formatValue.bind(this)).join(', ') + ']';
    }
    if (typeof val === 'object') {
      var entries = Object.entries(val).map(function (kv) {
        return '"' + kv[0] + '": ' + this.formatValue(kv[1]);
      }.bind(this));
      return '{' + entries.join(', ') + '}';
    }
    return String(val);
  };

  // ── UI: Add Run buttons to code blocks ──

  var evaluator = new BioLangEval();

  function addPlaygroundButtons() {
    document.querySelectorAll('pre[data-runnable]').forEach(function (pre) {
      if (pre.parentNode.querySelector('.run-btn')) return;

      // Reuse existing wrapper from copy-code.js, or create one
      var wrapper = pre.parentNode;
      if (!wrapper.style.position || wrapper.style.position !== 'relative') {
        wrapper = document.createElement('div');
        wrapper.style.cssText = 'position:relative';
        pre.parentNode.insertBefore(wrapper, pre);
        wrapper.appendChild(pre);
      }

      // Run button — on the wrapper, not inside scrollable pre
      var btn = document.createElement('button');
      btn.className = 'run-btn';
      btn.style.cssText = 'position:absolute;top:8px;right:52px;padding:4px 10px;font-size:12px;border-radius:4px;background:rgba(124,58,237,0.8);color:#fff;border:none;cursor:pointer;opacity:0;transition:opacity 0.2s;z-index:10';
      btn.innerHTML = '&#9654; Run';
      wrapper.addEventListener('mouseenter', function(){btn.style.opacity='1';});
      wrapper.addEventListener('mouseleave', function(){btn.style.opacity='0';});
      wrapper.appendChild(btn);

      // Output panel
      var outputEl = document.createElement('div');
      outputEl.style.cssText = 'display:none;border:1px solid #334155;border-top:none;border-radius:0 0 8px 8px;overflow:hidden';
      outputEl.innerHTML = '<div style="padding:6px 12px;background:rgba(30,41,59,0.5);border-bottom:1px solid #334155;display:flex;align-items:center;justify-content:space-between"><span style="font-size:12px;color:#64748b;font-family:monospace">Result</span><button class="playground-close" style="font-size:14px;color:#64748b;background:none;border:none;cursor:pointer">&times;</button></div><pre style="border:0;border-radius:0;margin:0;background:rgba(15,23,42,0.8)"><code class="playground-result" style="font-size:14px;color:#4ade80"></code></pre>';
      wrapper.appendChild(outputEl);

      btn.addEventListener('click', function () {
        var code = pre.querySelector('code');
        var src = code ? code.textContent : pre.textContent;
        try {
          var result = evaluator.eval(src);
          var resultEl = outputEl.querySelector('.playground-result');
          resultEl.textContent = result || '(no output)';
          resultEl.style.color = '#4ade80';
        } catch (e) {
          var resultEl = outputEl.querySelector('.playground-result');
          resultEl.textContent = 'Error: ' + e.message;
          resultEl.style.color = '#f87171';
        }
        outputEl.style.display = 'block';
        pre.style.borderRadius = '8px 8px 0 0';
      });

      outputEl.querySelector('.playground-close').addEventListener('click', function () {
        outputEl.style.display = 'none';
        pre.style.borderRadius = '';
      });
    });
  }

  // Auto-marking disabled — only pages with explicit `data-runnable` attributes
  // on <pre> blocks will get Run buttons. This prevents false-positive mock Run
  // buttons on example/tutorial pages that use features not available in the
  // browser-side playground evaluator.
  function autoMarkRunnable() {
    // no-op: rely on explicit data-runnable attributes only
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
    var observer = new MutationObserver(function (mutations) {
      if (document.body.classList.contains('components-loaded')) {
        observer.disconnect();
        setTimeout(initPlayground, 200);
      }
    });
    observer.observe(document.body, { attributes: true, attributeFilter: ['class'] });
    // Fallback
    setTimeout(initPlayground, 3000);
  }
})();
