/**
 * Unified Clipboard Bridge for Ratacat
 *
 * Provides a single `__copy_text(text)` function that works across:
 * - Tauri desktop app (via plugin or JS API)
 * - Browser Extension (via background relay)
 * - Web browsers (via Navigator Clipboard API)
 * - Legacy browsers (via execCommand fallback)
 *
 * This file is loaded by both web/WASM and Tauri builds, enabling
 * a single WASM binary to work in multiple environments.
 */

/**
 * Attempt to copy via Navigator Clipboard API or legacy fallback
 * @param {string} text - Text to copy
 * @returns {Promise<boolean>} - Success status
 */
async function copyViaNavigator(text) {
  // Modern Clipboard API (requires HTTPS or localhost)
  if (typeof navigator !== "undefined" && navigator.clipboard && window.isSecureContext) {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch (err) {
      console.warn("Navigator clipboard failed:", err);
      // Fall through to legacy method
    }
  }

  // Legacy fallback using execCommand (works in many WebViews)
  try {
    const textarea = document.createElement("textarea");
    textarea.value = text;
    textarea.style.position = "fixed";
    textarea.style.top = "-1000px";
    textarea.style.left = "-1000px";
    document.body.appendChild(textarea);
    textarea.focus();
    textarea.select();

    const success = document.execCommand("copy");
    document.body.removeChild(textarea);

    if (success) {
      return true;
    }
  } catch (err) {
    console.warn("Legacy clipboard (execCommand) failed:", err);
  }

  return false;
}

/**
 * Attempt to copy via Tauri clipboard plugin or JS API
 * @param {string} text - Text to copy
 * @returns {Promise<boolean>} - Success status
 */
async function copyViaTauri(text) {
  // Tauri v2: Custom invoke-based command (preferred)
  if (globalThis.__TAURI__ && typeof __TAURI__.invoke === "function") {
    try {
      await __TAURI__.invoke("copy_text", { text });
      console.log("Copied via Tauri invoke command");
      return true;
    } catch (err) {
      console.warn("Tauri invoke copy_text failed:", err);
      // Fall through to next method
    }
  }

  // Tauri v1: Built-in clipboard API (if allowlist enabled)
  if (globalThis.__TAURI__?.clipboard?.writeText) {
    try {
      await __TAURI__.clipboard.writeText(text);
      console.log("Copied via Tauri clipboard API");
      return true;
    } catch (err) {
      console.warn("Tauri clipboard.writeText failed:", err);
      // Fall through
    }
  }

  return false;
}

/**
 * Attempt to copy via Browser Extension background relay
 * @param {string} text - Text to copy
 * @returns {Promise<boolean>} - Success status
 */
async function copyViaExtension(text) {
  // Check if running in extension context (MV3)
  if (typeof chrome !== "undefined" && chrome.runtime?.id) {
    try {
      const response = await new Promise((resolve) => {
        chrome.runtime.sendMessage(
          { type: "COPY_TEXT", text },
          (response) => resolve(response)
        );
      });

      if (response && response.ok) {
        console.log("Copied via browser extension relay");
        return true;
      }
    } catch (err) {
      console.warn("Extension clipboard relay failed:", err);
      // Fall through
    }
  }

  return false;
}

/**
 * Main clipboard function exposed to WASM
 * Tries methods in order: Tauri → Extension → Navigator/Legacy
 *
 * @param {string} text - Text to copy to clipboard
 * @returns {Promise<boolean>} - True if any method succeeded
 */
async function __copy_text(text) {
  // 1. Try Tauri plugin/API (if running in Tauri)
  if (await copyViaTauri(text)) {
    return true;
  }

  // 2. Try browser extension relay (if in extension context)
  if (await copyViaExtension(text)) {
    return true;
  }

  // 3. Try standard web APIs (Navigator or execCommand)
  if (await copyViaNavigator(text)) {
    return true;
  }

  // All methods failed
  console.error("All clipboard methods failed");
  return false;
}

// Expose to global scope for WASM-bindgen
globalThis.__copy_text = __copy_text;

console.log("Ratacat clipboard bridge loaded");
