mod events;

use parking_lot::RwLock;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
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
            message: String::from(i18n::t("app.welcome")),
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
        Self {
            state: Arc::new(RwLock::new(TUIState::default())),
            logger,
        }
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
                Constraint::Length(3),
            ])
            .split(f.area());

        self.render_header(f, chunks[0]);
        self.render_main_panels(f, chunks[1]);
        self.render_approval_panel(f, chunks[2]);
        self.render_input_panel(f, chunks[3]);
    }

    fn render_header(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let state = self.state.read();
        let project = state.current_project.as_deref().unwrap_or("No project");
        let status = format!("{:?}", state.task_status);
        let message = &state.message;

        let header_text = Line::from(vec![
            Span::raw("AgentOrchestrator | "),
            Span::styled(project, Style::default().fg(Color::Cyan)),
            Span::raw(" | Status: "),
            Span::styled(status, Style::default().fg(Color::Green)),
            Span::raw(" | "),
            Span::styled(message, Style::default().fg(Color::Yellow)),
        ]);

        let block = Block::default().borders(Borders::ALL).title(" Status ");

        let paragraph = Paragraph::new(header_text)
            .block(block)
            .style(Style::default().fg(Color::White));

        f.render_widget(paragraph, area);
    }

    fn render_main_panels(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        self.render_log_panel(f, chunks[0]);
        self.render_alert_panel(f, chunks[1]);
    }

    fn render_log_panel(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let state = self.state.read();

        let items: Vec<ListItem> = state
            .logs
            .iter()
            .rev()
            .take(50)
            .map(|log| {
                let color = match log.level {
                    crate::logger::LogLevel::Info => Color::White,
                    crate::logger::LogLevel::Warning => Color::Yellow,
                    crate::logger::LogLevel::Error => Color::Red,
                    crate::logger::LogLevel::Debug => Color::DarkGray,
                };

                let content = format!(
                    "[{}] [{}] {}",
                    log.timestamp.format("%H:%M:%S"),
                    log.agent,
                    log.message
                );

                ListItem::new(Span::styled(content, Style::default().fg(color)))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(i18n::t("logs.title")),
        );

        f.render_widget(list, area);
    }

    fn render_alert_panel(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let state = self.state.read();

        let items: Vec<ListItem> = state
            .error_logs
            .iter()
            .rev()
            .take(20)
            .map(|log| {
                let (prefix, color) = match log.level {
                    crate::logger::LogLevel::Warning => ("⚠️", Color::Yellow),
                    crate::logger::LogLevel::Error => ("❌", Color::Red),
                    _ => ("ℹ️", Color::White),
                };

                let content = format!("{} [{}] {}", prefix, log.agent, log.message);
                ListItem::new(Span::styled(content, Style::default().fg(color)))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(i18n::t("alerts.title")),
        );

        f.render_widget(list, area);
    }

    fn render_approval_panel(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let state = self.state.read();

        let approval_text = if state.pending_approvals.is_empty() {
            Line::from(i18n::t("approval.no_pending"))
        } else {
            let approval = &state.pending_approvals[0];
            Line::from(vec![
                Span::raw("Pending: "),
                Span::styled(&approval.agent, Style::default().fg(Color::Yellow)),
                Span::raw(" - "),
                Span::raw(&approval.description),
                Span::raw(" | [Approve] [Reject] [Auto] "),
            ])
        };

        let block = Block::default().borders(Borders::ALL).title(" Approvals ");

        let paragraph = Paragraph::new(approval_text)
            .block(block)
            .style(Style::default().fg(Color::White));

        f.render_widget(paragraph, area);
    }

    fn render_input_panel(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let state = self.state.read();

        let input_text = format!("> {}", state.input_buffer);

        let block = Block::default().borders(Borders::ALL).title(" Input ");

        let paragraph = Paragraph::new(input_text)
            .block(block)
            .style(Style::default().fg(Color::White));

        f.render_widget(paragraph, area);
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
