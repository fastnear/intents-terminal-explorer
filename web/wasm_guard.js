// Minimal guard to surface WASM load/runtime errors in-page (helps when a "blue slab" hides UI).
;(() => {
  if (window.__NEARx_WASM_GUARD__) return;
  window.__NEARx_WASM_GUARD__ = true;

  const show = (title, err) => {
    const pre = document.createElement('pre');
    pre.className = 'nx-panel';
    pre.style.position = 'fixed';
    pre.style.right = '8px';
    pre.style.bottom = '8px';
    pre.style.maxWidth = '48vw';
    pre.style.maxHeight = '42vh';
    pre.style.overflow = 'auto';
    pre.style.padding = '8px';
    pre.style.zIndex = '99999';
    pre.textContent = `[WASM ERROR] ${title}\n\n` + String(err && (err.stack || err.message || err));
    document.body.appendChild(pre);
  };

  window.addEventListener('error', (e) => show('window.onerror', e.error || e.message), { capture: true });
  window.addEventListener('unhandledrejection', (e) => show('unhandledrejection', e.reason), { capture: true });
})();
