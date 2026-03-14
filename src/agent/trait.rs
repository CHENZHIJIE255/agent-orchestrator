use async_trait::async_trait;
use crate::agent::{AgentInfo, AgentMessage, TaskStatus};

#[async_trait]
pub trait Agent: Send + Sync {
    fn get_info(&self) -> &AgentInfo;
    fn get_info_mut(&mut self) -> &mut AgentInfo;
    
    async fn start(&mut self) -> anyhow::Result<()>;
    async fn execute(&mut self, message: AgentMessage) -> anyhow::Result<AgentMessage>;
    async fn stop(&mut self) -> anyhow::Result<()>;
    
    fn set_status(&mut self, status: TaskStatus) {
        self.get_info_mut().status = status;
    }
    
    fn get_status(&self) -> TaskStatus {
        self.get_info().status
    }
    
    fn is_branch(&self) -> bool {
        matches!(self.get_info().agent_type, crate::agent::AgentType::Branch)
    }
    
    fn is_leaf(&self) -> bool {
        matches!(self.get_info().agent_type, crate::agent::AgentType::Leaf)
    }
    
    fn get_level(&self) -> crate::agent::AgentLevel {
        self.get_info().level
    }
}
