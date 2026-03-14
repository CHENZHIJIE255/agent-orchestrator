use super::trait_decls::{AgentInfo, AgentMessage, AgentType, AgentLevel, TaskStatus};
use crate::logger::Logger;
use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone, Serialize, Deserialize)]
pub struct AgentHandle {
    pub info: AgentInfo,
    pub status: TaskStatus,
}

pub struct AgentPool {
    agents: Arc<RwLock<HashMap<String, AgentHandle>>>,
    sender: mpsc::Sender<AgentMessage>,
    receiver: mpsc::Receiver<AgentMessage>,
    max_size: usize,
    current_size: usize,
    logger: Arc<Logger>,
}

impl AgentPool {
    pub fn new(max_size: usize, logger: Arc<Logger>) -> Self {
        let (sender, receiver) = mpsc::channel(100);
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            sender,
            receiver,
            max_size,
            current_size: 0,
            logger,
        }
    }

    pub async fn spawn_agent(
        &mut self, 
        name: String, 
        agent_type: AgentType, 
        level: AgentLevel,
    ) -> Result<String> {
        if self.current_size >= self.max_size {
            anyhow::bail!("Agent pool is full");
        }

        let id = uuid::Uuid::new_v4().to_string();
        let info = AgentInfo::new(name, agent_type, level);

        let handle = AgentHandle {
            info,
            status: TaskStatus::Pending,
        };

        self.agents.write().insert(id.clone(), handle);
        self.current_size += 1;

        self.logger.info(&format!("Agent-{}", id), &format!("Agent spawned: {:?}", level));

        Ok(id)
    }

    pub async fn send_message(&self, message: AgentMessage) -> Result<()> {
        self.sender.send(message).await?;
        Ok(())
    }

    pub async fn process_messages(&mut self) {
        while let Some(message) = self.receiver.recv().await {
            let to_id = message.to.0.clone();
            let mut agents = self.agents.write();
            
            if let Some(agent) = agents.get_mut(&to_id) {
                self.logger.info(&to_id, &format!("Processing message: {:?}", message.action));
                agent.status = TaskStatus::Running;
            }
        }
    }

    pub fn get_agent(&self, id: &str) -> Option<AgentHandle> {
        self.agents.read().get(id).cloned()
    }

    pub fn remove_agent(&mut self, id: &str) -> Option<AgentHandle> {
        if let Some(agent) = self.agents.write().remove(id) {
            self.current_size -= 1;
            self.logger.info(id, "Agent removed");
            Some(agent)
        } else {
            None
        }
    }

    pub fn get_active_agents(&self) -> Vec<AgentInfo> {
        self.agents.read()
            .values()
            .map(|a| a.info.clone())
            .collect()
    }

    pub fn get_sender(&self) -> mpsc::Sender<AgentMessage> {
        self.sender.clone()
    }

    pub fn update_status(&self, id: &str, status: TaskStatus) {
        if let Some(agent) = self.agents.write().get_mut(id) {
            agent.status = status;
        }
    }

    pub fn get_logs(&self) -> Vec<crate::logger::LogEntry> {
        self.logger.get_logs()
    }

    pub fn get_error_logs(&self) -> Vec<crate::logger::LogEntry> {
        self.logger.get_error_logs()
    }
}
