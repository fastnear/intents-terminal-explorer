// DOM frontend for NEARx using WasmApp + UiSnapshot/UiAction.
//
// Requires wasm-bindgen output for the `nearx-web-dom` binary:
//
//   cargo build --bin nearx-web-dom --target wasm32-unknown-unknown --features dom-web
//
//   wasm-bindgen \
//     --target web \
//     --no-typescript \
//     --out-dir web/pkg \
//     --out-name nearx_web_dom \
//     target/wasm32-unknown-unknown/debug/nearx-web-dom.wasm
//
// This will produce `web/pkg/nearx_web_dom.js` and `web/pkg/nearx_web_dom_bg.wasm`.
// Then you can open `web/index.html` directly (or via Tauri).

import init, * as wasm from "./pkg/nearx_web_dom.js";

let wasmApp = null;
let lastSnapshot = null;
let suppressFilterEvent = false;

// Track viewport size to avoid redundant updates
let lastViewportLines = 0;
let currentViewportLines = 40; // Default estimate, updated by updateDetailsViewport

// Update details viewport dynamically
function updateDetailsViewport() {
  const detailsPre = document.getElementById("pane-details-pre");
  if (!detailsPre || !wasmApp || !wasmApp.setDetailsViewportLines) return;

  const detailsHeight = detailsPre.clientHeight || 400;
  const estimatedLineHeight = 16; // matches our row height
  const viewportLines = Math.max(1, Math.floor(detailsHeight / estimatedLineHeight));

  // Only update if changed
  if (viewportLines !== lastViewportLines) {
    lastViewportLines = viewportLines;
    currentViewportLines = viewportLines;
    wasmApp.setDetailsViewportLines(viewportLines);
  }
}

// Set up ResizeObserver for viewport tracking
function setupResizeObserver() {
  const detailsPre = document.getElementById("pane-details-pre");
  if (detailsPre && window.ResizeObserver) {
    const resizeObserver = new ResizeObserver(() => {
      updateDetailsViewport();
    });
    resizeObserver.observe(detailsPre);

    // Initial measurement
    updateDetailsViewport();
  }
}

// Client-side toast management
let activeToastTimeout = null;

function showToastClientSide(message) {
  const toastEl = document.getElementById("nearx-toast");
  if (!toastEl) return;

  // Clear any existing timeout
  if (activeToastTimeout) {
    clearTimeout(activeToastTimeout);
  }

  // Show toast immediately
  toastEl.textContent = message;
  toastEl.hidden = false;

  // Auto-hide after 4 seconds
  activeToastTimeout = setTimeout(() => {
    toastEl.hidden = true;
    toastEl.textContent = "";
    activeToastTimeout = null;
  }, 4000);
}

async function main() {
  await init();

  // Expose wasm exports globally so router_shim.js can call
  // window.wasm_bindgen.nearx_auth_callback(qs).
  window.wasm_bindgen = wasm;

  wasmApp = new wasm.WasmApp();
  hookEvents();

  // Set up ResizeObserver for dynamic viewport tracking
  setupResizeObserver();

  // Start continuous render loop (drains RPC events on every frame)
  startRenderLoop();
}

function snapshot() {
  const json = wasmApp.snapshot_json();
  lastSnapshot = JSON.parse(json);
  return lastSnapshot;
}

function apply(action) {
  const json = wasmApp.handle_action_json(JSON.stringify(action));
  lastSnapshot = JSON.parse(json);
  render(lastSnapshot);
}

// Poll at 10 Hz (100ms) instead of 60 FPS to avoid wasteful serialization
// This matches ratacat's proven approach for better performance
function startRenderLoop() {
  function pollAndRender() {
    const snap = snapshot();  // Drains events from RPC poller
    render(snap);
    setTimeout(pollAndRender, 100);  // 10 Hz polling
  }
  pollAndRender();
}

/* ---------- JSON pretty-print + syntax highlight ---------- */

function safePrettyJson(text) {
  if (!text) return "";
  try {
    const obj = JSON.parse(text);
    return JSON.stringify(obj, null, 2);
  } catch {
    // Already formatted or non-JSON; show as-is.
    return text;
  }
}

function syntaxHighlightJson(text) {
  // Basic HTML escaping
  const escaped = text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");

  // Token highlighter for JSON: string, key, number, bool, null.
  return escaped.replace(
    /("(\\u[a-fA-F0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g,
    (match) => {
      let cls = "nx-json-number";
      if (/^"/.test(match)) {
        if (/:$/.test(match)) cls = "nx-json-key";
        else cls = "nx-json-string";
      } else if (/true|false/.test(match)) {
        cls = "nx-json-bool";
      } else if (/null/.test(match)) {
        cls = "nx-json-null";
      }
      return `<span class="${cls}">${match}</span>`;
    },
  );
}

/* ---------- DOM wiring ---------- */

function hookEvents() {
  const filter = document.getElementById("nearx-filter");

  const blocksPane = document.getElementById("pane-blocks");
  const blocksBody = document.getElementById("pane-blocks-body");
  const txPane = document.getElementById("pane-txs");
  const txBody = document.getElementById("pane-txs-body");
  const detailsPane = document.getElementById("pane-details");
  const detailsPre = document.getElementById("pane-details-pre");

  if (
    !filter ||
    !blocksPane ||
    !blocksBody ||
    !txPane ||
    !txBody ||
    !detailsPane ||
    !detailsPre
  ) {
    console.error("[nearx-web-dom] Missing DOM elements");
    return;
  }

  // Filter input → SetFilter (immediate).
  filter.addEventListener("input", (e) => {
    if (suppressFilterEvent) return;
    apply({ type: "SetFilter", text: e.target.value });
  });

  filter.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      e.preventDefault();
      filter.blur();
    }
  });

  // Mouse focus on panes.
  blocksPane.addEventListener("mousedown", () =>
    apply({ type: "FocusPane", pane: 0 }),
  );
  txPane.addEventListener("mousedown", () =>
    apply({ type: "FocusPane", pane: 1 }),
  );
  detailsPane.addEventListener("mousedown", () =>
    apply({ type: "FocusPane", pane: 2 }),
  );

  // Global keyboard navigation.
  document.addEventListener("keydown", (e) => {
    const filterActive = document.activeElement === filter;
    const modal = document.getElementById("nearx-help-modal");

    // '?' → toggle help modal (not when typing in filter)
    if (e.key === "?" && !filterActive) {
      e.preventDefault();
      apply({ type: "ToggleShortcuts" });
      return;
    }

    // Esc → close help modal if open (check snapshot state)
    if (lastSnapshot && lastSnapshot.show_shortcuts && e.key === "Escape") {
      e.preventDefault();
      apply({ type: "ToggleShortcuts" });  // Will hide modal
      return;
    }

    // '/' or 'f' / 'F' → focus filter (like TUI).
    if (e.key === "/" || e.key === "f" || e.key === "F") {
      if (e.ctrlKey || e.metaKey || e.altKey) return;
      e.preventDefault();
      filter.focus();
      filter.select();
      return;
    }

    // When typing into filter, let keystrokes through (Esc handled above).
    if (filterActive) return;

    // Plain 'c' → copy focused JSON (no modifiers).
    if (e.key === "c" || e.key === "C") {
      if (!e.ctrlKey && !e.metaKey && !e.altKey) {
        e.preventDefault();

        // Handle copy client-side (idiomatic web, no WASM round-trip)
        if (lastSnapshot) {
          handleCopyClientSide(lastSnapshot).catch((err) => {
            console.error("[nearx][copy] Failed:", err);
          });
        }
        return;
      }
    }

    // Tab key → cycle panes with optimistic UI
    if (e.key === "Tab") {
      e.preventDefault(); // Prevent browser tab navigation

      // Optimistic UI: instantly update pane focus before WASM round-trip
      if (lastSnapshot) {
        const currentPane = lastSnapshot.pane;
        const nextPane = e.shiftKey
          ? (currentPane - 1 + 3) % 3  // Shift+Tab: backwards
          : (currentPane + 1) % 3;      // Tab: forwards

        // Instant visual update (no WASM delay)
        const blocksPane = document.getElementById("pane-blocks");
        const txPane = document.getElementById("pane-txs");
        const detailsPane = document.getElementById("pane-details");

        blocksPane?.classList.toggle("nx-pane--focused", nextPane === 0);
        txPane?.classList.toggle("nx-pane--focused", nextPane === 1);
        detailsPane?.classList.toggle("nx-pane--focused", nextPane === 2);
      }

      // Sync to WASM (snapshot will confirm same state on next render)
      apply({
        type: "Key",
        code: "Tab",
        ctrl: e.ctrlKey || e.metaKey,
        alt: e.altKey,
        shift: e.shiftKey,
        meta: e.metaKey
      });
      return;
    }

    // Keys that map to UiAction::Key.
    const navKeys = [
      "ArrowUp",
      "ArrowDown",
      "ArrowLeft",
      "ArrowRight",
      "PageUp",
      "PageDown",
      "Home",
      "End",
      "Enter",
      " ",
      "Escape",  // Exit fullscreen / clear filter (priority-based)
      "j",
      "k",
      "h",
      "l",
      "J",
      "K",
      "H",
      "L",
    ];

    if (!navKeys.includes(e.key)) return;

    e.preventDefault();
    apply({
      type: "Key",
      code: e.key,
      ctrl: e.ctrlKey || e.metaKey,
      alt: e.altKey,
      shift: e.shiftKey,
      meta: e.metaKey,
    });
  });

  // Row clicks (blocks).
  blocksBody.addEventListener("click", (e) => {
    const row = e.target.closest("[data-index]");
    if (!row) return;
    const index = Number(row.dataset.index);
    if (Number.isNaN(index)) return;
    apply({ type: "SelectBlock", index });
  });

  // Row clicks (txs).
  txBody.addEventListener("click", (e) => {
    const row = e.target.closest("[data-index]");
    if (!row) return;
    const index = Number(row.dataset.index);
    if (Number.isNaN(index)) return;
    apply({ type: "SelectTx", index });
  });

  // Help modal close button (use UiAction instead of DOM manipulation)
  const modalCloseBtn = document.querySelector(".nx-modal-close");
  if (modalCloseBtn) {
    modalCloseBtn.addEventListener("click", () => {
      apply({ type: "ToggleShortcuts" });
    });
  }

  // Help modal backdrop click (close modal via UiAction)
  const modalBackdrop = document.querySelector(".nx-modal-backdrop");
  if (modalBackdrop) {
    modalBackdrop.addEventListener("click", () => {
      apply({ type: "ToggleShortcuts" });
    });
  }
}

// Store previous snapshot for scroll preservation
let prevSnapshot = null;
let prevBlocksHash = "";

function render(snapshot) {
  const filter = document.getElementById("nearx-filter");

  const blocksPane = document.getElementById("pane-blocks");
  const blocksBody = document.getElementById("pane-blocks-body");
  const blocksTitle = document.getElementById("pane-blocks-title");

  const txPane = document.getElementById("pane-txs");
  const txBody = document.getElementById("pane-txs-body");
  const txTitle = document.getElementById("pane-txs-title");

  const detailsPane = document.getElementById("pane-details");
  const detailsTitle = document.getElementById("pane-details-title");
  const detailsPre = document.getElementById("pane-details-pre");

  const footer = document.getElementById("nearx-footer");
  const toastEl = document.getElementById("nearx-toast");

  if (
    !filter ||
    !blocksPane ||
    !blocksBody ||
    !txPane ||
    !txBody ||
    !detailsPane ||
    !detailsPre ||
    !footer
  ) {
    return;
  }

  // Store scroll positions before re-render
  const scrollPositions = {
    blocks: blocksBody.scrollTop,
    txs: txBody.scrollTop,
    details: detailsPre.scrollTop,
  };

  // Detect if selection changed (to preserve scroll when it hasn't)
  const blocksSelectionChanged =
    !prevSnapshot ||
    prevSnapshot.blocks?.find(b => b.is_selected)?.index !==
    snapshot.blocks?.find(b => b.is_selected)?.index;
  const txsSelectionChanged =
    !prevSnapshot ||
    prevSnapshot.txs?.find(t => t.is_selected)?.index !==
    snapshot.txs?.find(t => t.is_selected)?.index;

  // Filter text (keep in sync).
  suppressFilterEvent = true;
  filter.value = snapshot.filter_query || "";
  suppressFilterEvent = false;

  // Pane focus highlight (four-point focus system).
  blocksPane.classList.toggle("nx-pane--focused", snapshot.pane === 0);
  txPane.classList.toggle("nx-pane--focused", snapshot.pane === 1);
  detailsPane.classList.toggle("nx-pane--focused", snapshot.pane === 2);

  // Selection slot (shows current block/tx selection prominently)
  const selectionSlot = document.getElementById("selection-slot");
  if (selectionSlot) {
    selectionSlot.textContent = snapshot.selection_slot_text || "";
  }

  // Blocks pane: two-list rendering (forward + backfill)
  const blocks = snapshot.blocks || [];
  const loadingBlock = snapshot.loading_block;

  // Simple hash to detect if blocks have changed
  const blocksHash = blocks.map(b => `${b.height}-${b.is_selected}`).join(",");

  // Skip re-rendering blocks if nothing changed
  if (blocksHash === prevBlocksHash) {
    // Still update the title in case counts changed
    if (blocksTitle) {
      let title = "Blocks";
      if (snapshot.blocks_total != null) {
        title = `Blocks (${blocks.length}/${snapshot.blocks_total})`;
      }
      if (snapshot.viewing_cached) {
        title += " · cached (← recent)";
      }
      blocksTitle.textContent = title;
    }
  } else {
    // Blocks have changed, re-render
    blocksBody.innerHTML = "";

    blocks.forEach((b) => {
      const row = document.createElement("div");
      row.className = "nx-row nx-row--block";

      // Apply source-based styling (forward vs backfill)
      if (b.source === "backfill_pending") {
        row.classList.add("nx-row--backfill-pending");
      } else if (b.source === "backfill_loading") {
        row.classList.add("nx-row--backfill-loading");
      } else {
        row.classList.add("nx-row--forward");
      }

      if (b.is_selected) row.classList.add("nx-row--selected");
      if (!b.available) row.style.opacity = "0.6";
      row.dataset.index = String(b.index);
      row.setAttribute("role", "option");
      row.setAttribute("aria-selected", b.is_selected ? "true" : "false");

      // Backfill placeholders use label directly (already formatted in Rust)
      if (b.source === "backfill_pending" || b.source === "backfill_loading") {
        // Use the pre-formatted label from Rust (e.g., "12345  |  archival lookup queued…")
        const parts = [
          `#${b.height}`,
          b.source === "backfill_loading" ? "archival lookup in flight…" : "archival lookup queued…"
        ];
        row.innerHTML = parts.join(" · ");
      } else {
        // Forward blocks show full details
        row.textContent = `#${b.height} · ${b.tx_count} tx · ${b.when}`;
      }

      blocksBody.appendChild(row);
    });

    // Apply vertical centering via scroll offset (like TUI)
    if (snapshot.blocks_scroll_offset != null && snapshot.blocks_scroll_offset > 0) {
      const rowHeight = 24;  // Approximate based on CSS line-height
      blocksBody.scrollTop = snapshot.blocks_scroll_offset * rowHeight;
    }

    // Blocks title with counts.
    if (blocksTitle) {
      let title = "Blocks";
      if (snapshot.blocks_total != null) {
        title = `Blocks (${blocks.length}/${snapshot.blocks_total})`;
      }
      if (snapshot.viewing_cached) {
        title += " · cached (← recent)";
      }
      blocksTitle.textContent = title;
    }

    // Update hash after rendering
    prevBlocksHash = blocksHash;
  }

  // Txs pane.
  txBody.innerHTML = "";
  const txs = snapshot.txs || [];
  txs.forEach((t) => {
    const row = document.createElement("div");
    row.className = "nx-row nx-row--tx";
    if (t.is_selected) row.classList.add("nx-row--selected");
    row.dataset.index = String(t.index);
    row.setAttribute("role", "option");
    row.setAttribute("aria-selected", t.is_selected ? "true" : "false");

    const signer = t.signer_id || "";
    const receiver = t.receiver_id || "";
    const label =
      signer && receiver
        ? `${signer} → ${receiver}`
        : signer || receiver || t.hash;

    row.textContent = label;
    txBody.appendChild(row);
  });

  // Tx title with position.
  if (txTitle) {
    let title = "Tx hashes";
    const total = snapshot.txs_total ?? txs.length;
    if (total > 0) {
      const selIdx = txs.find((t) => t.is_selected)?.index ?? 0;
      const pos = selIdx + 1;
      title = `Tx hashes (${pos}/${total})`;
    }
    txTitle.textContent = title;
  }

  // Viewport size is now handled by ResizeObserver for better responsiveness

  // Details pane: colorize JSON client-side
  const rawDetails = snapshot.details || "";
  const pretty = safePrettyJson(rawDetails);
  const html = syntaxHighlightJson(pretty);

  detailsPane.classList.toggle(
    "nx-details--fullscreen",
    !!snapshot.details_fullscreen,
  );

  // Update title with mode indicator, content type, and scroll indicator
  if (snapshot.details_fullscreen) {
    const modeLabel = snapshot.fullscreen_mode === "Scroll" ? "↕ Scroll" : "↑↓ Navigate";
    const contentTypeLabel = {
      "BlockRawJson": "Block Raw JSON",
      "TransactionRawJson": "Transaction Raw JSON",
      "ParsedDetails": "Transaction Details"
    }[snapshot.fullscreen_content_type] || "Details";

    // Show scroll position: "Lines 1-40/523"
    const scrollStart = (snapshot.details_scroll_line ?? 0) + 1;
    const scrollEnd = Math.min(scrollStart + currentViewportLines - 1, snapshot.details_total_lines ?? 0);
    const scrollIndicator = snapshot.details_total_lines > 0
      ? ` (Lines ${scrollStart}-${scrollEnd}/${snapshot.details_total_lines})`
      : "";

    detailsTitle.textContent = `${contentTypeLabel} - ${modeLabel}${scrollIndicator} • Tab=switch • c=copy • Space=exit`;
  } else {
    // Non-fullscreen: show scroll indicator if content is large
    const scrollIndicator = snapshot.details_total_lines > currentViewportLines
      ? ` (${snapshot.details_scroll_line + 1}-${Math.min(snapshot.details_scroll_line + currentViewportLines, snapshot.details_total_lines)}/${snapshot.details_total_lines})`
      : "";
    detailsTitle.textContent = `Transaction details${scrollIndicator} – c: copy • Space: expand`;
  }

  // Always show content (already windowed by Rust, no scrolling needed)
  detailsPre.innerHTML = html;
  detailsPre.scrollTop = 0; // Windowing handles scroll position, always show from top

  // Footer.
  const parts = [];
  parts.push(`Blocks ${snapshot.blocks_total ?? 0}`);
  parts.push(`Txs ${snapshot.txs_total ?? 0}`);
  if (snapshot.selected_block_height != null)
    parts.push(`Block #${snapshot.selected_block_height}`);

  footer.textContent = parts.join("  •  ");

  // Toast - handled client-side now for immediate feedback
  // (keeping the check for backwards compatibility but preferring client-side)
  if (toastEl && snapshot.toast && !activeToastTimeout) {
    // Only show server toast if no client toast is active
    showToastClientSide(snapshot.toast);
  }

  // Keyboard shortcuts modal visibility (driven by snapshot state).
  const modal = document.getElementById("nearx-help-modal");
  if (modal) {
    if (snapshot.show_shortcuts) {
      modal.classList.remove("hidden");
    } else {
      modal.classList.add("hidden");
    }
  }

  // Restore scroll positions if selection didn't change
  if (!blocksSelectionChanged) {
    blocksBody.scrollTop = scrollPositions.blocks;
  }
  if (!txsSelectionChanged) {
    txBody.scrollTop = scrollPositions.txs;
  }
  // Details scroll is controlled by snapshot.details_scroll (applied above)

  // Store current snapshot for next render comparison
  prevSnapshot = snapshot;
}

/**
 * Handle copy action entirely client-side (no WASM round-trip).
 * Extracts content based on focused pane and uses JavaScript clipboard API.
 */
async function handleCopyClientSide(snapshot) {
  const paneNames = ["block", "transaction", "details"];
  const paneName = paneNames[snapshot.pane] || "data";

  // Extract content based on focused pane
  let content = "";
  switch (snapshot.pane) {
    case 0: // Blocks
      content = snapshot.raw_block_json;
      break;
    case 1: // Transactions
      content = snapshot.raw_tx_json;
      break;
    case 2: // Details
      content = snapshot.details;
      break;
  }

  // Handle empty content
  if (!content || content.startsWith("No ")) {
    showToastClientSide("Nothing to copy");
    return;
  }

  // Call clipboard facade (platform.js provides window.__copy_text)
  try {
    const success = await window.__copy_text(content);

    // Show toast (client-side, bypasses WASM)
    if (success) {
      showToastClientSide(`Copied ${paneName}`);
      flashPaneCopied(snapshot.pane);
    } else {
      showToastClientSide("Copy failed");
    }
  } catch (err) {
    console.error("[nearx][copy] Error:", err);
    showToastClientSide("Copy failed");
  }
}


/**
 * Flash pane border to indicate copy success.
 */
function flashPaneCopied(paneIndex) {
  // Use querySelector instead of scoped variables (avoids ReferenceError)
  const paneIds = ["pane-blocks", "pane-txs", "pane-details"];
  const paneId = paneIds[paneIndex];
  if (paneId) {
    const focusedPane = document.getElementById(paneId);
    if (focusedPane) {
      focusedPane.classList.add("nx-flash-copied");
      setTimeout(() => focusedPane.classList.remove("nx-flash-copied"), 300);
    }
  }
}

document.addEventListener("DOMContentLoaded", () => {
  main().catch((err) => {
    console.error("[nearx-web-dom] Failed to start:", err);
  });
});
