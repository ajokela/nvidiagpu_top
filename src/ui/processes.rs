use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

use crate::data::DataStore;

pub fn render_process_view(frame: &mut Frame, area: Rect, data: &DataStore) {
    let processes = data.get_processes();

    let header_cells = ["GPU", "PID", "Type", "SM%", "Mem%", "Command"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = processes
        .iter()
        .map(|proc_info| {
            let p = &proc_info.sample;

            let sm_str = p.sm_util.map(|v| format!("{}%", v)).unwrap_or("-".into());
            let mem_str = p.mem_util.map(|v| format!("{}%", v)).unwrap_or("-".into());

            let type_style = match p.process_type.as_str() {
                "C" => Style::default().fg(Color::Green),
                "G" => Style::default().fg(Color::Blue),
                _ => Style::default(),
            };

            Row::new(vec![
                Cell::from(format!("{}", p.gpu_idx)),
                Cell::from(format!("{}", p.pid)),
                Cell::from(p.process_type.clone()).style(type_style),
                Cell::from(sm_str),
                Cell::from(mem_str),
                Cell::from(p.command.clone()),
            ])
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(4),   // GPU
        Constraint::Length(8),   // PID
        Constraint::Length(5),   // Type
        Constraint::Length(6),   // SM%
        Constraint::Length(6),   // Mem%
        Constraint::Min(20),     // Command
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" GPU Processes ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        );

    frame.render_widget(table, area);
}
