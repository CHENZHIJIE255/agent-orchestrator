use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug)]
pub struct ProjectMemory {
    pub project_name: String,
    pub root_dir: PathBuf,
    pub current: MemoryVersion,
    pub history: Vec<MemoryVersion>,
}

#[derive(Debug, Clone)]
pub struct MemoryVersion {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureMemory {
    pub overview: String,
    pub modules: Vec<ModuleRef>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleRef {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMemory {
    pub name: String,
    pub design: String,
    pub classes: Vec<ClassInfo>,
    pub files: HashMap<String, FolderMemory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub path: String,
    pub line_count: u32,
    pub public_methods: u32,
    pub is_complex: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderMemory {
    pub path: String,
    pub exposed_interfaces: Vec<InterfaceInfo>,
    pub implementation: String,
    pub capabilities: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceInfo {
    pub name: String,
    pub parameters: Vec<String>,
    pub return_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMemory {
    pub name: String,
    pub spec: String,
    pub status: FunctionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctionStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl ProjectMemory {
    pub fn new(project_name: String, install_dir: PathBuf) -> Self {
        let root_dir = install_dir.join(&project_name);
        let memory_dir = root_dir.join("memory");
        
        fs::create_dir_all(&memory_dir).ok();
        fs::create_dir_all(memory_dir.join("current")).ok();
        fs::create_dir_all(memory_dir.join("history")).ok();

        let pointer_path = memory_dir.join(".pointer");
        let current_path = memory_dir.join("current");
        
        fs::write(&pointer_path, current_path.to_string_lossy().to_string()).ok();

        Self {
            project_name,
            root_dir,
            current: MemoryVersion {
                timestamp: chrono::Utc::now(),
                path: current_path,
            },
            history: Vec::new(),
        }
    }

    pub fn load(project_name: String, install_dir: PathBuf) -> Option<Self> {
        let root_dir = install_dir.join(&project_name);
        let memory_dir = root_dir.join("memory");

        if !memory_dir.exists() {
            return None;
        }

        let pointer_path = memory_dir.join(".pointer");
        let current_path = fs::read_to_string(&pointer_path).ok()?;
        let current = MemoryVersion {
            timestamp: chrono::Utc::now(),
            path: PathBuf::from(current_path),
        };

        Some(Self {
            project_name,
            root_dir,
            current,
            history: Vec::new(),
        })
    }

    pub fn get_current_path(&self) -> &PathBuf {
        &self.current.path
    }

    pub fn get_architecture_path(&self) -> PathBuf {
        self.current.path.join("architecture.md")
    }

    pub fn get_modules_path(&self) -> PathBuf {
        self.current.path.join("modules")
    }

    pub fn get_module_path(&self, module_name: &str) -> PathBuf {
        self.get_modules_path().join(module_name)
    }

    pub fn get_functions_path(&self) -> PathBuf {
        self.current.path.join("functions")
    }

    pub fn get_function_path(&self, func_name: &str) -> PathBuf {
        self.get_functions_path().join(func_name)
    }

    pub fn save_architecture(&self, content: &str) -> anyhow::Result<()> {
        let path = self.get_architecture_path();
        fs::write(path, content)?;
        Ok(())
    }

    pub fn load_architecture(&self) -> Option<String> {
        fs::read_to_string(self.get_architecture_path()).ok()
    }

    pub fn create_module(&self, module_name: &str) -> anyhow::Result<()> {
        let module_dir = self.get_module_path(module_name);
        fs::create_dir_all(&module_dir)?;
        fs::create_dir_all(module_dir.join("files"))?;
        Ok(())
    }

    pub fn save_module_design(&self, module_name: &str, content: &str) -> anyhow::Result<()> {
        let path = self.get_module_path(module_name).join("design.md");
        fs::write(path, content)?;
        Ok(())
    }

    pub fn save_class_info(&self, module_name: &str, class_info: &ClassInfo) -> anyhow::Result<()> {
        let classes_path = self.get_module_path(module_name).join("classes.json");
        let mut classes: Vec<ClassInfo> = if classes_path.exists() {
            let content = fs::read_to_string(&classes_path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };

        classes.retain(|c| c.name != class_info.name);
        classes.push(class_info.clone());

        let content = serde_json::to_string_pretty(&classes)?;
        fs::write(classes_path, content)?;
        Ok(())
    }

    pub fn create_function(&self, func_name: &str) -> anyhow::Result<()> {
        let func_dir = self.get_function_path(func_name);
        fs::create_dir_all(&func_dir)?;
        Ok(())
    }

    pub fn save_function_spec(&self, func_name: &str, content: &str) -> anyhow::Result<()> {
        let path = self.get_function_path(func_name).join("spec.md");
        fs::write(path, content)?;
        Ok(())
    }

    pub fn save_folder_memory(&self, module_name: &str, folder_path: &str, memory: &FolderMemory) -> anyhow::Result<()> {
        let folder_dir = self.get_module_path(module_name).join("files").join(folder_path);
        fs::create_dir_all(&folder_dir)?;
        
        let path = folder_dir.join(".folder.md");
        let content = format!(
            "# Folder: {}\n\n## Exposed Interfaces\n{}\n\n## Implementation\n{}\n\n## Capabilities\n{}\n\n## Limitations\n{}",
            folder_path,
            memory.exposed_interfaces.iter().map(|i| format!("- {}: {} -> {}", i.name, i.parameters.join(", "), i.return_type)).collect::<Vec<_>>().join("\n"),
            memory.implementation,
            memory.capabilities.iter().map(|c| format!("- {}", c)).collect::<Vec<_>>().join("\n"),
            memory.limitations.iter().map(|l| format!("- {}", l)).collect::<Vec<_>>().join("\n")
        );
        fs::write(path, content)?;
        Ok(())
    }

    pub fn analyze_context_usage(&self, total_tokens: u32, limit: u32) -> ContextAnalysis {
        let usage = total_tokens as f64 / limit as f64;
        
        let mut warnings = Vec::new();
        
        if usage >= 0.7 {
            warnings.push(ContextWarning {
                level: crate::agent::AlertLevel::P2,
                message: "Context usage exceeds 70% threshold".to_string(),
                suggestions: vec![
                    "Consider splitting the task into smaller modules".to_string(),
                    "Add intermediate layer (L1) for better task management".to_string(),
                    "Review code architecture for potential refactoring".to_string(),
                ],
            });
        }

        if usage >= 0.9 {
            warnings.push(ContextWarning {
                level: crate::agent::AlertLevel::P0,
                message: "Critical: Context usage exceeds 90%".to_string(),
                suggestions: vec![
                    "Task must be split immediately".to_string(),
                    "Consider creating new module branch".to_string(),
                ],
            });
        }

        ContextAnalysis {
            usage_percentage: usage,
            warnings,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContextWarning {
    pub level: crate::agent::AlertLevel,
    pub message: String,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ContextAnalysis {
    pub usage_percentage: f64,
    pub warnings: Vec<ContextWarning>,
}
