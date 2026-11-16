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
    pub tier: Option<String>,
    pub risk_scope: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopDefinition {
    pub name: String,
    pub version: String,
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

/// Minimal zero-dependency parser for stdlib-lite YAML loop manifests.
pub fn parse_loop_yaml(raw: &str) -> Result<LoopDefinition, String> {
    let mut name = String::new();
    let mut version = "1.0.0".to_string();
    let mut steps: Vec<LoopStep> = Vec::new();
    let mut current: Option<LoopStep> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "steps:" {
            continue;
        }
        if let Some(val) = trimmed.strip_prefix("name:") {
            name = val.trim().trim_matches('"').to_string();
        } else if let Some(val) = trimmed.strip_prefix("version:") {
            version = val.trim().trim_matches('"').to_string();
        } else if let Some(val) = trimmed.strip_prefix("- id:") {
            if let Some(step) = current.take() {
                steps.push(step);
            }
            current = Some(LoopStep {
                id: val.trim().trim_matches('"').to_string(),
                mode: StepMode::Deterministic,
                cmd: None,
                tier: None,
                risk_scope: None,
            });
        } else if let Some(step) = current.as_mut() {
            if let Some(val) = trimmed.strip_prefix("mode:") {
                step.mode = match val.trim().trim_matches('"') {
                    "deterministic" => StepMode::Deterministic,
                    "llm" => StepMode::Llm,
                    "governance_gate" => StepMode::GovernanceGate,
                    other => return Err(format!("Unsupported step mode: {other}")),
                };
            } else if let Some(val) = trimmed.strip_prefix("cmd:") {
                step.cmd = Some(val.trim().trim_matches('"').to_string());
            } else if let Some(val) = trimmed.strip_prefix("tier:") {
                step.tier = Some(val.trim().trim_matches('"').to_string());
            } else if let Some(val) = trimmed.strip_prefix("risk_scope:") {
                step.risk_scope = Some(val.trim().trim_matches('"').to_string());
            }
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
}

/// Evaluates a loop plan, pausing on `governance_gate` unless `governance_approved` is true.
pub fn simulate_loop_execution(def: &LoopDefinition, governance_approved: bool) -> LoopRunReport {
    execute_loop(def, governance_approved, false)
}

/// Executes or dry-runs a loop plan. When `execute_commands` is true, deterministic steps
/// with `cmd` are executed via `sh -c` and non-zero exits halt the loop with status `FAILED`.
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
        },
    )
}

pub fn execute_loop_with_options(def: &LoopDefinition, opts: &LoopExecOptions) -> LoopRunReport {
    let governance_approved = opts.governance_approved;
    let execute_commands = opts.execute_commands;
    let mut results = Vec::new();
    for step in &def.steps {
        match step.mode {
            StepMode::Deterministic => {
                if execute_commands && let Some(cmd) = step.cmd.as_deref() {
                    match std::process::Command::new("sh").arg("-c").arg(cmd).output() {
                        Ok(out) => {
                            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                            let detail = if !stdout.is_empty() {
                                stdout
                            } else if !stderr.is_empty() {
                                stderr
                            } else {
                                format!("exit={}", out.status.code().unwrap_or(-1))
                            };
                            if !out.status.success() {
                                results.push(LoopRunStepResult {
                                    step_id: step.id.clone(),
                                    mode: step.mode.clone(),
                                    status: "FAILED".into(),
                                    detail,
                                });
                                return LoopRunReport {
                                    loop_name: def.name.clone(),
                                    status: "FAILED".into(),
                                    results,
                                };
                            }
                            results.push(LoopRunStepResult {
                                step_id: step.id.clone(),
                                mode: step.mode.clone(),
                                status: "EXECUTED".into(),
                                detail,
                            });
                        }
                        Err(e) => {
                            results.push(LoopRunStepResult {
                                step_id: step.id.clone(),
                                mode: step.mode.clone(),
                                status: "FAILED".into(),
                                detail: format!("Failed to spawn command: {e}"),
                            });
                            return LoopRunReport {
                                loop_name: def.name.clone(),
                                status: "FAILED".into(),
                                results,
                            };
                        }
                    }
                } else {
                    results.push(LoopRunStepResult {
                        step_id: step.id.clone(),
                        mode: step.mode.clone(),
                        status: "EXECUTED".into(),
                        detail: step
                            .cmd
                            .clone()
                            .unwrap_or_else(|| "deterministic step ok".into()),
                    });
                }
            }
            StepMode::Llm => {
                if let Some(base_url) = opts.llm_endpoint.as_deref() {
                    let model = opts
                        .llm_model
                        .as_deref()
                        .unwrap_or("qwen3-coder:8b");
                    let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
                    let payload = serde_json::json!({
                        "model": model,
                        "prompt": format!("Execute loop step '{}' (tier={})", step.id, step.tier.as_deref().unwrap_or("balanced_1x")),
                        "stream": false,
                    });
                    match std::process::Command::new("curl")
                        .args(["-sS", "-f", "-X", "POST", "-H", "Content-Type: application/json", "-d", &payload.to_string(), &url])
                        .output()
                    {
                        Ok(out) if out.status.success() => {
                            let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
                            let detail = serde_json::from_str::<serde_json::Value>(&raw)
                                .ok()
                                .and_then(|v| v.get("response").and_then(|r| r.as_str()).map(str::to_string))
                                .unwrap_or(raw);
                            results.push(LoopRunStepResult {
                                step_id: step.id.clone(),
                                mode: step.mode.clone(),
                                status: "EXECUTED".into(),
                                detail,
                            });
                        }
                        Ok(out) => {
                            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                            results.push(LoopRunStepResult {
                                step_id: step.id.clone(),
                                mode: step.mode.clone(),
                                status: "FAILED".into(),
                                detail: if err.is_empty() { "LLM HTTP error".into() } else { err },
                            });
                            return LoopRunReport {
                                loop_name: def.name.clone(),
                                status: "FAILED".into(),
                                results,
                            };
                        }
                        Err(e) => {
                            results.push(LoopRunStepResult {
                                step_id: step.id.clone(),
                                mode: step.mode.clone(),
                                status: "FAILED".into(),
                                detail: format!("Failed to invoke curl: {e}"),
                            });
                            return LoopRunReport {
                                loop_name: def.name.clone(),
                                status: "FAILED".into(),
                                results,
                            };
                        }
                    }
                } else {
                    results.push(LoopRunStepResult {
                        step_id: step.id.clone(),
                        mode: step.mode.clone(),
                        status: "EXECUTED".into(),
                        detail: format!(
                            "tier={}",
                            step.tier.as_deref().unwrap_or("balanced_1x")
                        ),
                    });
                }
            }
            StepMode::GovernanceGate => {
                if !governance_approved {
                    results.push(LoopRunStepResult {
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
                        results,
                    };
                }
                results.push(LoopRunStepResult {
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
    LoopRunReport {
        loop_name: def.name.clone(),
        status: "COMPLETED".into(),
        results,
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
        assert_eq!(parsed.steps.len(), 3);
        assert_eq!(parsed.steps[0].mode, StepMode::Deterministic);
        assert_eq!(parsed.steps[1].mode, StepMode::Llm);

        let report = simulate_loop_execution(&parsed, false);
        assert_eq!(report.status, "COMPLETED");
        assert_eq!(report.results.len(), 3);
    }

    #[test]
    fn pauses_icp_lead_scraper_on_governance_gate() {
        let raw = include_str!("../../../stdlib-lite/loops/icp-lead-scraper.yaml");
        let parsed = parse_loop_yaml(raw).unwrap();
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
            },
        );
        handle.join().unwrap();
        assert_eq!(report.status, "COMPLETED");
        assert_eq!(report.results[0].detail, "patched bug via ollama");
    }
}
