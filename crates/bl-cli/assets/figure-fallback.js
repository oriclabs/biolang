// BioLang notebook figures: keep SVG as the primary rendering and prepare a
// canvas copy that readers can select when an embedding platform mishandles
// inline SVG. This is dependency-free and is embedded into exported HTML.
(function () {
  'use strict';

  function dimensions(svg) {
    var viewBox = svg.viewBox && svg.viewBox.baseVal;
    var width = viewBox && viewBox.width ? viewBox.width : parseFloat(svg.getAttribute('width'));
    var height = viewBox && viewBox.height ? viewBox.height : parseFloat(svg.getAttribute('height'));
    if (!width || !height) {
      var rect = svg.getBoundingClientRect();
      width = width || rect.width || 800;
      height = height || rect.height || 450;
    }
    return { width: Math.max(1, width), height: Math.max(1, height) };
  }

  function enhanceFigure(figure) {
    if (!figure || figure.dataset.blFigureReady === 'true') return;
    var svg = figure.querySelector('svg');
    if (!svg) return;
    figure.dataset.blFigureReady = 'true';

    var controls = document.createElement('div');
    controls.className = 'cell-figure-controls';
    var toggle = document.createElement('button');
    toggle.type = 'button';
    toggle.className = 'cell-figure-toggle';
    toggle.textContent = 'Use canvas';
    toggle.disabled = true;
    toggle.title = 'Prepare a raster canvas fallback while preserving the SVG';
    controls.appendChild(toggle);
    var download = document.createElement('button');
    download.type = 'button';
    download.className = 'cell-figure-toggle cell-figure-download';
    download.textContent = 'Download PNG';
    download.disabled = true;
    download.title = 'Download the prepared canvas fallback as a PNG image';
    controls.appendChild(download);
    figure.insertBefore(controls, figure.firstChild);

    var canvas = document.createElement('canvas');
    canvas.className = 'cell-figure-canvas';
    canvas.hidden = true;
    canvas.setAttribute('role', 'img');
    canvas.setAttribute('aria-label', svg.getAttribute('aria-label') || 'BioLang plot (canvas fallback)');
    figure.appendChild(canvas);

    var size = dimensions(svg);
    var scale = Math.min(window.devicePixelRatio || 1, 2);
    // Avoid browser canvas limits on unusually large scientific figures.
    if (size.width * size.height * scale * scale > 16000000) {
      scale = Math.sqrt(16000000 / (size.width * size.height));
    }
    canvas.width = Math.max(1, Math.round(size.width * scale));
    canvas.height = Math.max(1, Math.round(size.height * scale));
    canvas.style.width = size.width + 'px';
    canvas.style.maxWidth = '100%';
    canvas.style.height = 'auto';

    var copy = svg.cloneNode(true);
    if (!copy.getAttribute('xmlns')) copy.setAttribute('xmlns', 'http://www.w3.org/2000/svg');
    if (!copy.getAttribute('width')) copy.setAttribute('width', String(size.width));
    if (!copy.getAttribute('height')) copy.setAttribute('height', String(size.height));
    var source = new XMLSerializer().serializeToString(copy);
    var blob = new Blob([source], { type: 'image/svg+xml;charset=utf-8' });
    var url = URL.createObjectURL(blob);
    var image = new Image();

    function showCanvas() {
      if (toggle.disabled) return;
      svg.hidden = true;
      canvas.hidden = false;
      toggle.textContent = 'Use SVG';
      toggle.dataset.mode = 'canvas';
    }

    function showSvg() {
      canvas.hidden = true;
      svg.hidden = false;
      toggle.textContent = 'Use canvas';
      toggle.dataset.mode = 'svg';
    }

    image.onload = function () {
      try {
        var context = canvas.getContext('2d');
        context.setTransform(scale, 0, 0, scale, 0, 0);
        context.drawImage(image, 0, 0, size.width, size.height);
        toggle.disabled = false;
        download.disabled = false;
        toggle.title = 'Switch between the original SVG and its canvas fallback';
        // Some HTML hosts retain the element but collapse inline SVG. Prefer
        // the prepared canvas automatically only in that failure case.
        var rect = svg.getBoundingClientRect();
        if (!rect.width || !rect.height) showCanvas();
      } catch (_) {
        canvas.remove();
        controls.remove();
      } finally {
        URL.revokeObjectURL(url);
      }
    };
    image.onerror = function () {
      URL.revokeObjectURL(url);
      canvas.remove();
      controls.remove();
    };
    image.src = url;

    toggle.addEventListener('click', function () {
      if (toggle.dataset.mode === 'canvas') showSvg();
      else showCanvas();
    });
    download.addEventListener('click', function () {
      try {
        var link = document.createElement('a');
        link.download = figure.dataset.filename || 'biolang-plot.png';
        link.href = canvas.toDataURL('image/png');
        link.click();
      } catch (_) {
        download.disabled = true;
        download.title = 'PNG download is unavailable in this browser';
      }
    });
  }

  function enhance(root) {
    var scope = root && root.querySelectorAll ? root : document;
    if (scope.matches && scope.matches('.cell-figure')) enhanceFigure(scope);
    scope.querySelectorAll('.cell-figure').forEach(enhanceFigure);
  }

  window.__blEnhanceFigures = enhance;
  // The local notebook server replaces cell output after the initial page
  // load. Its completion event gives those plots the same fallback as static
  // exports and plots produced by the browser/WASM runtime.
  document.addEventListener('bl:figures-updated', function (event) {
    enhance(event.detail && event.detail.root ? event.detail.root : document);
  });
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', function () { enhance(document); });
  } else {
    enhance(document);
  }
})();
