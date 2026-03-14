use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPRequest {
    pub jsonrpc: String,
    pub id: Option<String>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResponse {
    pub jsonrpc: String,
    pub id: Option<String>,
    pub result: Option<serde_json::Value>,
    pub error: Option<MCPError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResource {
    pub uri: String,
    pub name: String,
    pub mime_type: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub enum MCPMessage {
    Request(MCPRequest),
    Response(MCPResponse),
    Notification(MCPNotification),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[async_trait]
pub trait MCPHandler: Send + Sync {
    async fn handle_request(&self, request: MCPRequest) -> MCPResponse;
    fn list_tools(&self) -> Vec<MCPTool>;
    fn list_resources(&self) -> Vec<MCPResource>;
}

pub struct MCPServer {
    handlers: HashMap<String, Box<dyn MCPHandler>>,
    tools: Vec<MCPTool>,
    resources: Vec<MCPResource>,
    sender: mpsc::Sender<MCPMessage>,
    receiver: mpsc::Receiver<MCPMessage>,
}

impl MCPServer {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(100);
        Self {
            handlers: HashMap::new(),
            tools: Vec::new(),
            resources: Vec::new(),
            sender,
            receiver,
        }
    }

    pub fn register_handler(&mut self, name: String, handler: Box<dyn MCPHandler>) {
        self.tools.extend(handler.list_tools());
        self.resources.extend(handler.list_resources());
        self.handlers.insert(name, handler);
    }

    pub async fn handle_message(&self, message: MCPMessage) -> Option<MCPResponse> {
        match message {
            MCPMessage::Request(request) => {
                Some(self.handle_request(request).await)
            }
            MCPMessage::Notification(notification) => {
                self.handle_notification(notification).await;
                None
            }
            MCPMessage::Response(_) => None,
        }
    }

    async fn handle_request(&self, request: MCPRequest) -> MCPResponse {
        let method = &request.method;
        
        match method.as_str() {
            "tools/list" => {
                MCPResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(serde_json::json!({
                        "tools": self.tools
                    })),
                    error: None,
                }
            }
            "resources/list" => {
                MCPResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(serde_json::json!({
                        "resources": self.resources
                    })),
                    error: None,
                }
            }
            _ => {
                if let Some(pos) = method.find('/') {
                    let (handler_name, tool_name) = method.split_at(pos);
                    let tool_name = &tool_name[1..];
                    
                    if let Some(handler) = self.handlers.get(handler_name) {
                        return handler.handle_request(request).await;
                    }
                }
                
                MCPResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(MCPError {
                        code: -32601,
                        message: format!("Method not found: {}", method),
                        data: None,
                    }),
                }
            }
        }
    }

    async fn handle_notification(&self, _notification: MCPNotification) {}

    pub fn get_tools(&self) -> &Vec<MCPTool> {
        &self.tools
    }

    pub fn get_resources(&self) -> &Vec<MCPResource> {
        &self.resources
    }
}

impl Default for MCPServer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MCPClient {
    server_url: String,
    sender: mpsc::Sender<MCPMessage>,
    receiver: mpsc::Receiver<MCPMessage>,
}

impl MCPClient {
    pub fn new(server_url: String) -> Self {
        let (sender, receiver) = mpsc::channel(100);
        Self {
            server_url,
            sender,
            receiver,
        }
    }

    pub async fn call_tool(&mut self, tool_name: &str, arguments: serde_json::Value) -> Result<MCPResponse, String> {
        let request = MCPRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(uuid::Uuid::new_v4().to_string()),
            method: tool_name.to_string(),
            params: Some(arguments),
        };

        self.sender.send(MCPMessage::Request(request)).await
            .map_err(|e| e.to_string())?;

        if let Some(response) = self.receiver.recv().await {
            match response {
                MCPMessage::Response(resp) => Ok(resp),
                _ => Err("Unexpected message type".to_string()),
            }
        } else {
            Err("No response received".to_string())
        }
    }

    pub async fn list_tools(&mut self) -> Result<Vec<MCPTool>, String> {
        let request = MCPRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(uuid::Uuid::new_v4().to_string()),
            method: "tools/list".to_string(),
            params: None,
        };

        self.sender.send(MCPMessage::Request(request)).await
            .map_err(|e| e.to_string())?;

        if let Some(response) = self.receiver.recv().await {
            match response {
                MCPMessage::Response(resp) => {
                    if let Some(result) = resp.result {
                        let tools: Vec<MCPTool> = serde_json::from_value(
                            result.get("tools").cloned().unwrap_or_default()
                        ).unwrap_or_default();
                        Ok(tools)
                    } else {
                        Err(resp.error.map(|e| e.message).unwrap_or_default())
                    }
                }
                _ => Err("Unexpected message type".to_string()),
            }
        } else {
            Err("No response received".to_string())
        }
    }
}
