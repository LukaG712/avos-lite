use crate::config::DaemonModelsConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepMode {
    Deterministic,
    Llm,
    GovernanceGate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopStep {
    pub id: String,
    pub mode: StepMode,
    pub cmd: Option<String>,
    pub script: Option<String>,
    pub tier: Option<String>,
    pub grammar: Option<String>,
    pub redact_pii: bool,
    pub rollback_on_fail: bool,
    pub risk_scope: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopDefinition {
    pub name: String,
    pub version: String,
    pub ratio: Option<String>,
    pub max_iterations: u32,
    pub max_leads: Option<u32>,
    pub steps: Vec<LoopStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopRunStepResult {
    pub step_id: String,
    pub mode: StepMode,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopRunReport {
    pub loop_name: String,
    pub status: String,
    pub results: Vec<LoopRunStepResult>,
}

fn clean_yaml_scalar(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix('"')
        && let Some(end) = rest.find('"')
    {
        return rest[..end].to_string();
    }
    if let Some(rest) = trimmed.strip_prefix('\'')
        && let Some(end) = rest.find('\'')
    {
        return rest[..end].to_string();
    }
    trimmed
        .split('#')
        .next()
        .unwrap_or(trimmed)
        .trim()
        .to_string()
}

/// Minimal zero-dependency parser for stdlib-lite YAML loop manifests.
pub fn parse_loop_yaml(raw: &str) -> Result<LoopDefinition, String> {
    let mut name = String::new();
    let mut version = "1.0.0".to_string();
    let mut ratio: Option<String> = None;
    let mut max_iterations: u32 = 1;
    let mut max_leads: Option<u32> = None;
    let mut steps: Vec<LoopStep> = Vec::new();
    let mut current: Option<LoopStep> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "steps:" {
            continue;
        }
        if let Some(val) = trimmed.strip_prefix("- id:") {
            if let Some(step) = current.take() {
                steps.push(step);
            }
            current = Some(LoopStep {
                id: clean_yaml_scalar(val),
                mode: StepMode::Deterministic,
                cmd: None,
                script: None,
                tier: None,
                grammar: None,
                redact_pii: false,
                rollback_on_fail: false,
                risk_scope: None,
            });
        } else if let Some(step) = current.as_mut() {
            if let Some(val) = trimmed.strip_prefix("mode:") {
                let mode_str = clean_yaml_scalar(val);
                step.mode = match mode_str.as_str() {
                    "deterministic" => StepMode::Deterministic,
                    "llm" => StepMode::Llm,
                    "governance_gate" => StepMode::GovernanceGate,
                    other => return Err(format!("Unsupported step mode: {other}")),
                };
            } else if let Some(val) = trimmed.strip_prefix("cmd:") {
                step.cmd = Some(clean_yaml_scalar(val));
            } else if let Some(val) = trimmed.strip_prefix("script:") {
                step.script = Some(clean_yaml_scalar(val));
            } else if let Some(val) = trimmed.strip_prefix("tier:") {
                step.tier = Some(clean_yaml_scalar(val));
            } else if let Some(val) = trimmed.strip_prefix("grammar:") {
                step.grammar = Some(clean_yaml_scalar(val));
            } else if let Some(val) = trimmed.strip_prefix("redact_pii:") {
                step.redact_pii = clean_yaml_scalar(val).parse::<bool>().unwrap_or(false);
            } else if let Some(val) = trimmed.strip_prefix("rollback_on_fail:") {
                step.rollback_on_fail = clean_yaml_scalar(val).parse::<bool>().unwrap_or(false);
            } else if let Some(val) = trimmed.strip_prefix("risk_scope:") {
                step.risk_scope = Some(clean_yaml_scalar(val));
            }
        } else if let Some(val) = trimmed.strip_prefix("name:") {
            name = clean_yaml_scalar(val);
        } else if let Some(val) = trimmed.strip_prefix("version:") {
            version = clean_yaml_scalar(val);
        } else if let Some(val) = trimmed.strip_prefix("ratio:") {
            ratio = Some(clean_yaml_scalar(val));
        } else if let Some(val) = trimmed.strip_prefix("max_iterations:") {
            if let Ok(n) = clean_yaml_scalar(val).parse::<u32>()
                && n >= 1
            {
                max_iterations = n;
            }
        } else if let Some(val) = trimmed.strip_prefix("max_leads:")
            && let Ok(n) = clean_yaml_scalar(val).parse::<u32>()
        {
            max_leads = Some(n);
        }
    }
    if let Some(step) = current.take() {
        steps.push(step);
    }
    if name.is_empty() || steps.is_empty() {
        return Err("Loop YAML must define 'name' and at least one step".into());
    }
    Ok(LoopDefinition {
        name,
        version,
        ratio,
        max_iterations,
        max_leads,
        steps,
    })
}

pub fn load_loop_from_file(path: &Path) -> Result<LoopDefinition, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read loop file {}: {e}", path.display()))?;
    parse_loop_yaml(&raw)
}

#[derive(Debug, Clone, Default)]
pub struct LoopExecOptions {
    pub governance_approved: bool,
    pub execute_commands: bool,
    pub llm_endpoint: Option<String>,
    pub llm_model: Option<String>,
    pub models_config: Option<DaemonModelsConfig>,
}

fn redact_basic_pii(input: &str) -> String {
    input
        .split_whitespace()
        .map(|tok| {
            if tok.contains('@') && tok.contains('.') {
                "[REDACTED_EMAIL]"
            } else {
                tok
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn resolve_tier_model(tier: Option<&str>, opts: &LoopExecOptions) -> String {
    if let Some(explicit) = opts.llm_model.as_deref()
        && !explicit.is_empty()
    {
        return explicit.to_string();
    }
    let models = opts.models_config.clone().unwrap_or_default();
    match tier.unwrap_or("balanced_1x") {
        "fast" | "fast_02x" | "fast_0_2x" | "fast_tier" => models.fast_tier,
        "architect" | "architect_tier" => models.architect_tier,
        _ => models.balanced_tier,
    }
}

fn run_curl_json(
    url: &str,
    headers: &[(&str, &str)],
    body: &serde_json::Value,
) -> Result<String, String> {
    let mut cmd = std::process::Command::new("curl");
    cmd.args([
        "-sS",
        "-f",
        "-X",
        "POST",
        "-H",
        "Content-Type: application/json",
    ]);
    for (k, v) in headers {
        cmd.arg("-H").arg(format!("{k}: {v}"));
    }
    cmd.arg("-d").arg(body.to_string()).arg(url);

    let out = cmd
        .output()
        .map_err(|e| format!("Failed to invoke curl: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            "LLM HTTP error".into()
        } else {
            err
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn invoke_llm_step(
    step: &LoopStep,
    prior_context: &str,
    opts: &LoopExecOptions,
) -> Result<Option<String>, String> {
    let resolved_model = resolve_tier_model(step.tier.as_deref(), opts);
    let mut prompt = if prior_context.is_empty() {
        format!(
            "Execute loop step '{}' (tier={})",
            step.id,
            step.tier.as_deref().unwrap_or("balanced_1x")
        )
    } else {
        format!(
            "Execute loop step '{}' (tier={}).\nPrevious step output:\n{}",
            step.id,
            step.tier.as_deref().unwrap_or("balanced_1x"),
            prior_context
        )
    };
    if step.redact_pii {
        prompt = redact_basic_pii(&prompt);
    }

    // 1. Explicit endpoint (e.g. Ollama / local HTTP)
    if let Some(base_url) = opts.llm_endpoint.as_deref() {
        let model_name = resolved_model
            .strip_prefix("ollama/")
            .unwrap_or(&resolved_model);
        let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
        let mut payload = serde_json::json!({
            "model": model_name,
            "prompt": prompt,
            "stream": false,
        });
        if step.grammar.is_some() {
            payload["format"] = serde_json::json!("json");
        }
        let raw = run_curl_json(&url, &[], &payload)?;
        let parsed = serde_json::from_str::<serde_json::Value>(&raw)
            .ok()
            .and_then(|v| {
                v.get("response")
                    .and_then(|r| r.as_str())
                    .map(str::to_string)
            })
            .unwrap_or(raw);
        return Ok(Some(if step.redact_pii {
            redact_basic_pii(&parsed)
        } else {
            parsed
        }));
    }

    // 2. Anthropic Messages API when model is anthropic/* and ANTHROPIC_API_KEY is set
    if let Some(model_name) = resolved_model.strip_prefix("anthropic/")
        && let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY")
        && !api_key.is_empty()
    {
        let payload = serde_json::json!({
            "model": model_name,
            "max_tokens": 1024,
            "messages": [{ "role": "user", "content": prompt }],
        });
        let raw = run_curl_json(
            "https://api.anthropic.com/v1/messages",
            &[("x-api-key", &api_key), ("anthropic-version", "2023-06-01")],
            &payload,
        )?;
        let parsed = serde_json::from_str::<serde_json::Value>(&raw)
            .ok()
            .and_then(|v| {
                v.get("content")
                    .and_then(|c| c.get(0))
                    .and_then(|b| b.get("text"))
                    .and_then(|t| t.as_str())
                    .map(str::to_string)
            })
            .unwrap_or(raw);
        return Ok(Some(if step.redact_pii {
            redact_basic_pii(&parsed)
        } else {
            parsed
        }));
    }

    // 3. OpenAI Chat Completions API when OPENAI_API_KEY is set
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY")
        && !api_key.is_empty()
    {
        let model_name = resolved_model
            .strip_prefix("openai/")
            .unwrap_or(&resolved_model);
        let base = std::env::var("OPENAI_BASE_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let url = format!("{}/chat/completions", base.trim_end_matches('/'));
        let auth = format!("Bearer {api_key}");
        let payload = serde_json::json!({
            "model": model_name,
            "messages": [{ "role": "user", "content": prompt }],
        });
        let raw = run_curl_json(&url, &[("Authorization", &auth)], &payload)?;
        let parsed = serde_json::from_str::<serde_json::Value>(&raw)
            .ok()
            .and_then(|v| {
                v.get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c0| c0.get("message"))
                    .and_then(|m| m.get("content"))
                    .and_then(|t| t.as_str())
                    .map(str::to_string)
            })
            .unwrap_or(raw);
        return Ok(Some(if step.redact_pii {
            redact_basic_pii(&parsed)
        } else {
            parsed
        }));
    }

    Ok(None)
}

/// Evaluates a loop plan, pausing on `governance_gate` unless `governance_approved` is true.
pub fn simulate_loop_execution(def: &LoopDefinition, governance_approved: bool) -> LoopRunReport {
    execute_loop(def, governance_approved, false)
}

/// Executes or dry-runs a loop plan. When `execute_commands` is true, deterministic steps
/// with `cmd` are executed via `sh -c` and non-zero exits halt the loop with status `FAILED`
/// (or retry up to `def.max_iterations` when `rollback_on_fail` is true).
pub fn execute_loop(
    def: &LoopDefinition,
    governance_approved: bool,
    execute_commands: bool,
) -> LoopRunReport {
    execute_loop_with_options(
        def,
        &LoopExecOptions {
            governance_approved,
            execute_commands,
            llm_endpoint: None,
            llm_model: None,
            models_config: None,
        },
    )
}

pub fn execute_loop_with_options(def: &LoopDefinition, opts: &LoopExecOptions) -> LoopRunReport {
    let governance_approved = opts.governance_approved;
    let execute_commands = opts.execute_commands;
    let max_attempts = if execute_commands && def.steps.iter().any(|s| s.rollback_on_fail) {
        def.max_iterations.max(1)
    } else {
        1
    };

    let mut all_results = Vec::new();

    for attempt in 1..=max_attempts {
        let mut last_context = String::new();
        let mut should_retry = false;

        for step in &def.steps {
            match step.mode {
                StepMode::Deterministic => {
                    if execute_commands && let Some(cmd) = step.cmd.as_deref() {
                        match std::process::Command::new("sh").arg("-c").arg(cmd).output() {
                            Ok(out) => {
                                let stdout =
                                    String::from_utf8_lossy(&out.stdout).trim().to_string();
                                let stderr =
                                    String::from_utf8_lossy(&out.stderr).trim().to_string();
                                let detail = if !stdout.is_empty() {
                                    stdout
                                } else if !stderr.is_empty() {
                                    stderr
                                } else {
                                    format!("exit={}", out.status.code().unwrap_or(-1))
                                };
                                last_context = detail.clone();
                                if !out.status.success() {
                                    if step.rollback_on_fail && attempt < max_attempts {
                                        all_results.push(LoopRunStepResult {
                                            step_id: step.id.clone(),
                                            mode: step.mode.clone(),
                                            status: "ROLLBACK_RETRY".into(),
                                            detail: format!(
                                                "attempt {attempt}/{max_attempts}: {detail}"
                                            ),
                                        });
                                        should_retry = true;
                                        break;
                                    }
                                    all_results.push(LoopRunStepResult {
                                        step_id: step.id.clone(),
                                        mode: step.mode.clone(),
                                        status: "FAILED".into(),
                                        detail,
                                    });
                                    return LoopRunReport {
                                        loop_name: def.name.clone(),
                                        status: "FAILED".into(),
                                        results: all_results,
                                    };
                                }
                                all_results.push(LoopRunStepResult {
                                    step_id: step.id.clone(),
                                    mode: step.mode.clone(),
                                    status: "EXECUTED".into(),
                                    detail,
                                });
                            }
                            Err(e) => {
                                all_results.push(LoopRunStepResult {
                                    step_id: step.id.clone(),
                                    mode: step.mode.clone(),
                                    status: "FAILED".into(),
                                    detail: format!("Failed to spawn command: {e}"),
                                });
                                return LoopRunReport {
                                    loop_name: def.name.clone(),
                                    status: "FAILED".into(),
                                    results: all_results,
                                };
                            }
                        }
                    } else {
                        let detail = step
                            .cmd
                            .clone()
                            .or_else(|| step.script.clone())
                            .unwrap_or_else(|| "deterministic step ok".into());
                        last_context = detail.clone();
                        all_results.push(LoopRunStepResult {
                            step_id: step.id.clone(),
                            mode: step.mode.clone(),
                            status: "EXECUTED".into(),
                            detail,
                        });
                    }
                }
                StepMode::Llm => match invoke_llm_step(step, &last_context, opts) {
                    Ok(Some(detail)) => {
                        last_context = detail.clone();
                        all_results.push(LoopRunStepResult {
                            step_id: step.id.clone(),
                            mode: step.mode.clone(),
                            status: "EXECUTED".into(),
                            detail,
                        });
                    }
                    Ok(None) => {
                        let detail = format!(
                            "tier={} (model={})",
                            step.tier.as_deref().unwrap_or("balanced_1x"),
                            resolve_tier_model(step.tier.as_deref(), opts)
                        );
                        all_results.push(LoopRunStepResult {
                            step_id: step.id.clone(),
                            mode: step.mode.clone(),
                            status: "EXECUTED".into(),
                            detail,
                        });
                    }
                    Err(err) => {
                        all_results.push(LoopRunStepResult {
                            step_id: step.id.clone(),
                            mode: step.mode.clone(),
                            status: "FAILED".into(),
                            detail: err,
                        });
                        return LoopRunReport {
                            loop_name: def.name.clone(),
                            status: "FAILED".into(),
                            results: all_results,
                        };
                    }
                },
                StepMode::GovernanceGate => {
                    if !governance_approved {
                        all_results.push(LoopRunStepResult {
                            step_id: step.id.clone(),
                            mode: step.mode.clone(),
                            status: "PAUSED_GOVERNANCE".into(),
                            detail: step
                                .risk_scope
                                .clone()
                                .unwrap_or_else(|| "WRITE_PRODUCTION_OR_SPEND".into()),
                        });
                        return LoopRunReport {
                            loop_name: def.name.clone(),
                            status: "PAUSED_GOVERNANCE".into(),
                            results: all_results,
                        };
                    }
                    all_results.push(LoopRunStepResult {
                        step_id: step.id.clone(),
                        mode: step.mode.clone(),
                        status: "APPROVED_AND_EXECUTED".into(),
                        detail: step
                            .risk_scope
                            .clone()
                            .unwrap_or_else(|| "WRITE_PRODUCTION_OR_SPEND".into()),
                    });
                }
            }
        }

        if !should_retry {
            break;
        }
    }

    LoopRunReport {
        loop_name: def.name.clone(),
        status: "COMPLETED".into(),
        results: all_results,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn parses_tdd_bug_fixer_yaml() {
        let raw = include_str!("../../../stdlib-lite/loops/tdd-bug-fixer.yaml");
        let parsed = parse_loop_yaml(raw).unwrap();
        assert_eq!(parsed.name, "loop/tdd-bug-fixer");
        assert_eq!(parsed.max_iterations, 4);
        assert_eq!(parsed.steps.len(), 3);
        assert_eq!(parsed.steps[0].mode, StepMode::Deterministic);
        assert_eq!(parsed.steps[1].mode, StepMode::Llm);
        assert_eq!(
            parsed.steps[1].grammar.as_deref(),
            Some("../gbnf/tool_call.gbnf")
        );
        assert!(parsed.steps[2].rollback_on_fail);

        let report = simulate_loop_execution(&parsed, false);
        assert_eq!(report.status, "COMPLETED");
        assert_eq!(report.results.len(), 3);
    }

    #[test]
    fn pauses_icp_lead_scraper_on_governance_gate() {
        let raw = include_str!("../../../stdlib-lite/loops/icp-lead-scraper.yaml");
        let parsed = parse_loop_yaml(raw).unwrap();
        assert_eq!(parsed.max_leads, Some(25));
        assert!(parsed.steps[0].redact_pii);
        assert_eq!(
            parsed.steps[1].script.as_deref(),
            Some("dedupe_domain_list")
        );

        let paused = simulate_loop_execution(&parsed, false);
        assert_eq!(paused.status, "PAUSED_GOVERNANCE");

        let resumed = simulate_loop_execution(&parsed, true);
        assert_eq!(resumed.status, "COMPLETED");
    }

    #[test]
    fn executes_real_deterministic_commands_when_enabled() {
        let yaml = r#"
name: loop/echo-test
version: 1.0.0
steps:
  - id: step_ok
    mode: deterministic
    cmd: "echo hello-avos-lite"
  - id: step_fail
    mode: deterministic
    cmd: "exit 1"
"#;
        let parsed = parse_loop_yaml(yaml).unwrap();
        let report = execute_loop(&parsed, false, true);
        assert_eq!(report.status, "FAILED");
        assert_eq!(report.results.len(), 2);
        assert_eq!(report.results[0].detail, "hello-avos-lite");
        assert_eq!(report.results[1].status, "FAILED");
    }

    #[test]
    fn calls_local_ollama_endpoint_when_configured() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let body = r#"{"response":"patched bug via ollama"}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        });

        let yaml = r#"
name: loop/ollama-test
version: 1.0.0
steps:
  - id: generate_patch
    mode: llm
    tier: fast_0_2x
"#;
        let parsed = parse_loop_yaml(yaml).unwrap();
        let report = execute_loop_with_options(
            &parsed,
            &LoopExecOptions {
                governance_approved: false,
                execute_commands: false,
                llm_endpoint: Some(format!("http://{addr}")),
                llm_model: Some("qwen3-coder:8b".into()),
                models_config: None,
            },
        );
        handle.join().unwrap();
        assert_eq!(report.status, "COMPLETED");
        assert_eq!(report.results[0].detail, "patched bug via ollama");
    }

    #[test]
    fn retries_up_to_max_iterations_on_rollback_on_fail() {
        let yaml = r#"
name: loop/retry-test
version: 1.0.0
max_iterations: 2
steps:
  - id: verify
    mode: deterministic
    cmd: "exit 1"
    rollback_on_fail: true
"#;
        let parsed = parse_loop_yaml(yaml).unwrap();
        let report = execute_loop(&parsed, false, true);
        assert_eq!(report.status, "FAILED");
        assert_eq!(report.results.len(), 2);
        assert_eq!(report.results[0].status, "ROLLBACK_RETRY");
        assert_eq!(report.results[1].status, "FAILED");
    }
}
