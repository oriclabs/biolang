// BioLang Docs — Component loader, dark mode, mobile menu, sidebar, search

(function () {
  'use strict';

  // ── Component Loader (parallel fetch) ──
  async function loadComponents() {
    var slots = document.querySelectorAll('[data-component]');
    // Fetch all components in parallel
    var promises = [];
    for (var i = 0; i < slots.length; i++) {
      promises.push(fetchComponent(slots[i]));
    }
    await Promise.all(promises);

    // All components injected — reveal the page
    document.body.classList.add('components-loaded');

    highlightActiveNav();
    initMobileMenu();
    initSidebar();
    initDarkMode();
    initSearch();
    initHighlighting();
  }

  async function fetchComponent(slot) {
    var name = slot.dataset.component;
    var base = slot.dataset.basePath || '.';
    var url = base + '/components/' + name + '.html';
    try {
      var resp = await fetch(url);
      if (!resp.ok) {
        console.error('[BioLang] Component fetch failed:', url, resp.status);
        return;
      }
      var html = await resp.text();
      html = html.replace(/\{BASE\}/g, base);
      slot.innerHTML = html;
    } catch (err) {
      console.error('[BioLang] Component error:', url, err);
    }
  }

  function initHighlighting() {
    if (typeof hljs !== 'undefined') {
      if (typeof window.registerBioLang === 'function' && !hljs.getLanguage('biolang')) {
        window.registerBioLang(hljs);
      }
      hljs.highlightAll();
    } else {
      var tries = 0;
      var wait = setInterval(function () {
        if (typeof hljs !== 'undefined') {
          if (typeof window.registerBioLang === 'function' && !hljs.getLanguage('biolang')) {
            window.registerBioLang(hljs);
          }
          hljs.highlightAll();
          clearInterval(wait);
        }
        if (++tries > 20) clearInterval(wait);
      }, 100);
    }
  }

  // ── Active Nav Highlight ──
  function highlightActiveNav() {
    // Find the element with data-active (could be on a data-component div)
    var activeEl = document.querySelector('[data-active]');
    if (!activeEl) return;
    var key = activeEl.dataset.active;
    document.querySelectorAll('#sidebar a[data-nav]').forEach(function (a) {
      if (a.dataset.nav === key) {
        a.classList.add('text-violet-400', 'font-semibold', 'border-l-2', 'border-violet-400');
        a.classList.remove('text-slate-400');
        var section = a.closest('[data-section]');
        if (section) section.open = true;
      }
    });
    document.querySelectorAll('#topnav a[data-nav]').forEach(function (a) {
      if (a.dataset.nav === key || key.startsWith(a.dataset.nav + '/')) {
        a.classList.add('text-violet-400');
        a.classList.remove('text-slate-400');
      }
    });
  }

  // ── Dark Mode ──
  function initDarkMode() {
    var toggle = document.getElementById('dark-toggle');
    if (!toggle) return;
    var html = document.documentElement;
    if (localStorage.getItem('theme') === 'light') {
      html.classList.remove('dark');
    } else {
      html.classList.add('dark');
    }
    toggle.addEventListener('click', function () {
      html.classList.toggle('dark');
      localStorage.setItem('theme', html.classList.contains('dark') ? 'dark' : 'light');
      updateToggleIcon();
    });
    updateToggleIcon();

    function updateToggleIcon() {
      var isDark = html.classList.contains('dark');
      toggle.innerHTML = isDark
        ? '<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z"/></svg>'
        : '<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z"/></svg>';
      var meta = document.querySelector('meta[name="theme-color"]');
      if (meta) meta.content = isDark ? '#0f172a' : '#ffffff';
      // Swap highlight.js theme
      var hljsLink = document.getElementById('hljs-theme');
      if (hljsLink) {
        hljsLink.href = isDark
          ? 'https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github-dark.min.css'
          : 'https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github.min.css';
      }
    }
  }

  // ── Mobile Menu ──
  function initMobileMenu() {
    var btn = document.getElementById('mobile-menu-btn');
    var sidebar = document.getElementById('sidebar');
    var overlay = document.getElementById('sidebar-overlay');
    var dropdown = document.getElementById('mobile-nav-dropdown');
    if (!btn) return;

    btn.addEventListener('click', function () {
      if (sidebar) {
        // Docs page with sidebar — toggle the sidebar
        sidebar.classList.toggle('-translate-x-full');
        if (overlay) overlay.classList.toggle('hidden');
      } else if (dropdown) {
        // Standalone page (no sidebar) — toggle the dropdown nav
        dropdown.classList.toggle('hidden');
      }
    });

    if (overlay) {
      overlay.addEventListener('click', function () {
        if (sidebar) sidebar.classList.add('-translate-x-full');
        overlay.classList.add('hidden');
      });
    }

    // Close dropdown when clicking outside
    if (dropdown) {
      document.addEventListener('click', function (e) {
        if (!btn.contains(e.target) && !dropdown.contains(e.target)) {
          dropdown.classList.add('hidden');
        }
      });
    }
  }

  // ── Tools Dropdown ──
  // ── Sidebar Collapse ──
  function initSidebar() {
    document.querySelectorAll('#sidebar details[data-section]').forEach(function (det) {
      var chevron = det.querySelector('summary svg');
      if (!chevron) return;
      function update() {
        chevron.style.transform = det.open ? 'rotate(180deg)' : '';
      }
      det.addEventListener('toggle', update);
      update();
    });
  }

  // ── Client-side Search ──
  function initSearch() {
    var input = document.getElementById('search-input');
    var results = document.getElementById('search-results');
    if (!input || !results) return;
    var searchData = null;

    input.addEventListener('focus', function () {
      if (!searchData) {
        searchData = [];
        document.querySelectorAll('#sidebar a[data-nav]').forEach(function (a) {
          searchData.push({ title: a.textContent.trim(), href: a.href, key: a.dataset.nav });
        });
      }
    });

    input.addEventListener('input', function () {
      var q = input.value.toLowerCase().trim();
      if (!q || !searchData) { results.classList.add('hidden'); results.innerHTML = ''; return; }
      var matches = searchData.filter(function (item) {
        return item.title.toLowerCase().includes(q) || item.key.toLowerCase().includes(q);
      }).slice(0, 8);

      if (matches.length === 0) {
        results.innerHTML = '<div class="px-4 py-3 text-slate-500 text-sm">No results</div>';
      } else {
        results.innerHTML = matches.map(function (m) {
          return '<a href="' + m.href + '" class="block px-4 py-2 text-sm text-slate-300 hover:bg-slate-700/50 hover:text-violet-400 transition-colors">' + m.title + '</a>';
        }).join('');
      }
      results.classList.remove('hidden');
    });

    document.addEventListener('click', function (e) {
      if (!input.contains(e.target) && !results.contains(e.target)) {
        results.classList.add('hidden');
      }
    });

    document.addEventListener('keydown', function (e) {
      if (e.key === '/' && document.activeElement.tagName !== 'INPUT' && document.activeElement.tagName !== 'TEXTAREA') {
        e.preventDefault();
        input.focus();
      }
    });
  }

  // ── Init ──
  // Safety timeout: reveal page even if component loading stalls
  var revealTimeout = setTimeout(function () {
    document.body.classList.add('components-loaded');
  }, 3000);

  function initAll() {
    loadComponents().finally(function () {
      clearTimeout(revealTimeout);
    });
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initAll);
  } else {
    initAll();
  }
})();
