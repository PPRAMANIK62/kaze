# kaze

A memory-minimal AI coding agent for the terminal.

Written in Rust. MIT licensed. Early development.

## Why kaze

Most AI coding tools carry serious memory overhead. Claude Code (Node.js) idles at 300MB and peaks above 8.5GB with known memory leaks. OpenCode (TypeScript) sits at 40-80MB idle, peaking around 400MB. kaze targets under 25MB idle and under 80MB peak. The name means "wind" in Japanese.

kaze is built on [rig-core](https://github.com/0xPlaygrounds/rig) for LLM abstraction and runs on a single-threaded tokio runtime to keep overhead minimal. The release binary is optimized for size with `opt-level="z"`, LTO, and symbol stripping.

## Current Features

- `kaze ask "question"` ... one-shot streaming responses from Anthropic Claude
- `kaze chat` ... interactive multi-turn REPL with readline support (arrow keys, history recall, Ctrl+R search)
- `kaze chat --session {id}` ... resume a previous conversation by session ID
- `kaze session list` ... browse saved sessions with formatted table
- `kaze session resume {id}` ... resume a session by full or partial ID
- `kaze session delete {id}` ... delete a session
- `kaze session new` ... start a new session (alias for `kaze chat`)
- Partial session ID matching (git-style short IDs)
- `kaze config show` ... view current configuration
- Streaming token-by-token output
- TOML configuration with XDG paths (`~/.config/kaze/config.toml`)
- Per-project config override (`kaze.toml` in project root)
- Environment variable resolution (`{env:VAR}` syntax)
- Persistent readline history across sessions
- Slash commands in chat: `/history`, `/clear`, `/help`
- Markdown-lite formatting for assistant responses (bold, inline code, fenced code blocks)
- Default system prompt (configurable via `system_prompt` in config)
- Session persistence: conversations saved as JSONL files, survive restarts

## Quick Start

```bash
# Build from source (requires Rust toolchain)
cargo install --path .

# Set your API key
export ANTHROPIC_API_KEY="your-key-here"

# One-shot question
kaze ask "explain ownership in rust"

# Interactive chat (creates a new session automatically)
kaze chat

# Resume a previous session
kaze chat --session <session-id>

# List saved sessions
kaze session list

# Resume a session by short ID
kaze session resume abc12345

# Delete a session
kaze session delete abc12345
```

## Configuration

Global config lives at `~/.config/kaze/config.toml`. Drop a `kaze.toml` in your project root to override it per-project.

```toml
model = "claude-sonnet-4-5"
system_prompt = "You are a senior Rust developer. Be concise and precise."

[provider.anthropic]
api_key = "{env:ANTHROPIC_API_KEY}"
```

## Roadmap

kaze is being built incrementally in 34 steps across 8 phases.

| Phase | Description | Status |
|-------|-------------|--------|
| 0 | Project scaffold | Done |
| 1 | Core (ask, streaming, config) | Done |
| 2 | Multi-turn chat + sessions | Done |
| 3 | Multi-provider (OpenAI, Ollama) | Planned |
| 4 | Context management (token counting, compaction) | Planned |
| 5 | Tools (read, write, edit, grep, bash) | Planned |
| 6 | Agent loop | Planned |
| 7 | TUI (ratatui) | Planned |
| 8 | Advanced (MCP, custom agents, rules) | Planned |

Inspired by [OpenCode](https://github.com/sst/opencode) and [aichat](https://github.com/sigoden/aichat).

## License

[MIT](LICENSE)
