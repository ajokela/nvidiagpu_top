use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    DefaultTerminal, Frame,
};
use std::time::Duration;

use crate::data::DataStore;
use crate::process::{NvidiaMonitor, NvidiaMessage};
use crate::ui::dashboard::render_dashboard;
use crate::ui::charts::render_chart_view;
use crate::ui::status::{render_status_bar, render_help_bar};
use crate::ui::topology::render_topology_view;
use crate::ui::info::render_info_view;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Dashboard,
    Charts,
}

impl ViewMode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Charts => "Charts",
        }
    }

    pub fn all() -> &'static [ViewMode] {
        &[ViewMode::Dashboard, ViewMode::Charts]
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Dashboard => Self::Charts,
            Self::Charts => Self::Dashboard,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    None,
    Info,
    Topology,
}

pub struct App {
    data: DataStore,
    view_mode: ViewMode,
    overlay: Overlay,
    selected_gpu: usize,
    error: Option<String>,
    should_quit: bool,
}

impl App {
    pub fn new(history_seconds: u64) -> Self {
        Self {
            data: DataStore::new(history_seconds),
            view_mode: ViewMode::Dashboard,
            overlay: Overlay::None,
            selected_gpu: 0,
            error: None,
            should_quit: false,
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        // Query topology once at startup
        match NvidiaMonitor::query_topology().await {
            Ok(topo) => self.data.set_topology(topo),
            Err(e) => self.error = Some(format!("Topology: {}", e)),
        }

        // Spawn all monitoring processes
        let (_monitor, mut rx) = match NvidiaMonitor::spawn().await {
            Ok((m, r)) => (m, r),
            Err(e) => {
                self.error = Some(e.to_string());
                while !self.should_quit {
                    terminal.draw(|frame| self.render(frame))?;
                    if self.handle_events()? {
                        break;
                    }
                }
                return Ok(());
            }
        };

        loop {
            terminal.draw(|frame| self.render(frame))?;

            if event::poll(Duration::from_millis(100))? {
                if self.handle_events()? {
                    break;
                }
            }

            while let Ok(msg) = rx.try_recv() {
                match msg {
                    NvidiaMessage::GpuSample(sample) => {
                        self.data.add_sample(sample);
                        self.error = None;
                    }
                    NvidiaMessage::ProcessSample(sample) => {
                        self.data.add_process_sample(sample);
                    }
                    NvidiaMessage::GpuInfo(info) => {
                        self.data.update_gpu_info(info);
                    }
                    NvidiaMessage::ComputeApps(apps) => {
                        self.data.update_compute_apps(apps);
                    }
                    NvidiaMessage::ProcessSystemInfo(infos) => {
                        self.data.update_process_sys_info(infos);
                    }
                    NvidiaMessage::Error(e) => {
                        self.error = Some(e);
                    }
                    NvidiaMessage::Exited(which) => {
                        self.error = Some(format!("{} exited", which));
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    fn handle_events(&mut self) -> Result<bool> {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                // If overlay is open, Esc/Enter/same key closes it
                if self.overlay != Overlay::None {
                    match key.code {
                        KeyCode::Esc | KeyCode::Enter => {
                            self.overlay = Overlay::None;
                        }
                        KeyCode::Char('i') => {
                            self.overlay = if self.overlay == Overlay::Info {
                                Overlay::None
                            } else {
                                Overlay::Info
                            };
                        }
                        KeyCode::Char('t') => {
                            self.overlay = if self.overlay == Overlay::Topology {
                                Overlay::None
                            } else {
                                Overlay::Topology
                            };
                        }
                        KeyCode::Char('q') => {
                            self.should_quit = true;
                            return Ok(true);
                        }
                        _ => {}
                    }
                    return Ok(false);
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        self.should_quit = true;
                        return Ok(true);
                    }
                    KeyCode::Tab => {
                        self.view_mode = self.view_mode.next();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if self.selected_gpu > 0 {
                            self.selected_gpu -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let max_gpu = self.data.gpu_indices().len().saturating_sub(1);
                        if self.selected_gpu < max_gpu {
                            self.selected_gpu += 1;
                        }
                    }
                    KeyCode::Char('1') => self.view_mode = ViewMode::Dashboard,
                    KeyCode::Char('2') => self.view_mode = ViewMode::Charts,
                    KeyCode::Char('i') => self.overlay = Overlay::Info,
                    KeyCode::Char('t') => self.overlay = Overlay::Topology,
                    _ => {}
                }
            }
        }
        Ok(false)
    }

    fn render(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.area());

        // Status bar
        render_status_bar(
            frame,
            chunks[0],
            self.data.total_samples(),
            self.data.uptime(),
            &self.view_mode,
            self.error.as_deref(),
        );

        // Main content
        match self.view_mode {
            ViewMode::Dashboard => {
                render_dashboard(frame, chunks[1], &self.data, self.selected_gpu);
            }
            ViewMode::Charts => {
                render_chart_view(frame, chunks[1], &self.data, self.selected_gpu);
            }
        }

        // Help bar
        render_help_bar(frame, chunks[2]);

        // Render overlay if active
        match self.overlay {
            Overlay::None => {}
            Overlay::Info => {
                self.render_overlay(frame, "GPU Info", |f, area| {
                    render_info_view(f, area, &self.data, self.selected_gpu);
                });
            }
            Overlay::Topology => {
                self.render_overlay(frame, "Topology", |f, area| {
                    render_topology_view(f, area, &self.data);
                });
            }
        }
    }

    fn render_overlay<F>(&self, frame: &mut Frame, title: &str, render_fn: F)
    where
        F: FnOnce(&mut Frame, Rect),
    {
        let area = frame.area();

        // Center the overlay, taking 80% of screen
        let popup_width = (area.width as f32 * 0.8) as u16;
        let popup_height = (area.height as f32 * 0.8) as u16;
        let popup_x = (area.width - popup_width) / 2;
        let popup_y = (area.height - popup_height) / 2;

        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        // Clear the area behind the popup
        frame.render_widget(Clear, popup_area);

        // Render a border
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(format!(" {} ", title))
            .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        // Render content
        render_fn(frame, inner);

        // Hint at bottom
        let hint = Paragraph::new(Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled(" or ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(" to close", Style::default().fg(Color::DarkGray)),
        ]));
        let hint_area = Rect::new(popup_x + 2, popup_y + popup_height - 1, popup_width - 4, 1);
        frame.render_widget(hint, hint_area);
    }
}
