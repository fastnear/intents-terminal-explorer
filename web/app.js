/**
 * NEARx Web App - DOM Renderer
 *
 * Renders UI from Rust snapshots using native DOM elements.
 * One-way data flow: Rust state → JS rendering → DOM events → Rust actions
 */

import init, { WasmApp } from '../pkg/nearx_web_dom.js';

let app = null;
let renderLoopId = null;

// ----- Initialization -----

async function main() {
    console.log('🦀 Initializing NEARx Web (DOM)...');

    // Initialize WASM module
    await init();

    // Create app instance
    app = new WasmApp();
    console.log('✅ WasmApp created');

    // Wire up event handlers
    setupEventHandlers();

    // Start render loop (30 FPS)
    startRenderLoop();

    console.log('🚀 NEARx ready!');

    // Hide loading screen
    const loading = document.querySelector('.loading');
    if (loading) loading.classList.add('hidden');

    // Update status
    if (window.updateStatus) {
        window.updateStatus('Connected to mainnet RPC', 'connected');
    }
}

// ----- Render Loop -----

function startRenderLoop() {
    const fps = 30;
    const frameTime = 1000 / fps;

    function loop() {
        const snapshotJson = app.tick();
        const snapshot = JSON.parse(snapshotJson);
        render(snapshot);
        renderLoopId = setTimeout(loop, frameTime);
    }

    loop();
}

// ----- Rendering -----

function render(snapshotValue) {
    const snap = typeof snapshotValue === 'string'
        ? JSON.parse(snapshotValue)
        : snapshotValue;

    renderTopBar(snap);
    renderBlocks(snap);
    renderTxs(snap);
    renderDetails(snap);
    renderToast(snap);
}

function renderTopBar(snap) {
    const container = document.getElementById('top-bar');
    if (!container) return;

    // Filter input
    const filterInput = document.getElementById('filter-input');
    if (filterInput && filterInput.value !== snap.filter.text) {
        filterInput.value = snap.filter.text;
    }

    // Auth buttons
    const authContainer = document.getElementById('auth-container');
    if (authContainer) {
        if (snap.auth.signed_in) {
            authContainer.innerHTML = `
                <span class="auth-email">${snap.auth.email || 'Signed in'}</span>
                <button id="sign-out-btn" class="btn-secondary">Sign out</button>
            `;
        } else {
            authContainer.innerHTML = `
                <button id="sign-in-google-btn" class="btn-primary">Sign in with Google</button>
            `;
        }
    }
}

function renderBlocks(snap) {
    const container = document.getElementById('pane-blocks');
    if (!container) return;

    const focused = snap.focused_pane === 0;
    container.className = `pane pane-blocks ${focused ? 'pane--focused' : ''}`;

    // Header
    const header = container.querySelector('.pane-header') || document.createElement('div');
    header.className = 'pane-header';
    const filterSuffix = snap.blocks.total_count > snap.blocks.rows.length
        ? ` (${snap.blocks.rows.length} / ${snap.blocks.total_count})`
        : ` (${snap.blocks.total_count})`;
    header.textContent = `Blocks${filterSuffix}`;
    if (!container.querySelector('.pane-header')) {
        container.insertBefore(header, container.firstChild);
    }

    // Rows
    let rowsContainer = container.querySelector('.pane-rows');
    if (!rowsContainer) {
        rowsContainer = document.createElement('div');
        rowsContainer.className = 'pane-rows';
        container.appendChild(rowsContainer);
    }

    rowsContainer.innerHTML = '';
    snap.blocks.rows.forEach((block, index) => {
        const row = document.createElement('div');
        row.className = 'row' + (index === snap.blocks.selected_index ? ' row--selected' : '');
        row.dataset.index = index;
        row.textContent = `#${block.height}  ·  ${block.tx_count} txs  ·  ${block.time_utc}`;
        rowsContainer.appendChild(row);
    });
}

function renderTxs(snap) {
    const container = document.getElementById('pane-txs');
    if (!container) return;

    const focused = snap.focused_pane === 1;
    container.className = `pane pane-txs ${focused ? 'pane--focused' : ''}`;

    // Header
    const header = container.querySelector('.pane-header') || document.createElement('div');
    header.className = 'pane-header';
    const filterSuffix = snap.txs.total_count > snap.txs.rows.length
        ? ` (${snap.txs.rows.length} / ${snap.txs.total_count})`
        : ` (${snap.txs.total_count})`;
    header.textContent = `Transactions${filterSuffix}`;
    if (!container.querySelector('.pane-header')) {
        container.insertBefore(header, container.firstChild);
    }

    // Rows
    let rowsContainer = container.querySelector('.pane-rows');
    if (!rowsContainer) {
        rowsContainer = document.createElement('div');
        rowsContainer.className = 'pane-rows';
        container.appendChild(rowsContainer);
    }

    rowsContainer.innerHTML = '';
    snap.txs.rows.forEach((tx, index) => {
        const row = document.createElement('div');
        row.className = 'row' + (index === snap.txs.selected_index ? ' row--selected' : '');
        row.dataset.index = index;
        row.textContent = `${tx.hash}  ·  ${tx.signer_id}  ·  ${tx.action_summary}`;
        rowsContainer.appendChild(row);
    });
}

function renderDetails(snap) {
    const container = document.getElementById('pane-details');
    if (!container) return;

    const focused = snap.focused_pane === 2;
    container.className = `pane pane-details ${focused ? 'pane--focused' : ''}`;

    if (snap.details.fullscreen) {
        container.classList.add('pane--fullscreen');
    } else {
        container.classList.remove('pane--fullscreen');
    }

    // Header
    const header = container.querySelector('.pane-header') || document.createElement('div');
    header.className = 'pane-header';
    header.textContent = snap.details.fullscreen
        ? 'Transaction Details (Press Space to exit fullscreen)'
        : 'Transaction Details';
    if (!container.querySelector('.pane-header')) {
        container.insertBefore(header, container.firstChild);
    }

    // Content
    let content = container.querySelector('.details-content');
    if (!content) {
        content = document.createElement('pre');
        content.className = 'details-content';
        container.appendChild(content);
    }

    content.textContent = snap.details.json;
}

function renderToast(snap) {
    let toast = document.getElementById('toast');
    if (!toast) {
        toast = document.createElement('div');
        toast.id = 'toast';
        toast.className = 'toast';
        document.body.appendChild(toast);
    }

    if (snap.toast) {
        toast.textContent = snap.toast;
        toast.classList.add('toast--visible');
    } else {
        toast.classList.remove('toast--visible');
    }
}

// ----- Event Handlers -----

function setupEventHandlers() {
    // Filter input
    document.addEventListener('input', (e) => {
        if (e.target.id === 'filter-input') {
            dispatch({ type: 'UpdateFilterText', text: e.target.value });
        }
    });

    document.addEventListener('keydown', (e) => {
        // Filter: Enter to apply
        if (e.target.id === 'filter-input' && e.key === 'Enter') {
            const text = e.target.value;
            dispatch({ type: 'ApplyFilter', text });
            e.preventDefault();
        }
    });

    // Global keyboard shortcuts
    document.addEventListener('keydown', (e) => {
        // Ignore if typing in filter
        if (e.target.id === 'filter-input') return;

        const key = e.key;
        const ctrl = e.ctrlKey || e.metaKey;
        const shift = e.shiftKey;

        // Tab cycling
        if (key === 'Tab') {
            dispatch(shift ? { type: 'PrevPane' } : { type: 'NextPane' });
            e.preventDefault();
            return;
        }

        // Focus filter
        if (key === '/' || (ctrl && key === 'f')) {
            document.getElementById('filter-input')?.focus();
            e.preventDefault();
            return;
        }

        // Copy
        if (key === 'c' && !ctrl) {
            dispatch({ type: 'CopyFocusedJson' });
            e.preventDefault();
            return;
        }

        // Space for fullscreen (only in details pane)
        if (key === ' ') {
            dispatch({ type: 'ToggleDetailsFullscreen' });
            e.preventDefault();
            return;
        }

        // Arrow keys
        if (key === 'ArrowUp') {
            dispatch({ type: 'BlockUp' });
            e.preventDefault();
        } else if (key === 'ArrowDown') {
            dispatch({ type: 'BlockDown' });
            e.preventDefault();
        } else if (key === 'PageUp') {
            dispatch({ type: 'BlockPageUp' });
            e.preventDefault();
        } else if (key === 'PageDown') {
            dispatch({ type: 'BlockPageDown' });
            e.preventDefault();
        } else if (key === 'Home') {
            dispatch({ type: 'BlockHome' });
            e.preventDefault();
        } else if (key === 'End') {
            dispatch({ type: 'BlockEnd' });
            e.preventDefault();
        }
    });

    // Click handlers (event delegation)
    document.addEventListener('click', (e) => {
        // Auth buttons
        if (e.target.id === 'sign-in-google-btn') {
            dispatch({ type: 'SignInGoogle' });
            return;
        }
        if (e.target.id === 'sign-out-btn') {
            dispatch({ type: 'SignOut' });
            return;
        }

        // Pane focus + row selection
        const pane = e.target.closest('.pane');
        if (pane) {
            // Focus pane
            const paneId = pane.id;
            if (paneId === 'pane-blocks') {
                dispatch({ type: 'FocusPane', pane: 0 });
            } else if (paneId === 'pane-txs') {
                dispatch({ type: 'FocusPane', pane: 1 });
            } else if (paneId === 'pane-details') {
                dispatch({ type: 'FocusPane', pane: 2 });
            }

            // Row selection
            const row = e.target.closest('.row');
            if (row) {
                const index = parseInt(row.dataset.index, 10);
                if (paneId === 'pane-blocks') {
                    dispatch({ type: 'SelectBlock', index });
                } else if (paneId === 'pane-txs') {
                    dispatch({ type: 'SelectTx', index });
                }
            }
        }
    });

    // Double-click for fullscreen
    document.addEventListener('dblclick', (e) => {
        if (e.target.closest('#pane-details')) {
            dispatch({ type: 'ToggleDetailsFullscreen' });
        }
    });
}

function dispatch(action) {
    if (!app) return;
    const actionJson = JSON.stringify(action);
    const updatedJson = app.handle_action_json(actionJson);
    const updated = JSON.parse(updatedJson);
    render(updated);
}

// ----- Start -----

main().catch(err => {
    console.error('❌ Failed to initialize NEARx:', err);
    if (window.updateStatus) {
        window.updateStatus(`Error: ${err.message}`, 'error');
    }
});
