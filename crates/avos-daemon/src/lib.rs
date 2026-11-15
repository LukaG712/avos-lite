use avos_license::LicenseEdition;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteMission {
    pub id: String,
    pub prompt: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPane {
    pub name: String,
    pub status: String,
    pub inbox: Vec<String>,
}

pub struct LiteDaemonState {
    pub runtime_mode: String,
    pub license_edition: LicenseEdition,
    pub autonomy_level: u8,
    pub missions: Vec<LiteMission>,
    pub panes: HashMap<String, AgentPane>,
}

impl LiteDaemonState {
    pub fn new(license_edition: LicenseEdition) -> Self {
        Self {
            runtime_mode: "local_lite".to_string(),
            license_edition,
            autonomy_level: 1,
            missions: Vec::new(),
            panes: HashMap::new(),
        }
    }
}

fn ok_response(id: Value, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(result),
        error: None,
    }
}

fn err_response(id: Value, code: i32, message: impl Into<String>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.into(),
        }),
    }
}

pub fn handle_rpc_line(state: &Arc<Mutex<LiteDaemonState>>, line: &str) -> String {
    let req: JsonRpcRequest = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => {
            return serde_json::to_string(&err_response(
                Value::Null,
                -32700,
                format!("Parse error: {e}"),
            ))
            .unwrap();
        }
    };

    let id = req.id.clone().unwrap_or(Value::Null);
    if req.jsonrpc != "2.0" {
        return serde_json::to_string(&err_response(id, -32600, "Invalid JSON-RPC version"))
            .unwrap();
    }

    let resp = match req.method.as_str() {
        "system.health" => {
            let guard = state.lock().unwrap();
            ok_response(
                id,
                json!({
                    "status": "ok",
                    "runtime_mode": guard.runtime_mode,
                    "license_edition": guard.license_edition,
                    "autonomy_level": guard.autonomy_level,
                }),
            )
        }
        "autonomy.get" => {
            let guard = state.lock().unwrap();
            ok_response(id, json!({ "level": guard.autonomy_level }))
        }
        "autonomy.set" => {
            let Some(level) = req.params.get("level").and_then(|v| v.as_u64()) else {
                return serde_json::to_string(&err_response(id, -32602, "Missing params.level"))
                    .unwrap();
            };
            if level > 4 {
                return serde_json::to_string(&err_response(
                    id,
                    -32602,
                    "Autonomy level must be between 0 and 4",
                ))
                .unwrap();
            }
            let mut guard = state.lock().unwrap();
            guard.autonomy_level = level as u8;
            ok_response(id, json!({ "level": guard.autonomy_level }))
        }
        "mission.create" => {
            let Some(prompt) = req.params.get("prompt").and_then(|v| v.as_str()) else {
                return serde_json::to_string(&err_response(id, -32602, "Missing params.prompt"))
                    .unwrap();
            };
            let mut guard = state.lock().unwrap();
            let rec = LiteMission {
                id: format!("lite_mis_{}", guard.missions.len() + 1),
                prompt: prompt.to_string(),
                status: "RUNNING".into(),
            };
            guard.missions.push(rec.clone());
            ok_response(id, serde_json::to_value(rec).unwrap())
        }
        "mission.list" => {
            let guard = state.lock().unwrap();
            ok_response(id, json!({ "items": guard.missions }))
        }
        "mission.get" => {
            let Some(mission_id) = req.params.get("mission_id").and_then(|v| v.as_str()) else {
                return serde_json::to_string(&err_response(
                    id,
                    -32602,
                    "Missing params.mission_id",
                ))
                .unwrap();
            };
            let guard = state.lock().unwrap();
            match guard.missions.iter().find(|m| m.id == mission_id) {
                Some(m) => ok_response(id, serde_json::to_value(m).unwrap()),
                None => err_response(id, -32004, format!("Mission not found: {mission_id}")),
            }
        }
        "pane.register" => {
            let Some(name) = req.params.get("name").and_then(|v| v.as_str()) else {
                return serde_json::to_string(&err_response(id, -32602, "Missing params.name"))
                    .unwrap();
            };
            let status = req
                .params
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("WORKING")
                .to_string();
            let mut guard = state.lock().unwrap();
            let pane = guard.panes.entry(name.to_string()).or_insert_with(|| AgentPane {
                name: name.to_string(),
                status: status.clone(),
                inbox: Vec::new(),
            });
            pane.status = status;
            let cloned = pane.clone();
            ok_response(id, serde_json::to_value(cloned).unwrap())
        }
        "pane.send" => {
            let Some(target) = req.params.get("target").and_then(|v| v.as_str()) else {
                return serde_json::to_string(&err_response(id, -32602, "Missing params.target"))
                    .unwrap();
            };
            let Some(message) = req.params.get("message").and_then(|v| v.as_str()) else {
                return serde_json::to_string(&err_response(id, -32602, "Missing params.message"))
                    .unwrap();
            };
            let mut guard = state.lock().unwrap();
            let pane = guard.panes.entry(target.to_string()).or_insert_with(|| AgentPane {
                name: target.to_string(),
                status: "WORKING".to_string(),
                inbox: Vec::new(),
            });
            pane.inbox.push(message.to_string());
            let cloned = pane.clone();
            ok_response(id, serde_json::to_value(cloned).unwrap())
        }
        "pane.list" => {
            let guard = state.lock().unwrap();
            let items: Vec<&AgentPane> = guard.panes.values().collect();
            ok_response(id, json!({ "items": items }))
        }
        other => err_response(id, -32601, format!("Method not found: {other}")),
    };

    serde_json::to_string(&resp).unwrap()
}

fn handle_client(stream: UnixStream, state: Arc<Mutex<LiteDaemonState>>) {
    let Ok(writer_stream) = stream.try_clone() else {
        return;
    };
    let reader = BufReader::new(stream);
    let mut writer = writer_stream;

    for line in reader.lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }
        let response = handle_rpc_line(&state, &line);
        if writer.write_all(response.as_bytes()).is_err()
            || writer.write_all(b"\n").is_err()
            || writer.flush().is_err()
        {
            break;
        }
    }
}

pub fn serve_unix_socket(socket_path: &Path, state: Arc<Mutex<LiteDaemonState>>) -> std::io::Result<()> {
    if socket_path.exists() {
        let _ = fs::remove_file(socket_path);
    }
    let listener = UnixListener::bind(socket_path)?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state_clone = Arc::clone(&state);
                thread::spawn(move || handle_client(stream, state_clone));
            }
            Err(e) => eprintln!("avos-daemon accept error: {e}"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_mission_and_panes_work() {
        let state = Arc::new(Mutex::new(LiteDaemonState::new(
            LicenseEdition::CommunityFree,
        )));

        let health: Value = serde_json::from_str(&handle_rpc_line(
            &state,
            r#"{"jsonrpc":"2.0","id":1,"method":"system.health","params":{}}"#,
        ))
        .unwrap();
        assert_eq!(health["result"]["runtime_mode"], "local_lite");

        let created: Value = serde_json::from_str(&handle_rpc_line(
            &state,
            r#"{"jsonrpc":"2.0","id":2,"method":"mission.create","params":{"prompt":"Fix bug"}}"#,
        ))
        .unwrap();
        assert_eq!(created["result"]["id"], "lite_mis_1");

        let sent: Value = serde_json::from_str(&handle_rpc_line(
            &state,
            r#"{"jsonrpc":"2.0","id":3,"method":"pane.send","params":{"target":"qa-droid","message":"tests passed"}}"#,
        ))
        .unwrap();
        assert_eq!(sent["result"]["inbox"][0], "tests passed");
    }
}
