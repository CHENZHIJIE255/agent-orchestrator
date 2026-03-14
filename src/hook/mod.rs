use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    pub id: String,
    pub event: HookEvent,
    pub mode: HookMode,
    pub timeout_seconds: u32,
    pub on_approve: Option<HookAction>,
    pub on_reject: Option<HookAction>,
    pub on_timeout: Option<HookAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HookEvent {
    L0_方案评审,
    L0_测试分发,
    L1_代码评审,
    L1_方案评审,
    L2_实现完成,
    ContextWarning,
    TaskComplete,
    TaskFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HookMode {
    Auto,
    Manual,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookAction {
    pub action_type: HookActionType,
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookActionType {
    Proceed,
    Reject,
    Retry,
    Skip,
    Notify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookRequest {
    pub hook_id: String,
    pub event: HookEvent,
    pub agent_id: String,
    pub description: String,
    pub details: serde_json::Value,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResponse {
    pub request_id: String,
    pub approved: bool,
    pub action: HookActionType,
    pub comment: Option<String>,
    pub timestamp: i64,
}

pub struct HookManager {
    hooks: HashMap<HookEvent, Hook>,
    pending_requests: Vec<HookRequest>,
}

impl HookManager {
    pub fn new() -> Self {
        let mut manager = Self {
            hooks: HashMap::new(),
            pending_requests: Vec::new(),
        };
        
        manager.load_default_hooks();
        manager
    }

    fn load_default_hooks(&mut self) {
        let default_hooks = vec![
            Hook {
                id: "l0_方案评审".to_string(),
                event: HookEvent::L0_方案评审,
                mode: HookMode::Auto,
                timeout_seconds: 300,
                on_approve: Some(HookAction {
                    action_type: HookActionType::Proceed,
                    payload: None,
                }),
                on_reject: Some(HookAction {
                    action_type: HookActionType::Reject,
                    payload: None,
                }),
                on_timeout: Some(HookAction {
                    action_type: HookActionType::Proceed,
                    payload: None,
                }),
            },
            Hook {
                id: "l1_代码评审".to_string(),
                event: HookEvent::L1_代码评审,
                mode: HookMode::Manual,
                timeout_seconds: 600,
                on_approve: Some(HookAction {
                    action_type: HookActionType::Proceed,
                    payload: None,
                }),
                on_reject: Some(HookAction {
                    action_type: HookActionType::Retry,
                    payload: None,
                }),
                on_timeout: None,
            },
            Hook {
                id: "l0_测试分发".to_string(),
                event: HookEvent::L0_测试分发,
                mode: HookMode::Auto,
                timeout_seconds: 60,
                on_approve: Some(HookAction {
                    action_type: HookActionType::Proceed,
                    payload: None,
                }),
                on_reject: None,
                on_timeout: Some(HookAction {
                    action_type: HookActionType::Proceed,
                    payload: None,
                }),
            },
            Hook {
                id: "context_warning".to_string(),
                event: HookEvent::ContextWarning,
                mode: HookMode::Manual,
                timeout_seconds: 0,
                on_approve: None,
                on_reject: None,
                on_timeout: None,
            },
        ];

        for hook in default_hooks {
            self.hooks.insert(hook.event.clone(), hook);
        }
    }

    pub fn register(&mut self, hook: Hook) {
        self.hooks.insert(hook.event.clone(), hook);
    }

    pub fn get_hook(&self, event: &HookEvent) -> Option<&Hook> {
        self.hooks.get(event)
    }

    pub fn create_request(&mut self, event: &HookEvent, agent_id: &str, description: &str, details: serde_json::Value) -> Option<HookRequest> {
        let hook = self.hooks.get(event)?;
        
        let request = HookRequest {
            hook_id: hook.id.clone(),
            event: event.clone(),
            agent_id: agent_id.to_string(),
            description: description.to_string(),
            details,
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        self.pending_requests.push(request.clone());
        Some(request)
    }

    pub fn approve(&mut self, request_id: &str, comment: Option<String>) -> Option<HookResponse> {
        if let Some(pos) = self.pending_requests.iter().position(|r| r.hook_id == request_id) {
            let request = self.pending_requests.remove(pos);
            
            Some(HookResponse {
                request_id: request.hook_id,
                approved: true,
                action: HookActionType::Proceed,
                comment,
                timestamp: chrono::Utc::now().timestamp(),
            })
        } else {
            None
        }
    }

    pub fn reject(&mut self, request_id: &str, comment: Option<String>) -> Option<HookResponse> {
        if let Some(pos) = self.pending_requests.iter().position(|r| r.hook_id == request_id) {
            let request = self.pending_requests.remove(pos);
            
            Some(HookResponse {
                request_id: request.hook_id,
                approved: false,
                action: HookActionType::Reject,
                comment,
                timestamp: chrono::Utc::now().timestamp(),
            })
        } else {
            None
        }
    }

    pub fn get_pending(&self) -> Vec<&HookRequest> {
        self.pending_requests.iter().collect()
    }

    pub fn should_auto_approve(&self, event: &HookEvent) -> bool {
        self.hooks
            .get(event)
            .map(|h| h.mode == HookMode::Auto)
            .unwrap_or(false)
    }

    pub fn get_timeout(&self, event: &HookEvent) -> u32 {
        self.hooks
            .get(event)
            .map(|h| h.timeout_seconds)
            .unwrap_or(0)
    }
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}
