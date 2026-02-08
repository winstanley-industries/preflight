# preflight

A local code review tool for AI-generated changes. Review diffs in the browser, comment on specific lines, and let your AI agent explain code and apply revisions — all before the work reaches formal review.

Preflight calls no LLM APIs. It exposes an [MCP](https://modelcontextprotocol.io/) server that your existing AI agent (Claude Code, Codex, OpenCode, etc.) connects to, so the agent can participate in review conversations directly.

## How It Works

```
You (browser) <-> Preflight <-> MCP <-> Your AI Agent
```

1. Start the server
2. Your agent creates a review via MCP (or you create one from the browser)
3. Review diffs, leave comments, ask for explanations
4. The agent responds, submits revisions, and the diff updates in place

## Install

**macOS (Homebrew):**

```bash
brew install winstanley-industries/preflight/preflight
```

**Linux:** Download a binary from [GitHub Releases](https://github.com/ASmithOWL/preflight/releases).

**From source:**

```bash
cargo build --release -p preflight-server
```

## Quick Start

```bash
# Start the server (default port 3000)
preflight serve

# Start on a different port
preflight serve --port 8080

# Discard existing state and start fresh
preflight serve --fresh
```

Then open http://127.0.0.1:3000 in your browser.

## MCP Configuration

The MCP server connects to a running preflight web server (default port 3000).

### Claude Code

```bash
claude mcp add --scope user --transport stdio preflight -- preflight mcp
```

Or add to `.mcp.json`:

```json
{
  "mcpServers": {
    "preflight": {
      "command": "preflight",
      "args": ["mcp"]
    }
  }
}
```

### Codex

Add to `~/.codex/config.toml` (or `.codex/config.toml` in your project):

```toml
[mcp_servers.preflight]
command = "preflight"
args = ["mcp"]
```

### OpenCode

Add to `opencode.json`:

```json
{
  "mcp": {
    "preflight": {
      "type": "local",
      "command": ["preflight", "mcp"]
    }
  }
}
```

To use a non-default port, append `"--port", "8080"` to the args/command for any of the above.

## Usage

Once configured, ask your AI agent to create a review:

> "Send my changes to preflight for review"
>
> "Create a preflight review"

The agent will create a review via MCP. Open the preflight UI to review diffs, leave comments, and ask questions. The agent can respond to your comments and submit revisions.

**Tip:** Ask your agent to monitor the review in the background so it responds to your comments automatically.

_A dedicated `/preflight` skill for Claude Code is coming soon._

## Features

- Browser-based diff viewer with syntax highlighting
- Inline comment threads between you and your AI agent
- Agent-submitted revisions with interdiff to see what changed
- Revision timeline for navigating review history
- Real-time updates via WebSocket
- Single binary, no external dependencies

## CLI Reference

```
preflight serve [OPTIONS]    Start the web server (default command)
  --port <PORT>              Port to listen on [default: 3000]
  --fresh                    Discard existing state and start fresh

preflight mcp [OPTIONS]      Start the MCP stdio server
  --port <PORT>              Port of the running web server [default: 3000]
```

## Tech Stack

Rust (Axum) backend, Svelte 5 frontend, bundled into one binary via rust-embed.

## License

Apache-2.0
