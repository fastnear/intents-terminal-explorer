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

function updateDetailsViewport() {
  const detailsPre = document.getElementById("pane-details-pre");
  if (!detailsPre || !wasmApp || !wasmApp.setDetailsViewportLines) return;

  const detailsHeight = detailsPre.clientHeight || 400;
  const estimatedLineHeight = 16; // 12px font-size * 1.35 line-height ≈ 16px
  const viewportLines = Math.max(1, Math.floor(detailsHeight / estimatedLineHeight));

  // Only update if changed
  if (viewportLines !== lastViewportLines) {
    lastViewportLines = viewportLines;
    wasmApp.setDetailsViewportLines(viewportLines);
  }
}

async function main() {
  await init();

  // Expose wasm exports globally so router_shim.js can call
  // window.wasm_bindgen.nearx_auth_callback(qs).
  window.wasm_bindgen = wasm;

  wasmApp = new wasm.WasmApp();
  hookEvents();

  // Set initial viewport size
  updateDetailsViewport();

  // Update viewport on resize
  const detailsPre = document.getElementById("pane-details-pre");
  if (detailsPre && window.ResizeObserver) {
    const resizeObserver = new ResizeObserver(() => {
      updateDetailsViewport();
    });
    resizeObserver.observe(detailsPre);
  }

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

// Event-driven render with throttled polling
// Poll at 10 Hz (100ms) instead of 60 FPS to avoid wasteful serialization
function startRenderLoop() {
  function pollAndRender() {
    const snap = snapshot();  // Drains events from RPC poller
    render(snap);             // Update DOM with latest state
    setTimeout(pollAndRender, 100);  // 10 Hz polling
  }
  pollAndRender();
}

/* ---------- JSON syntax highlight ---------- */

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

    // When help modal is open, ignore all other keys (only ? and Esc allowed above)
    if (lastSnapshot && lastSnapshot.show_shortcuts) {
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

    // Special handling for Tab - works even when filter is focused
    if (e.key === "Tab") {
      e.preventDefault();
      apply({
        type: "Key",
        code: e.key,
        ctrl: e.ctrlKey || e.metaKey,
        alt: e.altKey,
        shift: e.shiftKey,
        meta: e.metaKey,
      });
      return;
    }

    // When typing into filter, let keystrokes through (Esc and Tab handled above).
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
      "Tab",
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
  blocksPane.classList.toggle("focused", snapshot.pane === 0);
  txPane.classList.toggle("focused", snapshot.pane === 1);
  detailsPane.classList.toggle("focused", snapshot.pane === 2);

  // Selection slot (shows current block/tx selection prominently)
  const selectionSlot = document.getElementById("selection-slot");
  if (selectionSlot) {
    selectionSlot.textContent = snapshot.selection_slot_text || "";
  }

  // Blocks pane: two-list rendering (forward + backfill)
  blocksBody.innerHTML = "";
  const blocks = snapshot.blocks || [];
  const loadingBlock = snapshot.loading_block;

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
    if (snapshot.viewing_cached) {
      title = "Blocks (cached) — (↑↓ nav • ← recent)";
    } else if (snapshot.blocks_total != null && blocks.length < snapshot.blocks_total) {
      title = `Blocks (${blocks.length}/${snapshot.blocks_total}) — (↑↓ nav • Enter select)`;
    } else {
      title = "Blocks — (↑↓ nav • Enter select)";
    }
    blocksTitle.textContent = title;
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
    let title = "Txs";
    const total = snapshot.txs_total ?? txs.length;
    if (txs.length < total) {
      title = `Txs (${txs.length}/${total}) — (↑↓ nav • Enter select)`;
    } else if (total > 0) {
      title = `Txs (${total}) — (↑↓ nav • Enter select)`;
    } else {
      title = "Txs — (↑↓ nav • Enter select)";
    }
    txTitle.textContent = title;
  }

  // Details pane: render windowed JSON (already windowed and formatted by Rust)
  const rawDetails = snapshot.details || "";
  let html = syntaxHighlightJson(rawDetails);

  // Add truncation message if content was cut off
  if (snapshot.details_truncated) {
    html += '<br><br><span style="color: var(--fg-dim); font-style: italic;">… large output truncated at 5000 lines; press \'c\' to copy full JSON</span>';
  }

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

    // Show scroll position: "(42/1234)" format to match TUI
    const scrollIndicator = snapshot.details_total_lines > 1
      ? ` (${(snapshot.details_scroll_line ?? 0) + 1}/${snapshot.details_total_lines})`
      : "";

    detailsTitle.textContent = `${contentTypeLabel}${scrollIndicator} - ${modeLabel} • Tab=switch • c=copy • Space=exit`;
  } else {
    // Non-fullscreen: show scroll indicator if content has multiple lines
    const scrollIndicator = snapshot.details_total_lines > 1
      ? ` (${(snapshot.details_scroll_line ?? 0) + 1}/${snapshot.details_total_lines})`
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

  // Toast.
  if (toastEl) {
    if (snapshot.toast) {
      toastEl.textContent = snapshot.toast;
      toastEl.hidden = false;
    } else {
      toastEl.hidden = true;
      toastEl.textContent = "";
    }
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
 * Handle copy action with on-demand content fetching.
 * Gets content from WASM only when needed (not on every frame).
 */
async function handleCopyClientSide(snapshot) {
  const paneNames = ["block", "transaction", "details"];
  const paneName = paneNames[snapshot.pane] || "data";

  // Get content on-demand from WASM (only when user presses 'c')
  if (!wasmApp || !wasmApp.getClipboardContent) {
    showToastClientSide("Copy not available");
    return;
  }

  const content = wasmApp.getClipboardContent();

  // Handle empty content
  if (!content || content.startsWith("No ") || content === "") {
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
 * Show toast notification client-side (bypasses WASM snapshot polling).
 */
function showToastClientSide(message) {
  const toastEl = document.getElementById("nearx-toast");
  if (!toastEl) return;

  toastEl.textContent = message;
  toastEl.hidden = false;

  // Auto-hide after 2 seconds (matches TUI behavior)
  setTimeout(() => {
    toastEl.hidden = true;
    toastEl.textContent = "";
  }, 2000);
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
