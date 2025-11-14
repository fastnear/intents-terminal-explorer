# Zcash Native Bridge Extension

Privacy-first Chrome/Brave extension that enables secure Zcash transactions through native app integration.

## Features

- 🔒 **Privacy-First**: Private keys never leave the native app
- 🔐 **Biometric Security**: Touch ID/PIN confirmation for all transactions
- 🌐 **Seamless Integration**: Intercepts `zcash:` payment links on any webpage
- 🔄 **Native Messaging**: Secure STDIN/STDOUT communication with Tauri app
- ✨ **User-Friendly**: Toast notifications and visual feedback

## Architecture

```
Webpage
   ↓ (user clicks zcash: link)
Content Script (intercepts)
   ↓
Background Service Worker
   ↓ (native messaging)
Tauri App
   ↓ (Touch ID/PIN prompt)
User Approval
   ↓ (sign transaction)
Response via Native Messaging + Deep Link
   ↓
Content Script (show success/error)
   ↓
Webpage (updated)
```

## Installation

### 1. Load the Extension

1. Open Chrome/Brave and navigate to `chrome://extensions/`
2. Enable "Developer mode" (toggle in top-right)
3. Click "Load unpacked"
4. Select the `extension/` directory from this project
5. **Note the Extension ID** (you'll need it for native messaging setup)

### 2. Set Up Native Messaging

See the `native-messaging/README.md` file for detailed instructions on:
- Building and installing the Tauri app
- Configuring the native messaging host manifest
- Installing the manifest in the correct Chrome directory

### 3. Create Extension Icons (Optional)

For a complete installation, create placeholder icons:

```bash
mkdir -p extension/icons
# Create placeholder icons (or use your own)
# You can use any PNG images, or leave them missing for testing
```

## Usage

### For Users

1. Visit any webpage with a Zcash payment link (format: `zcash:ADDRESS?amount=X&memo=Y`)
2. Click the payment link
3. Extension intercepts the action and prompts for approval via native app
4. Approve with Touch ID or PIN in the native app dialog
5. Receive confirmation toast notification on the webpage

### For Developers (Testing)

Create a test HTML page:

```html
<!DOCTYPE html>
<html>
<head>
  <title>Zcash Payment Test</title>
</head>
<body>
  <h1>Test Zcash Payments</h1>
  
  <!-- Method 1: zcash: protocol link -->
  <a href="zcash:zs1qq402u5t8e0hp4hjskd0nhrw92hes9u0dmu36r4s5upk3cazys?amount=1.5&memo=Coffee">
    Pay 1.5 ZEC with zcash: link
  </a>
  
  <br><br>
  
  <!-- Method 2: Custom data attributes -->
  <button 
    data-zcash-action="pay"
    data-zcash-to="zs1qq402u5t8e0hp4hjskd0nhrw92hes9u0dmu36r4s5upk3cazys"
    data-zcash-amount="2.0"
    data-zcash-memo="Donation">
    Pay 2.0 ZEC with data attributes
  </button>

  <script>
    // Listen for transaction results
    document.addEventListener('zcash-transaction-approved', (e) => {
      console.log('Transaction approved:', e.detail);
      alert(`Success! TX ID: ${e.detail.txid}`);
    });

    document.addEventListener('zcash-transaction-denied', (e) => {
      console.log('Transaction denied:', e.detail);
      alert('Transaction was denied');
    });
  </script>
</body>
</html>
```

## Message Flow

### Extension → Native App (Request)

```json
{
  "action": "signTransaction",
  "params": {
    "to": "zs1qq402u...",
    "amount": 1.5,
    "memo": "Coffee"
  },
  "session": "1699564800000-abc123"
}
```

### Native App → Extension (Response)

```json
{
  "status": "approved",
  "txid": "f3e1b2...abcd",
  "session": "1699564800000-abc123"
}
```

## Debugging

### Check Extension Console

1. Go to `chrome://extensions/`
2. Find "Zcash Native Bridge Extension"
3. Click "Details"
4. Click "Inspect views: service worker"
5. Look for console logs with `[Zcash Extension]` prefix

### Check Content Script Console

1. Open any webpage
2. Open DevTools (F12)
3. Go to Console tab
4. Look for logs with `[Zcash Extension]` prefix

### Common Issues

**"Specified native messaging host not found"**
- Verify native messaging manifest is installed correctly
- Check `path` in manifest points to correct binary location
- See `native-messaging/README.md` for installation steps

**No response from native app**
- Check if Tauri app is built and installed in `/Applications/`
- Test the app manually: `echo '{"action":"ping"}' | /Applications/ZcashSigner.app/Contents/MacOS/nearx-tauri`
- Check extension ID matches in native messaging manifest

**Clicks not being intercepted**
- Check contentScript.js is loaded (look for console log on page load)
- Verify the link/button format matches the expected patterns
- Try reloading the extension

## Security

- ✅ Private keys stored only in native app (never in browser/extension)
- ✅ All transactions require biometric/PIN confirmation
- ✅ Extension has minimal permissions (only nativeMessaging, webRequest, tabs)
- ✅ Native messaging channel is restricted to specific extension ID
- ✅ Deep link callbacks use custom protocol to prevent phishing

## Development

The extension uses Manifest V3 with:
- **Service Worker background** (background.js)
- **Content Script** (contentScript.js) - injected into all pages
- **Native Messaging** - communicates with Tauri app via STDIN/STDOUT
- **Web Request API** - intercepts deep link callbacks

## License

See main project license.
