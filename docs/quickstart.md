# AVOS-LITE QUICKSTART GUIDE

## 1. Prerequisites
- Rust toolchain (`1.85+`) or Homebrew on macOS/Linux
- An API key (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`) or local **Ollama** (`http://localhost:11434`)

## 2. Running Local Ollama Models
```bash
ollama pull qwen3-coder:8b
cp avos.toml.example avos.toml
avos mission start "Add unit tests for checkout module"
```

## 3. Upgrading to AVOS Cloud
Run `avos login --cloud` to connect your local workspace to 60fps WebRTC sandboxes, 50 managed MCP servers, and multi-cloud failover.
