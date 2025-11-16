#![cfg_attr(target_arch = "wasm32", no_main)]

use eframe::egui;
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};

#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

use nearx::{platform, theme, ui_core, App};

/// Debouncer for filter input (150ms delay)
struct Debounce {
    last_text: String,
    last_change: Instant,
    delay: Duration,
    pending: bool,
}

impl Debounce {
    fn new(ms: u64) -> Self {
        Self {
            last_text: String::new(),
            last_change: Instant::now(),
            delay: Duration::from_millis(ms),
            pending: false,
        }
    }

    fn set(&mut self, s: &str) {
        if s != self.last_text {
            self.last_text.clear();
            self.last_text.push_str(s);
            self.last_change = Instant::now();
            self.pending = true;
        }
    }

    fn next_deadline(&self) -> Option<Duration> {
        if self.pending {
            let elapsed = self.last_change.elapsed();
            if elapsed < self.delay {
                return Some(self.delay - elapsed);
            }
        }
        None
    }

    fn ready(&mut self) -> Option<String> {
        if self.pending && self.last_change.elapsed() >= self.delay {
            self.pending = false;
            return Some(self.last_text.clone());
        }
        None
    }
}

/// Normalize filter text: convert newlines/tabs to spaces, collapse multiple spaces, trim.
fn normalize_filter(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.chars() {
        let spacey = ch == '\n' || ch == '\r' || ch == '\t' || ch == ' ';
        if spacey {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

struct NearxWeb {
    app: App,
    event_rx: tokio::sync::mpsc::UnboundedReceiver<nearx::types::AppEvent>,
    filter_text: String,
    filter_id: egui::Id,
    filter_db: Debounce,
    input_policy: ui_core::policy::InputPolicy,
    did_snap_dpr: bool,
}

impl NearxWeb {
    fn new(
        app: App,
        event_rx: tokio::sync::mpsc::UnboundedReceiver<nearx::types::AppEvent>,
    ) -> Self {
        let filter = app.filter_query().to_string();
        Self {
            app,
            event_rx,
            filter_text: filter,
            filter_id: egui::Id::new("nearx.filter"),
            filter_db: Debounce::new(150),
            input_policy: ui_core::policy::default_policy(),
            did_snap_dpr: false,
        }
    }

    fn focus_frame(&self, focused: bool) -> egui::Frame {
        let t = *self.app.theme();
        let tok = theme::tokens::tokens().visuals;
        let accent = theme::eg::c(t.accent_strong);
        let border = theme::eg::c(t.border);
        let stroke = if focused {
            egui::Stroke::new(tok.focus_stroke_px, accent)
        } else {
            egui::Stroke::new(tok.unfocus_stroke_px, border)
        };
        egui::Frame::default()
            .stroke(stroke)
            .corner_radius(egui::CornerRadius::same(tok.widget_radius_px))
            .inner_margin(egui::Margin::symmetric(6, 4))
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        let tok = theme::tokens::tokens().visuals;

        // Dynamic row sizing: expand when focused or non-empty (like TUI's 3-line bar)
        let has_focus = ui.ctx().memory(|m| m.has_focus(self.filter_id));
        let expanded = has_focus || !self.filter_text.trim().is_empty();
        let rows = if expanded {
            tok.filter_rows
        } else {
            tok.filter_rows_collapsed
        };

        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

            let edit = egui::TextEdit::multiline(&mut self.filter_text)
                .hint_text("Filter (acct:…, action:…, method:… ; comma AND)")
                .desired_rows(rows)
                .font(egui::TextStyle::Body)
                .desired_width(ui.available_width() * 0.70)
                .id(self.filter_id);
            let resp = ui.add(edit);
            if resp.changed() {
                self.filter_db.set(&self.filter_text);
            }
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                // Normalize: convert newlines to spaces, collapse whitespace
                let normalized = normalize_filter(&self.filter_text);
                self.filter_text = normalized.clone();
                self.app.set_filter_query(normalized);
                self.filter_db.pending = false;
                changed = true;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // No PRETTY/RAW toggle in alpha: always pretty+colored JSON
                #[allow(unused_mut)]
                let mut signed_in = nearx::auth::token_string().is_some();
                if signed_in {
                    ui.label(egui::RichText::new("Signed in").strong());
                    if ui.button("Sign out").clicked() {
                        nearx::auth::clear();
                        changed = true;
                    }
                } else if ui.button("Sign in with Google").clicked() {
                    #[cfg(target_arch = "wasm32")]
                    nearx::webshim::auth_login_google();
                }
            });
        });
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);
        changed
    }

    fn draw_blocks(&mut self, ui: &mut egui::Ui) {
        let len = self.app.blocks_len();
        let sel = self.app.sel_block();
        let row_h = theme::tokens::tokens().visuals.row_height_px;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(egui_extras::Column::remainder())
                    .body(|mut body| {
                        for row_index in 0..len {
                            body.row(row_h, |mut row| {
                                row.col(|ui| {
                                    if let Some(b) = self.app.block_lite(row_index) {
                                        let label = format!(
                                            "#{}  ·  {} txs  ·  {}",
                                            b.height, b.tx_count, b.time_utc
                                        );
                                        let selected = row_index == sel;

                                        // Manual background highlighting to constrain to this pane
                                        let t = *self.app.theme();
                                        let frame = if selected {
                                            egui::Frame::default()
                                                .fill(theme::eg::c(t.sel_bg))
                                                .inner_margin(egui::Margin::symmetric(4, 2))
                                        } else {
                                            egui::Frame::default()
                                                .inner_margin(egui::Margin::symmetric(4, 2))
                                        };

                                        let resp = frame
                                            .show(ui, |ui| {
                                                let text = if selected {
                                                    egui::RichText::new(&label).strong()
                                                } else {
                                                    egui::RichText::new(&label)
                                                };
                                                ui.add(
                                                    egui::Label::new(text)
                                                        .sense(egui::Sense::click()),
                                                )
                                            })
                                            .inner;

                                        if resp.hovered() {
                                            ui.ctx()
                                                .set_cursor_icon(egui::CursorIcon::PointingHand);
                                        }
                                        if resp.clicked() {
                                            self.app.select_block_clamped(row_index);
                                        }
                                        if selected {
                                            resp.scroll_to_me(Some(egui::Align::Center));
                                        }
                                    }
                                });
                            });
                        }
                    });
            });
    }

    fn draw_txs(&mut self, ui: &mut egui::Ui) {
        let len = self.app.txs_len();
        let sel = self.app.sel_tx();
        let row_h = theme::tokens::tokens().visuals.row_height_px;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(egui_extras::Column::remainder())
                    .body(|mut body| {
                        for row_index in 0..len {
                            body.row(row_h, |mut row| {
                                row.col(|ui| {
                                    if let Some(t) = self.app.tx_lite(row_index) {
                                        let signer = t.signer_id.as_deref().unwrap_or("?");
                                        let action = if let Some(acts) = &t.actions {
                                            if acts.is_empty() {
                                                "No actions".to_string()
                                            } else {
                                                format!("{:?}", acts[0])
                                            }
                                        } else {
                                            "?".to_string()
                                        };
                                        // Show full hash (no silent shortening). Tooltip repeats full hash.
                                        let hash_full = t.hash.clone();
                                        let label =
                                            format!("{}  ·  {}  ·  {}", hash_full, signer, action);
                                        let selected = row_index == sel;

                                        // Manual background highlighting to constrain to this pane
                                        let theme = *self.app.theme();
                                        let frame = if selected {
                                            egui::Frame::default()
                                                .fill(theme::eg::c(theme.sel_bg))
                                                .inner_margin(egui::Margin::symmetric(4, 2))
                                        } else {
                                            egui::Frame::default()
                                                .inner_margin(egui::Margin::symmetric(4, 2))
                                        };

                                        let resp = frame
                                            .show(ui, |ui| {
                                                let text = if selected {
                                                    egui::RichText::new(&label).strong()
                                                } else {
                                                    egui::RichText::new(&label)
                                                };
                                                ui.add(
                                                    egui::Label::new(text)
                                                        .sense(egui::Sense::click()),
                                                )
                                            })
                                            .inner;

                                        // Show full hash on hover (tooltip)
                                        let resp = resp.on_hover_ui(|ui| {
                                            ui.code(&hash_full);
                                        });

                                        if resp.hovered() {
                                            ui.ctx()
                                                .set_cursor_icon(egui::CursorIcon::PointingHand);
                                        }
                                        if resp.clicked() {
                                            self.app.select_tx_clamped(row_index);
                                        }
                                        if selected {
                                            resp.scroll_to_me(Some(egui::Align::Center));
                                        }
                                    }
                                });
                            });
                        }
                    });
            });
    }

    fn draw_details(&mut self, ui: &mut egui::Ui) {
        // Always pretty print + colorized
        let pretty = self.app.details_pretty_string();
        // Soft limit: ~300k chars to avoid huge layout jobs
        const MAX_CHARS: usize = 300_000;
        let (visible, truncated) = if pretty.len() > MAX_CHARS {
            (&pretty[..MAX_CHARS], true)
        } else {
            (&pretty[..], false)
        };
        let job = self.highlight_json_job(visible);
        let resp = egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                // Monospace, no wrapping; colored via LayoutJob
                // Note: egui 0.32 Label doesn't wrap by default
                let lbl = egui::Label::new(job);
                let label_resp = ui.add(lbl);
                if truncated {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("… truncated for performance").weak());
                }
                label_resp
            });
        if resp.inner.double_clicked() {
            self.app.toggle_details_fullscreen();
        }
    }

    /// JSON syntax highlighter → LayoutJob using theme colors.
    fn highlight_json_job(&self, s: &str) -> egui::text::LayoutJob {
        use egui::text::{LayoutJob, TextFormat};
        use serde_json::Value;

        let t = *self.app.theme();
        let tok = theme::tokens::tokens().visuals;
        let color_key = theme::eg::c(t.json_key);
        let color_string = theme::eg::c(t.json_string);
        let color_number = theme::eg::c(t.json_number);
        let color_bool = theme::eg::c(t.json_bool);
        let color_null = theme::eg::c(t.json_bool);
        let color_punct = theme::eg::c(t.json_struct);

        let font = egui::FontId::monospace(tok.code_font_px);
        let mut job = LayoutJob::default();

        fn push(job: &mut LayoutJob, text: &str, font: egui::FontId, color: egui::Color32) {
            job.append(
                text,
                0.0,
                TextFormat {
                    font_id: font,
                    color,
                    ..Default::default()
                },
            );
        }

        fn render(
            job: &mut LayoutJob,
            v: &Value,
            indent: usize,
            font: egui::FontId,
            ck: egui::Color32,
            cs: egui::Color32,
            cn: egui::Color32,
            cb: egui::Color32,
            cnull: egui::Color32,
            cp: egui::Color32,
        ) {
            let ind = "  ".repeat(indent);
            match v {
                Value::Object(map) => {
                    push(job, "{\n", font.clone(), cp);
                    let mut first = true;
                    for (k, vv) in map {
                        if !first {
                            push(job, ",\n", font.clone(), cp);
                        }
                        first = false;
                        push(job, &ind, font.clone(), cp);
                        // key
                        push(job, "\"", font.clone(), cp);
                        push(job, k, font.clone(), ck);
                        push(job, "\"", font.clone(), cp);
                        push(job, ": ", font.clone(), cp);
                        render(job, vv, indent + 1, font.clone(), ck, cs, cn, cb, cnull, cp);
                    }
                    push(job, "\n", font.clone(), cp);
                    if indent > 0 {
                        push(job, &"  ".repeat(indent - 1), font.clone(), cp);
                    }
                    push(job, "}", font, cp);
                }
                Value::Array(arr) => {
                    push(job, "[\n", font.clone(), cp);
                    let mut first = true;
                    for vv in arr {
                        if !first {
                            push(job, ",\n", font.clone(), cp);
                        }
                        first = false;
                        push(job, &ind, font.clone(), cp);
                        render(job, vv, indent + 1, font.clone(), ck, cs, cn, cb, cnull, cp);
                    }
                    push(job, "\n", font.clone(), cp);
                    if indent > 0 {
                        push(job, &"  ".repeat(indent - 1), font.clone(), cp);
                    }
                    push(job, "]", font, cp);
                }
                Value::String(x) => {
                    push(job, "\"", font.clone(), cp);
                    push(job, x, font.clone(), cs);
                    push(job, "\"", font, cp);
                }
                Value::Number(x) => {
                    push(job, &x.to_string(), font, cn);
                }
                Value::Bool(b) => {
                    push(job, if *b { "true" } else { "false" }, font, cb);
                }
                Value::Null => {
                    push(job, "null", font, cnull);
                }
            }
        }

        match serde_json::from_str::<Value>(s) {
            Ok(v) => render(
                &mut job,
                &v,
                1,
                font,
                color_key,
                color_string,
                color_number,
                color_bool,
                color_null,
                color_punct,
            ),
            Err(_) => {
                // Fallback: plain text if JSON fails (shouldn't happen with details_pretty_string()).
                push(&mut job, s, font, color_string);
            }
        }
        job
    }

    fn handle_keys(&mut self, ctx: &egui::Context) -> bool {
        let mut changed = false;

        // Slash or Cmd/Ctrl+F focuses filter
        if ctx.input(|i| {
            i.key_pressed(egui::Key::Slash)
                || ((i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::F))
        }) {
            ctx.memory_mut(|m| m.request_focus(self.filter_id));
        }

        let events = ctx.input(|i| i.events.clone());
        for ev in events {
            match ev {
                egui::Event::Key {
                    key: egui::Key::Tab,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if self.input_policy.tab_cycles_panes {
                        if modifiers.shift {
                            self.app.prev_pane();
                        } else {
                            self.app.next_pane();
                        }
                        // Ensure egui doesn't try to focus next widget
                        ctx.input_mut(|i| {
                            i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
                            i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab);
                        });
                        // Prevent Tab from focusing inputs (including "Login with Google")
                        if !self.input_policy.tab_focus_inputs {
                            ctx.memory_mut(|m| m.request_focus(egui::Id::NULL));
                        }
                        changed = true;
                    }
                }
                egui::Event::Key {
                    key: egui::Key::Space,
                    pressed: true,
                    ..
                } => {
                    if self.app.pane() == 2 {
                        self.app.toggle_details_fullscreen();
                        changed = true;
                    }
                }
                egui::Event::Key {
                    key: egui::Key::C,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if !modifiers.command && !modifiers.ctrl {
                        let filter_focused = ctx.memory(|m| m.has_focus(self.filter_id));
                        if !filter_focused {
                            if let Some(s) = self.app.focused_json_string() {
                                let _ = platform::copy_to_clipboard(&s);
                            }
                        }
                    }
                }
                egui::Event::Key {
                    key: egui::Key::ArrowUp,
                    pressed: true,
                    ..
                } => {
                    match self.app.pane() {
                        0 => {
                            let cur = self.app.sel_block() as isize;
                            self.app
                                .select_block_clamped(cur.saturating_sub(1) as usize);
                        }
                        1 => {
                            let cur = self.app.sel_tx() as isize;
                            self.app.select_tx_clamped(cur.saturating_sub(1) as usize);
                        }
                        _ => {}
                    }
                    changed = true;
                }
                egui::Event::Key {
                    key: egui::Key::ArrowDown,
                    pressed: true,
                    ..
                } => {
                    match self.app.pane() {
                        0 => {
                            let cur = self.app.sel_block() as isize + 1;
                            self.app.select_block_clamped(cur as usize);
                        }
                        1 => {
                            let cur = self.app.sel_tx() as isize + 1;
                            self.app.select_tx_clamped(cur as usize);
                        }
                        _ => {}
                    }
                    changed = true;
                }
                egui::Event::Key {
                    key: egui::Key::PageUp,
                    pressed: true,
                    ..
                } => {
                    match self.app.pane() {
                        0 => {
                            let cur = self.app.sel_block() as isize - 20;
                            self.app.select_block_clamped(cur.max(0) as usize);
                        }
                        1 => {
                            let cur = self.app.sel_tx() as isize - 20;
                            self.app.select_tx_clamped(cur.max(0) as usize);
                        }
                        _ => {}
                    }
                    changed = true;
                }
                egui::Event::Key {
                    key: egui::Key::PageDown,
                    pressed: true,
                    ..
                } => {
                    match self.app.pane() {
                        0 => {
                            let cur = self.app.sel_block() as isize + 20;
                            self.app.select_block_clamped(cur as usize);
                        }
                        1 => {
                            let cur = self.app.sel_tx() as isize + 20;
                            self.app.select_tx_clamped(cur as usize);
                        }
                        _ => {}
                    }
                    changed = true;
                }
                egui::Event::Key {
                    key: egui::Key::Home,
                    pressed: true,
                    ..
                } => {
                    match self.app.pane() {
                        0 => self.app.select_block_clamped(0),
                        1 => self.app.select_tx_clamped(0),
                        _ => {}
                    }
                    changed = true;
                }
                egui::Event::Key {
                    key: egui::Key::End,
                    pressed: true,
                    ..
                } => {
                    match self.app.pane() {
                        0 => {
                            let last = self.app.blocks_len().saturating_sub(1);
                            self.app.select_block_clamped(last);
                        }
                        1 => {
                            let last = self.app.txs_len().saturating_sub(1);
                            self.app.select_tx_clamped(last);
                        }
                        _ => {}
                    }
                    changed = true;
                }
                _ => {}
            }
        }
        changed
    }

    fn layout_main(&mut self, ctx: &egui::Context) {
        let spec = ui_core::layout::LayoutSpec {
            top_ratio: theme::tokens::tokens().layout.top_ratio,
            ..Default::default()
        };

        if self.app.details_fullscreen() {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Transaction Details (Press Space to exit fullscreen)");
                ui.separator();
                self.draw_details(ui);
            });
            return;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let total = ui.available_size();
            let (top_h, bot_h) = ui_core::layout::split_pixels(total.y, spec);
            egui_extras::StripBuilder::new(ui)
                .size(egui_extras::Size::exact(top_h))
                .size(egui_extras::Size::exact(bot_h))
                .vertical(|mut strip| {
                    strip.cell(|ui| {
                        // Use StripBuilder for top row to add spacing between Blocks/Transactions
                        egui_extras::StripBuilder::new(ui)
                            .size(egui_extras::Size::relative(0.5))
                            .size(egui_extras::Size::exact(6.0)) // Gap between columns
                            .size(egui_extras::Size::relative(0.5))
                            .horizontal(|mut strip| {
                                // Left column: Blocks
                                strip.cell(|ui| {
                                    let frame = self.focus_frame(self.app.pane() == 0);
                                    frame.show(ui, |ui| {
                                        let resp = ui.interact(
                                            ui.max_rect(),
                                            egui::Id::new("pane.blocks"),
                                            egui::Sense::click(),
                                        );
                                        if resp.clicked() {
                                            self.app.set_pane_direct(0);
                                        }
                                        ui.heading(format!("Blocks ({})", self.app.blocks_len()));
                                        ui.separator();
                                        self.draw_blocks(ui);
                                    });
                                });
                                // Gap (empty)
                                strip.empty();
                                // Right column: Transactions
                                strip.cell(|ui| {
                                    let frame = self.focus_frame(self.app.pane() == 1);
                                    frame.show(ui, |ui| {
                                        let resp = ui.interact(
                                            ui.max_rect(),
                                            egui::Id::new("pane.txs"),
                                            egui::Sense::click(),
                                        );
                                        if resp.clicked() {
                                            self.app.set_pane_direct(1);
                                        }
                                        ui.heading(format!(
                                            "Transactions ({})",
                                            self.app.txs_len()
                                        ));
                                        ui.separator();
                                        self.draw_txs(ui);
                                    });
                                });
                            });
                    });
                    strip.cell(|ui| {
                        let frame = self.focus_frame(self.app.pane() == 2);
                        frame.show(ui, |ui| {
                            let resp = ui.interact(
                                ui.max_rect(),
                                egui::Id::new("pane.details"),
                                egui::Sense::click(),
                            );
                            if resp.clicked() {
                                self.app.set_pane_direct(2);
                            }
                            ui.heading("Transaction Details");
                            ui.separator();
                            self.draw_details(ui);
                        });
                    });
                });
        });
    }
}

impl eframe::App for NearxWeb {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply theme
        theme::eg::apply(ctx, self.app.theme());

        // Snap DPR once to keep font sizes sensible on retina/hi-dpi
        if !self.did_snap_dpr {
            let ppp = ctx.pixels_per_point().max(1.0);
            ctx.set_pixels_per_point(ppp.round());
            self.did_snap_dpr = true;
        }

        let mut repaint_now = false;

        // Process RPC events (non-blocking drain)
        while let Ok(ev) = self.event_rx.try_recv() {
            self.app.on_event(ev);
            repaint_now = true;
        }

        // Check debounced filter
        if let Some(wait) = self.filter_db.next_deadline() {
            ctx.request_repaint_after(wait);
        }
        if let Some(q) = self.filter_db.ready() {
            self.app.set_filter_query(q);
            repaint_now = true;
        }

        // Handle keyboard input
        if self.handle_keys(ctx) {
            repaint_now = true;
        }

        // Top bar
        egui::TopBottomPanel::top("nearx.top").show(ctx, |ui| {
            if self.top_bar(ui) {
                repaint_now = true;
            }
        });

        // Main layout
        self.layout_main(ctx);

        // Repaint timing (30 FPS base)
        if repaint_now {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(33));
        }
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    // Install panic hook for human-readable errors in DevTools
    #[cfg(target_arch = "wasm32")]
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    wasm_logger::init(wasm_logger::Config::default());

    // Create event channel for RPC -> UI communication
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

    // Initialize App with defaults
    let fps = 30;
    let fps_choices = vec![20, 30, 60];
    let keep_blocks = 100;
    let default_filter = "".to_string();
    let archival_fetch_tx = None;

    // Build RPC config with mainnet defaults
    let config = nearx::Config {
        source: nearx::Source::Rpc,
        ws_url: "".to_string(), // Not used in RPC mode
        ws_fetch_blocks: false,
        render_fps: fps,
        render_fps_choices: fps_choices.clone(),
        poll_interval_ms: 1000,
        poll_max_catchup: 5,
        poll_chunk_concurrency: 4,
        keep_blocks,
        near_node_url: "https://rpc.mainnet.fastnear.com/".to_string(),
        near_node_url_explicit: false,
        archival_rpc_url: None,
        rpc_timeout_ms: 8000,
        rpc_retries: 2,
        fastnear_auth_token: nearx::config::fastnear_token(), // Check localStorage first
        default_filter: default_filter.clone(),
        theme: nearx::theme::Theme::default(),
    };

    let app = App::new(
        fps,
        fps_choices,
        keep_blocks,
        default_filter,
        archival_fetch_tx,
    );
    let web = NearxWeb::new(app, event_rx);
    let opts = eframe::WebOptions::default();

    // Spawn RPC poller in background
    let event_tx_clone = event_tx.clone();
    let config_clone = config.clone();
    wasm_bindgen_futures::spawn_local(async move {
        log::info!("🚀 Starting RPC poller ({})", config_clone.near_node_url);
        match nearx::source_rpc::run_rpc(&config_clone, event_tx_clone).await {
            Ok(_) => log::info!("✅ RPC poller completed"),
            Err(e) => log::error!("❌ RPC poller error: {}", e),
        }
    });

    // Start eframe
    wasm_bindgen_futures::spawn_local(async move {
        let canvas = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("nearx_canvas"))
            .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("Failed to find canvas element with id 'nearx_canvas'");

        let _ = eframe::WebRunner::new()
            .start(canvas, opts, Box::new(|_cc| Ok(Box::new(web))))
            .await;
    });
}
