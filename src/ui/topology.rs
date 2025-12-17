use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::data::DataStore;
use crate::parser::GpuLink;

pub fn render_topology_view(frame: &mut Frame, area: Rect, data: &DataStore) {
    let topo = match data.get_topology() {
        Some(t) => t,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" GPU Topology - No data ")
                .title_style(Style::default().fg(Color::Yellow));
            frame.render_widget(block, area);
            return;
        }
    };

    if topo.matrix.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" GPU Topology - No GPUs found ")
            .title_style(Style::default().fg(Color::Yellow));
        frame.render_widget(block, area);
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" GPU Topology ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build header row
    let mut header_cells = vec![Cell::from("").style(Style::default())];
    for i in 0..topo.matrix.len() {
        header_cells.push(
            Cell::from(format!("GPU{}", i))
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        );
    }
    // Add CPU/NUMA affinity headers
    header_cells.push(Cell::from("CPU Affinity").style(Style::default().fg(Color::Yellow)));
    header_cells.push(Cell::from("NUMA").style(Style::default().fg(Color::Yellow)));

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    // Build data rows
    let mut rows = Vec::new();
    for (i, row) in topo.matrix.iter().enumerate() {
        let mut cells = vec![
            Cell::from(format!("GPU{}", i))
                .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ];

        for link in row.iter() {
            let (text, style) = match link {
                Some(GpuLink::Self_) => ("X", Style::default().fg(Color::DarkGray)),
                Some(GpuLink::PIX) => ("PIX", Style::default().fg(Color::Green)),
                Some(GpuLink::PXB) => ("PXB", Style::default().fg(Color::Yellow)),
                Some(GpuLink::PHB) => ("PHB", Style::default().fg(Color::Yellow)),
                Some(GpuLink::NODE) => ("NODE", Style::default().fg(Color::Cyan)),
                Some(GpuLink::SYS) => ("SYS", Style::default().fg(Color::Red)),
                Some(GpuLink::NVLink(n)) => {
                    // NVLink is fastest - format as NVx
                    (Box::leak(format!("NV{}", n).into_boxed_str()) as &str, Style::default().fg(Color::Magenta))
                }
                None => ("-", Style::default().fg(Color::DarkGray)),
            };
            cells.push(Cell::from(text).style(style));
        }

        // Add CPU/NUMA affinity
        let cpu_aff = topo.cpu_affinity.get(i).map(|s| s.as_str()).unwrap_or("-");
        let numa_aff = topo.numa_affinity.get(i).map(|s| s.as_str()).unwrap_or("-");
        cells.push(Cell::from(cpu_aff).style(Style::default().fg(Color::DarkGray)));
        cells.push(Cell::from(numa_aff).style(Style::default().fg(Color::DarkGray)));

        rows.push(Row::new(cells).height(1));
    }

    // Build constraints
    let mut widths = vec![Constraint::Length(5)]; // Row label
    for _ in 0..topo.matrix.len() {
        widths.push(Constraint::Length(5)); // GPU columns
    }
    widths.push(Constraint::Length(16)); // CPU Affinity
    widths.push(Constraint::Length(6));  // NUMA

    let table = Table::new(rows, widths).header(header);

    // Split area for table and legend
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(6)])
        .split(inner);

    frame.render_widget(table, chunks[0]);

    // Legend
    let legend = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Legend: ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("PIX", Style::default().fg(Color::Green)),
            Span::raw(" = Single PCIe bridge (fast)  "),
            Span::styled("PXB", Style::default().fg(Color::Yellow)),
            Span::raw(" = Multiple PCIe bridges  "),
            Span::styled("PHB", Style::default().fg(Color::Yellow)),
            Span::raw(" = PCIe Host Bridge"),
        ]),
        Line::from(vec![
            Span::styled("NODE", Style::default().fg(Color::Cyan)),
            Span::raw(" = Same NUMA node  "),
            Span::styled("SYS", Style::default().fg(Color::Red)),
            Span::raw(" = Cross NUMA (slow)  "),
            Span::styled("NVx", Style::default().fg(Color::Magenta)),
            Span::raw(" = NVLink (fastest)"),
        ]),
    ]);

    frame.render_widget(legend, chunks[1]);
}
