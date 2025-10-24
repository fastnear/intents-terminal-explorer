use ratatui::{
    Frame,
    layout::{Layout, Direction, Constraint, Rect},
    widgets::{Block, Borders, BorderType, List, ListItem, ListState, Paragraph, Wrap, Tabs, Clear},
    style::{Style, Color, Modifier},
    text::{Line, Span},
};
use crate::app::{App, InputMode};
use crate::history::HistoryHit;

#[cfg(feature = "native")]
use crate::types::Mark;

#[cfg(not(feature = "native"))]
#[derive(Clone, Debug, Default)]
pub struct Mark {
    pub label: String,
    pub pane: u8,
    pub height: Option<u64>,
    pub tx_hash: Option<String>,
    pub when_ms: i64,
    pub pinned: bool,
}

// ===============================
// Top-level draw
// ===============================
pub fn draw(f:&mut Frame, app:&mut App, marks:&[Mark]){
    // Dynamic chrome: keep only what we need so the body always gets the rest.
    let filter_expanded = app.input_mode() == InputMode::Filter || !app.filter_query().is_empty();
    let show_debug = app.debug_visible() && !app.debug_log().is_empty();

    let mut constraints: Vec<Constraint> = Vec::with_capacity(5);
    constraints.push(Constraint::Length(1));                                // header
    if filter_expanded { constraints.push(Constraint::Length(3)); }         // filter (only when expanded)
    constraints.push(Constraint::Min(0));                                   // body (fills remainder)
    if show_debug { constraints.push(Constraint::Length(3)); }              // debug (auto-collapses)
    constraints.push(Constraint::Length(1));                                // footer

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    let mut idx = 0usize;
    header(f, chunks[idx], app); idx += 1;
    if filter_expanded {
        filter_bar(f, chunks[idx], app); idx += 1;
    }
    body(f, chunks[idx], app); idx += 1;
    if show_debug {
        debug_panel(f, chunks[idx], app); idx += 1;
    }
    footer(f, chunks[idx], app, marks);

    // Overlays render last
    if app.input_mode() == InputMode::Search {
        draw_search_overlay(f, app.search_query(), app.search_results(), app.search_selection());
    }
    if app.input_mode() == InputMode::Marks {
        draw_marks_overlay(f, marks, app.marks_selection());
    }
    if let Some(toast) = app.toast_message() {
        draw_toast_modal(f, toast);
    }
}

// ===============================
// Header / Filter
// ===============================
fn header(f:&mut Frame, area:Rect, app:&App){
    let titles = ["Blocks","Tx Hashes","Tx Details"].map(|t| Line::from(Span::raw(t))).to_vec();
    let tabs = Tabs::new(titles)
        .select(app.pane())
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::BOTTOM).border_type(BorderType::Plain))
        .divider(Span::raw(" | "));
    f.render_widget(tabs, area);
}

fn filter_bar(f:&mut Frame, area:Rect, app:&App){
    let focused = app.input_mode() == InputMode::Filter;
    let filter_text = app.filter_query();

    // Collapsed one-line rule when idle; expanded input box when focused or non-empty
    if area.height <= 1 && !focused && filter_text.is_empty() {
        let rule = Block::default().borders(Borders::BOTTOM).border_type(BorderType::Plain).border_style(Style::default().fg(Color::DarkGray));
        f.render_widget(rule, area);
        return;
    }

    let border_color = if focused { Color::Yellow } else { Color::Gray };
    let hint = "(Press / or f to filter transactions)";
    let text = if filter_text.is_empty() && !focused { hint } else { filter_text };

    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(if focused { Color::Yellow } else { Color::White }))
        .block(Block::default()
            .title(" Filter ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color)));

    f.render_widget(paragraph, area);

    if focused && area.width > 2 {
        // Cursor inside the input box
        let x = area.x + 1 + (filter_text.len().min((area.width.saturating_sub(2)) as usize) as u16);
        let y = area.y + 1;
        f.set_cursor_position((x, y));
    }
}

// ===============================
// Body
// ===============================
fn body(f:&mut Frame, area:Rect, app:&mut App){
    // Show warning if terminal is too small to be usable
    const MIN_WIDTH: u16 = 60;
    const MIN_HEIGHT: u16 = 15;

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let warning_text = format!(
            "Terminal too small!\n\nMinimum size: {}×{}\nCurrent size: {}×{}\n\nPlease resize your terminal.",
            MIN_WIDTH, MIN_HEIGHT, area.width, area.height
        );

        let warning = Paragraph::new(warning_text)
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Color::Red)));

        // Center the warning box
        let vertical_center = Layout::vertical([
            Constraint::Percentage(40),
            Constraint::Length(7),
            Constraint::Percentage(40),
        ]).split(area);

        let horizontal_center = Layout::horizontal([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ]).split(vertical_center[1]);

        f.render_widget(warning, horizontal_center[1]);
        return;
    }

    // Fullscreen details mode (Spacebar toggle when details pane focused)
    if app.details_fullscreen() && app.pane() == 2 {
        render_details_pane(f, area, app);
        return;
    }

    // Responsive layout: stack vertically on narrow terminals (< 80 cols)
    const NARROW_THRESHOLD: u16 = 80;
    let is_narrow = area.width < NARROW_THRESHOLD;

    if is_narrow {
        // Narrow layout: stack all three panes vertically
        // Blocks 20% → Txs 20% → Details 60%
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(60),
            ])
            .split(area);

        render_blocks_pane(f, rows[0], app);
        render_txs_pane(f, rows[1], app);
        render_details_pane(f, rows[2], app);
    } else {
        // Wide layout: (Blocks + Txs) 30% top, Details 70% bottom
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Ratio(3,10), Constraint::Ratio(7,10)])
            .split(area);

        // Top row: split horizontally (40% blocks, 60% txs - tx pane needs more width for signer→receiver)
        let top_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(2,5), Constraint::Ratio(3,5)])
            .split(rows[0]);

        render_blocks_pane(f, top_cols[0], app);
        render_txs_pane(f, top_cols[1], app);
        render_details_pane(f, rows[1], app);
    }
}

// Helper function to render blocks pane
fn render_blocks_pane(f: &mut Frame, area: Rect, app: &App) {
    let focus_color = Color::Yellow;
    let unfocused_color = Color::Gray;

    let (filtered_blocks, sel_block_opt, total) = app.filtered_blocks();
    let mut st_blocks = ListState::default();
    // Only highlight if block is in filtered list
    st_blocks.select(sel_block_opt);

    let items_blocks: Vec<ListItem> = filtered_blocks.iter().map(|b| {
        let owned = app.owned_count(b.height);
        let badge = if owned > 0 { format!(" ★{owned}") } else { String::new() };
        let text = format!("{}  | {} txs{} | {}", b.height, b.tx_count, badge, b.when);

        // Gray out blocks not in cache when viewing cached selection
        let available = app.is_block_height_available(b.height);
        if available {
            ListItem::new(text)
        } else {
            ListItem::new(text).style(Style::default().fg(Color::DarkGray))
        }
    }).collect();

    let blocks_focused = app.pane() == 0;

    // Dynamic title based on filtering and cache state
    let blocks_title = if app.is_viewing_cached_block() {
        " Blocks (cached) · ← Recent ".to_string()
    } else if filtered_blocks.len() < total {
        format!(" Blocks ({} / {}) ", filtered_blocks.len(), total)
    } else {
        " Blocks ".to_string()
    };

    let blocks_widget = List::new(items_blocks)
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD))
        .highlight_symbol("› ")
        .block(Block::default()
            .title(blocks_title)
            .borders(Borders::ALL)
            .border_type(if blocks_focused { BorderType::QuadrantOutside } else { BorderType::Rounded })
            .border_style(Style::default()
                .fg(if blocks_focused { focus_color } else { unfocused_color })
                .add_modifier(if blocks_focused { Modifier::BOLD } else { Modifier::empty() })));

    f.render_stateful_widget(blocks_widget, area, &mut st_blocks);
}

// Helper function to render txs pane
fn render_txs_pane(f: &mut Frame, area: Rect, app: &App) {
    let focus_color = Color::Yellow;
    let unfocused_color = Color::Gray;

    let (txs, sel_tx, total) = app.txs();
    let mut st_txs = ListState::default();
    if !txs.is_empty(){ st_txs.select(Some(sel_tx)); }

    let tx_items: Vec<ListItem> = txs.iter().map(|t| {
        let display = if let (Some(signer), Some(receiver)) = (&t.signer_id, &t.receiver_id) {
            format!("{} → {}", truncate_account(signer, 22), truncate_account(receiver, 22))
        } else {
            truncate_hash(&t.hash, 48)
        };
        ListItem::new(display)
    }).collect();

    // Get owned count for the currently selected block (by height, not index)
    let owned = app.selected_block_height()
        .map(|height| app.owned_count(height))
        .unwrap_or(0);
    let title = if app.owned_only_filter() {
        format!(" Txs (own: {} of {}) ", owned.min(total), total)
    } else if txs.len() < total {
        // Show filtered count when filter is hiding some transactions
        format!(" Txs ({} / {}) ", txs.len(), total)
    } else {
        format!(" Txs ({}) ", txs.len())
    };

    let txs_focused = app.pane() == 1;
    let tx_widget = List::new(tx_items)
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD))
        .highlight_symbol("› ")
        .block(Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(if txs_focused { BorderType::QuadrantOutside } else { BorderType::Rounded })
            .border_style(Style::default()
                .fg(if txs_focused { focus_color } else { unfocused_color })
                .add_modifier(if txs_focused { Modifier::BOLD } else { Modifier::empty() })));

    f.render_stateful_widget(tx_widget, area, &mut st_txs);
}

// Helper function to render details pane
fn render_details_pane(f: &mut Frame, area: Rect, app: &mut App) {
    // Update viewport height for accurate scroll clamping
    // Subtract 2 for top/bottom borders (even though we removed bottom border, title takes space)
    app.set_details_viewport_height(area.height.saturating_sub(2));

    let focus_color = Color::Yellow;
    let unfocused_color = Color::Gray;

    let details_focused = app.pane() == 2;

    // Dynamic title: show copy and fullscreen hints when focused
    let title = if details_focused {
        if app.details_fullscreen() {
            " Transaction details - Press 'c' to copy • Spacebar exits fullscreen "
        } else {
            " Transaction details - Press 'c' to copy • Spacebar to expand "
        }
    } else {
        " Transaction details "
    };

    // Show loading state if archival fetch in progress
    let details_text = if let Some(loading_height) = app.loading_block() {
        format!("⏳ Loading block #{loading_height} from archival...\n\nThis may take 1-2 seconds.\n\nNavigate away to cancel.")
    } else {
        app.details().to_string()
    };

    let details = Paragraph::new(details_text)
        .wrap(Wrap { trim: false })
        .scroll((app.details_scroll(), 0))
        .block(Block::default()
            .title(title)
            .borders(Borders::TOP | Borders::RIGHT) // no LEFT for easier copying, no BOTTOM to avoid double border with footer
            .border_type(if details_focused { BorderType::QuadrantOutside } else { BorderType::Rounded })
            .border_style(Style::default()
                .fg(if details_focused { focus_color } else { unfocused_color })
                .add_modifier(if details_focused { Modifier::BOLD } else { Modifier::empty() })));

    f.render_widget(details, area);
}

// ===============================
// Footer / Debug
// ===============================
fn footer(f:&mut Frame, area:Rect, app:&App, marks:&[Mark]){
    // Build pinned marks chip (max 3)
    let pinned_total = marks.iter().filter(|m| m.pinned).count();
    let mut spans: Vec<Span> = Vec::with_capacity(32);

    spans.push(Span::styled("Tab", Style::default().fg(Color::Yellow)));
    spans.push(Span::raw(" switch  "));
    spans.push(Span::styled("/", Style::default().fg(Color::Yellow)));
    spans.push(Span::raw(" filter  "));
    spans.push(Span::styled("Ctrl+F", Style::default().fg(Color::Yellow)));
    spans.push(Span::raw(" search  "));
    spans.push(Span::styled("←/→", Style::default().fg(Color::Yellow)));
    spans.push(Span::raw(" page  "));
    spans.push(Span::styled("m", Style::default().fg(Color::Yellow)));
    spans.push(Span::raw(" mark  "));
    spans.push(Span::styled("Ctrl+P", Style::default().fg(Color::Yellow)));
    spans.push(Span::raw(" pin  "));
    spans.push(Span::styled("Ctrl+D", Style::default().fg(Color::Yellow)));
    spans.push(Span::raw(" debug  "));
    spans.push(Span::styled("q", Style::default().fg(Color::Yellow)));
    spans.push(Span::raw(" quit  "));

    if app.owned_only_filter() {
        spans.push(Span::raw(" • "));
        spans.push(Span::styled("[OWNED]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
    }
    if pinned_total > 0 {
        spans.push(Span::raw(" • "));
        spans.push(Span::styled(format!("★ {pinned_total}"), Style::default().fg(Color::Yellow)));
    }
    if app.debug_visible() {
        spans.push(Span::raw(" • "));
        spans.push(Span::styled("[DEBUG]", Style::default().fg(Color::Magenta)));
    }
    if let Some(toast) = app.toast_message() {
        spans.push(Span::raw(" • "));
        spans.push(Span::styled(toast, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));
    }
    spans.push(Span::raw(format!(" • FPS {}", app.fps())));

    let line = Line::from(spans);
    let w = Paragraph::new(line).block(Block::default().borders(Borders::TOP).border_type(BorderType::Plain));
    f.render_widget(w, area);
}

fn debug_panel(f:&mut Frame, area:Rect, app:&App){
    let log = app.debug_log();
    if area.height <= 1 {
        let rule = Block::default().borders(Borders::TOP).border_type(BorderType::Plain).border_style(Style::default().fg(Color::DarkGray));
        f.render_widget(rule, area);
        return;
    }

    let lines_to_show = (area.height.saturating_sub(2)) as usize; // inner height
    let start = log.len().saturating_sub(lines_to_show);
    let lines: Vec<Line> = log[start..].iter().map(|msg| Line::from(Span::raw(msg.as_str()))).collect();

    let paragraph = Paragraph::new(lines)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default()
            .title(" Debug ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)));

    f.render_widget(paragraph, area);
}

// ===============================
// Overlays
// ===============================
fn draw_search_overlay(f:&mut Frame, query:&str, results:&[HistoryHit], sel:usize){
    // Centered overlay (90% width, 80% height)
    let area = f.area();
    let width = (area.width * 9) / 10;
    let height = (area.height * 8) / 10;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let overlay = Rect { x, y, width, height };

    f.render_widget(Clear, overlay);

    let container = Block::default()
        .title(" History Search (Ctrl+F) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Black));
    f.render_widget(container, overlay);

    let inner = Rect {
        x: overlay.x + 1,
        y: overlay.y + 1,
        width: overlay.width.saturating_sub(2),
        height: overlay.height.saturating_sub(2),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner);

    // Query input
    let q = Paragraph::new(query)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().title(" Query ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Color::Yellow)));
    f.render_widget(q, chunks[0]);

    if !query.is_empty() && chunks[0].width > 2 {
        let x = chunks[0].x + 1 + (query.len().min((chunks[0].width.saturating_sub(2)) as usize) as u16);
        let y = chunks[0].y + 1;
        f.set_cursor_position((x, y));
    }

    // Results
    let items: Vec<ListItem> = results.iter().map(|h| {
        let ts = chrono::DateTime::from_timestamp_millis(h.ts_ms)
            .map(|dt| dt.format("%H:%M:%S").to_string())
            .unwrap_or_else(|| "-".into());
        let signer = h.signer.as_deref().unwrap_or("-");
        let receiver = h.receiver.as_deref().unwrap_or("-");
        let methods = h.methods.as_deref().unwrap_or("");
        let line = format!(
            "#{:<8} {} {:20} → {:<20} {}",
            h.height, ts, &signer[..signer.len().min(20)], &receiver[..receiver.len().min(20)], methods
        );
        ListItem::new(line)
    }).collect();

    let mut st = ListState::default();
    if !results.is_empty() {
        st.select(Some(sel.min(results.len().saturating_sub(1))));
    }
    let list = List::new(items)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .block(Block::default().borders(Borders::ALL).title(format!(" Results ({}) ", results.len())).border_type(BorderType::Rounded).border_style(Style::default().fg(Color::Yellow)));
    f.render_stateful_widget(list, chunks[1], &mut st);
}

fn draw_marks_overlay(f:&mut Frame, marks:&[Mark], sel:usize){
    // Centered overlay (70% width, 60% height)
    let area = f.area();
    let width = (area.width * 7) / 10;
    let height = (area.height * 6) / 10;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let overlay = Rect { x, y, width, height };

    f.render_widget(Clear, overlay);

    let container = Block::default()
        .title(" Jump Marks (m: set, Ctrl+P: pin, ': jump) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Black));
    f.render_widget(container, overlay);

    let inner = Rect {
        x: overlay.x + 1,
        y: overlay.y + 1,
        width: overlay.width.saturating_sub(2),
        height: overlay.height.saturating_sub(2),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(inner);

    let items: Vec<ListItem> = marks.iter().map(|m| {
        let pin = if m.pinned { "★" } else { " " };
        let pane = match m.pane { 0 => "Blocks", 1 => "Txs", 2 => "Details", _ => "?" };
        let height_str = m.height.map(|h| format!("#{h}")).unwrap_or_else(|| "-".into());
        let tx_str = m.tx_hash.as_deref().map(|h| &h[..8.min(h.len())]).unwrap_or("-");
        ListItem::new(format!("{} {:3} | {:8} | {:8} | {}", pin, m.label, pane, height_str, tx_str))
    }).collect();

    let mut st = ListState::default();
    if !marks.is_empty() {
        st.select(Some(sel.min(marks.len().saturating_sub(1))));
    }
    let list = List::new(items)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .block(Block::default().borders(Borders::ALL).title(format!(" Marks ({}) ", marks.len())).border_type(BorderType::Rounded).border_style(Style::default().fg(Color::Yellow)));
    f.render_stateful_widget(list, chunks[0], &mut st);

    // KEEP ORIGINAL KEYBINDINGS: 'd' for delete, not Space for pin
    let help = Paragraph::new(Line::from(vec![
        Span::raw("↑/↓ move  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)), Span::raw(" jump  "),
        Span::styled("d", Style::default().fg(Color::Yellow)), Span::raw(" delete  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)), Span::raw(" close"),
    ]));
    f.render_widget(help, chunks[1]);
}

fn draw_toast_modal(f: &mut Frame, message: &str) {
    // Small centered box (40% width, 3 lines height)
    let area = f.area();
    let width = (area.width * 4) / 10;
    let height = 3;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let overlay = Rect { x, y, width, height };

    f.render_widget(Clear, overlay);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green));

    let text = Paragraph::new(format!("✓ {message}"))
        .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .block(block);

    f.render_widget(text, overlay);
}

// ===============================
// Helpers
// ===============================
fn truncate_account(account: &str, max_len: usize) -> String {
    if account.len() <= max_len {
        return account.to_string();
    }
    if max_len <= 3 {
        return account[..max_len].to_string();
    }
    // Keep suffix (e.g. .near)
    if let Some(idx) = account.rfind('.') {
        let suffix = &account[idx..];
        let keep = max_len.saturating_sub(3 + suffix.len());
        if keep > 0 {
            return format!("{}...{}", &account[..keep], suffix);
        }
    }
    format!("{}...", &account[..max_len.saturating_sub(3)])
}

fn truncate_hash(hash: &str, max_len: usize) -> String {
    if hash.len() <= max_len {
        hash.to_string()
    } else {
        format!("{}...", &hash[..max_len.saturating_sub(3)])
    }
}
