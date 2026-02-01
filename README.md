# preflight

A local code review tool for AI-generated changes. Review diffs in the browser, comment on specific lines, and let your AI agent explain code and apply revisions -- all before the work reaches formal review.

Preflight calls no LLM APIs. It exposes an [MCP](https://modelcontextprotocol.io/) server that your existing AI agent (Claude Code, Codex, OpenCode, etc.) connects to, so the agent can participate in review conversations directly.

## How It Works

```
You (browser) <-> Preflight <-> MCP <-> Your AI Agent
```

**Review mode** -- point preflight at a set of changes and review them:

```bash
preflight                              # uncommitted changes
preflight --range HEAD~3..HEAD         # commit range
preflight --patch changes.patch        # patch file
git diff main | preflight --stdin      # piped diff
```

**Server mode** -- run preflight persistently and let agents push reviews to you:

```bash
preflight serve
```

## Features

- Browser-based diff viewer with syntax highlighting
- Inline comment threads between you and your AI agent
- "Explain" -- select code and request an explanation from the agent
- Agent-submitted revisions update the diff in place for re-review
- Single binary, no external dependencies

## Tech Stack

Rust (Axum) backend, Svelte 5 frontend, bundled into one binary via rust-embed.

## License

Apache-2.0
