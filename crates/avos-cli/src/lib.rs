pub mod tui;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VentureConfig {
    pub name: String,
    pub template: Option<String>,
    pub runtime_mode: String,
    pub autonomy_level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelsConfig {
    pub fast_tier: String,
    pub balanced_tier: String,
    pub architect_tier: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceConfig {
    pub require_approval_for_spend: bool,
    pub require_approval_for_prod_push: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvosConfig {
    pub venture: VentureConfig,
    pub models: ModelsConfig,
    pub governance: GovernanceConfig,
}

pub fn format_osc_7501(agent: &str, status: &str, detail: &str) -> String {
    format!("\x1b]7501;agent={agent};status={status};detail={detail}\x07")
}

pub fn render_avos_toml(name: &str, template: &str) -> String {
    format!(
        "[venture]\n\
         name = \"{name}\"\n\
         template = \"{template}\"\n\
         runtime_mode = \"local_lite\"\n\
         autonomy_level = 1\n\
         \n\
         [models]\n\
         fast_tier = \"ollama/qwen3-coder:8b\"\n\
         balanced_tier = \"anthropic/claude-sonnet-4-6\"\n\
         architect_tier = \"anthropic/claude-opus-4-6\"\n\
         \n\
         [governance]\n\
         require_approval_for_spend = true\n\
         require_approval_for_prod_push = true\n"
    )
}

pub fn parse_avos_toml(raw: &str) -> Result<AvosConfig, String> {
    let mut section = String::new();
    let mut venture_name = String::new();
    let mut venture_template: Option<String> = None;
    let mut runtime_mode = "local_lite".to_string();
    let mut autonomy_level: u8 = 1;
    let mut fast_tier = "ollama/qwen3-coder:8b".to_string();
    let mut balanced_tier = "anthropic/claude-sonnet-4-6".to_string();
    let mut architect_tier = "anthropic/claude-opus-4-6".to_string();
    let mut require_approval_for_spend = true;
    let mut require_approval_for_prod_push = true;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed[1..trimmed.len() - 1].trim().to_string();
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        let key = k.trim();
        let val = v.trim().trim_matches('"');
        match (section.as_str(), key) {
            ("venture", "name") => venture_name = val.to_string(),
            ("venture", "template") => venture_template = Some(val.to_string()),
            ("venture", "runtime_mode") => runtime_mode = val.to_string(),
            ("venture", "autonomy_level") => {
                autonomy_level = val
                    .parse::<u8>()
                    .map_err(|_| format!("Invalid autonomy_level: {val}"))?;
            }
            ("models", "fast_tier") => fast_tier = val.to_string(),
            ("models", "balanced_tier") => balanced_tier = val.to_string(),
            ("models", "architect_tier") => architect_tier = val.to_string(),
            ("governance", "require_approval_for_spend") => {
                require_approval_for_spend = val
                    .parse::<bool>()
                    .map_err(|_| format!("Invalid boolean for require_approval_for_spend: {val}"))?;
            }
            ("governance", "require_approval_for_prod_push") => {
                require_approval_for_prod_push = val.parse::<bool>().map_err(|_| {
                    format!("Invalid boolean for require_approval_for_prod_push: {val}")
                })?;
            }
            _ => {}
        }
    }

    if venture_name.is_empty() {
        return Err("avos.toml must specify [venture].name".to_string());
    }

    Ok(AvosConfig {
        venture: VentureConfig {
            name: venture_name,
            template: venture_template,
            runtime_mode,
            autonomy_level,
        },
        models: ModelsConfig {
            fast_tier,
            balanced_tier,
            architect_tier,
        },
        governance: GovernanceConfig {
            require_approval_for_spend,
            require_approval_for_prod_push,
        },
    })
}

pub fn init_workspace(dir: &Path, name: &str, template: &str) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    fs::write(dir.join("avos.toml"), render_avos_toml(name, template))
}

pub fn send_rpc(socket_path: &Path, method: &str, params: Value) -> Result<Value, String> {
    let mut stream = UnixStream::connect(socket_path)
        .map_err(|e| format!("Failed to connect to avos-daemon at {}: {e}", socket_path.display()))?;
    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });
    stream
        .write_all(format!("{req}\n").as_bytes())
        .and_then(|_| stream.flush())
        .map_err(|e| format!("Failed to write RPC request: {e}"))?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("Failed to read RPC response: {e}"))?;
    let parsed: Value = serde_json::from_str(line.trim())
        .map_err(|e| format!("Invalid JSON-RPC response: {e}"))?;
    if let Some(err) = parsed.get("error") {
        return Err(format!("Daemon error: {err}"));
    }
    Ok(parsed.get("result").cloned().unwrap_or(Value::Null))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_osc_7501_sequence() {
        let seq = format_osc_7501("qa-droid", "WORKING", "Running tests");
        assert!(seq.starts_with("\x1b]7501;"));
        assert!(seq.contains("agent=qa-droid"));
        assert!(seq.ends_with('\x07'));
    }

    #[test]
    fn scaffolds_and_parses_avos_toml() {
        let tmp = std::env::temp_dir().join(format!("avos_lite_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        init_workspace(&tmp, "my-saas", "b2b-micro-saas").unwrap();
        let raw = fs::read_to_string(tmp.join("avos.toml")).unwrap();
        let cfg = parse_avos_toml(&raw).unwrap();
        assert_eq!(cfg.venture.name, "my-saas");
        assert_eq!(cfg.venture.template.as_deref(), Some("b2b-micro-saas"));
        assert_eq!(cfg.venture.runtime_mode, "local_lite");
        assert_eq!(cfg.models.fast_tier, "ollama/qwen3-coder:8b");
        assert!(cfg.governance.require_approval_for_spend);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn parses_avos_toml_example() {
        let raw = include_str!("../../../avos.toml.example");
        let cfg = parse_avos_toml(raw).unwrap();
        assert_eq!(cfg.venture.name, "my-saas");
        assert_eq!(cfg.venture.autonomy_level, 1);
        assert!(cfg.governance.require_approval_for_prod_push);
    }
}
