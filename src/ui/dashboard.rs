use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table},
    Frame,
};

use crate::data::DataStore;

// Simple color scheme: green and cyan
const COLOR_ACCENT: Color = Color::Cyan;
const COLOR_HEADER: Color = Color::Cyan;
const COLOR_BAR: Color = Color::Green;
const COLOR_HIGHLIGHT: Color = Color::Cyan;

/// Sparkline characters (8 levels)
const SPARKLINE_CHARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

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
    while result.chars().count() < width {
        result.insert(0, ' ');
    }
    result
}

fn fmt_val(val: Option<u32>, unit: &str) -> String {
    match val {
        Some(v) => format!("{}{}", v, unit),
        None => "-".to_string(),
    }
}

pub fn render_dashboard(frame: &mut Frame, area: Rect, data: &DataStore, selected_gpu: usize) {
    let gpu_indices = data.gpu_indices();
    let gpu_count = gpu_indices.len().max(1);

    // Calculate layout based on GPU count
    let table_height = (gpu_count as u16 + 3).min(10); // header + rows + margin
    let memory_height = (gpu_count as u16 * 2 + 2).min(12);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(table_height),   // GPU metrics table
            Constraint::Length(memory_height),  // Memory/power bars
            Constraint::Min(6),                 // Processes
        ])
        .split(area);

    // === GPU Metrics Table ===
    render_gpu_table(frame, chunks[0], data, selected_gpu);

    // === Memory & Power Section ===
    render_memory_section(frame, chunks[1], data);

    // === Processes Section ===
    render_processes_section(frame, chunks[2], data);
}

fn render_gpu_table(frame: &mut Frame, area: Rect, data: &DataStore, selected_gpu: usize) {
    let gpu_indices = data.gpu_indices();

    let header_cells = ["GPU", "Power", "Temp", "SM%", "Mem%", "Enc", "Dec", "MCLK", "PCLK"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(COLOR_HEADER).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(0);

    let rows: Vec<Row> = gpu_indices
        .iter()
        .enumerate()
        .map(|(i, &gpu_idx)| {
            let history = data.get_gpu(gpu_idx);
            let latest = history.and_then(|h| h.latest());

            let (power, temp, _sm, _mem, enc, dec, mclk, pclk) = match latest {
                Some(s) => (
                    fmt_val(s.power_w, "W"),
                    fmt_val(s.gpu_temp_c, "°"),
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
        Constraint::Length(4),
        Constraint::Length(5),
        Constraint::Length(4),
        Constraint::Length(9),
        Constraint::Length(9),
        Constraint::Length(4),
        Constraint::Length(4),
        Constraint::Length(5),
        Constraint::Length(5),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" GPU Metrics ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        );

    frame.render_widget(table, area);
}

fn render_memory_section(frame: &mut Frame, area: Rect, data: &DataStore) {
    let gpu_infos = data.all_gpu_info();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Memory & Power ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    if gpu_infos.is_empty() {
        frame.render_widget(block, area);
        return;
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Two rows per GPU: memory bar + power info
    let constraints: Vec<Constraint> = gpu_infos
        .iter()
        .map(|_| Constraint::Length(2))
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

        let row_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(6), Constraint::Min(20), Constraint::Length(25)])
            .split(chunks[i]);

        // GPU label
        let label = Paragraph::new(format!("GPU{}", gpu.index))
            .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
        frame.render_widget(label, row_chunks[0]);

        // Memory gauge
        let used = gpu.memory_used_mib;
        let total = gpu.memory_total_mib;
        let pct = if total > 0 { (used as f64 / total as f64 * 100.0) as u16 } else { 0 };

        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(COLOR_BAR).bg(Color::DarkGray))
            .percent(pct)
            .label(format!("{}/{} MiB", used, total));
        frame.render_widget(gauge, row_chunks[1]);

        // Power/temp info
        let power_str = gpu.power_draw_w
            .map(|p| format!("{:.0}W", p))
            .unwrap_or("-".into());
        let temp_str = gpu.temperature_c
            .map(|t| format!("{}°C", t))
            .unwrap_or("-".into());

        let info = Paragraph::new(Line::from(vec![
            Span::styled(power_str, Style::default().fg(Color::White)),
            Span::raw(" "),
            Span::styled(temp_str, Style::default().fg(Color::White)),
        ]));
        frame.render_widget(info, row_chunks[2]);
    }
}

fn format_vram(mib: u64) -> String {
    if mib >= 1024 {
        format!("{:.1} GiB", mib as f64 / 1024.0)
    } else {
        format!("{} MiB", mib)
    }
}

fn format_ram(mb: u64) -> String {
    if mb >= 1024 {
        format!("{:.1}G", mb as f64 / 1024.0)
    } else {
        format!("{}M", mb)
    }
}

fn render_processes_section(frame: &mut Frame, area: Rect, data: &DataStore) {
    let processes = data.get_enriched_processes();

    let header_cells = ["GPU", "PID", "VRAM", "SM%", "CPU%", "RAM", "Time", "Command"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(COLOR_HEADER).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(0);

    let rows: Vec<Row> = processes
        .iter()
        .map(|p| {
            // VRAM - always show actual allocation
            let vram_str = format_vram(p.vram_mib);

            // SM utilization from pmon (instantaneous - may be "-" when idle)
            let sm_str = p.sm_util.map(|v| format!("{}%", v)).unwrap_or("-".into());

            // CPU and RAM from /proc
            let cpu_str = if p.cpu_percent > 0.0 {
                format!("{:.1}%", p.cpu_percent)
            } else {
                "-".into()
            };
            let ram_str = if p.rss_mb > 0 {
                format_ram(p.rss_mb)
            } else {
                "-".into()
            };

            Row::new(vec![
                Cell::from(format!("{}", p.gpu_idx)),
                Cell::from(format!("{}", p.pid)),
                Cell::from(vram_str).style(Style::default().fg(COLOR_HIGHLIGHT)),
                Cell::from(sm_str).style(Style::default().fg(Color::Green)),
                Cell::from(cpu_str),
                Cell::from(ram_str),
                Cell::from(p.elapsed.clone()).style(Style::default().fg(Color::Gray)),
                Cell::from(p.command.clone()),
            ])
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(4),   // GPU
        Constraint::Length(7),   // PID
        Constraint::Length(9),   // VRAM
        Constraint::Length(5),   // SM%
        Constraint::Length(6),   // CPU%
        Constraint::Length(6),   // RAM
        Constraint::Length(8),   // Time
        Constraint::Min(12),     // Command
    ];

    let title = if processes.is_empty() {
        " Processes (none) "
    } else {
        " Processes "
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD)),
        );

    frame.render_widget(table, area);
}
