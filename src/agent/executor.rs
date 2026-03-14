use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::llm::{ChatMessage, LLMClient};
use crate::memory::ProjectMemory;
use crate::agent::{AgentType as AgentTypeDecl, AgentLevel as AgentLevelDecl};

#[derive(Debug, Clone)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub agent_type: AgentTypeDecl,
    pub level: AgentLevelDecl,
    pub system_prompt: String,
    pub conversation: Vec<ChatMessage>,
    pub context_limit: u32,
    pub memory: Option<Arc<RwLock<ProjectMemory>>>,
    pub llm_client: Option<Arc<LLMClient>>,
}

impl Agent {
    pub fn new_branch(name: &str, level: AgentLevelDecl, prompt: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            agent_type: AgentTypeDecl::Branch,
            level,
            system_prompt: prompt.to_string(),
            conversation: Vec::new(),
            context_limit: 128000,
            memory: None,
            llm_client: None,
        }
    }

    pub fn new_leaf(name: &str, prompt: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            agent_type: AgentTypeDecl::Leaf,
            level: AgentLevelDecl::L2,
            system_prompt: prompt.to_string(),
            conversation: Vec::new(),
            context_limit: 128000,
            memory: None,
            llm_client: None,
        }
    }

    pub fn with_llm(mut self, client: LLMClient) -> Self {
        self.llm_client = Some(Arc::new(client));
        self
    }

    pub fn with_memory(mut self, memory: Arc<RwLock<ProjectMemory>>) -> Self {
        self.memory = Some(memory);
        self
    }

    pub fn with_context_limit(mut self, limit: u32) -> Self {
        self.context_limit = limit;
        self
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.conversation.push(ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
        });
    }

    pub fn add_user_message(&mut self, content: &str) {
        self.add_message("user", content);
    }

    pub fn add_system_message(&mut self, content: &str) {
        self.add_message("system", content);
    }

    pub fn add_assistant_message(&mut self, content: &str) {
        self.add_message("assistant", content);
    }

    pub fn build_prompt(&self) -> String {
        let mut prompt = self.system_prompt.clone();
        
        if let Some(ref memory) = self.memory {
            let mem = memory.blocking_read();
            let arch = mem.load_architecture();
            if let Some(arch) = arch {
                prompt.push_str("\n\n# 当前架构\n");
                prompt.push_str(&arch);
            }
        }
        
        prompt.push_str("\n\n# 对话历史\n");
        for msg in &self.conversation {
            prompt.push_str(&format!("\n{}: {}", msg.role, msg.content));
        }
        
        prompt
    }

    pub async fn chat(&mut self, input: &str) -> Result<String, String> {
        self.add_user_message(input);
        
        let messages: Vec<ChatMessage> = vec![
            ChatMessage {
                role: "system".to_string(),
                content: self.build_prompt(),
            }
        ].into_iter()
        .chain(self.conversation.clone())
        .collect();

        let client_opt = self.llm_client.clone();
        
        if let Some(client) = client_opt {
            match client.chat(messages).await {
                Ok(response) => {
                    let content = response.content;
                    self.add_assistant_message(&content);
                    Ok(content)
                }
                Err(e) => Err(e.to_string()),
            }
        } else {
            Err("LLM client not initialized".to_string())
        }
    }

    pub fn get_context_usage(&self) -> f64 {
        let total_tokens = self.conversation.iter()
            .map(|m| m.content.len() as u32 / 4)
            .sum::<u32>();
        
        total_tokens as f64 / self.context_limit as f64
    }

    pub fn is_context_exceeded(&self, threshold: f64) -> bool {
        self.get_context_usage() >= threshold
    }

    pub fn clear_conversation(&mut self) {
        self.conversation.clear();
    }

    pub fn get_conversation_tokens(&self) -> u32 {
        self.conversation.iter()
            .map(|m| m.content.len() as u32 / 4)
            .sum()
    }
}

pub fn create_branch_agent(level: AgentLevelDecl, prompt: &str) -> Agent {
    let name = match level {
        AgentLevelDecl::L0 => "L0-Branch-Architect",
        AgentLevelDecl::L1 => "L1-Branch-Module",
        AgentLevelDecl::L2 => "L2-Branch-Function",
    };
    Agent::new_branch(name, level, prompt)
}

pub fn create_leaf_agent(prompt: &str) -> Agent {
    Agent::new_leaf("Leaf-Agent", prompt)
}

pub fn load_prompt_from_file(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|_| {
        "You are an AI agent.".to_string()
    })
}
