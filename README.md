<h1 align="center">
  AVOS Lite
</h1>

<p align="center">
  <a href="https://github.com/LukaG712/avos-lite"><img src="https://img.shields.io/github/stars/LukaG712/avos-lite?style=flat&amp;label=%E2%98%85&amp;color=08C" alt="GitHub stars" /></a>
  <img src="https://img.shields.io/badge/version-1.0.0-08C?style=flat" alt="Version: 1.0.0" />
  <img src="https://img.shields.io/badge/license-MIT-08C?style=flat" alt="License: MIT" />
  <img src="https://img.shields.io/badge/Rust-2024_Edition-4493F8?style=flat-square" alt="Rust 2024 Edition" />
  <img src="https://img.shields.io/badge/macOS%20%7C%20Linux%20%7C%20Windows-4493F8?style=flat-square" alt="Supported platforms: macOS, Linux, and Windows" />
</p>

<p align="center">
  <strong>The Open-Core Autonomous Venture CLI &amp; Local Runtime.</strong><br/>
  Run deterministic YAML agent loops, manage autonomy levels (0–4), and orchestrate local or BYOK models from one Rust binary and a single <code>avos.toml</code> manifest.
</p>

<h3 align="center"><a href="#install"><ins>Quickstart in 60 Seconds</ins></a></h3>

---

## Features

<table>
<tr>
<td width="50%" valign="middle">

### Terminal TUI &amp; CLI

Launch the interactive Ratatui workspace with <code>avos</code> or drive missions, autonomy, and loops directly from scripts and CI.

[CLI Source →](crates/avos-cli/src/main.rs)

</td>
<td width="50%">

```bash
# Launch the interactive TUI
avos

# Or drive missions directly from the CLI
avos init my-saas --template b2b-micro-saas
avos mission start "Add unit tests for checkout"
avos mission list
```

</td>
</tr>
<tr>
<td width="50%" valign="middle">

### Deterministic 80/20 Loops

Execute repeatable YAML workflows like <code>tdd-bug-fixer</code> and <code>icp-lead-scraper</code> with explicit governance approval and command execution flags.

[Stdlib Loops →](stdlib-lite/loops)

</td>
<td width="50%">

```bash
# Inspect available stdlib loops
avos loop list

# Run a loop with explicit governance sign-off
avos loop run tdd-bug-fixer --approve --exec
```

</td>
</tr>
<tr>
<td width="50%" valign="middle">

### Tiered Autonomy (0–4)

Dial agent permissions from strict step-by-step approval up to autonomous local execution—with spend and production-push guards enforced in <code>avos.toml</code>.

[Config Template →](avos.toml.example)

</td>
<td width="50%">

```toml
[venture]
name = "my-saas"
runtime_mode = "local_lite"
autonomy_level = 1

[governance]
require_approval_for_spend = true
require_approval_for_prod_push = true
```

</td>
</tr>
<tr>
<td width="50%" valign="middle">

### Local Daemon &amp; Socket IPC

<code>avos-daemon</code> runs as a fast, single-node Rust runtime communicating over Unix domain sockets (<code>/tmp/avos.sock</code>) with structured JSON-RPC responses.

[Daemon Source →](crates/avos-daemon)

</td>
<td width="50%">

```bash
# Check daemon health over JSON-RPC
avos health

# Query or update runtime autonomy
avos autonomy
avos autonomy 2
```

</td>
</tr>
<tr>
<td width="50%" valign="middle">

### Tiered Model Routing &amp; GBNF

Route tasks across <code>fast_tier</code>, <code>balanced_tier</code>, and <code>architect_tier</code> using local Ollama models or your own API keys, with GBNF grammars for structured tool calls.

[GBNF Grammars →](stdlib-lite/gbnf/tool_call.gbnf)

</td>
<td width="50%">

```toml
[models]
fast_tier = "ollama/qwen3-coder:8b"
balanced_tier = "anthropic/claude-sonnet-4-6"
architect_tier = "anthropic/claude-opus-4-6"
```

</td>
</tr>
<tr>
<td width="50%" valign="middle">

### Offline Ed25519 Licensing

Verify commercial or pro entitlements completely offline using <code>ed25519-dalek</code> public-key verification—private signing keys never leave the backend.

[License Crate →](crates/avos-license)

</td>
<td width="50%">

```text
avos-lite/crates/avos-license
├── Ed25519 public-key signature verification
├── Offline license.avoskey entitlement checks
└── Zero network round-trips required locally
```

</td>
</tr>
</table>

**Also in the box:**

- **[Quickstart Guide](docs/quickstart.md)**: step-by-step setup for local Ollama (`qwen3-coder:8b`) and BYOK API keys.
- **[Workspace Scaffolding](crates/avos-cli/src/main.rs)**: initialize new venture directories with `avos init <name> --template <template>`.
- **[Cloud Bridge](crates/avos-cli/src/main.rs)**: run `avos login` when you are ready to connect your local workspace to [AVOS Cloud](https://app.avos.sh).

---

## Supported Model Providers

Works with local runtimes and bring-your-own-key (BYOK) cloud providers configured in `avos.toml`:

<p>
  <kbd>Ollama (Local)</kbd> &nbsp;
  <kbd>Anthropic (Claude)</kbd> &nbsp;
  <kbd>OpenAI</kbd> &nbsp;
  <kbd>Qwen Coder</kbd> &nbsp;
  <kbd>DeepSeek</kbd> &nbsp;
  <kbd>Llama</kbd> &nbsp;
  <kbd>+ GBNF Structured Tool Calling</kbd>
</p>

---

## Install

### From Source (Cargo)

Requires Rust `1.85+` (Edition 2024).

```bash
git clone https://github.com/LukaG712/avos-lite.git
cd avos-lite
cargo install --path crates/avos-cli
```

### Quickstart

```bash
# 1. Initialize a venture workspace
avos init my-saas --template b2b-micro-saas
cd my-saas

# 2. Configure local Ollama or BYOK API keys
cp ../avos.toml.example avos.toml
export ANTHROPIC_API_KEY="sk-ant-..."

# 3. Start the interactive TUI or launch a mission
avos tui
avos mission start "Add unit tests for checkout module"
```

> See [`docs/quickstart.md`](docs/quickstart.md) for local Ollama setup (`ollama pull qwen3-coder:8b`).

---

## `avos-lite` vs. AVOS Cloud

`avos-lite` is the open-core local CLI and daemon. For managed cloud execution, multi-provider LLM failover, and team billing, see [`LukaG712/avos`](https://github.com/LukaG712/avos) or connect via `avos login`.

| Capability | `avos-lite` (Open-Core Local) | AVOS Platform (`LukaG712/avos`) |
| :--- | :--- | :--- |
| **Core CLI (`avos`), TUI &amp; `avos.toml`** | Included | Included |
| **Local Daemon &amp; Socket IPC** | Included (`local_lite`) | Included + Cloud Control Plane |
| **Model Routing** | Local Ollama / Single-Key BYOK | Provider-Agnostic Router + Upstash Credit Locks |
| **Execution Isolation** | Local workspace | Sandboxed Execution Plane (`avos-sandbox-agent`) |
| **MCP Security &amp; Gateway** | — | `avos-firewall` + `mcp-gateway` |
| **Workflow Checkpoints &amp; Billing** | — | `orchestrator-graph` + PostgreSQL/Supabase Ledger |

---

## Repository Structure

```text
avos-lite/
├── Cargo.toml                  # Rust 2024 workspace manifest
├── avos.toml.example           # Single-file venture configuration template
├── crates/
│   ├── avos-cli/               # CLI + Ratatui TUI (`avos`)
│   ├── avos-daemon/            # Local single-node runtime & JSON-RPC IPC
│   └── avos-license/           # Offline Ed25519 entitlement verifier
├── stdlib-lite/
│   ├── loops/                  # Deterministic YAML loops (`tdd-bug-fixer`, `icp-lead-scraper`)
│   └── gbnf/                   # GBNF grammar for structured tool calls
└── docs/
    └── quickstart.md           # Local CLI, BYOK & Ollama setup guide
```

---

## Developing

```bash
# Check and test all Rust crates
cargo check --workspace
cargo test --workspace
```

## License

`avos-lite` is free and open source under the [MIT License](LICENSE).
