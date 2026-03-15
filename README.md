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
- [Deployment](#deployment)
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
│   └── ask.rs           # Stateless ask() function: connects to MCP, prompts the LLM, returns
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
- **`agents::ask()`** — Stateless async function that cold-starts an MCP connection and a rig agent on each invocation. Everything is dropped when the call completes, so there is no shared state or long-lived connection to manage.

---

## Deployment

The project includes a CI/CD pipeline that automatically builds a Docker image and pushes it to GitHub Container Registry (GHCR) when a version tag is pushed. A Portainer stack then pulls and runs the image on your server.

### How it works

1. You push a tag like `v1.0.0` to GitHub
2. GitHub Actions builds the Docker image and pushes it to `ghcr.io/lmandrelli/some-roast-bot`
3. Portainer detects the new image and redeploys the container via its stack webhook

### Initial Portainer setup (one-time)

1. **Create the stack** in Portainer:
   - Go to **Stacks > Add stack**
   - Choose **Web editor** and paste the content of `docker-compose.yml`
   - Under **Environment variables**, add the variables from `.env.example` (`DISCORD_TOKEN`, `OPENAI_API_KEY`, `OPENAI_BASE_URL`) with your actual values. Alternatively, upload a `stack.env` file.

2. **Enable the stack webhook** for automatic redeployment:
   - After creating the stack, go to the stack's settings
   - Enable **Stack webhook**
   - Copy the generated webhook URL (it looks like `https://<portainer-host>/api/stacks/webhooks/<uuid>`)

3. **Trigger redeployment** after a new image is pushed:
   - Since Portainer is on your local network and not exposed to the internet, you can trigger the webhook locally with:
     ```bash
     curl -X POST https://192.168.7.102:9443/api/stacks/webhooks/<uuid> -k
     ```
   - Alternatively, set up a cron job or a small script on a machine on your local network to poll and trigger it periodically.

### Publishing a new version

```bash
git tag v1.0.0
git push origin v1.0.0
```

This triggers the GitHub Actions workflow, which builds and pushes the image tagged as `1.0.0`, `1.0`, and `latest`.

---

## License

This project is licensed under the [MIT License](LICENSE).  
