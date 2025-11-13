// Native binary for Ratacat - Terminal UI mode

use anyhow::{Context, Result};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    collections::HashSet,
    io,
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::task::JoinHandle;

use nearx::{
    app::{App, InputMode},
    archival_fetch,
    config::{load, Source},
    credentials,
    marks::JumpMarks,
    platform::{BlockPersist, History, TxPersist},
    source_rpc, source_ws,
    types::AppEvent,
    ui,
    util::dblclick::DblClick,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if it exists (safe to ignore if not found)
    #[cfg(feature = "native")]
    {
        let _ = dotenvy::dotenv();
    }

    let cfg = load().context("Failed to load configuration")?;

    // Initialize SQLite history (non-blocking)
    let db_path = std::env::var("SQLITE_DB_PATH").unwrap_or_else(|_| "./nearx_history.db".into());
    let history = History::start(&db_path)?;

    // Start credentials watcher
    let (creds_tx, creds_rx) = unbounded_channel::<HashSet<String>>();
    let creds_base = std::env::var("NEAR_CREDENTIALS_DIR")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".near-credentials"))
        })
        .unwrap_or_else(|| PathBuf::from(".near-credentials"));
    let network = std::env::var("NEAR_NETWORK").unwrap_or_else(|_| "mainnet".into());

    // Start watcher (don't fail if directory doesn't exist - it will be created)
    tokio::spawn(async move {
        let _ = credentials::start_credentials_watcher(creds_base, network, creds_tx).await;
    });

    // terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // app + channels
    let (tx, rx) = unbounded_channel::<AppEvent>();

    // Archival fetch channel (optional, only if archival URL configured)
    let (archival_tx, archival_rx) = unbounded_channel::<u64>();
    let archival_task: Option<JoinHandle<Result<()>>> = if cfg.archival_rpc_url.is_some() {
        let cfg_arch = cfg.clone();
        let tx_arch = tx.clone();
        Some(tokio::spawn(async move {
            archival_fetch::run_archival_fetch(cfg_arch, archival_rx, tx_arch).await
        }))
    } else {
        None
    };

    let mut app = App::new(
        cfg.render_fps,
        cfg.render_fps_choices.clone(),
        cfg.keep_blocks,
        cfg.default_filter.clone(),
        if cfg.archival_rpc_url.is_some() {
            Some(archival_tx)
        } else {
            None
        },
    );

    // Apply deep link route from CLI args (if provided)
    // Example: ./nearx nearx://v1/tx/ABC123
    {
        let args: Vec<String> = std::env::args().collect();
        for arg in args.iter().skip(1) {
            // Check if argument looks like a deep link
            if arg.starts_with("nearx://") || arg.starts_with("/v1/") || arg.contains("#/v1/") {
                if let Some(route) = nearx::router::parse(arg) {
                    app.apply_route(&route);
                    log::info!("Applied deep link route from CLI: {arg}");
                    break; // Only process first route
                }
            }
        }
    }

    // source task
    let cfg_clone = cfg.clone();
    let history_clone_tx = tx.clone();
    let source_task: JoinHandle<Result<()>> = match cfg.source {
        Source::Ws => {
            tokio::spawn(async move { source_ws::run_ws(&cfg_clone, history_clone_tx).await })
        }
        Source::Rpc => {
            tokio::spawn(async move { source_rpc::run_rpc(&cfg_clone, history_clone_tx).await })
        }
    };

    // jump marks
    let mut jump_marks = JumpMarks::new(history.clone());
    jump_marks.load_from_persistence().await;

    // main loop
    let mouse_enabled =
        run_loop(&mut app, &mut terminal, rx, history, jump_marks, creds_rx).await?;

    // cleanup
    source_task.abort();
    if let Some(task) = archival_task {
        task.abort();
    }
    if mouse_enabled {
        execute!(terminal.backend_mut(), DisableMouseCapture)?;
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

async fn run_loop(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut rx: UnboundedReceiver<AppEvent>,
    history: History,
    mut jump_marks: JumpMarks,
    mut creds_rx: UnboundedReceiver<HashSet<String>>,
) -> Result<bool> {
    let mut last_frame = Instant::now();
    let mut mouse_enabled = false;
    let mut dbl = DblClick::new(Duration::from_millis(280));

    loop {
        // frame budget (coalesced renders)
        let frame_ms = 1000u32.saturating_div(app.fps()) as u64;
        let budget = Duration::from_millis(frame_ms.max(1));
        let wait = budget.saturating_sub(last_frame.elapsed());

        // input or source events
        if event::poll(wait)? {
            match event::read()? {
                Event::Key(k) => {
                    if k.kind == KeyEventKind::Press || k.kind == KeyEventKind::Repeat {
                        // Check for mouse toggle before other handling
                        if let (KeyCode::Char('m'), KeyModifiers::CONTROL) = (k.code, k.modifiers) {
                            mouse_enabled = !mouse_enabled;
                            if mouse_enabled {
                                execute!(terminal.backend_mut(), EnableMouseCapture)?;
                                app.show_toast("Mouse enabled (Ctrl+M to disable)".to_string());
                            } else {
                                execute!(terminal.backend_mut(), DisableMouseCapture)?;
                                app.show_toast("Mouse disabled".to_string());
                            }
                        } else {
                            handle_key(app, k, &history, &mut jump_marks).await;
                        }
                    }
                }
                Event::Mouse(m) => {
                    if mouse_enabled && app.ui_flags().mouse_map {
                        handle_mouse(app, m, terminal, &mut dbl)?;
                    }
                }
                _ => {}
            }
        }
        while let Ok(ev) = rx.try_recv() {
            // Persist blocks to history
            if let AppEvent::NewBlock(ref block) = ev {
                let persist = BlockPersist {
                    height: block.height,
                    hash: block.hash.clone(),
                    ts_ms: block.timestamp as i64,
                    txs: block
                        .transactions
                        .iter()
                        .map(|tx| TxPersist {
                            hash: tx.hash.clone(),
                            height: block.height,
                            signer: None,
                            receiver: None,
                            actions_json: None,
                            raw_json: Some(
                                serde_json::to_string(&serde_json::json!({"hash": tx.hash}))
                                    .unwrap_or_default(),
                            ),
                        })
                        .collect(),
                };
                history.persist_block(persist);
            }
            app.on_event(ev);
        }

        // Handle credential updates
        while let Ok(accounts) = creds_rx.try_recv() {
            app.set_owned_accounts(accounts);
        }

        if last_frame.elapsed() >= budget {
            let marks_list = jump_marks.list();
            terminal.draw(|f| ui::draw(f, app, &marks_list))?;
            last_frame = Instant::now();
        }
        if app.quit_flag() {
            break;
        }
    }
    Ok(mouse_enabled)
}

fn handle_mouse(
    app: &mut App,
    mouse: MouseEvent,
    terminal: &Terminal<CrosstermBackend<io::Stdout>>,
    dbl: &mut DblClick,
) -> Result<()> {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let (col, row) = (mouse.column, mouse.row);
            let size = terminal.size()?;

            let mid_row = (size.height as i32) / 2;
            let mid_col = (size.width as i32) / 2;

            // Click detection: same layout as Web/Tauri
            // Top half: Blocks (left) and Txs (right)
            // Bottom half: Details (full width)
            if (row as i32) >= mid_row {
                // Details pane - check for double-click
                // Only if Details is already focused (pane index 2)
                if app.pane() == 2 && dbl.register(col, row) {
                    // Double-click detected! Toggle fullscreen details
                    app.toggle_details_fullscreen();
                    app.log_debug("Mouse double-click → toggle details fullscreen".to_string());
                    return Ok(()); // Skip normal click handling
                }

                // Single click - focus Details pane
                app.set_pane_direct(2);
                app.log_debug("Mouse select Details pane".to_string());
            } else if (col as i32) < mid_col {
                // Blocks pane (top-left)
                app.set_pane_direct(0);
                // Account for header rows (typically 2-3 rows)
                let idx = (row as i32 - 2).max(0) as usize;
                app.select_block_row(idx);
                app.log_debug(format!("Mouse select Blocks pane, row {idx}"));
            } else {
                // Transactions pane (top-right)
                app.set_pane_direct(1);
                let idx = (row as i32 - 2).max(0) as usize;
                app.select_tx_row(idx);
                app.log_debug(format!("Mouse select Txs pane, row {idx}"));
            }
        }
        MouseEventKind::ScrollUp => {
            // Scroll up in current pane
            app.page_up(3);
        }
        MouseEventKind::ScrollDown => {
            // Scroll down in current pane
            app.page_down(3);
        }
        _ => {}
    }
    Ok(())
}

async fn handle_key(app: &mut App, k: KeyEvent, history: &History, jump_marks: &mut JumpMarks) {
    // Handle filter input mode separately
    if app.input_mode() == InputMode::Filter {
        match k.code {
            KeyCode::Char(c) => app.filter_add_char(c),
            KeyCode::Backspace => app.filter_backspace(),
            KeyCode::Enter => app.apply_filter(),
            KeyCode::Esc => app.clear_filter(),
            _ => {}
        }
        return;
    }

    // Handle search input mode
    if app.input_mode() == InputMode::Search {
        match k.code {
            KeyCode::Char(c) => app.search_add_char(c),
            KeyCode::Backspace => app.search_backspace(),
            KeyCode::Enter => {
                // If results exist and one is selected, open it
                if let Some(hit) = app.get_selected_search_result() {
                    let hash = hit.hash.clone();
                    if let Some(raw_json) = history.get_tx(hash).await {
                        app.display_tx_from_json(&raw_json);
                        app.close_search();
                    }
                } else {
                    // Run search
                    let query = app.search_query().to_string();
                    let results = history.search(query, 200).await;
                    app.set_search_results(results);
                }
            }
            KeyCode::Up => app.search_up(),
            KeyCode::Down => app.search_down(),
            KeyCode::Esc => app.close_search(),
            _ => {}
        }
        return;
    }

    // Handle marks overlay mode
    if app.input_mode() == InputMode::Marks {
        match k.code {
            KeyCode::Up => app.marks_up(),
            KeyCode::Down => app.marks_down(),
            KeyCode::Enter => {
                // Jump to selected mark
                if let Some(mark) = app.get_selected_mark().cloned() {
                    app.jump_to_mark(&mark);
                    app.close_marks();
                }
            }
            KeyCode::Char('d') => {
                // Delete selected mark
                if let Some(mark) = app.get_selected_mark() {
                    let label = mark.label.clone();
                    jump_marks.remove_by_label(&label).await;
                    // Reload marks list
                    let marks_list = jump_marks.list();
                    app.open_marks(marks_list);
                }
            }
            KeyCode::Esc => app.close_marks(),
            _ => {}
        }
        return;
    }

    // Handle jump-pending mode (waiting for label character)
    if app.input_mode() == InputMode::JumpPending {
        if let KeyCode::Char(c) = k.code {
            let label = c.to_string();
            if let Some(mark) = jump_marks.get_by_label(&label) {
                app.jump_to_mark(mark);
            }
        }
        app.close_marks(); // Exit jump-pending mode
        return;
    }

    // Normal mode keys
    // Focus pattern: Tab/Shift+Tab switch between panes (0=Blocks, 1=Txs, 2=Details)
    // Arrow keys only affect the currently focused pane
    match (k.code, k.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            app.on_event(AppEvent::Quit);
        }

        // Pane focus switching (circular navigation)
        (KeyCode::Tab, _) => {
            app.log_debug(format!("KEY: Tab pressed, pane={}", app.pane()));
            app.next_pane();
        }
        (KeyCode::BackTab, _) => {
            app.log_debug(format!("KEY: BackTab pressed, pane={}", app.pane()));
            app.prev_pane();
        }

        // Navigation within focused pane
        (KeyCode::Up, _) => {
            app.log_debug(format!("KEY: Up pressed, pane={}", app.pane()));
            app.up();
        }
        (KeyCode::Down, _) => {
            app.log_debug(format!("KEY: Down pressed, pane={}", app.pane()));
            app.down();
        }
        (KeyCode::Left, _) => app.left(), // Jump to top of current list
        (KeyCode::Right, _) => app.right(), // Paginate down 12 items
        (KeyCode::PageUp, _) => app.page_up(20),
        (KeyCode::PageDown, _) => app.page_down(20),
        (KeyCode::Home, _) => {
            app.log_debug(format!("KEY: Home pressed, pane={}", app.pane()));
            if app.pane() == 0 {
                // Home in blocks pane: return to auto-follow
                app.return_to_auto_follow();
            } else {
                app.home();
            }
        }
        (KeyCode::End, _) => app.end(),
        (KeyCode::Enter, _) => app.select_tx(),
        (KeyCode::Char(' '), _) => app.toggle_details_fullscreen(), // Spacebar to toggle fullscreen
        (KeyCode::Char('o'), KeyModifiers::CONTROL) => app.cycle_fps(),
        (KeyCode::Char('c'), _) => {
            // Copy content using unified copy_api (pane-aware)
            if nearx::copy_api::copy_current(app) {
                let msg = match app.pane() {
                    0 => "Copied block info".to_string(),
                    1 => "Copied tx hash".to_string(),
                    2 => "Copied details".to_string(),
                    _ => "Copied".to_string(),
                };
                app.show_toast(msg);
            } else {
                app.show_toast("Copy failed".to_string());
            }
        }
        (KeyCode::Char('f'), KeyModifiers::CONTROL) => app.start_search(),
        (KeyCode::Char('/'), _) | (KeyCode::Char('f'), _) => app.start_filter(),
        (KeyCode::Esc, _) => app.clear_filter(),
        // Jump marks
        (KeyCode::Char('m'), _) => {
            // Set mark with auto-label
            let label = jump_marks.next_auto_label();
            let (pane, height, tx_hash) = app.current_context();
            jump_marks
                .add_or_replace(label, pane, height, tx_hash)
                .await;
        }
        (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
            // Pin/unpin current context
            let (pane, height, tx_hash) = app.current_context();

            // Check if mark exists at this context
            if let Some(label) = jump_marks.find_by_context(pane, height, tx_hash.as_deref()) {
                // Toggle pin on existing mark
                jump_marks.toggle_pin(&label).await;
            } else {
                // Create new auto-labeled mark and pin it
                let label = jump_marks.next_auto_label();
                jump_marks
                    .add_or_replace(label.clone(), pane, height, tx_hash)
                    .await;
                jump_marks.set_pinned(&label, true).await;
            }
        }
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
            // Toggle owned-only filter
            app.toggle_owned_filter();
        }
        (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
            // Toggle debug panel visibility
            app.toggle_debug_panel();
        }
        (KeyCode::Char('M'), KeyModifiers::SHIFT) => {
            // Open marks overlay
            let marks_list = jump_marks.list();
            app.open_marks(marks_list);
        }
        (KeyCode::Char('\''), _) => {
            // Enter jump-pending mode (wait for label)
            app.start_jump_pending();
        }
        (KeyCode::Char('['), _) => {
            // Jump to previous mark
            if let Some(mark) = jump_marks.prev_mark() {
                app.jump_to_mark(&mark);
            }
        }
        (KeyCode::Char(']'), _) => {
            // Jump to next mark
            if let Some(mark) = jump_marks.next_mark() {
                app.jump_to_mark(&mark);
            }
        }
        _ => {}
    }
}
