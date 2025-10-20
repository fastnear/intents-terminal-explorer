use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Style, Color, Modifier},
    text::{Line, Span, Text},
    widgets::{Block, Borders, BorderType, List, ListItem, Paragraph, Wrap, Gauge},
};
use ratacat_plugin_core::traits::{PluginWidget, NotificationLevel};

/// Render plugin widgets in the dashboard UI
pub fn render_plugin_widgets(frame: &mut Frame, area: Rect, widgets: &[PluginWidget]) {
    if widgets.is_empty() {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            widgets.iter().map(|w| match w {
                PluginWidget::StatusBar(_) => Constraint::Length(1),
                PluginWidget::Sidebar { .. } => Constraint::Min(10),
                PluginWidget::Modal { .. } => Constraint::Min(15),
                PluginWidget::Notification { .. } => Constraint::Length(3),
            }).collect::<Vec<_>>()
        )
        .split(area);

    for (i, widget) in widgets.iter().enumerate() {
        if i < chunks.len() {
            render_widget(frame, chunks[i], widget);
        }
    }
}

fn render_widget(frame: &mut Frame, area: Rect, widget: &PluginWidget) {
    match widget {
        PluginWidget::StatusBar(text) => {
            let status = Paragraph::new(text.as_str())
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Right);
            frame.render_widget(status, area);
        }

        PluginWidget::Sidebar { title, content } => {
            let items: Vec<ListItem> = content.iter()
                .map(|s| ListItem::new(s.as_str()))
                .collect();

            let list = List::new(items)
                .block(Block::default()
                    .title(title.as_str())
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Blue))
                )
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol("▶ ");

            frame.render_widget(list, area);
        }

        PluginWidget::Modal { title, content, actions } => {
            let mut text = vec![
                Line::from(""),
                Line::from(content.as_str()),
                Line::from(""),
            ];

            if !actions.is_empty() {
                text.push(Line::from(""));
                let action_spans: Vec<Span> = actions.iter()
                    .enumerate()
                    .flat_map(|(i, action)| {
                        if i > 0 {
                            vec![
                                Span::raw("  "),
                                Span::styled(action, Style::default().fg(Color::Yellow).add_modifier(Modifier::UNDERLINED))
                            ]
                        } else {
                            vec![Span::styled(action, Style::default().fg(Color::Yellow).add_modifier(Modifier::UNDERLINED))]
                        }
                    })
                    .collect();
                text.push(Line::from(action_spans));
            }

            let modal = Paragraph::new(text)
                .block(Block::default()
                    .title(title.as_str())
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(Color::Red))
                )
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false });

            frame.render_widget(modal, area);
        }

        PluginWidget::Notification { message, level } => {
            let (symbol, color) = match level {
                NotificationLevel::Info => ("ℹ", Color::Blue),
                NotificationLevel::Success => ("✓", Color::Green),
                NotificationLevel::Warning => ("⚠", Color::Yellow),
                NotificationLevel::Error => ("✗", Color::Red),
            };

            let notification = Paragraph::new(Line::from(vec![
                Span::styled(format!("{} ", symbol), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Span::raw(message),
            ]))
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color))
            );

            frame.render_widget(notification, area);
        }
    }
}

/// Create a plugin status panel for the dashboard
pub fn create_plugin_status_widget(
    active_plugins: usize,
    total_alerts: usize,
    patterns_detected: usize,
) -> Paragraph<'static> {
    let lines = vec![
        Line::from(vec![
            Span::styled("Plugins: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{} active", active_plugins),
                Style::default().fg(Color::Green)
            ),
        ]),
        Line::from(vec![
            Span::styled("Alerts: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{}", total_alerts),
                Style::default().fg(if total_alerts > 0 { Color::Yellow } else { Color::Gray })
            ),
        ]),
        Line::from(vec![
            Span::styled("Patterns: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{} detected", patterns_detected),
                Style::default().fg(Color::Cyan)
            ),
        ]),
    ];

    Paragraph::new(lines)
        .block(Block::default()
            .title("Plugin Status")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray))
        )
}

/// Create a validator health widget
pub fn create_validator_health_widget(validators: &[(String, f64, bool)]) -> List<'static> {
    let items: Vec<ListItem> = validators.iter()
        .map(|(name, uptime, healthy)| {
            let health_indicator = if *healthy { "🟢" } else { "🔴" };
            let color = if *healthy { Color::Green } else { Color::Red };

            ListItem::new(Line::from(vec![
                Span::raw(format!("{} ", health_indicator)),
                Span::styled(name.clone(), Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" - "),
                Span::styled(
                    format!("{:.1}% uptime", uptime),
                    Style::default().fg(color)
                ),
            ]))
        })
        .collect();

    List::new(items)
        .block(Block::default()
            .title("Validator Health")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Magenta))
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
}

/// Create transaction pattern visualization
pub fn create_pattern_gauge(pattern_name: &str, frequency: f64) -> Gauge<'static> {
    Gauge::default()
        .block(Block::default()
            .title(pattern_name)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
        )
        .gauge_style(Style::default()
            .fg(match frequency {
                f if f > 80.0 => Color::Red,
                f if f > 50.0 => Color::Yellow,
                _ => Color::Green,
            })
            .bg(Color::Black)
        )
        .percent(frequency as u16)
        .label(format!("{:.0}%", frequency))
}

/// Render plugin notifications as an overlay
pub fn render_plugin_notifications(frame: &mut Frame, notifications: &[(String, String, NotificationLevel)]) {
    if notifications.is_empty() {
        return;
    }

    let area = centered_rect(60, 20, frame.size());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            notifications.iter()
                .map(|_| Constraint::Length(3))
                .collect::<Vec<_>>()
        )
        .split(area);

    for (i, (plugin_id, message, level)) in notifications.iter().enumerate() {
        if i >= chunks.len() {
            break;
        }

        let widget = PluginWidget::Notification {
            message: format!("[{}] {}", plugin_id, message),
            level: *level,
        };

        render_widget(frame, chunks[i], &widget);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}