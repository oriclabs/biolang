// BioPeek zoom controls — uses CSS zoom to scale all content
(function() {
  var LEVELS = [80, 85, 90, 95, 100, 110, 120, 130];
  var DEFAULT = 100;
  var KEY = 'biopeek_zoom';

  function getZoom() { return parseInt(localStorage.getItem(KEY)) || DEFAULT; }
  function applyZoom(z) {
    localStorage.setItem(KEY, z);
    document.body.style.zoom = (z / 100).toString();
    var label = document.getElementById('zoom-label');
    if (label) label.textContent = z + '%';
  }
  applyZoom(getZoom());

  var out = document.getElementById('zoom-out');
  var inc = document.getElementById('zoom-in');
  var rst = document.getElementById('zoom-reset');
  if (out) out.addEventListener('click', function(e) {
    e.stopPropagation();
    var cur = getZoom(), idx = LEVELS.indexOf(cur);
    applyZoom(idx > 0 ? LEVELS[idx - 1] : LEVELS[0]);
  });
  if (inc) inc.addEventListener('click', function(e) {
    e.stopPropagation();
    var cur = getZoom(), idx = LEVELS.indexOf(cur);
    applyZoom(idx < LEVELS.length - 1 ? LEVELS[idx + 1] : LEVELS[LEVELS.length - 1]);
  });
  if (rst) rst.addEventListener('click', function(e) {
    e.stopPropagation();
    applyZoom(DEFAULT);
  });
})();
