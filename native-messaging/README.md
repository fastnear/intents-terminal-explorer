# Native Messaging Host Configuration

This directory contains the manifest file for Chrome/Brave to recognize the Tauri app as a native messaging host.

## Installation (macOS)

1. **Build the Tauri app** first:
   ```bash
   cd tauri-workspace
   cargo tauri build
   ```

2. **Copy the app to Applications**:
   ```bash
   cp -r target/release/bundle/macos/NEARx.app /Applications/ZcashSigner.app
   ```

3. **Update the manifest with your extension ID**:
   - Load the extension in Chrome (chrome://extensions/)
   - Enable "Developer mode"
   - Note the Extension ID (e.g., `abcdefghijklmnopqrstuvwxyz123456`)
   - Edit `com.zypherpunk.zcashsigner.json` and replace `EXTENSION_ID_PLACEHOLDER` with your actual extension ID

4. **Install the native messaging host manifest**:
   ```bash
   # For Chrome
   mkdir -p ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/
   cp com.zypherpunk.zcashsigner.json ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/

   # For Brave
   mkdir -p ~/Library/Application\ Support/BraveSoftware/Brave-Browser/NativeMessagingHosts/
   cp com.zypherpunk.zcashsigner.json ~/Library/Application\ Support/BraveSoftware/Brave-Browser/NativeMessagingHosts/
   ```

5. **Verify registration**:
   ```bash
   # Check if file exists
   cat ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/com.zypherpunk.zcashsigner.json
   ```

## Troubleshooting

**"Specified native messaging host not found" error:**
- Ensure the `path` in the manifest points to the correct binary location
- Verify the app binary is executable: `chmod +x /Applications/ZcashSigner.app/Contents/MacOS/nearx-tauri`
- Check the extension ID matches exactly (no trailing slash inside the ID itself)

**"Access to the specified native messaging host is forbidden" error:**
- The extension ID in `allowed_origins` must match your extension's actual ID
- The extension ID format must be: `chrome-extension://YOUR_ID_HERE/`

**Testing native messaging:**
You can test the native host manually from terminal:
```bash
echo '{"action":"ping"}' | /Applications/ZcashSigner.app/Contents/MacOS/nearx-tauri
```

This should launch the app and it will read from STDIN.
