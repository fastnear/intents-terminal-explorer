const HOST = "com.ratacat.native";
let port = null;
let hostVersion = null;

function connect() {
  if (port) return;
  port = chrome.runtime.connectNative(HOST);

  // Listen for messages from native host
  port.onMessage.addListener((msg) => {
    if (msg.type === "hello") {
      hostVersion = msg.version;
      console.log(`Connected to native host v${hostVersion}`);
    } else if (msg.type === "pong") {
      console.log(`Ping response: ${msg.id}`);
    } else if (msg.type === "ok") {
      console.log(`Operation succeeded: ${msg.op}`);
    } else if (msg.type === "err") {
      console.error(`Operation failed: ${msg.op} - ${msg.message}`);
    }
  });

  port.onDisconnect.addListener(() => {
    if (chrome.runtime.lastError) {
      console.error("Native host disconnected:", chrome.runtime.lastError.message);
    }
    port = null;
    hostVersion = null;
  });
}

function sendNative(msg) {
  connect();
  // Send handshake on first connection
  if (hostVersion === null) {
    port.postMessage({ type: "hello", requested_version: 1 });
  }
  port.postMessage(msg);
}

chrome.runtime.onMessage.addListener((req, _sender, sendResponse) => {
  if (req.type === "open_deeplink") {
    sendNative({ type: "open_deep_link", url: req.url });
    sendResponse({ ok: true });
  } else if (req.type === "open_session") {
    sendNative({ type: "open_session", id: req.id, read_only: !!req.readOnly });
    sendResponse({ ok: true });
  } else if (req.type === "ping") {
    sendNative({ type: "ping", id: req.id || "test-ping" });
    sendResponse({ ok: true });
  } else {
    sendResponse({ ok: false, error: "unknown request" });
  }
  return true;
});
