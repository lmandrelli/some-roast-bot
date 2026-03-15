# some-roast-bot

A sarcastic Discord bot that roasts you while answering your questions. It uses an AI agent (currently powered by Kimi K2.5, served by [Chutes AI](https://chutes.ai)) with real-time web search via the [Exa MCP](https://exa.ai) server.

---

## Table of Contents

- [Features](#features)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
- [Architecture](#architecture)
- [License](#license)

---

## Features

- **`/ask` command** - Asks the AI agent a question and receives a sarcastic response.

---

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- A [Discord bot token](https://discord.com/developers/applications)
- An OpenAI-compatible API key (the example uses [Chutes AI](https://chutes.ai))

---

## Installation

```bash
# Clone the repository
git clone https://github.com/lmandrelli/some-roast-bot.git
cd some-roast-bot

# Build the project
cargo build --release
```

---

## Configuration

Copy the example environment file and fill in your credentials:

```bash
cp .env.example .env
```

| Variable          | Description                                              | 
|-------------------|----------------------------------------------------------|
| `DISCORD_TOKEN`   | Your Discord bot token                                   |
| `OPENAI_API_KEY`  | API key for the LLM provider (Chutes or OpenAI-compat.)  |
| `OPENAI_BASE_URL` | Base URL of the OpenAI-compatible API endpoint           |

> **Note:** The bot reads these variables at startup via [`dotenv`](https://crates.io/crates/dotenv). Never commit your `.env` file.

---

## Usage

```bash
# Run from source
cargo run --release
```

Once running, invite the bot to your server using the OAuth2 URL, then type `/ask` in any channel the bot has access to.

---

## Architecture

```
src/
├── main.rs              # Startup wiring: env, framework, bot launch
├── agents/
│   ├── mod.rs           # Re-exports
│   └── ask.rs           # AskAgent: LLM agent wired to the Exa MCP web-search tool
└── bot/
    ├── mod.rs           # Shared types (Data, Error, Context)
    └── commands/
        ├── mod.rs       # Re-exports all commands
        └── ask.rs       # /ask slash command
```

**Key components:**

- **`poise` + `serenity`** — Discord bot framework and gateway client.
- **`rig-core`** — Agent builder that composes the LLM model with MCP tools.
- **`rmcp`** — MCP client that connects to `https://mcp.exa.ai/mcp` and exposes web-search tools to the agent.
- **`AskAgent`** — Wraps the rig `Agent`, holds the MCP `RunningService` alive for the lifetime of the bot, and exposes a simple `ask(&str)` interface.
- **`SharedAskAgent`** — `Arc<RwLock<Option<AskAgent>>>` shared across the async framework.

---

## License

This project is licensed under the [MIT License](LICENSE).  
