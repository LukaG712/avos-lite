# AVOS-LITE QUICKSTART GUIDE

## 1. Prerequisites
- Rust toolchain (`1.85+`, edition `2024`) or Homebrew on macOS/Linux
- An API key (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`) or local **Ollama** (`http://localhost:11434`)

## 2. Initialize & Start the Local Daemon
```bash
cp avos.toml.example avos.toml
# Or scaffold a new venture directory:
avos init my-saas --template b2b-micro-saas

# Terminal 1: start the local daemon (reads ./avos.toml or AVOS_CONFIG_PATH)
avos-daemon

# Terminal 2: check daemon health or launch the Droid TUI
avos health
avos tui
```

## 3. Running Loops & Local Ollama / BYOK Models
```bash
ollama pull qwen3-coder:8b
export OLLAMA_BASE_URL="http://localhost:11434"

# List built-in stdlib-lite loops
avos loop list

# Run a loop (dry-run by default; add --exec to run shell commands, --approve for governance gates)
avos loop run tdd-bug-fixer --exec
avos loop run icp-lead-scraper --approve --model qwen3-coder:8b --endpoint http://localhost:11434
```

## 4. Multi-Agent Multiplexer (`ac`)
```bash
ac run --bg --pane split-right qa-droid "Verify Stripe checkout webhook"
ac status qa-droid DONE "All webhook tests passed"
ac send lead-droid "QA passed"
ac inbox lead-droid --drain
ac list
```

## 5. Environment Variables
- `AVOS_SOCKET_PATH`: Unix socket path (default: `/tmp/avos.sock`)
- `AVOS_CONFIG_PATH`: Path to `avos.toml` (default: `avos.toml`)
- `AVOS_LICENSE_KEY`: Optional offline Ed25519 license token (verified by `avos-license`)
- `OLLAMA_BASE_URL`: Base URL for local Ollama `/api/generate` calls
- `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` (`OPENAI_BASE_URL`): Cloud BYOK credentials for `anthropic/*` and `openai/*` models

## 6. Upgrading to AVOS Cloud
Run `avos login` to get the OAuth CLI URL (`https://app.avos.sh/oauth/cli`) and connect your local workspace to 60fps WebRTC sandboxes, 50 managed MCP servers, and multi-cloud failover.
