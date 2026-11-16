<div align="center">
  <h1>AVOS Lite</h1>
  <p>The local-first autonomous venture OS in Rust</p>

  <p>
    <a href="docs/quickstart.md">Quickstart</a>
    ·
    <a href="avos.toml.example">avos.toml</a>
    ·
    <a href="stdlib-lite/loops">Stdlib Loops</a>
    ·
    <a href="https://avos.sh">AVOS Cloud</a>
    ·
    <a href="https://avos.sh/docs/api">API &amp; Docs</a>
    ·
    <a href="https://avos.sh/pricing">Pricing</a>
  </p>
</div>

<br />

AVOS Lite is a single-node, local-first operating system for autonomous engineering and venture workflows. It unifies a native Rust CLI (`avos`), a multi-agent terminal multiplexer (`ac` + `OSC 7501`), a JSON-RPC local daemon (`avos-daemon`), an 80/20 deterministic-first loop runner (`stdlib-lite`), and an offline Ed25519 license verifier (`avos-license`)—all governed by one human-readable `avos.toml` manifest.

<br />

# Why AVOS Lite

Most AI tooling stops at generating code snippets in a single chat window. When you try to run real workflows—reproducing bugs, patching code, verifying tests, scraping and deduplicating leads, or coordinating parallel sub-agents—you end up gluing together brittle shell scripts, paying tokens for deterministic work, and losing visibility into what each agent is doing.

AVOS Lite is built from the ground up as one coherent local runtime:

- **80% Deterministic / 20% LLM:** Shell verification, test suites, retries, and governance gates run as native Rust and deterministic commands at **0 tokens**. LLMs are invoked only where reasoning is actually needed.
- **Local-first, provider-agnostic:** Run completely offline with **Ollama**, or bring your own **Anthropic** (`ANTHROPIC_API_KEY`) or **OpenAI** (`OPENAI_API_KEY`) keys.
- **Zero-trust governance by default:** Autonomy is explicitly dialed from `L0` to `L4`, and `governance_gate` steps pause execution until you approve them.

Built in Rust (2024 edition) for speed, zero-dependency reliability, and clean single-binary distribution.

<br />

# Primitives

AVOS Lite is composed of modular Rust crates and YAML/GBNF primitives that share one local Unix socket (`/tmp/avos.sock`) and one `avos.toml` manifest:

| Primitive | Source | What it does |
| --- | --- | --- |
| **CLI &amp; Droid TUI** (`avos`) | [`crates/avos-cli`](crates/avos-cli/src/main.rs) | Venture scaffolding (`init`), mission dispatch, loop execution, and interactive Ratatui workspace (`tui`). |
| **Multiplexer** (`ac`) | [`crates/avos-cli/src/ac.rs`](crates/avos-cli/src/ac.rs) | Spawn, message, and track parallel agent panes (`run`, `send`, `inbox`, `status`) with `OSC 7501` status rollups. |
| **Local Daemon** (`avos-daemon`) | [`crates/avos-daemon`](crates/avos-daemon/src/lib.rs) | Single-node JSON-RPC 2.0 server over `/tmp/avos.sock` (`0600`) managing autonomy, missions, panes, and loops. |
| **80/20 Loop Engine** | [`crates/avos-daemon/src/loops.rs`](crates/avos-daemon/src/loops.rs) | Executes `deterministic`, `llm`, and `governance_gate` steps with context passing, PII redaction, and retry rollbacks. |
| **Stdlib-Lite Loops &amp; GBNF** | [`stdlib-lite`](stdlib-lite) | Ready-to-run YAML workflows (`tdd-bug-fixer`, `icp-lead-scraper`) and structured [`tool_call.gbnf`](stdlib-lite/gbnf/tool_call.gbnf) grammar. |
| **Offline Verifier** (`avos-license`) | [`crates/avos-license`](crates/avos-license/src/lib.rs) | Zero-network Ed25519 signature &amp; expiration verification for `license.avoskey` (`CommunityFree`, `PersonalPerpetual`, `CommercialPerpetual`). |

<br />

# **80% Deterministic / 20% LLM** loops

Instead of letting an agent burn tokens guessing whether tests pass, AVOS Lite models workflows as explicit YAML loops with three step modes: `deterministic`, `llm`, and `governance_gate`.

1. **Deterministic steps** execute native commands (`sh -c` with `--exec`) and capture `stdout`/`stderr` as context for the next step.
2. **LLM steps** receive the prior step's output automatically, map `tier` (`fast_0_2x`, `balanced_1x`, `architect`) to your `avos.toml` `[models]`, and route to Ollama, Anthropic, or OpenAI.
3. **Retry &amp; rollback:** When `max_iterations` and `rollback_on_fail: true` are set (as in [`tdd-bug-fixer.yaml`](stdlib-lite/loops/tdd-bug-fixer.yaml)), failed verification steps automatically trigger bounded retry loops.
4. **Governance gates:** Steps marked `governance_gate` (such as `WRITE_PRODUCTION_OR_SPEND` in [`icp-lead-scraper.yaml`](stdlib-lite/loops/icp-lead-scraper.yaml)) halt with `PAUSED_GOVERNANCE` unless `--approve` is passed.

```bash
# List available stdlib-lite loops
avos loop list

# Run TDD bug-fixer with real deterministic command execution
avos loop run tdd-bug-fixer --exec

# Run ICP lead scraper with governance approval and a specific Ollama model
avos loop run icp-lead-scraper --approve --model qwen3-coder:8b --endpoint http://localhost:11434
```

<br />

# **Multi-agent multiplexer** (`ac` &amp; `OSC 7501`)

Coordinate parallel agents across terminal panes using `ac` (`avosctl`). Every status transition emits a standard `OSC 7501` escape sequence (`\x1b]7501;agent=...;status=...;detail=...\x07`) and syncs with `avos-daemon` so the Droid TUI sidebar updates live.

```bash
# Register a QA Droid in a right split pane
ac run --bg --pane split-right qa-droid "Verify Stripe checkout webhook"

# Update status and notify the lead agent
ac status qa-droid DONE "All webhook tests passed"
ac send lead-droid "QA passed — ready for review"

# Drain messages from an agent's inbox
ac inbox lead-droid --drain
ac list
```

<br />

# **Droid TUI** &amp; local daemon

Launch `avos` (or `avos tui`) for a keyboard-first terminal workspace connected to `avos-daemon`:

```text
┌ AVOS-LITE // LOCAL_LITE  ● DAEMON CONNECTED │ AUTONOMY L1 ───────────────────┐
├ MISSION STREAM ────────────────────────────────────┬ AGENT PANES ────────────┤
│  [USER] Scaffold Stripe subscriptions              │  lead-droid WORKING     │
│  [MISSION] Dispatched: {"id":"lite_mis_1",...}     │  qa-droid DONE          │
│  [POLICY] Autonomy level set to L2                 │  aegis-droid ARMED      │
├ COMMAND / MISSION PROMPT ──────────────────────────┴─────────────────────────┤
│  ❯ /loops █                                                                  │
└ Enter: Run  F2: Sidebar  ↑/↓: History  Ctrl+L: Clear  Esc/Ctrl+C: Quit ──────┘
```

- **Live pane sync:** Polls `system.health` and `pane.list` from `avos-daemon` automatically (or on demand with `/panes`).
- **Slash commands:** `/mission <prompt>`, `/autonomy <0-4>`, `/panes`, `/loops`, `/health`, `/sidebar`, `/welcome`, `/clear`, `/quit`.
- **Keyboard-first navigation:** `↑`/`↓` command history, `←`/`→`/`Home`/`End` cursor editing, `PageUp`/`PageDown` stream scrolling, `F2` sidebar toggle.

<br />

# **Single-file venture manifest** (`avos.toml`)

One file configures your venture identity, runtime mode, default autonomy level (`L0`–`L4`), model tiers, and governance guardrails:

```toml
[venture]
name = "my-saas"
runtime_mode = "local_lite"
autonomy_level = 1

[models]
fast_tier = "ollama/qwen3-coder:8b"
balanced_tier = "anthropic/claude-sonnet-4-6"
architect_tier = "anthropic/claude-opus-4-6"

[governance]
require_approval_for_spend = true
require_approval_for_prod_push = true
```

Both `avos init` and `avos-daemon` read `avos.toml` (or `AVOS_CONFIG_PATH`), so your model routing and autonomy policy stay consistent across CLI, TUI, and loop runs.

<br />

# Running it locally

### 1. Install or build from source

```bash
# macOS / Linux (Homebrew)
brew install avos-sh/tap/avos

# Or via Cargo (Rust 1.85+, edition 2024)
cargo install avos-cli

# Or build the workspace directly
cargo build --release --workspace
```

### 2. Initialize a venture and start the daemon

```bash
avos init my-saas --template b2b-micro-saas
cd my-saas

# Terminal 1: start the local daemon
avos-daemon

# Terminal 2: verify health, dial autonomy, or open the TUI
avos health
avos autonomy 2
avos tui
```

### 3. Configure local Ollama or BYOK

```bash
# Option A: Local Ollama (zero cloud cost)
ollama pull qwen3-coder:8b
export OLLAMA_BASE_URL="http://localhost:11434"

# Option B: Cloud BYOK (Anthropic / OpenAI)
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-..."
```

See [docs/quickstart.md](docs/quickstart.md) for all CLI flags and environment variables (`AVOS_SOCKET_PATH`, `AVOS_CONFIG_PATH`, `AVOS_LICENSE_KEY`, `OPENAI_BASE_URL`).

<br />

# AVOS Lite vs. AVOS Cloud

`avos-lite` gives you a fast, self-hosted single-node CLI, TUI, and daemon. When you want managed cloud sandboxes, ambient OAuth MCP connectors, and multi-cloud failover, run `avos login` to connect to [AVOS Cloud](https://avos.sh):

| Capability | `avos-lite` (Self-Hosted Open-Core) | **AVOS Cloud (`app.avos.sh`)** |
| --- | --- | --- |
| **Core CLI (`avos` / `ac`) &amp; `avos.toml`** | Included | Included + multi-device cloud sync |
| **Model Routing** | Local Ollama + BYOK (Anthropic / OpenAI) | 4-stage multi-cloud failover + prompt caching |
| **Agent Workspaces** | Local terminal PTY &amp; `OSC 7501` panes | Cloud sandboxed VMs &amp; web control plane |
| **MCP Connectors** | Local configuration | Managed MCP gateway &amp; ambient OAuth |
| **Visual Surface** | Native Ratatui TUI | Web IDE, team cockpit &amp; approval inbox |

<br />

# Repository structure

```text
avos-lite/
├── Cargo.toml                  # Rust 2024 workspace (CLI, Daemon, License Verifier)
├── avos.toml.example           # Venture configuration template
├── crates/
│   ├── avos-cli/               # `avos` CLI, Droid TUI, and `ac` multiplexer
│   ├── avos-daemon/            # Local Unix socket daemon, config loader & loop engine
│   └── avos-license/           # Offline Ed25519 `license.avoskey` verifier
├── stdlib-lite/
│   ├── loops/                  # 80/20 YAML loops (`tdd-bug-fixer`, `icp-lead-scraper`)
│   └── gbnf/                   # GBNF grammar for structured tool calls
└── docs/
    └── quickstart.md           # Local CLI, Ollama & BYOK guide
```

<br />

# Security

- **Local socket isolation:** `avos-daemon` binds `/tmp/avos.sock` with `0600` permissions so only the local user can issue RPCs.
- **No silent spend or prod pushes:** High-risk loop steps require explicit `--approve` / `governance_approved: true`.
- **Offline cryptographic licensing:** `avos-license` verifies Ed25519 signatures locally without phoning home.

<br />

# License

AVOS Lite is open-core under the **AVOS Open-Core License (MIT + Self-Hosted Commercial Addendum)** — see [LICENSE](LICENSE) for details.

- `crates/avos-cli` and `stdlib-lite` are MIT-licensed for open-source use, modification, and distribution.
- Commercial multi-tenant SaaS hosting and proprietary AVOS Cloud features require an active [AVOS Cloud](https://avos.sh/pricing) subscription or commercial `license.avoskey`.

<br />

# Star us on GitHub

If AVOS Lite is useful for your workflows, scroll up and click **Star** on [LukaG712/avos-lite](https://github.com/LukaG712/avos-lite) so more builders can find it.
