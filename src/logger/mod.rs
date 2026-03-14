use parking_lot::RwLock;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Debug, Clone)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Debug,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: LogLevel,
    pub agent: String,
    pub message: String,
    pub task_id: Option<String>,
}

pub struct Logger {
    logs: Arc<RwLock<VecDeque<LogEntry>>>,
    log_dir: PathBuf,
    max_entries: usize,
}

impl Logger {
    pub fn new(log_dir: PathBuf, max_entries: usize) -> Self {
        std::fs::create_dir_all(&log_dir).ok();
        
        let file_appender = RollingFileAppender::new(
            Rotation::DAILY,
            &log_dir,
            "agent-orchestrator.log",
        );

        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"));

        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(non_blocking))
            .with(fmt::layer().with_writer(std::io::stderr))
            .init();

        Self {
            logs: Arc::new(RwLock::new(VecDeque::new())),
            log_dir,
            max_entries,
        }
    }

    pub fn log(&self, level: LogLevel, agent: &str, message: &str, task_id: Option<String>) {
        let entry = LogEntry {
            timestamp: chrono::Utc::now(),
            level: level.clone(),
            agent: agent.to_string(),
            message: message.to_string(),
            task_id,
        };

        match level {
            LogLevel::Info => tracing::info!("[{}] {}", agent, message),
            LogLevel::Warning => tracing::warn!("[{}] {}", agent, message),
            LogLevel::Error => tracing::error!("[{}] {}", agent, message),
            LogLevel::Debug => tracing::debug!("[{}] {}", agent, message),
        }

        let mut logs = self.logs.write();
        logs.push_back(entry);
        if logs.len() > self.max_entries {
            logs.pop_front();
        }
    }

    pub fn info(&self, agent: &str, message: &str) {
        self.log(LogLevel::Info, agent, message, None);
    }

    pub fn warn(&self, agent: &str, message: &str) {
        self.log(LogLevel::Warning, agent, message, None);
    }

    pub fn error(&self, agent: &str, message: &str) {
        self.log(LogLevel::Error, agent, message, None);
    }

    pub fn get_logs(&self) -> Vec<LogEntry> {
        self.logs.read().iter().cloned().collect()
    }

    pub fn get_agent_logs(&self, agent: &str) -> Vec<LogEntry> {
        self.logs.read()
            .iter()
            .filter(|e| e.agent == agent)
            .cloned()
            .collect()
    }

    pub fn get_error_logs(&self) -> Vec<LogEntry> {
        self.logs.read()
            .iter()
            .filter(|e| matches!(e.level, LogLevel::Error | LogLevel::Warning))
            .cloned()
            .collect()
    }

    pub fn get_log_dir(&self) -> &PathBuf {
        &self.log_dir
    }
}

pub fn init_logger(install_dir: &PathBuf) -> Logger {
    let log_dir = install_dir.join("logs");
    Logger::new(log_dir, 10000)
}
