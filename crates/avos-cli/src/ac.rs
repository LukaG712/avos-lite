use avos_cli::{format_osc_7501, send_rpc};
use serde_json::json;
use std::env;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();
    let socket_path = PathBuf::from(
        env::var("AVOS_SOCKET_PATH").unwrap_or_else(|_| "/tmp/avos.sock".to_string()),
    );

    let res = match args.get(1).map(String::as_str) {
        Some("run") => {
            let agent = args.get(2).cloned().unwrap_or_else(|| "worker-droid".to_string());
            let task = args.get(3).cloned().unwrap_or_else(|| "idle".to_string());
            print!("{}", format_osc_7501(&agent, "WORKING", &task));
            send_rpc(
                &socket_path,
                "pane.register",
                json!({ "name": agent, "status": "WORKING" }),
            )
        }
        Some("send") => {
            let target = args.get(2).cloned().unwrap_or_default();
            let message = args.get(3).cloned().unwrap_or_default();
            if target.is_empty() || message.is_empty() {
                Err("Usage: ac send <target_agent> <message>".to_string())
            } else {
                send_rpc(
                    &socket_path,
                    "pane.send",
                    json!({ "target": target, "message": message }),
                )
            }
        }
        Some("list") => send_rpc(&socket_path, "pane.list", json!({})),
        Some("inbox") => {
            let agent = args.get(2).cloned().unwrap_or_default();
            if agent.is_empty() {
                Err("Usage: ac inbox <agent> [--drain]".to_string())
            } else {
                let drain = args.iter().any(|a| a == "--drain");
                send_rpc(
                    &socket_path,
                    "pane.inbox",
                    json!({ "name": agent, "drain": drain }),
                )
            }
        }
        Some("status") => {
            let agent = args.get(2).cloned().unwrap_or_default();
            let status = args.get(3).cloned().unwrap_or_default();
            let detail = args.get(4).cloned().unwrap_or_default();
            if agent.is_empty() || status.is_empty() {
                Err("Usage: ac status <agent> <WORKING|BLOCKED|DONE> [detail]".to_string())
            } else {
                print!("{}", format_osc_7501(&agent, &status, &detail));
                send_rpc(
                    &socket_path,
                    "pane.status",
                    json!({ "name": agent, "status": status }),
                )
            }
        }
        _ => Err("ac (avosctl) — Usage: ac <run|send|list|inbox|status>".to_string()),
    };

    match res {
        Ok(val) => println!("{}", serde_json::to_string_pretty(&val).unwrap()),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}
