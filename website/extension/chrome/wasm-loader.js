// WASM loader for Chrome extension context (CSP blocks inline module scripts)
(async function() {
  try {
    var mod = await import("./wasm/br_wasm.js");
    await mod.default();
    mod.init();
    window.__blWasm = { evaluate: mod.evaluate, reset: mod.reset };
    window.dispatchEvent(new Event("bl-wasm-ready"));
  } catch(e) {
    window.__blWasmError = e;
    window.dispatchEvent(new Event("bl-wasm-error"));
  }
})();
