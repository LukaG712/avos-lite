use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonModelsConfig {
    pub fast_tier: String,
    pub balanced_tier: String,
    pub architect_tier: String,
}

impl Default for DaemonModelsConfig {
    fn default() -> Self {
        Self {
            fast_tier: "ollama/qwen3-coder:8b".to_string(),
            balanced_tier: "anthropic/claude-sonnet-4-6".to_string(),
            architect_tier: "anthropic/claude-opus-4-6".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonGovernanceConfig {
    pub require_approval_for_spend: bool,
    pub require_approval_for_prod_push: bool,
}

impl Default for DaemonGovernanceConfig {
    fn default() -> Self {
        Self {
            require_approval_for_spend: true,
            require_approval_for_prod_push: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub venture_name: String,
    pub runtime_mode: String,
    pub autonomy_level: u8,
    pub models: DaemonModelsConfig,
    pub governance: DaemonGovernanceConfig,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            venture_name: "default".to_string(),
            runtime_mode: "local_lite".to_string(),
            autonomy_level: 1,
            models: DaemonModelsConfig::default(),
            governance: DaemonGovernanceConfig::default(),
        }
    }
}

fn strip_toml_value(raw: &str) -> String {
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

pub fn parse_daemon_toml(raw: &str) -> DaemonConfig {
    let mut cfg = DaemonConfig::default();
    let mut section = String::new();

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
        let val = strip_toml_value(v);
        match (section.as_str(), key) {
            ("venture", "name") if !val.is_empty() => cfg.venture_name = val,
            ("venture", "runtime_mode") if !val.is_empty() => cfg.runtime_mode = val,
            ("venture", "autonomy_level") => {
                if let Ok(lvl) = val.parse::<u8>()
                    && lvl <= 4
                {
                    cfg.autonomy_level = lvl;
                }
            }
            ("models", "fast_tier") if !val.is_empty() => cfg.models.fast_tier = val,
            ("models", "balanced_tier") if !val.is_empty() => cfg.models.balanced_tier = val,
            ("models", "architect_tier") if !val.is_empty() => cfg.models.architect_tier = val,
            ("governance", "require_approval_for_spend") => {
                if let Ok(b) = val.parse::<bool>() {
                    cfg.governance.require_approval_for_spend = b;
                }
            }
            ("governance", "require_approval_for_prod_push") => {
                if let Ok(b) = val.parse::<bool>() {
                    cfg.governance.require_approval_for_prod_push = b;
                }
            }
            _ => {}
        }
    }

    cfg
}

pub fn load_daemon_config(path: &Path) -> DaemonConfig {
    fs::read_to_string(path)
        .map(|raw| parse_daemon_toml(&raw))
        .unwrap_or_default()
}
