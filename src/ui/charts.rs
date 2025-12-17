use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType},
    Frame,
};

use crate::data::DataStore;

pub fn render_chart_view(frame: &mut Frame, area: Rect, data: &DataStore, selected_gpu: usize) {
    let gpu_indices = data.gpu_indices();

    if gpu_indices.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Charts - No Data ")
            .title_style(Style::default().fg(Color::Yellow));
        frame.render_widget(block, area);
        return;
    }

    let gpu_idx = gpu_indices.get(selected_gpu).copied().unwrap_or(0);
    let history = match data.get_gpu(gpu_idx) {
        Some(h) => h,
        None => return,
    };

    // Split into 3 chart areas
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(area);

    // Get chart data
    let power_data: Vec<(f64, f64)> = history.chart_data(|s| s.power_w);
    let temp_data: Vec<(f64, f64)> = history.chart_data(|s| s.gpu_temp_c);
    let sm_data: Vec<(f64, f64)> = history.chart_data(|s| s.sm_util);
    let mem_data: Vec<(f64, f64)> = history.chart_data(|s| s.mem_util);

    // Calculate x-axis bounds
    let x_min = power_data
        .first()
        .map(|(x, _)| *x)
        .unwrap_or(-60.0)
        .min(-60.0);
    let x_max = 0.0;

    // Power chart
    render_single_chart(
        frame,
        chunks[0],
        &format!(" GPU {} - Power (W) ", gpu_idx),
        &power_data,
        x_min,
        x_max,
        0.0,
        400.0, // Max TDP for high-end GPUs
        Color::Yellow,
    );

    // Temperature chart
    render_single_chart(
        frame,
        chunks[1],
        &format!(" GPU {} - Temperature (°C) ", gpu_idx),
        &temp_data,
        x_min,
        x_max,
        0.0,
        100.0,
        Color::Red,
    );

    // Utilization chart (SM and Memory)
    render_dual_chart(
        frame,
        chunks[2],
        &format!(" GPU {} - Utilization (%) ", gpu_idx),
        &sm_data,
        &mem_data,
        x_min,
        x_max,
        "SM",
        "Mem",
        Color::Green,
        Color::Cyan,
    );
}

fn render_single_chart(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    data: &[(f64, f64)],
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    color: Color,
) {
    let dataset = Dataset::default()
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(color))
        .data(data);

    let chart = Chart::new(vec![dataset])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(Style::default().fg(color).add_modifier(Modifier::BOLD)),
        )
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::Gray))
                .bounds([x_min, x_max])
                .labels(vec![
                    Span::from(format!("{:.0}s", x_min)),
                    Span::from("now"),
                ]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::Gray))
                .bounds([y_min, y_max])
                .labels(vec![
                    Span::from(format!("{:.0}", y_min)),
                    Span::from(format!("{:.0}", y_max)),
                ]),
        );

    frame.render_widget(chart, area);
}

fn render_dual_chart(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    data1: &[(f64, f64)],
    data2: &[(f64, f64)],
    x_min: f64,
    x_max: f64,
    label1: &str,
    label2: &str,
    color1: Color,
    color2: Color,
) {
    let datasets = vec![
        Dataset::default()
            .name(label1)
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(color1))
            .data(data1),
        Dataset::default()
            .name(label2)
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(color2))
            .data(data2),
    ];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        )
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::Gray))
                .bounds([x_min, x_max])
                .labels(vec![
                    Span::from(format!("{:.0}s", x_min)),
                    Span::from("now"),
                ]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, 100.0])
                .labels(vec![
                    Span::from("0"),
                    Span::from("100"),
                ]),
        );

    frame.render_widget(chart, area);
}
