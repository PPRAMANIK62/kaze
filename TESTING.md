# kaze Manual Testing Guide

This document is a manual testing guide and regression checklist for the kaze CLI. Work through each section after making changes to verify nothing is broken. Every command is copy-pasteable.

## Prerequisites

You need a Rust toolchain installed (`rustup`, `cargo`). You also need at least one API key for the provider you plan to test.

Set the relevant environment variables:

```bash
export ANTHROPIC_API_KEY="your-anthropic-key"
export OPENAI_API_KEY="your-openai-key"
export OPENROUTER_API_KEY="your-openrouter-key"
```

You don't need all three. Just set the one(s) for the provider you're testing.

All commands below use `cargo run --` to run from source. If you've installed the binary, replace `cargo run --` with `kaze`.


## 1. Build Verification

These checks don't require any API keys.

### Debug build

```bash
cargo build
```

> Expected: compiles with zero errors. Warnings about unused methods are acceptable.

### Release build

```bash
cargo build --release
```

> Expected: compiles with zero errors. Takes longer due to LTO and size optimizations.


## 2. Help Text

No API keys needed. Verify the CLI structure and flag names are correct.

### Top-level help

```bash
cargo run -- --help
```

> Expected: shows "A memory-minimal AI coding agent" and lists subcommands: `ask`, `chat`, `config`, `session`.

### Ask help

```bash
cargo run -- ask --help
```

> Expected: shows "Ask a one-shot question". Lists flags:
> - `prompt` (positional, the question)
> - `-m, --model` (model override)
> - `-p, --provider` (provider selection: anthropic, openai, openrouter)

### Chat help

```bash
cargo run -- chat --help
```

> Expected: shows "Start an interactive chat session". Lists flags:
> - `-s, --session` (resume a specific session)
> - `--provider` (provider selection)
> - `-m, --model` (model override)

### Config help

```bash
cargo run -- config --help
```

> Expected: shows "Manage configuration" and lists subcommands: `show`, `set`.

### Session help

```bash
cargo run -- session --help
```

> Expected: shows "Manage chat sessions" and lists subcommands: `new`, `list`, `resume`, `delete`.


## 3. One-Shot Ask

Each test sends a simple prompt and checks that streaming output appears.

### Default provider (Anthropic)

```bash
cargo run -- ask "what is 2+2"
```

> Expected: header line shows `kaze [model: claude-sonnet-4-5]`. Streaming tokens appear. The answer includes "4".

### Anthropic with model override

```bash
cargo run -- ask --model claude-sonnet-4-5 "hello"
```

> Expected: header shows `[model: claude-sonnet-4-5]`. Response streams normally.

### OpenAI provider

```bash
cargo run -- ask --provider openai "hello"
```

> Expected: header shows `[model: gpt-4o]` (the default OpenAI model). Response streams.

### OpenAI with model override

```bash
cargo run -- ask --provider openai --model gpt-4o-mini "hello"
```

> Expected: header shows `[model: gpt-4o-mini]`. Response streams.

### OpenRouter provider

```bash
cargo run -- ask --provider openrouter "hello"
```

> Expected: header shows `[model: openai/gpt-4o]` (the default OpenRouter model). Response streams.

### OpenRouter with model override

```bash
cargo run -- ask --provider openrouter --model "anthropic/claude-3.5-sonnet" "hello"
```

> Expected: header shows `[model: anthropic/claude-3.5-sonnet]`. Response streams.

### Short flags

```bash
cargo run -- ask -p openai -m gpt-4o-mini "hello"
```

> Expected: same as the long-flag version. `-p` maps to `--provider`, `-m` maps to `--model`.


## 4. Interactive Chat

### Start a new session

```bash
cargo run -- chat
```

> Expected: prints `kaze chat [session: <8-char-id>] [model: claude-sonnet-4-5] (Ctrl+D to exit)`. Shows a green `>` prompt. Typing a message and pressing Enter streams a response.

### Chat with a specific provider

```bash
cargo run -- chat --provider openai
```

> Expected: header shows `[model: gpt-4o]`. Chat works with OpenAI.

### Chat with provider and model

```bash
cargo run -- chat --model gpt-4o-mini --provider openai
```

> Expected: header shows `[model: gpt-4o-mini]`.

### Resume a session by ID

First, note a session ID from `cargo run -- session list`, then:

```bash
cargo run -- chat --session <full-session-id>
```

> Expected: prints `resuming [session: <8-char-id>] [model: ...]`. Previous messages are displayed. New messages continue the conversation with full context.

### Multi-turn context

Inside a chat session:

1. Type: `my name is Alice`
2. Wait for the response.
3. Type: `what is my name?`

> Expected: the assistant remembers "Alice" from the previous turn.

### Slash commands

Inside a running chat session, test each slash command:

```
/help
```

> Expected: prints a list of commands: `/history`, `/clear`, `/help`, and `Ctrl+D` to exit.

```
/history
```

> Expected: prints all messages in the current session (user and assistant turns). System messages are hidden.

```
/clear
```

> Expected: prints "History cleared." Conversation context is wiped (system prompt is preserved). Subsequent messages won't have prior context.

### Unknown slash command

```
/foo
```

> Expected: prints `? Unknown command: /foo`.

### Empty input

Press Enter on an empty line.

> Expected: nothing happens. The prompt reappears. No message is sent.

### Exit with Ctrl+D

Press Ctrl+D at the prompt.

> Expected: prints `goodbye.` and exits cleanly.

### Cancel with Ctrl+C

Press Ctrl+C at the prompt.

> Expected: prints `^C` and returns to the prompt. Does not exit the session.

### Readline features

- Up/Down arrow keys recall previous inputs.
- Ctrl+R searches readline history.
- History persists across sessions (stored in `~/.cache/kaze/chat_history.txt`).


## 5. Session Management

### List sessions

```bash
cargo run -- session list
```

> Expected: if sessions exist, prints a table with columns: ID (8 chars), TITLE, MSGS, UPDATED, MODEL. Shows total count and a hint: `kaze session resume <id>`.

> If no sessions exist, prints: "No sessions found. Start one with: kaze chat".

### Create a new session

```bash
cargo run -- session new
```

> Expected: starts an interactive chat (same as `cargo run -- chat`). A new session is created.

### Resume by full ID

```bash
cargo run -- session resume <full-uuid>
```

> Expected: opens the chat session with all previous messages displayed. Conversation continues from where it left off.

### Resume by partial ID

```bash
cargo run -- session resume <first-8-chars>
```

> Expected: resolves the partial ID to the full session and resumes it. Works the same as full ID.

### Delete a session

```bash
cargo run -- session delete <id>
```

> Expected: prints `Deleting session <8-char-id> ("session title")` followed by `Deleted.` The session no longer appears in `session list`.

### Delete by partial ID

```bash
cargo run -- session delete <first-8-chars>
```

> Expected: resolves the partial ID and deletes the matching session.


## 6. Configuration

### Show current config

```bash
cargo run -- config show
```

> Expected: prints the config file path (`~/.config/kaze/config.toml`) and the current configuration as TOML. Shows `model`, `system_prompt`, and `[provider]` sections.

### Config file location

The global config file lives at:

```
~/.config/kaze/config.toml
```

If it doesn't exist, kaze creates a default one on first run with `{env:VAR}` placeholders for API keys.

### Per-project config

Create a `kaze.toml` in your project root:

```toml
model = "claude-sonnet-4-5"
system_prompt = "You are a Rust expert."
```

> Expected: project config values override global config. Verify with `cargo run -- config show`.

### Environment variable resolution

In `config.toml`, API keys use the `{env:VAR}` syntax:

```toml
[provider.anthropic]
api_key = "{env:ANTHROPIC_API_KEY}"
```

> Expected: kaze resolves `{env:ANTHROPIC_API_KEY}` to the actual environment variable value at runtime. If the env var is unset, the value resolves to an empty string.

### Config set (stub)

```bash
cargo run -- config set model "gpt-4o"
```

> Expected: prints `TODO: set model = gpt-4o`. This command is not yet implemented.


## 7. Error Handling

These tests verify that kaze fails gracefully with clear error messages.

### Unknown provider

```bash
cargo run -- ask --provider foo "test"
```

> Expected: error message: `Unknown provider: foo. Supported: anthropic, openai, openrouter`

### Missing API key (Anthropic)

```bash
unset ANTHROPIC_API_KEY
cargo run -- ask "test"
```

> Expected: error message mentions: `No API key found for Anthropic. Set ANTHROPIC_API_KEY or configure it in config.toml`

### Missing API key (OpenAI)

```bash
unset OPENAI_API_KEY
cargo run -- ask --provider openai "test"
```

> Expected: error message mentions: `No API key found for OpenAI. Set OPENAI_API_KEY or configure it in config.toml`

### Missing API key (OpenRouter)

```bash
unset OPENROUTER_API_KEY
cargo run -- ask --provider openrouter "test"
```

> Expected: error message mentions: `No API key found for OpenRouter. Set OPENROUTER_API_KEY or configure it in config.toml`

### Empty prompt

```bash
cargo run -- ask
```

> Expected: error message: `No prompt provided. Usage: kaze ask "your question here"`

### Non-existent session (resume)

```bash
cargo run -- session resume nonexistent
```

> Expected: error message: `No session found matching 'nonexistent'`

### Non-existent session (chat --session)

```bash
cargo run -- chat --session nonexistent-uuid-that-does-not-exist
```

> Expected: error about session not found.

### Ambiguous session ID

If you have multiple sessions, try resuming with a very short prefix that matches more than one:

```bash
cargo run -- session resume a
```

> Expected: prints `ambiguous: Multiple sessions match 'a':` followed by a list of matching sessions with their short IDs and titles. Then errors with: `Provide more characters to disambiguate`


## 8. Provider-Specific Tests

### Anthropic

- Default provider (no `--provider` flag needed).
- Default model: `claude-sonnet-4-5`.
- Streaming works token-by-token.
- Model name appears in the header line.

```bash
cargo run -- ask "say hello"
```

### OpenAI

- Requires `--provider openai`.
- Default model: `gpt-4o`.
- Streaming works token-by-token.
- Model name appears in the header line.

```bash
cargo run -- ask --provider openai "say hello"
```

### OpenRouter

- Requires `--provider openrouter`.
- Default model: `openai/gpt-4o`.
- Model names use the `provider/model` format (e.g., `anthropic/claude-3.5-sonnet`).
- Streaming works token-by-token.

```bash
cargo run -- ask --provider openrouter "say hello"
```


## 9. Session Persistence

### Sessions survive restarts

1. Start a chat: `cargo run -- chat`
2. Send a message. Note the session ID from the header.
3. Exit with Ctrl+D.
4. Resume: `cargo run -- chat --session <id>`

> Expected: previous messages are displayed. The conversation continues with full context.

### Session files

Sessions are stored as JSONL files under `~/.local/share/kaze/sessions/`. The index lives at `~/.local/share/kaze/sessions/index.json`.

```bash
ls ~/.local/share/kaze/sessions/
```

> Expected: `.jsonl` files (one per session) and an `index.json` file.

### Session title

The session title is derived from the first user message, truncated to 50 characters. Verify this in `session list` output.


## 10. Regression Checklist

Run through this after any change. Each item should pass.

- [ ] `cargo build` compiles without errors
- [ ] `cargo build --release` compiles without errors
- [ ] `cargo run -- --help` shows all subcommands
- [ ] `cargo run -- ask --help` shows correct flags
- [ ] `cargo run -- chat --help` shows correct flags
- [ ] `cargo run -- config --help` shows subcommands
- [ ] `cargo run -- session --help` shows subcommands
- [ ] Default provider (Anthropic) streams a response
- [ ] `--provider openai` streams a response
- [ ] `--provider openrouter` streams a response
- [ ] `--model` override works for each provider
- [ ] `cargo run -- chat` starts a new session
- [ ] Multi-turn context is maintained in chat
- [ ] `/help`, `/history`, `/clear` work in chat
- [ ] Ctrl+D exits chat cleanly
- [ ] Ctrl+C cancels input without exiting
- [ ] `session list` displays sessions correctly
- [ ] `session resume <id>` works with full and partial IDs
- [ ] `session delete <id>` removes the session
- [ ] `session new` starts a fresh chat
- [ ] `config show` displays current configuration
- [ ] Unknown provider gives a clear error
- [ ] Missing API key gives a clear error
- [ ] Empty prompt gives a clear error
- [ ] Non-existent session ID gives a clear error
- [ ] Ambiguous session ID lists matches and asks for more characters
