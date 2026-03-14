use agent_orchestrator::llm::{LLMClient, LLMProvider};
use agent_orchestrator::agent::{create_branch_agent, create_leaf_agent, AgentLevel};

#[tokio::main]
async fn main() {
    // 创建 LLM 客户端
    let provider = LLMProvider::OpenAI {
        api_key: "your-api-key".to_string(),
        model: "gpt-4".to_string(),
    };
    let client = LLMClient::new(provider);

    // 创建枝干 Agent (架构师)
    let branch_prompt = std::fs::read_to_string("prompts/branch-agent.md")
        .unwrap_or_else(|_| "You are a branch agent.".to_string());
    
    let mut branch_agent = create_branch_agent(AgentLevel::L0, &branch_prompt)
        .with_llm(client.clone())
        .with_context_limit(128000);

    // 创建叶子 Agent (功能实现)
    let leaf_prompt = std::fs::read_to_string("prompts/leaf-agent.md")
        .unwrap_or_else(|_| "You are a leaf agent.".to_string());
    
    let mut leaf_agent = create_leaf_agent(&leaf_prompt)
        .with_llm(client)
        .with_context_limit(128000);

    // 测试对话
    println!("=== Branch Agent (Architect) ===");
    match branch_agent.chat("我需要一个用户登录功能").await {
        Ok(response) => println!("{}", response),
        Err(e) => println!("Error: {}", e),
    }

    println!("\n=== Leaf Agent (Function) ===");
    match leaf_agent.chat("实现一个简单的登录功能，包含用户名密码验证").await {
        Ok(response) => println!("{}", response),
        Err(e) => println!("Error: {}", e),
    }

    // 检查上下文使用率
    println!("\n=== Context Usage ===");
    println!("Branch: {:.1}%", branch_agent.get_context_usage() * 100.0);
    println!("Leaf: {:.1}%", leaf_agent.get_context_usage() * 100.0);
}
