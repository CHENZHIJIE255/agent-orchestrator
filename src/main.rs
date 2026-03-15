mod config;
mod agent;
mod logger;
mod llm;
mod memory;
mod ui;
mod skill;
mod hook;
mod mcp;
mod workflow;

use config::Config;
use logger::Logger;
use ui::TUI;
use llm::{LLMClient, LLMProvider};
use agent::{create_branch_agent, load_prompt_from_file, AgentLevel as AgentLevelDecl, Agent};

use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use dirs;

use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;

struct App {
    config: Config,
    logger: Arc<Logger>,
    tui: TUI,
    memory: Option<memory::ProjectMemory>,
    llm_client: Option<Arc<LLMClient>>,
    current_agent: Option<Arc<RwLock<Agent>>>,
}

impl App {
    fn new() -> anyhow::Result<Self> {
        let install_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("agent-orchestrator");

        std::fs::create_dir_all(&install_dir).ok();

        let config = Config::load()?;
        
        let llm_client = if let Some(provider) = config.get_default_model() {
            let api_key = config.resolve_api_key(provider);
            if api_key.is_empty() {
                eprintln!("Warning: API key not configured. Please set your API key in config.json");
                None
            } else {
                let model_id = provider.models.first()
                    .map(|m| m.id.clone())
                    .unwrap_or_else(|| "gpt-4".to_string());
                let provider = LLMProvider::OpenAI {
                    api_key,
                    model: model_id,
                    base_url: provider.base_url.clone(),
                };
                Some(Arc::new(LLMClient::new(provider)))
            }
        } else {
            eprintln!("Warning: No default model configured");
            None
        };

        if llm_client.is_some() {
            println!("LLM client initialized successfully!");
        }

        let logger = Arc::new(logger::init_logger(&install_dir));
        logger.info("System", "AgentOrchestrator started");

        let tui = TUI::new(logger.clone());

        Ok(Self {
            config,
            logger,
            tui,
            memory: None,
            llm_client,
            current_agent: None,
        })
    }

    fn handle_command(&mut self, input: &str) -> bool {
        let input = input.trim();
        
        if input.is_empty() {
            return false;
        }

        if input.starts_with('/') {
            self.handle_software_command(input)
        } else if input.starts_with('!') {
            self.handle_terminal_command(input)
        } else {
            self.handle_user_input(input)
        }
    }

    fn handle_software_command(&mut self, input: &str) -> bool {
        match input {
            "/newproject" => {
                self.logger.info("Command", "Creating new project...");
                self.tui.set_message("Enter project name to create:");
                true
            }
            "/exit" | "/quit" => {
                self.logger.info("System", "Shutting down...");
                false
            }
            _ => {
                let path = input.trim_start_matches('/');
                if path.starts_with('/') {
                    let project_name = path.trim_start_matches('/');
                    self.create_project(project_name);
                } else if self.is_waiting_for_project_name() {
                    self.create_project(path);
                } else {
                    self.open_project(path);
                }
                true
            }
        }
    }

    fn is_waiting_for_project_name(&self) -> bool {
        let state = self.tui.get_state();
        let message = state.read().message.clone();
        message.contains("Enter project name")
    }

    fn create_project(&mut self, project_name: &str) {
        let project_path = self.config.install_dir.join(project_name);
        
        if project_path.exists() {
            self.logger.error("Project", &format!("Project already exists: {}", project_name));
            self.tui.set_message(&format!("Error: Project '{}' already exists", project_name));
            return;
        }

        std::fs::create_dir_all(&project_path).ok();
        std::fs::create_dir_all(project_path.join("memory")).ok();
        std::fs::create_dir_all(project_path.join("memory/current")).ok();
        std::fs::create_dir_all(project_path.join("memory/history")).ok();
        std::fs::create_dir_all(project_path.join("logs")).ok();

        self.logger.info("Project", &format!("Created project: {}", project_name));
        self.tui.set_message(&format!("Created project: {}", project_name));
        
        self.memory = Some(memory::ProjectMemory::new(
            project_name.to_string(),
            self.config.install_dir.clone()
        ));
    }

    fn handle_terminal_command(&mut self, input: &str) -> bool {
        let cmd = input.trim_start_matches('!');
        
        self.logger.info("Terminal", &format!("Executing: {}", cmd));
        
        match std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output() 
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                
                if !stdout.is_empty() {
                    self.logger.info("Terminal", &stdout);
                }
                if !stderr.is_empty() {
                    self.logger.error("Terminal", &stderr);
                }
            }
            Err(e) => {
                self.logger.error("Terminal", &format!("Command failed: {}", e));
            }
        }
        
        true
    }

    fn handle_user_input(&mut self, input: &str) -> bool {
        self.logger.info("User", input);
        
        if self.llm_client.is_none() {
            self.tui.set_message("Error: LLM not configured. Please set API key in config.json");
            return true;
        }

        let client = self.llm_client.clone().unwrap();
        let client_clone = (*client).clone();
        let logger = self.logger.clone();
        let tui = self.tui.clone();
        
        let input_owned = input.to_string();
        tui.set_message(&format!("Processing: {}", input));

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let branch_prompt = load_prompt_from_file("prompts/branch-agent.md");

                let mut agent = create_branch_agent(AgentLevelDecl::L0, &branch_prompt);
                agent = agent.with_llm(client_clone).with_context_limit(128000);

                logger.info("Agent", "Calling LLM...");

                match agent.chat(&input_owned).await {
                    Ok(response) => {
                        logger.info("Agent", &response);
                        tui.set_message(&response);
                    }
                    Err(e) => {
                        logger.error("Agent", &format!("LLM Error: {}", e));
                        tui.set_message(&format!("Error: {}", e));
                    }
                }
            });
        });
        
        true
    }

    fn open_project(&mut self, project_name: &str) {
        let project_path = self.config.install_dir.join(project_name);
        
        if !project_path.exists() {
            self.logger.error("Project", &format!("Project not found: {}", project_name));
            self.tui.set_message(&format!("Error: Project '{}' not found", project_name));
            return;
        }

        let mem = memory::ProjectMemory::load(project_name.to_string(), self.config.install_dir.clone());
        
        if let Some(m) = mem {
            self.memory = Some(m);
            self.logger.info("Project", &format!("Opened project: {}", project_name));
            self.tui.set_message(&format!("Opened project: {}", project_name));
        } else {
            let m = memory::ProjectMemory::new(project_name.to_string(), self.config.install_dir.clone());
            self.memory = Some(m);
            self.logger.info("Project", &format!("Created project: {}", project_name));
            self.tui.set_message(&format!("Created project: {}", project_name));
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("");
    
    match command {
        "Orchestrator" | "orchestrator" | "tui" => {
            run_tui()?;
        }
        "-h" | "--help" | "help" => {
            println!("Agent Orchestrator");
            println!();
            println!("Usage:");
            println!("  Orchestrator            Start TUI interface");
            println!("  Orchestrator -h        Show help");
            println!("  Orchestrator test      Test LLM with '你好'");
            println!();
            println!("Commands in TUI:");
            println!("  /newproject             Create new project");
            println!("  /projectname            Open project");
            println!("  !command                Execute terminal command");
            println!("  /exit                   Exit");
        }
        "test" => {
            test_llm()?;
        }
        _ => {
            run_tui()?;
        }
    }
    
    Ok(())
}

fn test_llm() -> anyhow::Result<()> {
    let app = App::new()?;
    
    if app.llm_client.is_none() {
        println!("Error: LLM client not initialized");
        return Ok(());
    }
    
    let client = app.llm_client.unwrap();
    let client_clone = (*client).clone();
    
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let messages = vec![
            crate::llm::ChatMessage {
                role: "user".to_string(),
                content: "你好".to_string(),
            }
        ];
        
        match client_clone.chat(messages).await {
            Ok(response) => {
                println!("AI Response: {}", response.content);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    });
    
    Ok(())
}

fn run_tui() -> anyhow::Result<()> {
    let mut app = App::new()?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    loop {
        app.tui.update_logs();
        
        terminal.draw(|f| {
            app.tui.render(f);
        })?;

        if let Ok(event) = crossterm::event::read() {
            match event {
                crossterm::event::Event::Key(key) => {
                    match key.code {
                        crossterm::event::KeyCode::Char(c) => {
                            let state = app.tui.get_state();
                            let mut state = state.write();
                            state.input_buffer.push(c);
                        }
                        crossterm::event::KeyCode::Backspace => {
                            let state = app.tui.get_state();
                            let mut state = state.write();
                            state.input_buffer.pop();
                        }
                        crossterm::event::KeyCode::Enter => {
                            let state = app.tui.get_state();
                            let input = {
                                let mut state = state.write();
                                let input = state.input_buffer.clone();
                                state.input_buffer.clear();
                                input
                            };
                            
                            if !app.handle_command(&input) {
                                break;
                            }
                        }
                        crossterm::event::KeyCode::Esc => {
                            break;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}
