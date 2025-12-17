use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::data::DataStore;

pub fn render_info_view(frame: &mut Frame, area: Rect, data: &DataStore, selected_gpu: usize) {
    let gpu_infos = data.all_gpu_info();
    let gpu_indices = data.gpu_indices();

    if gpu_infos.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" GPU Info - Waiting for data... ")
            .title_style(Style::default().fg(Color::Yellow));
        frame.render_widget(block, area);
        return;
    }

    let gpu_idx = gpu_indices.get(selected_gpu).copied().unwrap_or(0);
    let gpu = match data.get_gpu_info(gpu_idx) {
        Some(g) => g,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" GPU Info - No data for selected GPU ")
                .title_style(Style::default().fg(Color::Yellow));
            frame.render_widget(block, area);
            return;
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" GPU {} Info ", gpu_idx))
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into sections
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Basic info
            Constraint::Length(6),  // Memory info
            Constraint::Length(6),  // Power info
            Constraint::Length(4),  // PCIe info
            Constraint::Min(0),     // Extra space
        ])
        .split(inner);

    // Basic info section
    let basic_info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Name: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&gpu.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("UUID: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&gpu.uuid, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Driver: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&gpu.driver_version, Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("P-State: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&gpu.pstate, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Fan Speed: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                gpu.fan_speed_pct.map(|f| format!("{}%", f)).unwrap_or("N/A".into()),
                Style::default().fg(Color::White),
            ),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Device "));
    frame.render_widget(basic_info, sections[0]);

    // Memory info section
    let mem_pct = if gpu.memory_total_mib > 0 {
        gpu.memory_used_mib as f64 / gpu.memory_total_mib as f64 * 100.0
    } else {
        0.0
    };
    let mem_color = if mem_pct >= 90.0 {
        Color::Red
    } else if mem_pct >= 70.0 {
        Color::Yellow
    } else {
        Color::Green
    };

    let mem_info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Total: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} MiB", gpu.memory_total_mib), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Used:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} MiB ({:.1}%)", gpu.memory_used_mib, mem_pct), Style::default().fg(mem_color)),
        ]),
        Line::from(vec![
            Span::styled("Free:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} MiB", gpu.memory_free_mib), Style::default().fg(Color::Green)),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Memory "));
    frame.render_widget(mem_info, sections[1]);

    // Power info section
    let power_pct = match (gpu.power_draw_w, gpu.power_limit_w) {
        (Some(draw), Some(limit)) if limit > 0.0 => draw / limit * 100.0,
        _ => 0.0,
    };
    let power_color = if power_pct >= 90.0 {
        Color::Red
    } else if power_pct >= 70.0 {
        Color::Yellow
    } else {
        Color::Cyan
    };

    let power_info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Draw:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                gpu.power_draw_w.map(|p| format!("{:.1} W", p)).unwrap_or("N/A".into()),
                Style::default().fg(power_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("Limit: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                gpu.power_limit_w.map(|p| format!("{:.1} W", p)).unwrap_or("N/A".into()),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("Temp:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                gpu.temperature_c.map(|t| format!("{}°C", t)).unwrap_or("N/A".into()),
                Style::default().fg(if gpu.temperature_c.unwrap_or(0) > 80 { Color::Red } else { Color::White }),
            ),
            Span::styled(" / ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                gpu.temperature_limit_c.map(|t| format!("{}°C", t)).unwrap_or("N/A".into()),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Power & Thermal "));
    frame.render_widget(power_info, sections[2]);

    // PCIe info section
    let pcie_info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Link: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "Gen{} x{}",
                    gpu.pcie_gen_current.unwrap_or(0),
                    gpu.pcie_width_current.unwrap_or(0)
                ),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(" (max: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "Gen{} x{}",
                    gpu.pcie_gen_max.unwrap_or(0),
                    gpu.pcie_width_max.unwrap_or(0)
                ),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(")", Style::default().fg(Color::DarkGray)),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title(" PCIe "));
    frame.render_widget(pcie_info, sections[3]);
}
