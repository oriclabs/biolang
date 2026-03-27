// Font size controls (A- A A+)
(function() {
  var SIZES = [11, 12, 13, 14, 15, 16, 18];
  var DEFAULT = 14;

  function getSize() {
    return parseInt(localStorage.getItem('bk_fontSize')) || DEFAULT;
  }
  function setSize(s) {
    localStorage.setItem('bk_fontSize', s);
    document.body.style.fontSize = s + 'px';
    var label = document.getElementById('font-size-label');
    if (label) label.textContent = s + 'px';
  }
  setSize(getSize());

  document.getElementById('font-decrease').addEventListener('click', function() {
    var cur = getSize(), idx = SIZES.indexOf(cur);
    setSize(idx > 0 ? SIZES[idx - 1] : SIZES[0]);
  });
  document.getElementById('font-increase').addEventListener('click', function() {
    var cur = getSize(), idx = SIZES.indexOf(cur);
    setSize(idx < SIZES.length - 1 ? SIZES[idx + 1] : SIZES[SIZES.length - 1]);
  });
  document.getElementById('font-reset').addEventListener('click', function() {
    setSize(DEFAULT);
  });
})();
