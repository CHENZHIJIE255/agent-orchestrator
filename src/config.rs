use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub models: ModelsConfig,
    pub context: ContextConfig,
    pub agents: AgentsConfig,
    pub opencode: OpenCodeConfig,
    pub language: String,
    #[cfg(windows)]
    pub install_dir: PathBuf,
    #[cfg(not(windows))]
    pub install_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsConfig {
    pub default: String,
    pub providers: HashMap<String, ModelProvider>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProvider {
    #[serde(rename = "type")]
    pub provider_type: String,
    pub model: String,
    pub api_key_env: String,
    pub context_limit: u32,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub warning_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsConfig {
    pub pool_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCodeConfig {
    pub path: String,
}

fn get_default_install_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("agent-orchestrator")
}

fn get_openclaw_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".openclaw/openclaw.json"))
}

impl Default for Config {
    fn default() -> Self {
        let install_dir = get_default_install_dir();
        
        let providers = HashMap::new();

        Self {
            models: ModelsConfig {
                default: "openai".to_string(),
                providers,
            },
            context: ContextConfig {
                warning_threshold: 0.7,
            },
            agents: AgentsConfig {
                pool_size: 25,
            },
            opencode: OpenCodeConfig {
                path: "opencode".to_string(),
            },
            language: "en".to_string(),
            install_dir,
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let install_dir = get_default_install_dir();
        
        if !install_dir.exists() {
            fs::create_dir_all(&install_dir)?;
        }

        let config_path = install_dir.join("config.json");
        
        let mut config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let mut c: Config = serde_json::from_str(&content)?;
            c.install_dir = install_dir;
            c
        } else {
            let default_config = Config::default();
            let content = serde_json::to_string_pretty(&default_config)?;
            fs::write(&config_path, content)?;
            default_config
        };

        if let Some(openclaw_path) = get_openclaw_config_path() {
            if openclaw_path.exists() {
                if let Ok(openclaw_config) = fs::read_to_string(&openclaw_path) {
                    if let Ok(openclaw) = serde_json::from_str::<serde_json::Value>(&openclaw_config) {
                        config = Self::merge_openclaw_config(config, &openclaw);
                    }
                }
            }
        }

        Ok(config)
    }

    fn merge_openclaw_config(mut config: Config, openclaw: &serde_json::Value) -> Config {
        if let Some(models) = openclaw.get("models") {
            if let Some(default) = models.get("default_model")
                .or_else(|| models.get("default"))
                .and_then(|v| v.as_str())
            {
                config.models.default = default.to_string();
            }

            if let Some(providers) = models.as_object() {
                for (name, value) in providers {
                    if let Some(provider_type) = value.get("type").and_then(|v| v.as_str()) {
                        let model = value.get("model")
                            .or_else(|| value.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("default")
                            .to_string();
                        
                        let api_key_env = value.get("api_key")
                            .and_then(|v| v.as_str())
                            .map(|s| {
                                if s.starts_with("env:") {
                                    s.to_string()
                                } else {
                                    format!("env:{}", s)
                                }
                            })
                            .unwrap_or_else(|| "env:OPENAI_API_KEY".to_string());

                        let base_url = value.get("base_url")
                            .or_else(|| value.get("endpoint"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        let context_limit = value.get("context_limit")
                            .or_else(|| value.get("max_tokens"))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(128000) as u32;

                        config.models.providers.insert(
                            name.clone(),
                            ModelProvider {
                                provider_type: provider_type.to_string(),
                                model,
                                api_key_env,
                                context_limit,
                                base_url,
                            },
                        );
                    }
                }
            }
        }

        config
    }

    pub fn get_model(&self, name: &str) -> Option<&ModelProvider> {
        self.models.providers.get(name)
    }

    pub fn get_default_model(&self) -> Option<&ModelProvider> {
        self.get_model(&self.models.default)
    }

    pub fn resolve_api_key(&self, provider: &ModelProvider) -> String {
        if provider.api_key_env.starts_with("env:") {
            let env_var = provider.api_key_env.strip_prefix("env:").unwrap();
            std::env::var(env_var).unwrap_or_default()
        } else {
            std::env::var(&provider.api_key_env).unwrap_or_default()
        }
    }
}
