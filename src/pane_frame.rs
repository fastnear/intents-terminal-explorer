//! csli-style pane frame helper
//!
//! Provides focus-aware pane backgrounds and borders that match csli-dashboard style.
//! Focused panes get panel_alt background and accent borders, unfocused get panel background.

#[cfg(feature = "native")]
use ratatui::{
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

#[cfg(feature = "native")]
use crate::theme::{
    rat::{c, styles},
    Theme,
};

/// Draw a csli-style pane frame and return the **inner** rect for content rendering.
///
/// ## Behavior
/// - **Focused pane**: Uses `panel_alt` background + `accent_strong` border + `accent` title
/// - **Unfocused pane**: Uses `panel` background + `border` color + normal title
///
/// ## Usage
/// ```ignore
/// let theme = Theme::default();
/// let inner = pane_frame::draw(f, area, &theme, "My Pane", app.pane == 0);
/// // Render content into `inner` rect
/// ```
#[cfg(feature = "native")]
pub fn draw(f: &mut Frame, area: Rect, theme: &Theme, title: &str, focused: bool) -> Rect {
    let st = styles(theme);

    // Choose background color based on focus state
    let bg = if focused {
        theme.panel_alt
    } else {
        theme.panel
    };
    let bg_style = Style::default().bg(c(bg));

    // Clear and fill background (ensures full coverage even if content doesn't fill)
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new("").style(bg_style), area);

    // Create border block with focus-aware styling
    let block = if focused {
        Block::default()
            .title(Span::styled(title, st.title_focus))
            .borders(Borders::ALL)
            .border_style(st.border_focus)
            .style(bg_style)
    } else {
        Block::default()
            .title(Span::styled(title, st.title))
            .borders(Borders::ALL)
            .border_style(st.border)
            .style(bg_style)
    };

    // Calculate inner rect and render block
    let inner = block.inner(area);
    f.render_widget(block, area);

    inner
}
