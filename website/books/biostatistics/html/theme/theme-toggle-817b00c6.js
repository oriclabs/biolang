// mdBook already provides theme switching (Light, Rust, Coal, Navy, Ayu)
// via the paintbrush icon in the top bar. This script just ensures
// the user's OS preference is respected on first visit.

(function() {
    // Only set theme if user hasn't already chosen one
    if (!localStorage.getItem('mdbook-theme')) {
        var prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        var theme = prefersDark ? 'coal' : 'light';
        document.documentElement.className = theme;
        localStorage.setItem('mdbook-theme', theme);
    }
})();
