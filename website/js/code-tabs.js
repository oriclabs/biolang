// Language tabs for code blocks.
//
// A block that has a verified Python and R equivalent is rendered as three
// panes; this builds the tab strip and switches between them. Progressive
// enhancement: without JavaScript all three panes show stacked and labelled,
// which is worse looking but loses nothing.
//
// Run buttons come from playground.js, which only targets code.language-biolang,
// so the Python and R panes get none without any work here — that is the
// intended behaviour, not an oversight. Copy buttons come from copy-code.js,
// which targets every <pre>, so all three panes get one.
//
// Hidden panes stay in the DOM rather than being removed, so they are copyable
// the moment a tab is selected and remain visible to crawlers. Note that
// display:none content is *not* reachable by the browser's find-in-page — that
// is a real cost of tabs over stacked blocks, and the reason this is applied
// only where a verified equivalent exists rather than site-wide.
(function () {
  'use strict';

  function buildTabs(container) {
    var panes = Array.prototype.slice.call(
      container.querySelectorAll('.code-tab-pane')
    );
    if (panes.length < 2) return;

    var strip = document.createElement('div');
    strip.className = 'code-tab-strip';
    strip.setAttribute('role', 'tablist');

    var buttons = [];

    function select(index) {
      panes.forEach(function (pane, i) {
        var active = i === index;
        pane.hidden = !active;
        buttons[i].setAttribute('aria-selected', active ? 'true' : 'false');
        buttons[i].tabIndex = active ? 0 : -1;
        buttons[i].className = 'code-tab' + (active ? ' code-tab-active' : '');
      });
    }

    panes.forEach(function (pane, i) {
      var lang = pane.getAttribute('data-lang') || ('Tab ' + (i + 1));
      var btn = document.createElement('button');
      btn.type = 'button';
      btn.textContent = lang;
      btn.setAttribute('role', 'tab');
      btn.addEventListener('click', function () { select(i); });
      // Left/right arrows move between tabs, which is what a screen reader
      // user expects from role=tablist.
      btn.addEventListener('keydown', function (event) {
        if (event.key !== 'ArrowRight' && event.key !== 'ArrowLeft') return;
        event.preventDefault();
        var step = event.key === 'ArrowRight' ? 1 : -1;
        var next = (i + step + panes.length) % panes.length;
        select(next);
        buttons[next].focus();
      });
      buttons.push(btn);
      strip.appendChild(btn);
    });

    // A label for the pane that is not just a language name: the claim these
    // tabs make is that the three agree, so say where that was checked.
    var note = container.getAttribute('data-verified-note');
    if (note) {
      var badge = document.createElement('span');
      badge.className = 'code-tab-note';
      badge.textContent = note;
      var href = container.getAttribute('data-verified-href');
      if (href) {
        var link = document.createElement('a');
        link.href = href;
        link.textContent = note;
        link.className = 'code-tab-note';
        badge = link;
      }
      strip.appendChild(badge);
    }

    container.insertBefore(strip, container.firstChild);
    select(0);
  }

  function init() {
    document.querySelectorAll('.code-tabs').forEach(buildTabs);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
