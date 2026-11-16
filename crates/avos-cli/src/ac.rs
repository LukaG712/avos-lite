use avos_cli::{format_osc_7501, send_rpc};
use serde_json::json;
use std::env;
use std::path::PathBuf;

fn print_help() {
    println!("ac (avosctl) v1.0.0 — Multi-Agent Terminal Multiplexer");
    println!("Usage: ac <command> [options]");
    println!();
    println!("Commands:");
    println!("  run [--bg] [--pane <layout>] <agent> [task...]  Register & start an agent pane");
    println!("  send <target_agent> <message...>                Send a message to an agent inbox");
    println!("  list                                            List registered agent panes");
    println!("  inbox <agent> [--drain]                         Read (or drain) an agent's inbox");
    println!(
        "  status <agent> <WORKING|BLOCKED|DONE> [detail]  Update agent status & emit OSC 7501"
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let socket_path = PathBuf::from(
        env::var("AVOS_SOCKET_PATH").unwrap_or_else(|_| "/tmp/avos.sock".to_string()),
    );

    if args.len() == 1
        || matches!(
            args.get(1).map(String::as_str),
            Some("--help" | "-h" | "help")
        )
    {
        print_help();
        return;
    }
    if matches!(args.get(1).map(String::as_str), Some("--version" | "-V")) {
        println!("ac {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let res = match args.get(1).map(String::as_str) {
        Some("run") => {
            let mut bg = false;
            let mut pane: Option<String> = None;
            let mut positional: Vec<String> = Vec::new();
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--bg" => {
                        bg = true;
                        i += 1;
                    }
                    "--pane" => {
                        if let Some(val) = args.get(i + 1) {
                            pane = Some(val.clone());
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    _ => {
                        positional.push(args[i].clone());
                        i += 1;
                    }
                }
            }
            let agent = positional
                .first()
                .cloned()
                .unwrap_or_else(|| "worker-droid".to_string());
            let task = if positional.len() > 1 {
                positional[1..].join(" ")
            } else {
                "idle".to_string()
            };
            print!("{}", format_osc_7501(&agent, "WORKING", &task));
            send_rpc(
                &socket_path,
                "pane.register",
                json!({
                    "name": agent,
                    "status": "WORKING",
                    "pane": pane,
                    "bg": bg,
                }),
            )
        }
        Some("send") => {
            let target = args.get(2).cloned().unwrap_or_default();
            let message = if args.len() > 3 {
                args[3..].join(" ")
            } else {
                String::new()
            };
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
            let detail = if args.len() > 4 {
                args[4..].join(" ")
            } else {
                String::new()
            };
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
        _ => Err(
            "ac (avosctl) — Run `ac --help` for usage: ac <run|send|list|inbox|status>".to_string(),
        ),
    };

    match res {
        Ok(val) => println!("{}", serde_json::to_string_pretty(&val).unwrap()),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}
