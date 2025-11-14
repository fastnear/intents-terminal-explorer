// Content script for intercepting Zcash payment actions on webpages
console.log('[Zcash Extension] Content script loaded');

// Track pending transactions to correlate responses
const pendingTransactions = new Map();

// Intercept Zcash payment links and buttons
document.addEventListener('click', (e) => {
  const target = e.target;

  // Check for zcash: protocol links
  if (target.tagName === 'A' && target.getAttribute('href')?.startsWith('zcash:')) {
    e.preventDefault();
    e.stopPropagation();

    const paymentURL = target.getAttribute('href');
    console.log('[Zcash Extension] Intercepted zcash: link:', paymentURL);

    // Parse zcash: URL format: zcash:ADDRESS?amount=X&memo=Y
    const parsed = parseZcashURL(paymentURL);
    if (parsed) {
      handleZcashPayment(parsed);
    }
    return false;
  }

  // Check for elements with data-zcash-action attribute
  if (target.hasAttribute('data-zcash-action')) {
    e.preventDefault();
    e.stopPropagation();

    const action = target.getAttribute('data-zcash-action');
    console.log('[Zcash Extension] Intercepted Zcash action:', action);

    // Get payment details from data attributes
    const payload = {
      to: target.getAttribute('data-zcash-to'),
      amount: parseFloat(target.getAttribute('data-zcash-amount') || '0'),
      memo: target.getAttribute('data-zcash-memo') || ''
    };

    if (payload.to && payload.amount > 0) {
      handleZcashPayment(payload);
    } else {
      console.error('[Zcash Extension] Invalid payment data:', payload);
      showNotification('Invalid Zcash payment data', 'error');
    }
    return false;
  }
}, true); // Use capture phase to intercept before page handlers

// Parse zcash: URL format
function parseZcashURL(url) {
  try {
    // Format: zcash:ADDRESS?amount=X&memo=Y
    const match = url.match(/^zcash:([^?]+)(\?(.*))?$/);
    if (!match) return null;

    const address = match[1];
    const queryString = match[3] || '';
    const params = new URLSearchParams(queryString);

    return {
      to: address,
      amount: parseFloat(params.get('amount') || '0'),
      memo: params.get('memo') || ''
    };
  } catch (err) {
    console.error('[Zcash Extension] Error parsing zcash: URL:', err);
    return null;
  }
}

// Handle Zcash payment by sending to native app via background script
function handleZcashPayment(payload) {
  console.log('[Zcash Extension] Initiating payment:', payload);

  // Generate session ID to track this request
  const sessionId = generateSessionId();

  // Show loading indicator
  showNotification('Requesting approval from Zcash wallet...', 'info');

  // Send message to background script
  chrome.runtime.sendMessage({
    type: 'ZCASH_ACTION',
    payload: {
      ...payload,
      session: sessionId
    }
  }, (response) => {
    if (chrome.runtime.lastError) {
      console.error('[Zcash Extension] Error sending message:', chrome.runtime.lastError);
      showNotification('Failed to communicate with wallet extension', 'error');
    }
  });

  // Store pending transaction
  pendingTransactions.set(sessionId, {
    payload,
    timestamp: Date.now()
  });
}

// Listen for results from background script
chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  console.log('[Zcash Extension] Received message:', msg);

  if (msg.type === 'ZCASH_RESULT') {
    const { status, txid, session } = msg;

    // Clean up pending transaction
    if (session) {
      pendingTransactions.delete(session);
    }

    if (status === 'approved') {
      console.log('[Zcash Extension] Transaction approved:', txid);
      showNotification(`✅ Zcash transaction approved!\nTx ID: ${txid}`, 'success');

      // Dispatch custom event for webpage to listen to
      document.dispatchEvent(new CustomEvent('zcash-transaction-approved', {
        detail: { txid, session }
      }));
    } else if (status === 'denied') {
      console.log('[Zcash Extension] Transaction denied');
      showNotification('❌ Zcash transaction was denied by user', 'error');

      document.dispatchEvent(new CustomEvent('zcash-transaction-denied', {
        detail: { session }
      }));
    } else {
      console.error('[Zcash Extension] Unknown status:', status);
      showNotification('⚠️ Unknown transaction status', 'error');
    }
  }
});

// Show notification to user (inject styled toast)
function showNotification(message, type = 'info') {
  // Remove existing notification if any
  const existing = document.getElementById('zcash-extension-notification');
  if (existing) {
    existing.remove();
  }

  // Create notification element
  const notification = document.createElement('div');
  notification.id = 'zcash-extension-notification';
  notification.style.cssText = `
    position: fixed;
    top: 20px;
    right: 20px;
    padding: 16px 24px;
    background: ${type === 'success' ? '#10b981' : type === 'error' ? '#ef4444' : '#3b82f6'};
    color: white;
    border-radius: 8px;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    z-index: 999999;
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 14px;
    max-width: 400px;
    word-wrap: break-word;
    animation: slideIn 0.3s ease-out;
  `;

  notification.textContent = message;

  // Add animation keyframes
  if (!document.getElementById('zcash-extension-styles')) {
    const style = document.createElement('style');
    style.id = 'zcash-extension-styles';
    style.textContent = `
      @keyframes slideIn {
        from {
          transform: translateX(400px);
          opacity: 0;
        }
        to {
          transform: translateX(0);
          opacity: 1;
        }
      }
      @keyframes slideOut {
        from {
          transform: translateX(0);
          opacity: 1;
        }
        to {
          transform: translateX(400px);
          opacity: 0;
        }
      }
    `;
    document.head.appendChild(style);
  }

  document.body.appendChild(notification);

  // Auto-remove after 5 seconds
  setTimeout(() => {
    notification.style.animation = 'slideOut 0.3s ease-out';
    setTimeout(() => notification.remove(), 300);
  }, 5000);
}

// Generate unique session ID
function generateSessionId() {
  return `${Date.now()}-${Math.random().toString(36).substring(2, 15)}`;
}

// Clean up old pending transactions (> 5 minutes)
setInterval(() => {
  const now = Date.now();
  for (const [sessionId, data] of pendingTransactions.entries()) {
    if (now - data.timestamp > 5 * 60 * 1000) {
      pendingTransactions.delete(sessionId);
    }
  }
}, 60000); // Run every minute
