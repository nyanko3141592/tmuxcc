# TmuxCC

**AI Agent Dashboard for tmux** - Monitor and manage multiple AI coding agents from a single terminal interface.

TmuxCC is a TUI (Terminal User Interface) application that provides centralized monitoring and control of AI coding assistants running in tmux panes. It supports Claude Code, OpenCode, Codex CLI, and Gemini CLI.

---

## Screenshot

<!-- TODO: Add actual screenshot -->
```
+------------------------------------------------------------------+
|  TmuxCC - AI Agent Dashboard                   Agents: 3 Active: 1|
+------------------------------------------------------------------+
| main (Session)                    | Preview: main:0.0             |
| +-- 0: code                       |                               |
| |  +-- ~/project1                 | Claude Code wants to edit:    |
| |  |  * Claude Code  ! [Edit]     | src/main.rs                   |
| |  |     +-- > Explore (Running)  |                               |
| |  +-- ~/project2                 | - fn main() {                 |
| |     o OpenCode   @ Processing   | + fn main() -> Result<()> {   |
| +-- 1: shell                      |                               |
|    +-- ~/tools                    | Do you want to allow this     |
|       o Codex CLI  * Idle         | edit? [y/n]                   |
+------------------------------------------------------------------+
| [Y] Approve [N] Reject [A] All | [1-9] Choice | [Space] Select    |
+------------------------------------------------------------------+
```

*Replace with actual screenshot*

---

## Features

- **Multi-Agent Monitoring**: Track multiple AI agents across all tmux sessions and windows
- **Real-time Status**: See agent states at a glance (Idle, Processing, Awaiting Approval, Error)
- **Approval Management**: Approve or reject pending requests with single keystrokes
- **Batch Operations**: Select multiple agents and approve/reject all at once
- **Hierarchical View**: Tree display organized by Session/Window/Pane
- **Subagent Tracking**: Monitor spawned subagents (Task tool) with their status
- **Context Awareness**: View remaining context percentage when available
- **Pane Preview**: See live content from selected agent's tmux pane
- **Focus Integration**: Jump directly to any agent's pane in tmux
- **Customizable**: Configure polling interval, capture lines, and custom agent patterns

### Supported AI Agents

| Agent | Detection Method | Approval Keys |
|-------|------------------|---------------|
| **Claude Code** | `claude` command, version numbers, window title with icon | `y` / `n` |
| **OpenCode** | `opencode` command | `y` / `n` |
| **Codex CLI** | `codex` command | `y` / `n` |
| **Gemini CLI** | `gemini` command | `y` / `n` |

---

## Installation

### From crates.io (coming soon)

```bash
cargo install tmuxcc
```

### From Source

```bash
git clone https://github.com/yourusername/tmuxcc.git
cd tmuxcc
cargo build --release
cargo install --path .
```

### Requirements

- **tmux** (must be running with at least one session)
- **Rust** 1.70+ (for building from source)

---

## Usage

### Quick Start

1. Start tmux and run AI agents in different panes
2. Launch TmuxCC from any terminal:

```bash
tmuxcc
```

### Command Line Options

```
tmuxcc [OPTIONS]

Options:
  -p, --poll-interval <MS>      Polling interval in milliseconds [default: 500]
  -l, --capture-lines <LINES>   Lines to capture from each pane [default: 100]
  -f, --config <FILE>           Path to config file
  -d, --debug                   Enable debug logging to tmuxcc.log
      --show-config-path        Show config file path and exit
      --init-config             Create default config file and exit
  -h, --help                    Print help
  -V, --version                 Print version
```

### Examples

```bash
# Run with default settings
tmuxcc

# Set polling interval to 1 second
tmuxcc -p 1000

# Capture more lines for better context
tmuxcc -l 200

# Use custom config file
tmuxcc -f ~/.config/tmuxcc/custom.toml

# Enable debug logging
tmuxcc --debug

# Initialize default config file
tmuxcc --init-config
```

---

## Key Bindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `Down` | Next agent |
| `k` / `Up` | Previous agent |
| `Tab` | Cycle through agents |

### Selection

| Key | Action |
|-----|--------|
| `Space` | Toggle selection of current agent |
| `Ctrl+a` | Select all agents |
| `Esc` | Clear selection / Close popup |

### Actions

| Key | Action |
|-----|--------|
| `y` / `Y` | Approve pending request(s) |
| `n` / `N` | Reject pending request(s) |
| `a` / `A` | Approve ALL pending requests |
| `1`-`9` | Send numbered choice to agent |
| `f` / `F` | Focus on selected pane in tmux |
| `Left` / `Right` | Switch focus (Sidebar / Input) |

### View

| Key | Action |
|-----|--------|
| `s` / `S` | Toggle subagent log |
| `r` | Refresh agent list |
| `h` / `?` | Show help |
| `q` | Quit |

---

## Configuration

TmuxCC uses a TOML configuration file.

### Initialize Config

```bash
# Create default config file
tmuxcc --init-config

# Show config file location
tmuxcc --show-config-path
```

### Config File Location

| OS | Path |
|----|------|
| Linux | `~/.config/tmuxcc/config.toml` |
| macOS | `~/Library/Application Support/tmuxcc/config.toml` |
| Windows | `%APPDATA%\tmuxcc\config.toml` |

### Configuration Options

```toml
# Polling interval in milliseconds
poll_interval_ms = 500

# Number of lines to capture from each pane
capture_lines = 100

# Custom agent patterns (optional)
# Add patterns to detect additional AI agents
[[agent_patterns]]
pattern = "my-custom-agent"
agent_type = "CustomAgent"
```

---

## Status Indicators

| Icon | Status |
|------|--------|
| `!` `[Edit]` | File edit approval pending |
| `!` `[Shell]` | Shell command approval pending |
| `!` `[Question]` | User question awaiting response |
| `@` | Processing |
| `*` | Idle |
| `?` | Unknown |

---

## How It Works

1. **Discovery**: TmuxCC scans all tmux sessions, windows, and panes
2. **Detection**: Identifies AI agents by process name, window title, and command line
3. **Parsing**: Agent-specific parsers analyze pane content for status and approvals
4. **Monitoring**: Continuously polls panes at configurable intervals
5. **Actions**: Sends keystrokes to panes for approvals/rejections

---

## Project Structure

```
tmuxcc/
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # Library root
│   ├── agents/           # Agent type definitions
│   │   ├── types.rs      # AgentType, AgentStatus, MonitoredAgent
│   │   └── subagent.rs   # Subagent, SubagentType, SubagentStatus
│   ├── app/              # Application logic
│   │   ├── state.rs      # AppState, AgentTree, InputMode
│   │   ├── actions.rs    # Action enum
│   │   └── config.rs     # Configuration
│   ├── monitor/          # Monitoring
│   │   └── task.rs       # Async monitoring task
│   ├── parsers/          # Agent output parsers
│   │   ├── mod.rs        # AgentParser trait
│   │   ├── claude_code.rs
│   │   ├── opencode.rs
│   │   ├── codex_cli.rs
│   │   └── gemini_cli.rs
│   ├── tmux/             # tmux integration
│   │   ├── client.rs     # TmuxClient
│   │   └── pane.rs       # PaneInfo, process detection
│   └── ui/               # UI implementation
│       ├── app.rs        # Main loop
│       ├── layout.rs     # Layout definitions
│       └── components/   # UI components
└── Cargo.toml
```

---

## Tech Stack

- **Language**: Rust (Edition 2021)
- **TUI Framework**: [Ratatui](https://ratatui.rs/) 0.29
- **Terminal**: [Crossterm](https://github.com/crossterm-rs/crossterm) 0.28
- **Async Runtime**: [Tokio](https://tokio.rs/)
- **CLI Parser**: [Clap](https://clap.rs/) 4

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## Contributing

Contributions are welcome! Here's how you can help:

### Getting Started

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Run clippy (`cargo clippy`)
6. Format code (`cargo fmt`)
7. Commit your changes (`git commit -m 'Add amazing feature'`)
8. Push to the branch (`git push origin feature/amazing-feature`)
9. Open a Pull Request

### Areas for Contribution

- **New Agent Support**: Add parsers for other AI coding assistants
- **UI Improvements**: Enhance the terminal interface
- **Performance**: Optimize polling and parsing
- **Documentation**: Improve docs and examples
- **Bug Fixes**: Report and fix issues
- **Tests**: Improve test coverage

### Code Style

- Follow Rust conventions and idioms
- Run `cargo fmt` before committing
- Ensure `cargo clippy` passes without warnings
- Add tests for new functionality

---

## Related Projects

- [Claude Code](https://claude.ai/code) - Anthropic's AI coding assistant
- [OpenCode](https://github.com/opencode-ai/opencode) - Open-source AI coding assistant
- [Codex CLI](https://github.com/openai/codex-cli) - OpenAI's Codex CLI
- [Gemini CLI](https://github.com/google/gemini-cli) - Google's Gemini CLI
