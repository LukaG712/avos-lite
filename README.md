<div align="center">

# `AVOS // AUTONOMOUS VENTURE OS`

**Build, Secure, and Operate AI-Powered Businesses from One Command Bar.**  
*Open-Core Rust CLI, Multi-Agent Terminal Multiplexer (`OSC 7501`), and Autonomous Execution Engine.*

[![AVOS Version](https://img.shields.io/badge/AVOS-v1.0.0-232323?style=flat-square&labelColor=1A1A1A)](https://avos.sh)
[![Architecture](https://img.shields.io/badge/MCP_GATEWAY-50_CONNECTORS-232323?style=flat-square&labelColor=1A1A1A)](https://avos.sh/docs/api)
[![Security](https://img.shields.io/badge/AEGIS_SECURITY-100%2F100_VERIFIED-232323?style=flat-square&labelColor=1A1A1A)](https://avos.sh)
[![License](https://img.shields.io/badge/LICENSE-OPEN_CORE-797878?style=flat-square&labelColor=1A1A1A)](./LICENSE)

[**Launch on AVOS Cloud (`avos.sh`) →**](https://avos.sh) · [**Try "Roast & Rebuild My Site" (`/roast`) →**](https://avos.sh/roast) · [**API & Webhook Docs →**](https://avos.sh/docs/api)

</div>

---

## `01 // WHAT IS AVOS?`

Most AI tools stop at generating code snippets. **AVOS (Autonomous Venture Operating System)** orchestrates **6 specialized Agent Guilds** across engineering, outbound sales, Shopify/Stripe commerce, white-hat security, and legal/tax compliance—governed by a **4-Stage Rust Firewall** and a single human-readable `avos.toml` manifest.

* **⚡ 80% Deterministic / 20% LLM Execution:** Linting, AST parsing (`Tree-sitter`), secret scanning, and DOM verification run as native Rust binaries at **0 tokens**.
* **🖥️ Herdr × Factory Droid TUI (`avos` & `ac`):** Spawn parallel sub-agents across split terminal panes with real-time `OSC 7501` status rollups (`🟢 WORKING`, `🟡 BLOCKED`, `🔵 DONE`).
* **🛡️ Zero-Trust by Default:** Tainted-context air-gaps and risk-scoped tool approvals ensure no agent ever touches production or spends money without your 1-click sign-off.

---

## `02 // 60-SECOND QUICKSTART (LOCAL CLI)`

### 1. Install the Native Rust Binary

```bash
# macOS / Linux (Homebrew)
brew install avos-sh/tap/avos

# Or via Cargo (All Platforms)
cargo install avos-cli
```

### 2. Initialize a Venture Workspace

```bash
avos init my-saas --template b2b-micro-saas
cd my-saas
```

### 3. Run Local (BYOK / Ollama) or Connect to AVOS Cloud

```bash
# Option A: Connect to Managed AVOS Cloud (60fps Live VMs + 50 Managed MCPs)
avos login --cloud

# Option B: Run Self-Hosted Lite with your own API keys or local Ollama
export ANTHROPIC_API_KEY="sk-ant-..."
avos mission start "Scaffold Stripe subscriptions and run Aegis RLS audit"
```

---

## `03 // MULTI-AGENT TERMINAL MULTIPLEXER (ac)`

Spawn, steer, and synchronize parallel agents directly from your terminal using `ac` (`avosctl`) and `OSC 7501` status rollups:

```bash
# Lead Architect spawns a QA Droid in a right split pane and waits for its test report
ac run --bg --pane split-right qa-droid "Verify Stripe checkout webhook. Done: ac send lead-droid"

# Dial autonomy from Level 0 (step-by-step approval) to Level 4 (overnight sandbox autonomy)
avos autonomy 2
```

---

## `04 // AVOS CLOUD VS. SELF-HOSTED LITE`

`avos-lite` gives you a fast, local single-node CLI and daemon. When you are ready for **60fps Live WebRTC Visual Workspaces**, **50 zero-config MCP integrations**, **Multi-Cloud LLM Failover**, and **Autonomous Growth Loops**, upgrade in one command via `avos login --cloud`:

| Capability | `avos-lite` (Self-Hosted Open-Core) | **AVOS Cloud (`app.avos.sh`)** |
| :--- | :--- | :--- |
| **Core CLI (`avos` / `ac`) & `avos.toml`** | ✅ Included | ✅ Included + Multi-Device Cloud Sync |
| **Model Routing & Prompt Caching** | Single-provider BYOK / Local Ollama | **4-Stage Multi-Cloud Failover (`<50ms`) + 90% Cache Savings** |
| **Visual Agent Workspaces** | Local terminal PTY logs only | **60fps WebRTC Firecracker VMs, Ghost Cursors & 1-Click Takeover** |
| **MCP Gateway Connectors** | 6 Basic Local Connectors (Manual Keys) | **All 50 Managed MCPs + 1-Click Ambient OAuth + Custom BYO-MCP** |
| **Visual Mode & Surface** | Terminal TUI | **Zero-Chrome Web IDE (`☀️ Simple` & `🌙 Pro`), Multiplayer War Room & Mobile Bridge** |
| **Scale, Agency & HoldCo Engines** | Single Local Project | **HoldCo Cockpit (`Cmd+Shift+H`), White-Label Client Portal & Exit Data Room** |

> **[ // START FREE ON AVOS CLOUD ($0/MO PAYG OR $29 STARTER) → ](https://avos.sh/pricing)**

---

## `05 // REPOSITORY STRUCTURE`

```text
avos-lite/
├── Cargo.toml                  # Rust workspace (CLI, Local Daemon, Offline License Verifier)
├── avos.toml.example           # Single-file venture configuration template
├── crates/
│   ├── avos-cli/               # `avos` & `ac` TUI multiplexer (`OSC 7501` status rollup)
│   ├── avos-daemon/            # Local single-node runtime (`AVOS_RUNTIME_MODE=local_lite`)
│   └── avos-license/           # Offline Ed25519 `license.avoskey` cryptographic verifier
├── stdlib-lite/
│   ├── loops/                  # Open-source 80/20 YAML loops (`tdd-bug-fixer`, `icp-lead-scraper`)
│   └── gbnf/                   # Pre-compiled GBNF grammars for Qwen-3, DeepSeek & Llama-4
└── docs/
    └── quickstart.md           # Local CLI, BYOK & Ollama configuration guide
```
