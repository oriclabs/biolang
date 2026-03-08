// Copy button for <pre> code blocks
(function () {
  'use strict';
  function addCopyButtons() {
    document.querySelectorAll('pre').forEach(function (pre) {
      if (pre.querySelector('.copy-btn')) return;

      // Wrap pre in a container for proper button positioning
      var wrapper = document.createElement('div');
      wrapper.style.cssText = 'position:relative';
      pre.parentNode.insertBefore(wrapper, pre);
      wrapper.appendChild(pre);

      var btn = document.createElement('button');
      btn.className = 'copy-btn';
      btn.style.cssText = 'position:absolute;top:8px;right:8px;padding:4px 8px;font-size:12px;border-radius:4px;background:rgba(51,65,85,0.8);color:#94a3b8;border:none;cursor:pointer;opacity:0;transition:opacity 0.2s;z-index:10';
      btn.textContent = 'Copy';
      btn.addEventListener('click', function () {
        var code = pre.querySelector('code');
        var text = code ? code.textContent : pre.textContent;
        navigator.clipboard.writeText(text).then(function () {
          btn.textContent = 'Copied!';
          btn.style.color = '#a78bfa';
          setTimeout(function () { btn.textContent = 'Copy'; btn.style.color = '#94a3b8'; }, 2000);
        });
      });
      wrapper.addEventListener('mouseenter', function () { btn.style.opacity = '1'; });
      wrapper.addEventListener('mouseleave', function () { btn.style.opacity = '0'; });
      wrapper.appendChild(btn);
    });
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', function () { setTimeout(addCopyButtons, 500); });
  } else {
    setTimeout(addCopyButtons, 500);
  }
})();
