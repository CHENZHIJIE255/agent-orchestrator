mod events;

use parking_lot::RwLock;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation},
    Frame,
};
use std::sync::Arc;

use crate::agent::TaskStatus;
use crate::i18n;
use crate::logger::{LogEntry, Logger};

pub struct TUIState {
    pub current_project: Option<String>,
    pub task_status: TaskStatus,
    pub logs: Vec<LogEntry>,
    pub error_logs: Vec<LogEntry>,
    pub pending_approvals: Vec<ApprovalRequest>,
    pub input_buffer: String,
    pub message: String,
}

pub struct ApprovalRequest {
    pub id: String,
    pub agent: String,
    pub description: String,
    pub timestamp: i64,
}

impl Default for TUIState {
    fn default() -> Self {
        Self {
            current_project: None,
            task_status: TaskStatus::Pending,
            logs: Vec::new(),
            error_logs: Vec::new(),
            pending_approvals: Vec::new(),
            input_buffer: String::new(),
            message: String::new(),
        }
    }
}

pub struct TUI {
    state: Arc<RwLock<TUIState>>,
    logger: Arc<Logger>,
}

impl Clone for TUI {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            logger: self.logger.clone(),
        }
    }
}

impl TUI {
    pub fn new(logger: Arc<Logger>) -> Self {
        let tui = Self {
            state: Arc::new(RwLock::new(TUIState::default())),
            logger,
        };
        tui.set_message(&i18n::t("app.welcome"));
        tui
    }

    pub fn get_state(&self) -> Arc<RwLock<TUIState>> {
        self.state.clone()
    }

    pub fn render(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        self.render_header(f, chunks[0]);
        self.render_logs(f, chunks[1]);
        self.render_input(f, chunks[2]);
    }

    fn render_header(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let state = self.state.read();
        let project = state.current_project.as_deref().unwrap_or("No project");
        let status = format!("{:?}", state.task_status);

        let text = Line::from(vec![
            Span::raw("["),
            Span::styled("Agent", Style::default().fg(Color::Cyan)),
            Span::raw("] "),
            Span::styled(project, Style::default().fg(Color::Green)),
            Span::raw(" | "),
            Span::styled(status, Style::default().fg(Color::Yellow)),
        ]);

        f.render_widget(
            Paragraph::new(text).style(Style::default().fg(Color::White)),
            area,
        );
    }

    fn render_logs(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let state = self.state.read();

        let log_items: Vec<ListItem> = state
            .logs
            .iter()
            .rev()
            .take(30)
            .map(|log| {
                let color = match log.level {
                    crate::logger::LogLevel::Info => Color::White,
                    crate::logger::LogLevel::Warning => Color::Yellow,
                    crate::logger::LogLevel::Error => Color::Red,
                    crate::logger::LogLevel::Debug => Color::DarkGray,
                };

                let content = format!(
                    "[{}] {}: {}",
                    log.timestamp.format("%H:%M:%S"),
                    log.agent,
                    log.message
                );

                ListItem::new(Span::styled(content, Style::default().fg(color)))
            })
            .collect();

        let list = List::new(log_items);
        f.render_widget(list, area);

        if !state.message.is_empty() {
            let msg_y = area.y + area.height.saturating_sub(1);
            let msg_area = ratatui::layout::Rect::new(area.x, msg_y, area.width, 1);
            let msg_text = Line::from(vec![
                Span::styled("> ", Style::default().fg(Color::Cyan)),
                Span::styled(&state.message, Style::default().fg(Color::White)),
            ]);
            f.render_widget(Paragraph::new(msg_text), msg_area);
        }
    }

    fn render_input(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let state = self.state.read();
        let input = format!("> {}", state.input_buffer);
        let text = Line::from(Span::styled(input, Style::default().fg(Color::Green)));
        f.render_widget(Paragraph::new(text), area);
    }

    pub fn update_logs(&self) {
        let mut state = self.state.write();
        state.logs = self.logger.get_logs();
        state.error_logs = self.logger.get_error_logs();
    }

    pub fn set_message(&self, message: &str) {
        let mut state = self.state.write();
        state.message = message.to_string();
    }
}
