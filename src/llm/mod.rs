use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub message: ChatMessage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct LLMResponse {
    pub content: String,
    pub usage: Usage,
    pub model: String,
}

pub struct LLMClient {
    provider: LLMProvider,
    client: reqwest::Client,
}

impl Clone for LLMClient {
    fn clone(&self) -> Self {
        Self {
            provider: self.provider.clone(),
            client: reqwest::Client::new(),
        }
    }
}

impl std::fmt::Debug for LLMClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LLMClient")
            .field("provider", &self.provider)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub enum LLMProvider {
    OpenAI {
        api_key: String,
        model: String,
        base_url: String,
    },
    Anthropic {
        api_key: String,
        model: String,
    },
}

impl LLMClient {
    pub fn new(provider: LLMProvider) -> Self {
        Self {
            provider,
            client: reqwest::Client::new(),
        }
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<LLMResponse, LLMError> {
        match &self.provider {
            LLMProvider::OpenAI { api_key, model, base_url } => {
                self.chat_openai(api_key, model, base_url, messages).await
            }
            LLMProvider::Anthropic { api_key, model } => {
                self.chat_anthropic(api_key, model, messages).await
            }
        }
    }

    async fn chat_openai(
        &self, 
        api_key: &str, 
        model: &str, 
        base_url: &str,
        messages: Vec<ChatMessage>
    ) -> Result<LLMResponse, LLMError> {
        let request = ChatRequest {
            model: model.to_string(),
            messages,
            temperature: 0.7,
            max_tokens: None,
        };

        let response = self.client
            .post(format!("{}/v1/chat/completions", base_url.trim_end_matches('/')))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Parse(e.to_string()))?;

        let content = chat_response.choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(LLMResponse {
            content,
            usage: chat_response.usage,
            model: model.to_string(),
        })
    }

    async fn chat_anthropic(
        &self, 
        api_key: &str, 
        model: &str, 
        messages: Vec<ChatMessage>
    ) -> Result<LLMResponse, LLMError> {
        #[derive(Serialize)]
        struct AnthropicRequest {
            model: String,
            messages: Vec<AnthropicMessage>,
            max_tokens: u32,
        }

        #[derive(Serialize)]
        struct AnthropicMessage {
            role: String,
            content: String,
        }

        #[derive(Deserialize)]
        struct AnthropicResponse {
            content: Vec<AnthropicContent>,
            usage: AnthropicUsage,
        }

        #[derive(Deserialize)]
        struct AnthropicContent {
            text: String,
        }

        #[derive(Deserialize)]
        struct AnthropicUsage {
            input_tokens: u32,
            output_tokens: u32,
        }

        let anthropic_messages: Vec<AnthropicMessage> = messages
            .into_iter()
            .map(|m| AnthropicMessage {
                role: m.role,
                content: m.content,
            })
            .collect();

        let request = AnthropicRequest {
            model: model.to_string(),
            messages: anthropic_messages,
            max_tokens: 4096,
        };

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| LLMError::Parse(e.to_string()))?;

        let content = anthropic_response.content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        let usage = Usage {
            prompt_tokens: anthropic_response.usage.input_tokens,
            completion_tokens: anthropic_response.usage.output_tokens,
            total_tokens: anthropic_response.usage.input_tokens + anthropic_response.usage.output_tokens,
        };

        Ok(LLMResponse {
            content,
            usage,
            model: model.to_string(),
        })
    }

    pub fn calculate_context_usage(&self, tokens: u32, limit: u32) -> f64 {
        tokens as f64 / limit as f64
    }
}

#[derive(Debug)]
pub enum LLMError {
    Network(String),
    Parse(String),
    Api(String),
}

impl std::fmt::Display for LLMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMError::Network(e) => write!(f, "Network error: {}", e),
            LLMError::Parse(e) => write!(f, "Parse error: {}", e),
            LLMError::Api(e) => write!(f, "API error: {}", e),
        }
    }
}

impl std::error::Error for LLMError {}

pub fn create_client_from_config(config: &crate::config::Config) -> Option<LLMClient> {
    let model = config.get_default_model()?;
    let api_key = config.resolve_api_key(model);

    if api_key.is_empty() {
        return None;
    }

    let provider = match model.provider_type.as_str() {
        "openai" | "custom" => LLMProvider::OpenAI {
            api_key,
            model: model.model.clone(),
            base_url: model.base_url.clone().unwrap_or_else(|| "https://api.openai.com".to_string()),
        },
        "anthropic" => LLMProvider::Anthropic {
            api_key,
            model: model.model.clone(),
        },
        _ => return None,
    };

    Some(LLMClient::new(provider))
}
