// DOM frontend for NEARx using WasmApp.
//
// Expects the wasm-bindgen JS glue for the `nearx-web-dom` binary to be
// available as `nearx-web-dom.js` in the same directory.
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

import init, { WasmApp } from "./nearx-web-dom.js";

let wasmApp = null;
let lastSnapshot = null;
let suppressFilterEvent = false;

async function main() {
  await init();
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

  // Mouse focus: click on a pane focuses it.
  blocksPane.addEventListener("mousedown", () => {
    apply({ type: "FocusPane", pane: 0 });
  });

  txPane.addEventListener("mousedown", () => {
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

  // Details pane scroll is native; no extra wiring needed.
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
    } else {
      ownedToggle.classList.remove("nx-owned--active");
    }
  }

  // Pane focus (0=blocks,1=txs,2=details).
  blocksPane.classList.toggle("nx-pane--focused", snapshot.pane === 0);
  txPane.classList.toggle("nx-pane--focused", snapshot.pane === 1);
  details.classList.toggle("nx-pane--focused", snapshot.pane === 2);

  // Blocks pane.
  blocksPane.innerHTML = "";
  snapshot.blocks.forEach((b) => {
    const row = document.createElement("div");
    row.className = "nx-row nx-row--block";
    if (b.is_selected) row.classList.add("nx-row--selected");
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
    txPane.appendChild(row);
  });

  // Details pane.
  details.textContent = snapshot.details || "";

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
