// DOM frontend for Ratacat/NEARx using WasmApp.
//
// Requires wasm-bindgen output under ./pkg/nearx_web_dom.js:
//
//   cargo build --bin nearx-web-dom --features web-dom \
//     --target wasm32-unknown-unknown
//
//   wasm-bindgen \
//     --target web \
//     --no-typescript \
//     --out-dir web/pkg \
//     --out-name nearx_web_dom \
//     target/wasm32-unknown-unknown/debug/nearx-web-dom.wasm
//
// Then open web/index.html in a browser (or via Tauri).

import init, * as wasm from "./pkg/nearx_web_dom.js";

let wasmApp = null;
let lastSnapshot = null;
let suppressFilterEvent = false;

async function main() {
  await init();

  // Make wasm exports available for router_shim.js (nearx_auth_callback).
  window.wasm_bindgen = wasm;

  wasmApp = new wasm.WasmApp();

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
  const blocksPane = document.getElementById("pane-blocks");
  const txPane = document.getElementById("pane-txs");
  const ownedToggle = document.getElementById("nearx-owned-toggle");
  const details = document.getElementById("pane-details");

  if (!filter || !blocksPane || !txPane || !details) {
    console.error("[nearx-web-dom] Missing required DOM elements");
    return;
  }

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

  // Mouse focus.
  blocksPane.addEventListener("mousedown", () => {
    apply({ type: "FocusPane", pane: 0 });
  });

  txPane.addEventListener("mousedown", () => {
    apply({ type: "FocusPane", pane: 1 });
  });

  details.addEventListener("mousedown", () => {
    apply({ type: "FocusPane", pane: 2 });
  });

  // Owned-only toggle.
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

    // 'c' → copy JSON via copy_api (pane-aware).
    if (e.key === "c" || e.key === "C") {
      if (e.ctrlKey || e.metaKey) {
        // On web, Ctrl+C is used for browser copy; keep it simple:
        // plain 'c' copies, Ctrl+C is normal browser copy.
        return;
      }
      e.preventDefault();
      apply({ type: "CopyFocusedJson" });
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
      " ",
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
  blocksPane.addEventListener("click", (e) => {
    const row = e.target.closest("[data-index]");
    if (!row) return;
    const index = Number(row.dataset.index);
    if (Number.isNaN(index)) return;
    apply({ type: "SelectBlock", index });
  });

  // Row clicks (txs).
  txPane.addEventListener("click", (e) => {
    const row = e.target.closest("[data-index]");
    if (!row) return;
    const index = Number(row.dataset.index);
    if (Number.isNaN(index)) return;
    apply({ type: "SelectTx", index });
  });
}

function render(snapshot) {
  const filter = document.getElementById("nearx-filter");
  const blocksPane = document.getElementById("pane-blocks");
  const txPane = document.getElementById("pane-txs");
  const details = document.getElementById("pane-details");
  const toastEl = document.getElementById("nearx-toast");
  const ownedToggle = document.getElementById("nearx-owned-toggle");

  if (!filter || !blocksPane || !txPane || !details) return;

  // Filter text.
  suppressFilterEvent = true;
  filter.value = snapshot.filter_query || "";
  suppressFilterEvent = false;

  // Owned-only button state.
  if (ownedToggle) {
    if (snapshot.owned_only_filter) {
      ownedToggle.classList.add("nx-owned--active");
      ownedToggle.setAttribute("aria-pressed", "true");
    } else {
      ownedToggle.classList.remove("nx-owned--active");
      ownedToggle.setAttribute("aria-pressed", "false");
    }
  }

  // Pane focus.
  blocksPane.classList.toggle("nx-pane--focused", snapshot.pane === 0);
  txPane.classList.toggle("nx-pane--focused", snapshot.pane === 1);
  details.classList.toggle("nx-pane--focused", snapshot.pane === 2);

  // Blocks pane.
  blocksPane.innerHTML = "";
  snapshot.blocks.forEach((b) => {
    const row = document.createElement("div");
    row.className = "nx-row nx-row--block";
    row.setAttribute("role", "listitem");
    if (b.is_selected) {
      row.classList.add("nx-row--selected");
      row.setAttribute("aria-selected", "true");
    }
    row.dataset.index = String(b.index);

    const ownedSuffix =
      b.owned_tx_count && b.owned_tx_count > 0
        ? ` · ${b.owned_tx_count} owned`
        : "";

    row.textContent = `#${b.height} · ${b.tx_count} tx${ownedSuffix} · ${b.when}`;
    blocksPane.appendChild(row);
  });

  // Txs pane.
  txPane.innerHTML = "";
  snapshot.txs.forEach((t) => {
    const row = document.createElement("div");
    row.className = "nx-row nx-row--tx";
    row.setAttribute("role", "listitem");
    if (t.is_selected) {
      row.classList.add("nx-row--selected");
      row.setAttribute("aria-selected", "true");
    }
    if (t.is_owned) row.classList.add("nx-row--owned");
    row.dataset.index = String(t.index);

    const signer = t.signer_id || "";
    const receiver = t.receiver_id || "";
    const label =
      signer && receiver
        ? `${signer} → ${receiver}`
        : signer || receiver || t.hash;

    row.textContent = label;
    txPane.appendChild(row);
  });

  // Details pane (show loading state if archival fetch in progress).
  if (snapshot.loading_block) {
    details.textContent = `⏳ Loading block #${snapshot.loading_block} from archival...\n\nThis may take 1-2 seconds.\n\nNavigate away to cancel.`;
  } else {
    details.textContent = snapshot.details || "";
  }

  if (snapshot.details_fullscreen) {
    details.classList.add("nx-details--fullscreen");
  } else {
    details.classList.remove("nx-details--fullscreen");
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
