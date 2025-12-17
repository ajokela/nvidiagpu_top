use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::data::DataStore;

/// Sparkline characters (8 levels)
const SPARKLINE_CHARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Generate a sparkline string from values (0-100 scale)
fn sparkline(values: &[f64], width: usize) -> String {
    if values.is_empty() {
        return " ".repeat(width);
    }

    let values: Vec<f64> = if values.len() > width {
        values[values.len() - width..].to_vec()
    } else {
        values.to_vec()
    };

    let mut result = String::new();
    for &v in &values {
        let clamped = v.clamp(0.0, 100.0);
        let idx = ((clamped / 100.0) * 7.0).round() as usize;
        result.push(SPARKLINE_CHARS[idx.min(7)]);
    }

    // Pad with spaces if needed
    while result.chars().count() < width {
        result.insert(0, ' ');
    }

    result
}

/// Format optional value with unit
fn fmt_val(val: Option<u32>, unit: &str) -> String {
    match val {
        Some(v) => format!("{}{}", v, unit),
        None => "-".to_string(),
    }
}

pub fn render_table_view(frame: &mut Frame, area: Rect, data: &DataStore, selected_gpu: usize) {
    let gpu_indices = data.gpu_indices();

    let header_cells = ["GPU", "Power", "Temp", "SM%", "Mem%", "Enc%", "Dec%", "MCLK", "PCLK"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = gpu_indices
        .iter()
        .enumerate()
        .map(|(i, &gpu_idx)| {
            let history = data.get_gpu(gpu_idx);
            let latest = history.and_then(|h| h.latest());

            let (power, temp, _sm, _mem, enc, dec, mclk, pclk) = match latest {
                Some(s) => (
                    fmt_val(s.power_w, "W"),
                    fmt_val(s.gpu_temp_c, "°C"),
                    fmt_val(s.sm_util, "%"),
                    fmt_val(s.mem_util, "%"),
                    fmt_val(s.enc_util, "%"),
                    fmt_val(s.dec_util, "%"),
                    fmt_val(s.mem_clock_mhz, ""),
                    fmt_val(s.gpu_clock_mhz, ""),
                ),
                None => (
                    "-".into(), "-".into(), "-".into(), "-".into(),
                    "-".into(), "-".into(), "-".into(), "-".into(),
                ),
            };

            // Get sparklines for SM and Mem utilization
            let sm_spark = history
                .map(|h| sparkline(&h.recent_values(8, |s| s.sm_util), 8))
                .unwrap_or_else(|| " ".repeat(8));
            let mem_spark = history
                .map(|h| sparkline(&h.recent_values(8, |s| s.mem_util), 8))
                .unwrap_or_else(|| " ".repeat(8));

            let row_style = if i == selected_gpu {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(format!("{}", gpu_idx)),
                Cell::from(power),
                Cell::from(temp),
                Cell::from(sm_spark).style(Style::default().fg(Color::Green)),
                Cell::from(mem_spark).style(Style::default().fg(Color::Cyan)),
                Cell::from(enc),
                Cell::from(dec),
                Cell::from(mclk),
                Cell::from(pclk),
            ])
            .style(row_style)
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(4),   // GPU
        Constraint::Length(6),   // Power
        Constraint::Length(6),   // Temp
        Constraint::Length(10),  // SM% (sparkline)
        Constraint::Length(10),  // Mem% (sparkline)
        Constraint::Length(5),   // Enc%
        Constraint::Length(5),   // Dec%
        Constraint::Length(6),   // MCLK
        Constraint::Length(6),   // PCLK
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" GPU Metrics ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(table, area);
}
