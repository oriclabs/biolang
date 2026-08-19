// BioLang live notebook runtime. Exported notebooks embed this file, so the
// only downloaded artifacts are bl_wasm.js and bl_wasm_bg.wasm.
(function () {
  'use strict';

  var wasm = null;
  var loading = null;
  var executionCount = 0;
  // Cells [0, validThrough) have been evaluated in source order in the
  // current kernel. This lets a learner open a notebook and run any later
  // cell without first discovering its setup variables by trial and error.
  var validThrough = 0;
  var kernelStale = false;
  var cells = Array.prototype.slice.call(document.querySelectorAll('.bl-notebook-cell'));
  var runAll = document.getElementById('bl-run-all');
  var kernelStatus = document.getElementById('bl-kernel-status');

  if (!cells.length) return;

  function wasmBase() {
    var meta = document.querySelector('meta[name="bl-wasm-base"]');
    return (meta && meta.content ? meta.content : '/wasm').replace(/\/+$/, '');
  }

  function loadWasm() {
    if (wasm) return Promise.resolve(wasm);
    if (loading) return loading;
    kernelStatus.textContent = 'Loading browser kernel…';
    loading = import(wasmBase() + '/bl_wasm.js')
      .then(function (module) {
        return Promise.resolve(module.default()).then(function () { return module; });
      })
      .then(function (module) {
        module.init();
        runBootstrap(module);
        wasm = module;
        kernelStatus.textContent = 'Browser kernel · shared session';
        return wasm;
      })
      .catch(function (error) {
        loading = null;
        kernelStatus.textContent = 'Browser kernel unavailable';
        throw error;
      });
    return loading;
  }

  function runBootstrap(module) {
    document.querySelectorAll('.bl-notebook-bootstrap textarea').forEach(function (editor) {
      var result = JSON.parse(module.evaluate(editor.value));
      if (!result.ok) throw new Error('Notebook setup failed: ' + (result.error || 'unknown error'));
    });
  }

  function freshKernel() {
    return loadWasm().then(function (module) {
      module.reset();
      runBootstrap(module);
      validThrough = 0;
      kernelStale = false;
      return module;
    });
  }

  function sourceOf(cell) {
    var editor = cell.querySelector('.bl-cell-editor');
    return editor ? editor.value : '';
  }

  function autoSize(editor) {
    editor.style.height = 'auto';
    editor.style.height = Math.max(76, editor.scrollHeight + 2) + 'px';
  }

  function setBusy(cell, busy, label) {
    var button = cell.querySelector('.bl-cell-run');
    button.disabled = busy;
    button.textContent = busy ? (label || 'Running…') : 'Run';
    cell.classList.toggle('is-running', busy);
  }

  function outputPanel(cell) {
    var panel = cell.querySelector('.bl-live-output');
    panel.hidden = false;
    panel.replaceChildren();
    var saved = cell.querySelector('.bl-saved-output');
    if (saved) saved.hidden = true;
    return panel;
  }

  function appendText(panel, text, className) {
    if (!text || !text.trim()) return;
    var pre = document.createElement('pre');
    pre.className = className || 'bl-output-text';
    pre.textContent = text;
    panel.appendChild(pre);
  }

  function sanitizedSvg(markup) {
    var parsed = new DOMParser().parseFromString(markup, 'image/svg+xml');
    var root = parsed.documentElement;
    if (!root || root.localName !== 'svg' || parsed.querySelector('parsererror')) return null;
    root.querySelectorAll('script,foreignObject,iframe,object,embed,link,style').forEach(function (node) { node.remove(); });
    [root].concat(Array.prototype.slice.call(root.querySelectorAll('*'))).forEach(function (node) {
      Array.prototype.slice.call(node.attributes || []).forEach(function (attribute) {
        var name = attribute.name.toLowerCase();
        var value = attribute.value.trim().toLowerCase();
        if (name.indexOf('on') === 0 ||
            ((name === 'href' || name === 'xlink:href') && /^(?:javascript:|data:|https?:|\/\/)/.test(value)) ||
            (name === 'style' && /(?:javascript:|expression\s*\(|url\s*\()/i.test(value))) {
          node.removeAttribute(attribute.name);
        }
      });
    });
    return document.importNode(root, true);
  }

  function appendSvg(panel, markup) {
    var svg = sanitizedSvg(markup);
    if (!svg) {
      appendText(panel, markup, 'bl-output-error');
      return;
    }
    var figure = document.createElement('figure');
    figure.className = 'cell-figure';
    figure.appendChild(svg);
    panel.appendChild(figure);
    if (window.__blEnhanceFigures) window.__blEnhanceFigures(figure);
  }

  function appendMixed(panel, value, className) {
    if (!value) return;
    var expression = /<svg\b[\s\S]*?<\/svg>/gi;
    var cursor = 0;
    var match;
    while ((match = expression.exec(value))) {
      appendText(panel, value.slice(cursor, match.index), className);
      appendSvg(panel, match[0]);
      cursor = match.index + match[0].length;
    }
    appendText(panel, value.slice(cursor), className);
  }

  function unquote(value, type) {
    if (type === 'Str' && value && value.charAt(0) === '"' && value.charAt(value.length - 1) === '"') {
      return value.slice(1, -1);
    }
    return value;
  }

  function renderResult(cell, result, elapsed) {
    if (cell.dataset.hideOutput === 'true') {
      var hiddenPanel = cell.querySelector('.bl-live-output');
      hiddenPanel.hidden = true;
      var hiddenTiming = cell.querySelector('.bl-cell-timing');
      hiddenTiming.textContent = elapsed < 1 ? '<1 ms' : Math.round(elapsed) + ' ms';
      return;
    }
    var panel = outputPanel(cell);
    appendMixed(panel, result.output || '', 'bl-output-text');
    if (result.ok) {
      var value = unquote(result.value, result.type);
      if (value && !/^(?:nil|null|Nil|\(\)|None)$/.test(value)) {
        if (/^\s*<svg\b/i.test(value)) appendSvg(panel, value);
        else appendText(panel, '→ ' + result.value, 'bl-output-result');
      }
    } else {
      appendText(panel, result.error || 'Unknown error', 'bl-output-error');
    }
    if (!panel.childNodes.length) appendText(panel, '(no output)', 'bl-output-empty');
    var timing = cell.querySelector('.bl-cell-timing');
    timing.textContent = elapsed < 1 ? '<1 ms' : Math.round(elapsed) + ' ms';
  }

  function renderFailure(cell, error) {
    var panel = outputPanel(cell);
    appendText(panel, 'Browser execution failed: ' + String(error && error.message || error) +
      '\n\nNative-only analyses should be run with the bl CLI.', 'bl-output-error');
  }

  function executeCell(cell, focusNext) {
    var source = sourceOf(cell);
    if (!source.trim()) return Promise.resolve(true);
    setBusy(cell, true, wasm ? 'Running…' : 'Loading…');
    return loadWasm().then(function (module) {
      setBusy(cell, true, 'Running…');
      // Let the busy state paint before synchronous WASM evaluation begins.
      return new Promise(function (resolve) {
        requestAnimationFrame(function () {
          var started = performance.now();
          try {
            var result = JSON.parse(module.evaluate(source));
            executionCount += 1;
            cell.querySelector('.bl-cell-count').textContent = 'In [' + executionCount + ']';
            renderResult(cell, result, performance.now() - started);
            resolve(!!result.ok);
          } catch (error) {
            renderFailure(cell, error);
            resolve(false);
          } finally {
            setBusy(cell, false);
            if (focusNext) {
              var next = cells[cells.indexOf(cell) + 1];
              if (next) next.querySelector('.bl-cell-editor').focus();
            }
          }
        });
      });
    }).catch(function (error) {
      renderFailure(cell, error);
      setBusy(cell, false);
      return false;
    });
  }

  function executeThrough(cell, focusNext) {
    var target = cells.indexOf(cell);
    if (target < 0) return Promise.resolve(false);

    // Re-running an earlier cell, or editing an already-run cell, starts a
    // clean kernel so removed variables cannot leak into later results.
    var requiresFresh = kernelStale || target < validThrough;
    var start = requiresFresh ? 0 : validThrough;
    var chain = requiresFresh
      ? freshKernel().then(function () { return true; })
      : Promise.resolve(true);
    for (var index = start; index <= target; index += 1) {
      (function (cellIndex) {
        chain = chain.then(function (ok) {
          if (!ok) return false;
          return executeCell(cells[cellIndex], focusNext && cellIndex === target)
            .then(function (succeeded) {
              if (succeeded) validThrough = cellIndex + 1;
              return succeeded;
            });
        });
      })(index);
    }
    return chain;
  }

  function runAllCells() {
    runAll.disabled = true;
    runAll.textContent = 'Running…';
    var chain = freshKernel().then(function () { return true; });
    cells.forEach(function (cell, index) {
      chain = chain.then(function (ok) {
        if (!ok) return false;
        return executeCell(cell, false).then(function (succeeded) {
          if (succeeded) validThrough = index + 1;
          return succeeded;
        });
      });
    });
    chain.finally(function () {
      runAll.disabled = false;
      runAll.textContent = 'Run all';
    });
  }

  cells.forEach(function (cell) {
    var editor = cell.querySelector('.bl-cell-editor');
    var button = cell.querySelector('.bl-cell-run');
    autoSize(editor);
    editor.addEventListener('input', function () {
      var index = cells.indexOf(cell);
      if (index < validThrough) kernelStale = true;
      validThrough = Math.min(validThrough, index);
      autoSize(editor);
    });
    editor.addEventListener('keydown', function (event) {
      if (event.key === 'Tab') {
        event.preventDefault();
        var start = editor.selectionStart;
        var end = editor.selectionEnd;
        editor.setRangeText('  ', start, end, 'end');
        autoSize(editor);
      } else if (event.key === 'Enter' && (event.shiftKey || event.ctrlKey || event.metaKey)) {
        event.preventDefault();
        executeThrough(cell, event.shiftKey);
      }
    });
    button.title = 'Run any required earlier cells, then this cell';
    button.addEventListener('click', function () { executeThrough(cell, false); });
  });

  runAll.addEventListener('click', runAllCells);
})();
