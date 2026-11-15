pub mod tui;

use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

pub fn format_osc_7501(agent: &str, status: &str, detail: &str) -> String {
    format!("\x1b]7501;agent={agent};status={status};detail={detail}\x07")
}

pub fn render_avos_toml(name: &str, template: &str) -> String {
    format!(
        "[venture]\nname = \"{name}\"\ntemplate = \"{template}\"\nruntime_mode = \"local_lite\"\nautonomy_level = 1\n"
    )
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
    fn scaffolds_avos_toml() {
        let tmp = std::env::temp_dir().join(format!("avos_lite_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        init_workspace(&tmp, "my-saas", "b2b-micro-saas").unwrap();
        let raw = fs::read_to_string(tmp.join("avos.toml")).unwrap();
        assert!(raw.contains("runtime_mode = \"local_lite\""));
        assert!(raw.contains("name = \"my-saas\""));
        let _ = fs::remove_dir_all(&tmp);
    }
}
