use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentType {
    Branch,
    Leaf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentLevel {
    L0,
    L1,
    L2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    WaitingApproval,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertLevel {
    P0,
    P1,
    P2,
    P3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskId(pub String);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: AgentId,
    pub name: String,
    pub agent_type: AgentType,
    pub level: AgentLevel,
    pub task_id: Option<TaskId>,
    pub status: TaskStatus,
}

impl AgentInfo {
    pub fn new(name: String, agent_type: AgentType, level: AgentLevel) -> Self {
        Self {
            id: AgentId::new(),
            name,
            agent_type,
            level,
            task_id: None,
            status: TaskStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub from: AgentId,
    pub to: AgentId,
    pub task_id: Option<TaskId>,
    pub action: MessageAction,
    pub payload: serde_json::Value,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageAction {
    StartTask,
    SubmitDesign,
    SubmitCode,
    RequestApproval,
    Approve,
    Reject,
    ContextWarning,
    TaskComplete,
    TaskFailed,
}

impl AgentMessage {
    pub fn new(from: AgentId, to: AgentId, action: MessageAction, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from,
            to,
            task_id: None,
            action,
            payload,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}
