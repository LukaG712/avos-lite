use avos_cli::{init_workspace, send_rpc, tui};
use serde_json::json;
use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = env::args().collect();
    let socket_path = PathBuf::from(
        env::var("AVOS_SOCKET_PATH").unwrap_or_else(|_| "/tmp/avos.sock".to_string()),
    );

    if args.len() == 1 || args.get(1).map(String::as_str) == Some("tui") {
        if let Err(err) = tui::run_tui(&socket_path) {
            eprintln!("TUI error: {err}");
            std::process::exit(1);
        }
        return;
    }

    let res = match args.get(1).map(String::as_str) {
        Some("init") => {
            let name = args.get(2).cloned().unwrap_or_else(|| "my-venture".to_string());
            let template = args
                .iter()
                .position(|a| a == "--template")
                .and_then(|i| args.get(i + 1))
                .cloned()
                .unwrap_or_else(|| "b2b-micro-saas".to_string());
            init_workspace(Path::new(&name), &name, &template)
                .map(|_| json!({ "initialized": name, "template": template }))
                .map_err(|e| format!("Init failed: {e}"))
        }
        Some("health") => send_rpc(&socket_path, "system.health", json!({})),
        Some("autonomy") => match args.get(2) {
            None => send_rpc(&socket_path, "autonomy.get", json!({})),
            Some(lvl) => match lvl.parse::<u8>() {
                Ok(n) if n <= 4 => send_rpc(&socket_path, "autonomy.set", json!({ "level": n })),
                _ => Err("Autonomy level must be 0..4".to_string()),
            },
        },
        Some("mission") => match args.get(2).map(String::as_str) {
            Some("start") => {
                let prompt = args.get(3).cloned().unwrap_or_default();
                if prompt.is_empty() {
                    Err("Usage: avos mission start <prompt>".to_string())
                } else {
                    send_rpc(&socket_path, "mission.create", json!({ "prompt": prompt }))
                }
            }
            Some("list") => send_rpc(&socket_path, "mission.list", json!({})),
            _ => Err("Usage: avos mission <start|list>".to_string()),
        },
        Some("loop") => match args.get(2).map(String::as_str) {
            Some("list") => send_rpc(&socket_path, "loop.list", json!({})),
            Some("run") => {
                let target = args.get(3).cloned().unwrap_or_default();
                if target.is_empty() {
                    Err("Usage: avos loop run <path_or_name> [--approve] [--exec]".to_string())
                } else {
                    let approved = args.iter().any(|a| a == "--approve");
                    let exec_cmds = args.iter().any(|a| a == "--exec");
                    let target_path = Path::new(&target);
                    if target_path.exists() {
                        std::fs::read_to_string(target_path)
                            .map_err(|e| format!("Failed to read {target}: {e}"))
                            .and_then(|yaml| {
                                send_rpc(
                                    &socket_path,
                                    "loop.run",
                                    json!({
                                        "yaml": yaml,
                                        "governance_approved": approved,
                                        "execute_commands": exec_cmds
                                    }),
                                )
                            })
                    } else {
                        send_rpc(
                            &socket_path,
                            "loop.run",
                            json!({
                                "name": target,
                                "governance_approved": approved,
                                "execute_commands": exec_cmds
                            }),
                        )
                    }
                }
            }
            _ => Err("Usage: avos loop <list|run>".to_string()),
        },
        Some("login") => Ok(json!({
            "cloud_url": "https://app.avos.sh/oauth/cli",
            "status": "open_browser_for_oauth",
        })),
        _ => Err("AVOS CLI v1.0.0 — Usage: avos [tui] | <init|health|login|mission|autonomy|loop>".to_string()),
    };

    match res {
        Ok(val) => println!("{}", serde_json::to_string_pretty(&val).unwrap()),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}
