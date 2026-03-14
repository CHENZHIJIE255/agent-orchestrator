use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: SkillCategory,
    pub command: String,
    pub args: Vec<SkillArg>,
    pub env: HashMap<String, String>,
    pub working_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillCategory {
    CodeGeneration,
    CodeReview,
    Testing,
    Documentation,
    Deployment,
    Analysis,
    Custom(String),
}

impl PartialEq for SkillCategory {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SkillCategory::CodeGeneration, SkillCategory::CodeGeneration) => true,
            (SkillCategory::CodeReview, SkillCategory::CodeReview) => true,
            (SkillCategory::Testing, SkillCategory::Testing) => true,
            (SkillCategory::Documentation, SkillCategory::Documentation) => true,
            (SkillCategory::Deployment, SkillCategory::Deployment) => true,
            (SkillCategory::Analysis, SkillCategory::Analysis) => true,
            (SkillCategory::Custom(a), SkillCategory::Custom(b)) => a == b,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillArg {
    pub name: String,
    pub required: bool,
    pub default: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

pub struct SkillManager {
    skills: HashMap<String, Skill>,
    skills_dir: PathBuf,
}

impl SkillManager {
    pub fn new(skills_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&skills_dir).ok();
        
        let mut manager = Self {
            skills: HashMap::new(),
            skills_dir,
        };
        
        manager.load_default_skills();
        manager
    }

    fn load_default_skills(&mut self) {
        let default_skills = vec![
            Skill {
                id: "opencode".to_string(),
                name: "OpenCode".to_string(),
                description: "调用 OpenCode 进行代码生成和修改".to_string(),
                category: SkillCategory::CodeGeneration,
                command: "opencode".to_string(),
                args: vec![
                    SkillArg {
                        name: "task".to_string(),
                        required: true,
                        default: None,
                        description: "任务描述".to_string(),
                    },
                    SkillArg {
                        name: "path".to_string(),
                        required: false,
                        default: Some(".".to_string()),
                        description: "工作目录".to_string(),
                    },
                ],
                env: HashMap::new(),
                working_dir: None,
            },
            Skill {
                id: "shell".to_string(),
                name: "Shell".to_string(),
                description: "执行终端命令".to_string(),
                category: SkillCategory::Custom("system".to_string()),
                command: "sh".to_string(),
                args: vec![
                    SkillArg {
                        name: "command".to_string(),
                        required: true,
                        default: None,
                        description: "要执行的命令".to_string(),
                    },
                ],
                env: HashMap::new(),
                working_dir: None,
            },
            Skill {
                id: "git_status".to_string(),
                name: "GitStatus".to_string(),
                description: "查看 Git 状态".to_string(),
                category: SkillCategory::Custom("system".to_string()),
                command: "git".to_string(),
                args: vec![],
                env: HashMap::new(),
                working_dir: None,
            },
            Skill {
                id: "file_read".to_string(),
                name: "FileRead".to_string(),
                description: "读取文件内容".to_string(),
                category: SkillCategory::Custom("system".to_string()),
                command: "cat".to_string(),
                args: vec![
                    SkillArg {
                        name: "path".to_string(),
                        required: true,
                        default: None,
                        description: "文件路径".to_string(),
                    },
                ],
                env: HashMap::new(),
                working_dir: None,
            },
            Skill {
                id: "file_write".to_string(),
                name: "FileWrite".to_string(),
                description: "写入文件内容".to_string(),
                category: SkillCategory::Custom("system".to_string()),
                command: "tee".to_string(),
                args: vec![
                    SkillArg {
                        name: "path".to_string(),
                        required: true,
                        default: None,
                        description: "文件路径".to_string(),
                    },
                    SkillArg {
                        name: "content".to_string(),
                        required: true,
                        default: None,
                        description: "文件内容".to_string(),
                    },
                ],
                env: HashMap::new(),
                working_dir: None,
            },
        ];

        for skill in default_skills {
            self.skills.insert(skill.id.clone(), skill);
        }
    }

    pub fn register(&mut self, skill: Skill) {
        self.skills.insert(skill.id.clone(), skill);
    }

    pub fn get(&self, id: &str) -> Option<&Skill> {
        self.skills.get(id)
    }

    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    pub fn list_by_category(&self, category: &SkillCategory) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|s| &s.category == category)
            .collect()
    }

    pub async fn execute(&self, skill_id: &str, args: HashMap<String, String>) -> Result<SkillResult, String> {
        let skill = self.skills.get(skill_id).ok_or("Skill not found")?;
        
        let start = std::time::Instant::now();
        
        let mut cmd = std::process::Command::new(&skill.command);
        
        for arg in &skill.args {
            if let Some(value) = args.get(&arg.name) {
                cmd.arg(value);
            } else if arg.required {
                return Err(format!("Required argument missing: {}", arg.name));
            }
        }

        for (key, value) in &skill.env {
            cmd.env(key, value);
        }

        if let Some(ref dir) = skill.working_dir {
            cmd.current_dir(dir);
        }

        match cmd.output() {
            Ok(output) => {
                let execution_time_ms = start.elapsed().as_millis() as u64;
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                Ok(SkillResult {
                    success: output.status.success(),
                    output: stdout,
                    error: if stderr.is_empty() { None } else { Some(stderr) },
                    execution_time_ms,
                })
            }
            Err(e) => {
                Ok(SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                    execution_time_ms: start.elapsed().as_millis() as u64,
                })
            }
        }
    }
}
