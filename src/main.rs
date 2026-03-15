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
mod i18n;

use config::Config;
use logger::Logger;
use ui::TUI;
use llm::{LLMClient, LLMProvider};
use agent::{create_branch_agent, load_prompt_from_file, AgentLevel as AgentLevelDecl, Agent};
use i18n::{t, t_with_args};

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
        
        i18n::init(&config.language);
        
        let llm_client = if let Some(provider) = config.get_default_model() {
            let api_key = config.resolve_api_key(provider);
            if api_key.is_empty() {
                eprintln!("Warning: {}", t("errors.api_key_not_configured"));
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
            eprintln!("Warning: {}", t("errors.no_default_model"));
            None
        };

        if llm_client.is_some() {
            println!("{}", t("success.llm_initialized"));
        }

        let logger = Arc::new(logger::init_logger(&install_dir));
        logger.info("System", &t("system.started"));

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
                self.logger.info("Command", &t("project.creating"));
                self.tui.set_message(&t("input.enter_project_name"));
                true
            }
            "/exit" | "/quit" => {
                self.logger.info("System", &t("system.shutting_down"));
                false
            }
            cmd if cmd.starts_with("/lang ") => {
                let lang = cmd.trim_start_matches("/lang ").trim();
                if i18n::available_locales().contains(&lang) {
                    i18n::set_locale(lang);
                    self.tui.set_message(&format!("Language changed to: {}", lang));
                } else {
                    self.tui.set_message(&format!("Available locales: {}", i18n::available_locales().join(", ")));
                }
                true
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
        message.contains(&t("input.enter_project_name"))
    }

    fn create_project(&mut self, project_name: &str) {
        let project_path = self.config.install_dir.join(project_name);
        
        if project_path.exists() {
            self.logger.error("Project", &t_with_args("errors.project_already_exists", &[("0", project_name)]));
            self.tui.set_message(&t_with_args("errors.project_already_exists", &[("0", project_name)]));
            return;
        }

        std::fs::create_dir_all(&project_path).ok();
        std::fs::create_dir_all(project_path.join("memory")).ok();
        std::fs::create_dir_all(project_path.join("memory/current")).ok();
        std::fs::create_dir_all(project_path.join("memory/history")).ok();
        std::fs::create_dir_all(project_path.join("logs")).ok();

        self.logger.info("Project", &t_with_args("project.created", &[("0", project_name)]));
        self.tui.set_message(&t_with_args("project.created", &[("0", project_name)]));
        
        self.memory = Some(memory::ProjectMemory::new(
            project_name.to_string(),
            self.config.install_dir.clone()
        ));
    }

    fn handle_terminal_command(&mut self, input: &str) -> bool {
        let cmd = input.trim_start_matches('!');
        
        self.logger.info("Terminal", &t_with_args("terminal.executing", &[("0", cmd)]));
        
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
                self.logger.error("Terminal", &t_with_args("errors.command_failed", &[("0", &e.to_string())]));
            }
        }
        
        true
    }

    fn handle_user_input(&mut self, input: &str) -> bool {
        self.logger.info("User", input);
        
        if self.llm_client.is_none() {
            self.tui.set_message(&t("errors.api_key_not_configured"));
            return true;
        }

        let client = self.llm_client.clone().unwrap();
        let client_clone = (*client).clone();
        let logger = self.logger.clone();
        let tui = self.tui.clone();
        
        let input_owned = input.to_string();
        tui.set_message(&t_with_args("success.processing", &[("0", input)]));

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let branch_prompt = load_prompt_from_file("prompts/branch-agent.md");

                let mut agent = create_branch_agent(AgentLevelDecl::L0, &branch_prompt);
                agent = agent.with_llm(client_clone).with_context_limit(128000);

                logger.info("Agent", &t("agent.calling"));

                match agent.chat(&input_owned).await {
                    Ok(response) => {
                        logger.info("Agent", &response);
                        tui.set_message(&response);
                    }
                    Err(e) => {
                        logger.error("Agent", &t_with_args("errors.llm_error", &[("0", &e.to_string())]));
                        tui.set_message(&t_with_args("errors.llm_error", &[("0", &e.to_string())]));
                    }
                }
            });
        });
        
        true
    }

    fn open_project(&mut self, project_name: &str) {
        let project_path = self.config.install_dir.join(project_name);
        
        if !project_path.exists() {
            self.logger.error("Project", &t_with_args("errors.project_not_found", &[("0", project_name)]));
            self.tui.set_message(&t_with_args("errors.project_not_found", &[("0", project_name)]));
            return;
        }

        let mem = memory::ProjectMemory::load(project_name.to_string(), self.config.install_dir.clone());
        
        if let Some(m) = mem {
            self.memory = Some(m);
            self.logger.info("Project", &t_with_args("project.opened", &[("0", project_name)]));
            self.tui.set_message(&t_with_args("project.opened", &[("0", project_name)]));
        } else {
            let m = memory::ProjectMemory::new(project_name.to_string(), self.config.install_dir.clone());
            self.memory = Some(m);
            self.logger.info("Project", &t_with_args("project.created", &[("0", project_name)]));
            self.tui.set_message(&t_with_args("project.created", &[("0", project_name)]));
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
            println!("{}", t("help.title"));
            println!();
            println!("{}", t("help.usage"));
            println!("  {}", t("help.start_tui"));
            println!("  {}", t("help.show_help"));
            println!("  {}", t("help.test_llm"));
            println!();
            println!("{}", t("help.commands_title"));
            println!("  {}", t("help.cmd_newproject"));
            println!("  {}", t("help.cmd_openproject"));
            println!("  {}", t("help.cmd_terminal"));
            println!("  {}", t("help.cmd_exit"));
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
        println!("{}", t("errors.llm_client_not_initialized"));
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
