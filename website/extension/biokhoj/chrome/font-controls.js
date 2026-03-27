// Font size controls (A- A A+) — uses CSS zoom to scale all content
(function() {
  var LEVELS = [80, 85, 90, 95, 100, 110, 120, 130];
  var DEFAULT = 100;

  function getZoom() {
    return parseInt(localStorage.getItem('bk_zoom')) || DEFAULT;
  }
  function applyZoom(z) {
    localStorage.setItem('bk_zoom', z);
    document.body.style.zoom = (z / 100).toString();
    var label = document.getElementById('font-size-label');
    if (label) label.textContent = z + '%';
  }
  applyZoom(getZoom());

  document.getElementById('font-decrease').addEventListener('click', function() {
    var cur = getZoom(), idx = LEVELS.indexOf(cur);
    applyZoom(idx > 0 ? LEVELS[idx - 1] : LEVELS[0]);
  });
  document.getElementById('font-increase').addEventListener('click', function() {
    var cur = getZoom(), idx = LEVELS.indexOf(cur);
    applyZoom(idx < LEVELS.length - 1 ? LEVELS[idx + 1] : LEVELS[LEVELS.length - 1]);
  });
  document.getElementById('font-reset').addEventListener('click', function() {
    applyZoom(DEFAULT);
  });
})();
