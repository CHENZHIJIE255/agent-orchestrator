mod events;

use parking_lot::RwLock;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
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
    pub messages: Vec<ChatMessage>,
    pub waiting_for_project_name: bool,
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
            messages: Vec::new(),
            waiting_for_project_name: false,
        }
    }
}

pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

pub struct ApprovalRequest {
    pub id: String,
    pub agent: String,
    pub description: String,
    pub timestamp: i64,
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
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(f.area());

        self.render_header(f, chunks[0]);
        self.render_chat(f, chunks[1]);
        self.render_input(f, chunks[2]);
    }

    fn render_header(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let state = self.state.read();
        let project = state.current_project.as_deref().unwrap_or("No project");

        let text = Line::from(vec![
            Span::styled("Agent | ", Style::default().fg(Color::Cyan)),
            Span::styled(project, Style::default().fg(Color::Green)),
        ]);

        f.render_widget(
            Paragraph::new(text).style(Style::default().fg(Color::White)),
            area,
        );
    }

    fn render_chat(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        let state = self.state.read();

        let user_items: Vec<ListItem> = state
            .messages
            .iter()
            .filter(|m| m.role == "user")
            .map(|m| ListItem::new(Span::styled(&m.content, Style::default().fg(Color::White))))
            .collect();

        let ai_items: Vec<ListItem> = state
            .messages
            .iter()
            .filter(|m| m.role == "ai")
            .map(|m| ListItem::new(Span::styled(&m.content, Style::default().fg(Color::White))))
            .collect();

        let logs: Vec<ListItem> = state
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
                ListItem::new(Span::styled(&log.message, Style::default().fg(color)))
            })
            .collect();

        let left_items: Vec<ListItem> = user_items.into_iter().chain(ai_items).collect();
        let left_list = List::new(left_items);
        f.render_widget(left_list, chunks[0]);

        let right_list = List::new(logs);
        f.render_widget(right_list, chunks[1]);
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
        let now = chrono::Local::now().format("%H:%M").to_string();
        state.messages.push(ChatMessage {
            role: "ai".to_string(),
            content: message.to_string(),
            timestamp: now,
        });
    }

    pub fn set_waiting_for_project_name(&self, waiting: bool) {
        let mut state = self.state.write();
        state.waiting_for_project_name = waiting;
    }

    pub fn is_waiting_for_project_name(&self) -> bool {
        let state = self.state.read();
        state.waiting_for_project_name
    }

    pub fn add_user_message(&self, message: &str) {
        let mut state = self.state.write();
        let now = chrono::Local::now().format("%H:%M").to_string();
        state.messages.push(ChatMessage {
            role: "user".to_string(),
            content: message.to_string(),
            timestamp: now,
        });
    }
}
