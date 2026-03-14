# Agent Orchestrator

An AI agent orchestration framework with TUI interface.

## Features

- Multi-layer Agent architecture (Branch/Leaf)
- Thread pool management
- Memory system with versioning
- TUI interface with ratatui
- Support for multiple LLM providers (OpenAI, Anthropic)
- Context usage monitoring and warnings

## Build

```bash
cargo build --release
```

## Usage

```bash
# Start TUI
agent-orchestrator tui
```

## Commands

- `/newproject` - Create new project
- `/projectname` - Open project
- `!command` - Execute terminal command
- `/command` - Software command

## Configuration

Edit `config.json` in the installation directory to configure:
- LLM providers
- API keys
- Context thresholds

## License

MIT
