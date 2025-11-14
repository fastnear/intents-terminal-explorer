# Zypherpunk Zcash Integration - Privacy-First Transaction System

## 🎯 Project Overview

This branch implements a **privacy-first Zcash transaction system** using a Chrome extension + Tauri native app architecture. The design keeps private keys and signing operations secure within the native app while providing seamless browser integration.

**Hackathon**: Zypherpunk  
**Demo Goal**: Show how a browser extension can safely hand off sensitive crypto operations to a native app using biometric authentication.

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                  Privacy-First Zcash Flow                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Webpage                                                          │
│    ↓ (user clicks zcash: link)                                   │
│  Content Script (intercepts)                                     │
│    ↓                                                             │
│  Background Service Worker                                       │
│    ↓ (native messaging via STDIN/STDOUT)                        │
│  Tauri App (nearx-tauri binary)                                 │
│    ↓ (Touch ID/PIN prompt - macOS LocalAuthentication)          │
│  User Approval                                                   │
│    ↓ (sign transaction - private key never leaves app)          │
│  Response via:                                                   │
│    1. Native Messaging (STDOUT → extension)                     │
│    2. Deep Link Callback (https://return.zwallet/...)          │
│    ↓                                                             │
│  Content Script (show success/error toast)                      │
│    ↓                                                             │
│  Webpage (updated via custom events)                            │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## 📁 Project Structure

```
zypherpunk/
├── extension/                       # Chrome/Brave Extension (MV3)
│   ├── manifest.json                # Extension config with nativeMessaging permission
│   ├── contentScript.js             # Intercepts zcash: links and buttons
│   ├── background.js                # Native messaging bridge
│   ├── test-page.html               # Demo page with payment links
│   └── README.md                    # Extension installation guide
│
├── native-messaging/                # Native messaging configuration
│   ├── com.zypherpunk.zcashsigner.json    # Host manifest for Chrome
│   └── README.md                    # Installation instructions
│
├── tauri-workspace/src-tauri/src/   # Tauri App (Rust + Native Integration)
│   ├── zcash_native_msg.rs          # STDIN/STDOUT native messaging handler
│   ├── zcash_auth.rs                # Touch ID + PIN authentication
│   ├── zcash_signer.rs              # Transaction signing (demo stub)
│   ├── zcash_handler.rs             # Orchestrator (ties everything together)
│   └── lib.rs                       # Tauri setup with Zcash initialization
│
└── ZYPHERPUNK_README.md             # This file
```

## 🚀 Quick Start

### Prerequisites

- macOS (for Touch ID support)
- Rust + Cargo
- Chrome or Brave browser
- Tauri CLI: `cargo install tauri-cli`

### 1. Build the Tauri App

```bash
cd tauri-workspace
cargo tauri build

# Copy to Applications
cp -r src-tauri/target/release/bundle/macos/NEARx.app /Applications/ZcashSigner.app
```

### 2. Install Native Messaging Host

```bash
cd native-messaging

# Update the manifest with your extension ID (see step 3)
# Edit com.zypherpunk.zcashsigner.json and replace EXTENSION_ID_PLACEHOLDER

# Install for Chrome
mkdir -p ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/
cp com.zypherpunk.zcashsigner.json ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/

# Or for Brave
mkdir -p ~/Library/Application\ Support/BraveSoftware/Brave-Browser/NativeMessagingHosts/
cp com.zypherpunk.zcashsigner.json ~/Library/Application\ Support/BraveSoftware/Brave-Browser/NativeMessagingHosts/
```

### 3. Load the Extension

1. Open Chrome/Brave → `chrome://extensions/`
2. Enable "Developer mode"
3. Click "Load unpacked"
4. Select the `extension/` directory
5. **Copy the Extension ID** (e.g., `abcdefghijklmnopqrstuvwxyz123456`)
6. Go back to step 2 and update the manifest with this ID

### 4. Test the Flow

1. Open `extension/test-page.html` in your browser
2. Click any "Pay with Zcash" link or button
3. The native app should launch and prompt for Touch ID/PIN
4. Approve or deny the transaction
5. See the result toast notification on the webpage

## 🔐 Security Features

### Private Key Protection

- ✅ **Private keys stored only in native app** (never in browser/extension)
- ✅ **Transaction signing happens in isolated native process**
- ✅ **Extension only receives signed transaction (no key material)**

### Biometric Authentication

- ✅ **Touch ID (macOS LocalAuthentication framework)** - Primary method
- ✅ **PIN fallback** - For systems without biometric support
- ✅ **Transaction details shown in auth prompt** - User sees what they're approving

### Extension Security

- ✅ **Minimal permissions** (nativeMessaging, webRequest, tabs only)
- ✅ **Native messaging restricted to specific extension ID**
- ✅ **Content Security Policy** prevents XSS attacks
- ✅ **No inline scripts** (Manifest V3 compliance)

## 💡 Technical Highlights

### 1. Native Messaging Protocol

Chrome's native messaging uses a simple STDIN/STDOUT protocol:

**Request (Extension → App)**:
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

**Response (App → Extension)**:
```json
{
  "status": "approved",
  "txid": "f3e1b2...abcd",
  "session": "1699564800000-abc123"
}
```

Messages are prefixed with a 4-byte little-endian length field.

### 2. Touch ID Implementation

Uses AppleScript to invoke macOS LocalAuthentication:

```rust
// Simplified example from zcash_auth.rs
let script = r#"
use framework "LocalAuthentication"
set context to current application's LAContext's alloc()'s init()
set success to context's evaluatePolicy:1 localizedReason:"Approve transaction" reply:(missing value) |error|:(reference)
return success
"#;
```

### 3. Deep Link Callback

After signing, the app opens a special URL that the extension intercepts:

```
https://return.zwallet/txResult?status=approved&txid=ABC123&session=XYZ
```

The extension's webRequest listener catches this and notifies the content script.

### 4. Dual-Channel Response

Responses are sent via **two channels** for maximum reliability:

1. **Native Messaging** (STDOUT) - Direct pipe back to extension
2. **Deep Link** - Opens URL that extension intercepts via webRequest

This ensures the browser gets updated even if one channel fails.

## 🧪 Testing

### Manual Testing

1. **Test Native Messaging Connection**:
   ```bash
   echo '{"action":"ping","session":"test"}' | /Applications/ZcashSigner.app/Contents/MacOS/nearx-tauri
   ```

   Should output (with length prefix):
   ```json
   {"status":"pong","session":"test"}
   ```

2. **Test Extension Console**:
   - Go to `chrome://extensions/`
   - Click "Inspect views: service worker" on your extension
   - Check for `[Zcash Extension]` logs

3. **Test Content Script**:
   - Open `test-page.html`
   - Open DevTools Console (F12)
   - Click a payment link
   - Look for interception logs

### Demo Scenarios

#### Scenario 1: zcash: Protocol Link
```html
<a href="zcash:zs1test123?amount=1.5&memo=Coffee">Pay 1.5 ZEC</a>
```

#### Scenario 2: Data Attributes
```html
<button 
  data-zcash-action="pay"
  data-zcash-to="zs1test123"
  data-zcash-amount="2.0"
  data-zcash-memo="Donation">
  Donate 2.0 ZEC
</button>
```

## 🎓 Demo Presentation Tips

### Key Points to Highlight

1. **Privacy-First Design**
   - "Notice how the browser never sees the private key"
   - "All signing happens in the isolated native app"

2. **Biometric Security**
   - "User must approve every transaction with Touch ID"
   - "Transaction details are shown in the auth prompt"

3. **Seamless UX**
   - "User clicks a payment link → App launches → Touch ID → Done"
   - "No manual copying of addresses or amounts"

4. **Web Integration**
   - "Works on any website with zcash: links"
   - "Extension intercepts before page can handle it"

### Live Demo Flow

1. **Show the test page** (`test-page.html`)
2. **Click a payment link** → Extension intercepts
3. **Show Touch ID prompt** → Explain security
4. **Approve** → Show transaction signed
5. **Show success toast** → Browser gets result
6. **Check DevTools** → Show deep link callback

### Common Questions

**Q: Why not use a browser-based wallet?**
A: Private keys in browser storage are vulnerable. Our approach keeps keys in the OS keychain with biometric protection.

**Q: Does this work on Windows/Linux?**
A: The native messaging works on all platforms. Touch ID is macOS-only, but we have PIN fallback.

**Q: Can't the website fake the transaction details?**
A: The native app shows the actual transaction in the auth prompt, so users see exactly what they're signing.

## 🔧 Development

### Adding New Transaction Types

1. Update `NativeRequest` in `zcash_native_msg.rs`
2. Add handler in `zcash_handler.rs`
3. Implement signing logic in `zcash_signer.rs`
4. Update content script to recognize new actions

### Customizing Authentication

Edit `zcash_auth.rs`:
- Add additional auth methods (Face ID, hardware keys)
- Customize PIN requirements
- Add rate limiting or cooldown periods

### Real Zcash Integration

Replace stubs in `zcash_signer.rs` with actual Zcash SDK:

```rust
// Example using a hypothetical Zcash crate
use zcash::{Wallet, Transaction};

pub fn sign_transaction(request: &TransactionRequest) -> Result<SignedTransaction, String> {
    let wallet = Wallet::from_keychain()?;
    let tx = wallet
        .build_transaction(&request.to, request.amount)?
        .with_memo(&request.memo)?;
    
    let signed = tx.sign()?;
    Ok(signed)
}
```

## 📊 Metrics & Logging

All components use emoji-prefixed logging for easy debugging:

- 🔵 **Native Messaging** - STDIN/STDOUT communication
- 🔐 **Auth** - Touch ID/PIN operations
- ✍️  **Signer** - Transaction signing
- 💰 **Handler** - Request orchestration
- 🔗 **Deep Link** - Callback mechanism

View logs:
```bash
# Tauri app logs
tail -f ~/Library/Logs/com.fastnear.nearx/NEARx.log

# Or run in terminal to see stdout
/Applications/ZcashSigner.app/Contents/MacOS/nearx-tauri
```

## 🐛 Troubleshooting

### "Specified native messaging host not found"

- Check manifest is in correct directory:
  ```bash
  cat ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/com.zypherpunk.zcashsigner.json
  ```
- Verify `path` points to actual binary
- Ensure extension ID matches in `allowed_origins`

### Touch ID Not Working

- Check System Preferences → Touch ID → Unlock Mac is enabled
- Try PIN fallback (enter `1234` in demo)
- Check Console.app for LocalAuthentication errors

### Extension Not Intercepting Clicks

- Check content script loaded: Look for console log on page load
- Verify link format matches expected patterns
- Try reloading the extension

### Deep Link Callback Not Working

- Check webRequest permission in manifest.json
- Verify `https://return.zwallet/*` in host_permissions
- Look for intercepted requests in Network tab (should be canceled)

## 🎯 Future Enhancements

- [ ] Support shielded → shielded transactions (full privacy)
- [ ] Multi-signature transactions
- [ ] Hardware wallet integration (Ledger, Trezor)
- [ ] Browser extension for Firefox (using native messaging)
- [ ] Mobile support (iOS/Android with Tauri Mobile)
- [ ] Transaction history and analytics
- [ ] Gas/fee estimation
- [ ] Address book management

## 📄 License

See main project license.

## 🙏 Credits

- Built for the Zypherpunk Hackathon
- Uses Tauri, Rust, Chrome Extension APIs
- Inspired by 1Password's native messaging architecture

---

**Demo ready!** 🎉  
For questions or issues, check the logs and troubleshooting guide above.
