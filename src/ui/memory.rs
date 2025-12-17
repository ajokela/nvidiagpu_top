use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

use crate::data::DataStore;

pub fn render_memory_view(frame: &mut Frame, area: Rect, data: &DataStore) {
    let gpu_infos = data.all_gpu_info();

    if gpu_infos.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Memory Usage - Waiting for data... ")
            .title_style(Style::default().fg(Color::Yellow));
        frame.render_widget(block, area);
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Memory Usage ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Create a row for each GPU
    let gpu_height = 3u16;
    let constraints: Vec<Constraint> = gpu_infos
        .iter()
        .map(|_| Constraint::Length(gpu_height))
        .chain(std::iter::once(Constraint::Min(0)))
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, gpu) in gpu_infos.iter().enumerate() {
        if i >= chunks.len() - 1 {
            break;
        }

        let chunk = chunks[i];

        // Split into label and gauge
        let parts = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(20), Constraint::Min(20)])
            .split(chunk);

        // GPU label with name
        let label = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    format!("GPU {} ", gpu.index),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(&gpu.name, Style::default().fg(Color::DarkGray)),
            ]),
        ]);
        frame.render_widget(label, parts[0]);

        // Memory gauge
        let used = gpu.memory_used_mib;
        let total = gpu.memory_total_mib;
        let pct = if total > 0 {
            (used as f64 / total as f64 * 100.0) as u16
        } else {
            0
        };

        let color = if pct >= 90 {
            Color::Red
        } else if pct >= 70 {
            Color::Yellow
        } else {
            Color::Green
        };

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::NONE))
            .gauge_style(Style::default().fg(color).bg(Color::DarkGray))
            .percent(pct)
            .label(format!(
                "{} / {} MiB ({:.1}%)",
                used, total, used as f64 / total as f64 * 100.0
            ));

        // Add power info below gauge if available
        let gauge_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Min(0)])
            .split(parts[1]);

        frame.render_widget(gauge, gauge_area[0]);

        // Power info line
        if let (Some(draw), Some(limit)) = (gpu.power_draw_w, gpu.power_limit_w) {
            let power_pct = (draw / limit * 100.0) as u16;
            let power_color = if power_pct >= 90 {
                Color::Red
            } else if power_pct >= 70 {
                Color::Yellow
            } else {
                Color::Cyan
            };

            let power_line = Line::from(vec![
                Span::styled("Power: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{:.0}W", draw),
                    Style::default().fg(power_color),
                ),
                Span::styled(
                    format!(" / {:.0}W", limit),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw("  "),
                Span::styled("Temp: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{}°C", gpu.temperature_c.unwrap_or(0)),
                    Style::default().fg(Color::White),
                ),
            ]);
            frame.render_widget(Paragraph::new(power_line), gauge_area[1]);
        }
    }
}
