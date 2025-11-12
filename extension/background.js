chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (msg && msg.type === "COPY_TEXT" && typeof msg.text === "string") {
    (async () => {
      try {
        await navigator.clipboard.writeText(msg.text);
        sendResponse({ ok: true });
      } catch {
        sendResponse({ ok: false });
      }
    })();
    return true; // keep channel open for async
  }
});