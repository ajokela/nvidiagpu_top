use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::ViewMode;

// Use standard terminal colors
const COLOR_KEY: Color = Color::Cyan;
const COLOR_DANGER: Color = Color::LightRed;

fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

pub fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    samples: u64,
    uptime: std::time::Duration,
    view_mode: &ViewMode,
    error: Option<&str>,
) {
    let uptime_str = format_duration(uptime);

    let status_text = if let Some(err) = error {
        vec![
            Span::styled("ERROR: ", Style::default().fg(COLOR_DANGER).add_modifier(Modifier::BOLD)),
            Span::styled(err, Style::default().fg(COLOR_DANGER)),
            Span::raw("  "),
        ]
    } else {
        vec![
            Span::styled("Samples: ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{}", samples), Style::default().fg(Color::White)),
            Span::raw(" | "),
            Span::styled("Uptime: ", Style::default().fg(Color::Gray)),
            Span::styled(uptime_str, Style::default().fg(Color::White)),
            Span::raw("  "),
        ]
    };

    // Tab indicators
    let mut tabs = Vec::new();
    for (i, mode) in ViewMode::all().iter().enumerate() {
        let style = if view_mode == mode {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        tabs.push(Span::styled(format!(" [{}]{} ", i + 1, mode.name()), style));
    }

    let mut spans = status_text;
    spans.extend(tabs);

    let status = Paragraph::new(Line::from(spans));
    frame.render_widget(status, area);
}

pub fn render_help_bar(frame: &mut Frame, area: Rect) {
    let help = Paragraph::new(Line::from(vec![
        Span::styled("[q]", Style::default().fg(COLOR_KEY)),
        Span::raw(" quit  "),
        Span::styled("[Tab]", Style::default().fg(COLOR_KEY)),
        Span::raw(" switch  "),
        Span::styled("[j/k]", Style::default().fg(COLOR_KEY)),
        Span::raw(" select  "),
        Span::styled("[i]", Style::default().fg(COLOR_KEY)),
        Span::raw(" info  "),
        Span::styled("[t]", Style::default().fg(COLOR_KEY)),
        Span::raw(" topology"),
    ]))
    .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(help, area);
}
