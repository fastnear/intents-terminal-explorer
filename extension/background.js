// Background service worker for native messaging with Tauri app
console.log('[Zcash Extension] Background service worker started');

const NATIVE_HOST_NAME = 'com.zypherpunk.zcashsigner';
let nativePort = null;
const pendingRequests = new Map();

// Handle messages from content scripts
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  console.log('[Zcash Extension] Background received message:', request);

  if (request.type === 'ZCASH_ACTION') {
    handleZcashAction(request.payload, sender.tab?.id);
    sendResponse({ received: true });
    return true; // Keep channel open for async response
  }
});

// Handle Zcash payment action by forwarding to native app
function handleZcashAction(payload, tabId) {
  console.log('[Zcash Extension] Handling Zcash action:', payload);

  const sessionId = payload.session;

  try {
    // Connect to native messaging host (Tauri app)
    if (!nativePort) {
      console.log('[Zcash Extension] Connecting to native host:', NATIVE_HOST_NAME);
      nativePort = chrome.runtime.connectNative(NATIVE_HOST_NAME);

      // Set up message handler
      nativePort.onMessage.addListener((response) => {
        console.log('[Zcash Extension] Received from native app:', response);
        handleNativeResponse(response);
      });

      // Handle disconnect
      nativePort.onDisconnect.addListener(() => {
        console.log('[Zcash Extension] Native port disconnected');
        if (chrome.runtime.lastError) {
          console.error('[Zcash Extension] Native messaging error:', chrome.runtime.lastError);
        }
        nativePort = null;

        // Notify all pending requests of failure
        for (const [sid, data] of pendingRequests.entries()) {
          notifyTab(data.tabId, {
            type: 'ZCASH_RESULT',
            status: 'error',
            error: 'Native app disconnected',
            session: sid
          });
        }
        pendingRequests.clear();
      });
    }

    // Store pending request
    pendingRequests.set(sessionId, { tabId, timestamp: Date.now() });

    // Send request to native app
    const message = {
      action: 'signTransaction',
      params: {
        to: payload.to,
        amount: payload.amount,
        memo: payload.memo || ''
      },
      session: sessionId
    };

    console.log('[Zcash Extension] Sending to native app:', message);
    nativePort.postMessage(message);

  } catch (err) {
    console.error('[Zcash Extension] Error handling Zcash action:', err);
    notifyTab(tabId, {
      type: 'ZCASH_RESULT',
      status: 'error',
      error: err.message,
      session: sessionId
    });
  }
}

// Handle response from native app
function handleNativeResponse(response) {
  const { status, txid, session, error } = response;

  // Find pending request
  const pending = pendingRequests.get(session);
  if (!pending) {
    console.warn('[Zcash Extension] No pending request for session:', session);
    return;
  }

  // Remove from pending
  pendingRequests.delete(session);

  // Notify content script on originating tab
  notifyTab(pending.tabId, {
    type: 'ZCASH_RESULT',
    status,
    txid,
    session,
    error
  });
}

// Send message to content script on specific tab
function notifyTab(tabId, message) {
  if (!tabId || tabId < 0) {
    console.warn('[Zcash Extension] Invalid tab ID:', tabId);
    return;
  }

  chrome.tabs.sendMessage(tabId, message, (response) => {
    if (chrome.runtime.lastError) {
      console.error('[Zcash Extension] Error sending to tab:', chrome.runtime.lastError);
    }
  });
}

// Intercept deep link callback URLs from native app
chrome.webRequest.onBeforeRequest.addListener(
  (details) => {
    console.log('[Zcash Extension] Intercepted request:', details.url);

    try {
      const url = new URL(details.url);

      // Check if this is our callback URL
      if (url.hostname === 'return.zwallet' && url.pathname === '/txResult') {
        const status = url.searchParams.get('status');
        const txid = url.searchParams.get('txid');
        const session = url.searchParams.get('session');

        console.log('[Zcash Extension] Deep link callback:', { status, txid, session });

        // Find pending request
        const pending = pendingRequests.get(session);
        if (pending) {
          pendingRequests.delete(session);

          // Notify content script
          notifyTab(pending.tabId, {
            type: 'ZCASH_RESULT',
            status,
            txid,
            session
          });
        }

        // Block navigation to this URL
        return { cancel: true };
      }
    } catch (err) {
      console.error('[Zcash Extension] Error processing deep link:', err);
    }
  },
  { urls: ["https://return.zwallet/*"] },
  ["blocking"]
);

// Clean up old pending requests (> 5 minutes)
setInterval(() => {
  const now = Date.now();
  for (const [sessionId, data] of pendingRequests.entries()) {
    if (now - data.timestamp > 5 * 60 * 1000) {
      console.log('[Zcash Extension] Cleaning up stale request:', sessionId);
      pendingRequests.delete(sessionId);
    }
  }
}, 60000); // Run every minute

console.log('[Zcash Extension] Background service worker ready');
