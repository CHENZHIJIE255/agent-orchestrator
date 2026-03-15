use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub models: ModelsConfig,
    pub agents: AgentsConfig,
    pub language: String,
    #[serde(default)]
    pub install_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsConfig {
    #[serde(alias = "default_model")]
    pub default: String,
    pub providers: HashMap<String, ModelProvider>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProvider {
    #[serde(rename = "type")]
    pub provider_type: String,
    #[serde(alias = "baseUrl")]
    pub base_url: String,
    #[serde(alias = "apiKey")]
    pub api_key: String,
    pub api: Option<String>,
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    #[serde(alias = "contextWindow")]
    pub context_window: u32,
    #[serde(alias = "maxTokens")]
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsConfig {
    #[serde(alias = "pool_size")]
    pub pool_size: usize,
    #[serde(alias = "max_concurrent")]
    pub max_concurrent: Option<usize>,
}

fn get_default_install_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("agent-orchestrator")
}

fn get_default_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|h| h.join("orchestrator/config.json"))
}

impl Default for Config {
    fn default() -> Self {
        let install_dir = get_default_install_dir();
        
        let providers = HashMap::new();

        Self {
            models: ModelsConfig {
                default: "minimax-cn".to_string(),
                providers,
            },
            agents: AgentsConfig {
                pool_size: 25,
                max_concurrent: None,
            },
            language: "en".to_string(),
            install_dir,
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        if let Some(config_path) = get_default_config_path() {
            if config_path.exists() {
                let content = fs::read_to_string(&config_path)?;
                let mut config: Config = serde_json::from_str(&content)?;
                config.install_dir = get_default_install_dir();
                return Ok(config);
            }
        }

        let install_dir = get_default_install_dir();
        if !install_dir.exists() {
            fs::create_dir_all(&install_dir)?;
        }

        let default_config = Config::default();
        let content = serde_json::to_string_pretty(&default_config)?;
        fs::write(install_dir.join("config.json"), content)?;
        
        Ok(default_config)
    }

    pub fn get_model(&self, name: &str) -> Option<&ModelProvider> {
        if name.contains('/') {
            let parts: Vec<&str> = name.split('/').collect();
            if parts.len() == 2 {
                return self.models.providers.get(parts[0]);
            }
        }
        self.models.providers.get(name)
    }

    pub fn get_default_model(&self) -> Option<&ModelProvider> {
        self.get_model(&self.models.default)
    }

    pub fn resolve_api_key(&self, provider: &ModelProvider) -> String {
        if provider.api_key.starts_with("env:") {
            let env_var = provider.api_key.strip_prefix("env:").unwrap();
            std::env::var(env_var).unwrap_or_default()
        } else {
            provider.api_key.clone()
        }
    }
}
