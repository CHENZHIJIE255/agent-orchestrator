use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

fn evaluate_condition_static(condition: &EdgeCondition, _output: &serde_json::Value) -> bool {
    match condition.expression.as_str() {
        "success" => true,
        "failure" => false,
        "always" => true,
        _ => true,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub entry_node: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    pub name: String,
    pub node_type: WorkflowNodeType,
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowNodeType {
    Input,
    Branch,
    Leaf,
    OpenCode,
    Skill,
    Hook,
    LLM,
    Merge,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub from: String,
    pub to: String,
    pub condition: Option<EdgeCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeCondition {
    pub expression: String,
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub id: String,
    pub workflow_id: String,
    pub status: WorkflowStatus,
    pub current_node: Option<String>,
    pub variables: HashMap<String, serde_json::Value>,
    pub results: Vec<NodeResult>,
    pub started_at: i64,
    pub finished_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum WorkflowStatus {
    Pending,
    Running,
    WaitingApproval,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResult {
    pub node_id: String,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

pub struct WorkflowEngine {
    workflows: HashMap<String, Workflow>,
    executions: HashMap<String, WorkflowExecution>,
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self {
            workflows: HashMap::new(),
            executions: HashMap::new(),
        }
    }

    pub fn load_workflows_from_dir(&mut self, dir_path: &str) -> std::io::Result<()> {
        let path = Path::new(dir_path);
        if !path.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(workflow) = serde_json::from_str::<Workflow>(&content) {
                        self.register_workflow(workflow);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn register_workflow(&mut self, workflow: Workflow) {
        self.workflows.insert(workflow.id.clone(), workflow);
    }

    pub fn get_workflow(&self, id: &str) -> Option<&Workflow> {
        self.workflows.get(id)
    }

    pub fn list_workflows(&self) -> Vec<&Workflow> {
        self.workflows.values().collect()
    }

    pub fn create_execution(&mut self, workflow_id: &str) -> Option<String> {
        let workflow = self.workflows.get(workflow_id)?;

        let execution = WorkflowExecution {
            id: Uuid::new_v4().to_string(),
            workflow_id: workflow_id.to_string(),
            status: WorkflowStatus::Pending,
            current_node: Some(workflow.entry_node.clone()),
            variables: HashMap::new(),
            results: Vec::new(),
            started_at: chrono::Utc::now().timestamp(),
            finished_at: None,
        };

        let id = execution.id.clone();
        self.executions.insert(id.clone(), execution);
        Some(id)
    }

    pub fn start_execution(&mut self, execution_id: &str) -> Option<&mut WorkflowExecution> {
        let execution = self.executions.get_mut(execution_id)?;

        if execution.status == WorkflowStatus::Pending {
            execution.status = WorkflowStatus::Running;
        }

        Some(execution)
    }

    pub fn execute_node(
        &mut self,
        execution_id: &str,
        node_id: &str,
        output: serde_json::Value,
    ) -> Option<String> {
        let execution = self.executions.get_mut(execution_id)?;

        let result = NodeResult {
            node_id: node_id.to_string(),
            output: Some(output.clone()),
            error: None,
            execution_time_ms: 0,
        };

        execution.results.push(result);

        let workflow_id = execution.workflow_id.clone();
        let workflow = self.workflows.get(&workflow_id)?.clone();

        let next_edges: Vec<&WorkflowEdge> = workflow
            .edges
            .iter()
            .filter(|e| e.from == node_id)
            .collect();

        let next_node = next_edges.iter().find_map(|edge| {
            if let Some(ref condition) = edge.condition {
                if evaluate_condition_static(&condition, &output) {
                    Some(edge.to.clone())
                } else {
                    None
                }
            } else {
                Some(edge.to.clone())
            }
        });

        if let Some(node) = next_node {
            execution.current_node = Some(node.clone());
            Some(node)
        } else {
            execution.status = WorkflowStatus::Completed;
            execution.finished_at = Some(chrono::Utc::now().timestamp());
            None
        }
    }

    pub fn get_execution(&self, id: &str) -> Option<&WorkflowExecution> {
        self.executions.get(id)
    }

    pub fn cancel_execution(&mut self, execution_id: &str) -> bool {
        if let Some(execution) = self.executions.get_mut(execution_id) {
            execution.status = WorkflowStatus::Cancelled;
            execution.finished_at = Some(chrono::Utc::now().timestamp());
            true
        } else {
            false
        }
    }

    pub fn get_execution_status(&self, execution_id: &str) -> Option<WorkflowStatus> {
        self.executions.get(execution_id).map(|e| e.status)
    }

    pub fn create_default_workflows(&mut self) {
        if self.load_workflows_from_dir("workflows").is_ok() && !self.workflows.is_empty() {
            return;
        }

        let simple_workflow = Workflow {
            id: "simple-task".to_string(),
            name: "简单任务".to_string(),
            description: "基本任务工作流：输入 -> LLM -> OpenCode -> 输出".to_string(),
            entry_node: "input".to_string(),
            nodes: vec![
                WorkflowNode {
                    id: "input".to_string(),
                    name: "输入".to_string(),
                    node_type: WorkflowNodeType::Input,
                    config: HashMap::new(),
                },
                WorkflowNode {
                    id: "llm".to_string(),
                    name: "LLM 处理".to_string(),
                    node_type: WorkflowNodeType::LLM,
                    config: HashMap::new(),
                },
                WorkflowNode {
                    id: "opencode".to_string(),
                    name: "OpenCode 执行".to_string(),
                    node_type: WorkflowNodeType::OpenCode,
                    config: HashMap::new(),
                },
                WorkflowNode {
                    id: "output".to_string(),
                    name: "输出".to_string(),
                    node_type: WorkflowNodeType::Output,
                    config: HashMap::new(),
                },
            ],
            edges: vec![
                WorkflowEdge {
                    from: "input".to_string(),
                    to: "llm".to_string(),
                    condition: None,
                },
                WorkflowEdge {
                    from: "llm".to_string(),
                    to: "opencode".to_string(),
                    condition: None,
                },
                WorkflowEdge {
                    from: "opencode".to_string(),
                    to: "output".to_string(),
                    condition: None,
                },
            ],
        };

        let branch_workflow = Workflow {
            id: "branch-task".to_string(),
            name: "分支任务".to_string(),
            description: "带分支判断的工作流".to_string(),
            entry_node: "input".to_string(),
            nodes: vec![
                WorkflowNode {
                    id: "input".to_string(),
                    name: "输入".to_string(),
                    node_type: WorkflowNodeType::Input,
                    config: HashMap::new(),
                },
                WorkflowNode {
                    id: "branch".to_string(),
                    name: "判断".to_string(),
                    node_type: WorkflowNodeType::Branch,
                    config: HashMap::new(),
                },
                WorkflowNode {
                    id: "simple_path".to_string(),
                    name: "简单路径".to_string(),
                    node_type: WorkflowNodeType::Leaf,
                    config: HashMap::new(),
                },
                WorkflowNode {
                    id: "complex_path".to_string(),
                    name: "复杂路径".to_string(),
                    node_type: WorkflowNodeType::Branch,
                    config: HashMap::new(),
                },
                WorkflowNode {
                    id: "output".to_string(),
                    name: "输出".to_string(),
                    node_type: WorkflowNodeType::Output,
                    config: HashMap::new(),
                },
            ],
            edges: vec![
                WorkflowEdge {
                    from: "input".to_string(),
                    to: "branch".to_string(),
                    condition: None,
                },
                WorkflowEdge {
                    from: "branch".to_string(),
                    to: "simple_path".to_string(),
                    condition: Some(EdgeCondition {
                        expression: "simple".to_string(),
                        value: None,
                    }),
                },
                WorkflowEdge {
                    from: "branch".to_string(),
                    to: "complex_path".to_string(),
                    condition: Some(EdgeCondition {
                        expression: "complex".to_string(),
                        value: None,
                    }),
                },
                WorkflowEdge {
                    from: "simple_path".to_string(),
                    to: "output".to_string(),
                    condition: None,
                },
                WorkflowEdge {
                    from: "complex_path".to_string(),
                    to: "output".to_string(),
                    condition: None,
                },
            ],
        };

        self.register_workflow(simple_workflow);
        self.register_workflow(branch_workflow);
    }
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}
