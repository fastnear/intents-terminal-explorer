// DOM frontend for NEARx using WasmApp.
//
// Trunk auto-loads the WASM and exposes it as window.wasmBindings.
// We just need to access WasmApp from there.
//
// HTML requirements (see index-dom.html):
//
// <div id="nearx-root">
//   <div id="row-filter">
//     <input id="nearx-filter" />
//     <button id="nearx-owned-toggle" type="button">Owned only</button>
//   </div>
//   <div id="row-main">
//     <div id="pane-blocks" class="nx-pane nx-pane--blocks"></div>
//     <div id="pane-txs" class="nx-pane nx-pane--txs"></div>
//   </div>
//   <div id="row-details">
//     <pre id="pane-details" class="nx-details"></pre>
//   </div>
//   <div id="nearx-toast" class="nx-toast" hidden></div>
// </div>

let wasmApp = null;
let lastSnapshot = null;
let suppressFilterEvent = false;

/**
 * Simple JSON syntax highlighter for the details pane.
 * Returns HTML with CSS classes for different token types.
 */
function highlightJSON(json) {
  if (!json) return "";

  // Escape HTML to prevent XSS
  const escapeHtml = (str) =>
    str
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#039;");

  // Replace JSON tokens with styled spans
  return escapeHtml(json)
    .replace(/"([^"]+)":/g, '<span class="json-key">"$1"</span>:')
    .replace(/:\s*"([^"]*)"/g, ': <span class="json-string">"$1"</span>')
    .replace(/:\s*(true|false)/g, ': <span class="json-bool">$1</span>')
    .replace(/:\s*(null)/g, ': <span class="json-null">$1</span>')
    .replace(/:\s*(-?\d+\.?\d*)/g, ': <span class="json-number">$1</span>');
}

async function main() {
  // Wait for Trunk's auto-injected WASM loader
  while (!window.wasmBindings) {
    await new Promise(resolve => setTimeout(resolve, 10));
  }

  const { WasmApp } = window.wasmBindings;
  wasmApp = new WasmApp();

  hookEvents();
  const snap = snapshot();
  render(snap);
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

function hookEvents() {
  const filter = document.getElementById("nearx-filter");
  const blocksContent = document.getElementById("blocks-content");
  const txContent = document.getElementById("txs-content");
  const ownedToggle = document.getElementById("nearx-owned-toggle");
  const details = document.getElementById("pane-details");

  if (!filter || !blocksContent || !txContent || !details) {
    console.error("[nearx-web-dom] Missing required DOM elements");
    return;
  }

  // Header tab clicks -> FocusPane
  document.querySelectorAll(".nx-tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      const pane = Number(tab.dataset.pane);
      if (!Number.isNaN(pane)) {
        apply({ type: "FocusPane", pane });
      }
    });
  });

  // Filter text -> SetFilter action.
  filter.addEventListener("input", (e) => {
    if (suppressFilterEvent) return;
    const text = e.target.value;
    apply({ type: "SetFilter", text });
  });

  filter.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      e.preventDefault();
      filter.blur();
    }
  });

  // Mouse focus: click on a pane content focuses it.
  blocksContent.addEventListener("mousedown", () => {
    apply({ type: "FocusPane", pane: 0 });
  });

  txContent.addEventListener("mousedown", () => {
    apply({ type: "FocusPane", pane: 1 });
  });

  // Owned-only toggle button.
  if (ownedToggle) {
    ownedToggle.addEventListener("click", () => {
      apply({ type: "ToggleOwnedOnly" });
    });
  }

  // Global keyboard navigation.
  document.addEventListener("keydown", (e) => {
    // '/' focuses filter input.
    if (e.key === "/") {
      if (e.ctrlKey || e.metaKey || e.altKey) return;
      e.preventDefault();
      filter.focus();
      filter.select();
      return;
    }

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
      " ", // Space
      "j",
      "k",
      "h",
      "l",
      "J",
      "K",
      "H",
      "L",
      "u",
      "U",
      "Escape",
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
  blocksContent.addEventListener("click", (e) => {
    const row = e.target.closest("[data-index]");
    if (!row) return;
    const index = Number(row.dataset.index);
    if (Number.isNaN(index)) return;
    apply({ type: "SelectBlock", index });
  });

  // Row clicks (txs).
  txContent.addEventListener("click", (e) => {
    const row = e.target.closest("[data-index]");
    if (!row) return;
    const index = Number(row.dataset.index);
    if (Number.isNaN(index)) return;
    apply({ type: "SelectTx", index });
  });

  // Details pane scroll is native; no extra wiring needed.
}

function render(snapshot) {
  const filter = document.getElementById("nearx-filter");
  const blocksContent = document.getElementById("blocks-content");
  const txContent = document.getElementById("txs-content");
  const details = document.getElementById("pane-details");
  const toastEl = document.getElementById("nearx-toast");
  const ownedToggle = document.getElementById("nearx-owned-toggle");
  const blocksTitle = document.getElementById("blocks-title");
  const txsTitle = document.getElementById("txs-title");
  const detailsTitle = document.getElementById("details-title");
  const fpsDisplay = document.getElementById("fps-display");
  const ownedBadge = document.getElementById("owned-badge");

  if (!filter || !blocksContent || !txContent || !details) return;

  // Filter text.
  suppressFilterEvent = true;
  filter.value = snapshot.filter_query || "";
  suppressFilterEvent = false;

  // Owned-only button state.
  if (ownedToggle) {
    if (snapshot.owned_only_filter) {
      ownedToggle.classList.add("nx-owned--active");
    } else {
      ownedToggle.classList.remove("nx-owned--active");
    }
  }

  // Header tabs active state
  document.querySelectorAll(".nx-tab").forEach((tab) => {
    const pane = Number(tab.dataset.pane);
    tab.classList.toggle("nx-tab--active", pane === snapshot.pane);
  });

  // Pane focus borders (focused pane gets accent border)
  const blocksPane = document.getElementById("pane-blocks");
  const txPane = document.getElementById("pane-txs");
  const detailsPane = document.querySelector(".nx-pane--details");
  if (blocksPane) blocksPane.classList.toggle("nx-pane--focused", snapshot.pane === 0);
  if (txPane) txPane.classList.toggle("nx-pane--focused", snapshot.pane === 1);
  if (detailsPane) detailsPane.classList.toggle("nx-pane--focused", snapshot.pane === 2);

  // Dynamic titles (from UiSnapshot)
  if (blocksTitle) blocksTitle.textContent = snapshot.blocks_title || "Blocks";
  if (txsTitle) txsTitle.textContent = snapshot.txs_title || "Txs";
  if (detailsTitle) detailsTitle.textContent = snapshot.details_title || "Transaction details";

  // Blocks pane content.
  blocksContent.innerHTML = "";
  snapshot.blocks.forEach((b) => {
    const row = document.createElement("div");
    row.className = "nx-row nx-row--block";
    if (b.is_selected) row.classList.add("nx-row--selected");
    row.dataset.index = String(b.index);

    // Show owned badge (★n) like TUI
    const ownedBadge = b.owned_tx_count > 0 ? ` ★${b.owned_tx_count}` : "";
    row.textContent = `${b.height}  | ${b.tx_count} txs${ownedBadge} | ${b.when}`;
    blocksContent.appendChild(row);
  });

  // Txs pane content.
  txContent.innerHTML = "";
  snapshot.txs.forEach((t) => {
    const row = document.createElement("div");
    row.className = "nx-row nx-row--tx";
    if (t.is_selected) row.classList.add("nx-row--selected");
    if (t.is_owned) row.classList.add("nx-row--owned");
    row.dataset.index = String(t.index);

    const signer = t.signer_id || "";
    const receiver = t.receiver_id || "";
    const label =
      signer && receiver
        ? `${signer} → ${receiver}`
        : signer || receiver || t.hash;

    row.textContent = label;
    txContent.appendChild(row);
  });

  // Details pane (show loading state if archival fetch in progress).
  if (snapshot.loading_block) {
    details.textContent = `⏳ Loading block #${snapshot.loading_block} from archival...\n\nThis may take 1-2 seconds.\n\nNavigate away to cancel.`;
  } else {
    // Apply JSON syntax highlighting
    const jsonText = snapshot.details || "";
    details.innerHTML = highlightJSON(jsonText);
  }

  if (snapshot.details_fullscreen) {
    details.classList.add("nx-details--fullscreen");
  } else {
    details.classList.remove("nx-details--fullscreen");
  }

  // Footer: FPS display
  if (fpsDisplay) {
    fpsDisplay.textContent = `• FPS ${snapshot.fps}`;
  }

  // Footer: Owned-only badge
  if (ownedBadge) {
    if (snapshot.owned_only_filter) {
      ownedBadge.hidden = false;
    } else {
      ownedBadge.hidden = true;
    }
  }

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
}

document.addEventListener("DOMContentLoaded", () => {
  main().catch((err) => {
    console.error("[nearx-web-dom] Failed to start:", err);
  });
});
