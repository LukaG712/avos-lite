use avos_cli::{init_workspace, send_rpc, tui};
use serde_json::json;
use std::env;
use std::path::{Path, PathBuf};

fn print_help() {
    println!("AVOS CLI v1.0.0 — Autonomous Venture OS (local_lite)");
    println!("Usage: avos [tui] | <command> [options]");
    println!();
    println!("Commands:");
    println!("  tui                                Launch interactive Droid TUI (default)");
    println!("  init [name] [--template <t>]       Initialize venture workspace & avos.toml");
    println!("  health                             Query avos-daemon health & state");
    println!("  autonomy [0..4]                    Get or set autonomy level (0-4)");
    println!("  mission start <prompt...>          Create a new mission");
    println!("  mission list                       List missions");
    println!("  mission get <id>                   Inspect a mission by ID");
    println!("  loop list                          List available stdlib-lite loops");
    println!("  loop run <name_or_path> [flags]    Run a YAML loop");
    println!("       --approve                     Approve governance_gate steps");
    println!("       --exec                        Execute deterministic shell commands");
    println!("       --model <model>               Override LLM model");
    println!("       --endpoint <url>              Override LLM HTTP endpoint (e.g. Ollama)");
    println!("  login                              Print AVOS Cloud OAuth URL");
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

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

    if matches!(
        args.get(1).map(String::as_str),
        Some("--help" | "-h" | "help")
    ) {
        print_help();
        return;
    }
    if matches!(args.get(1).map(String::as_str), Some("--version" | "-V")) {
        println!("avos {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let res = match args.get(1).map(String::as_str) {
        Some("init") => {
            let mut name: Option<String> = None;
            let mut template = "b2b-micro-saas".to_string();
            let mut force = false;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--template" | "-t" => {
                        if let Some(val) = args.get(i + 1) {
                            template = val.clone();
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    "--force" | "-f" => {
                        force = true;
                        i += 1;
                    }
                    arg if !arg.starts_with('-') && name.is_none() => {
                        name = Some(arg.to_string());
                        i += 1;
                    }
                    _ => i += 1,
                }
            }
            let venture_name = name.unwrap_or_else(|| "my-venture".to_string());
            let target_toml = Path::new(&venture_name).join("avos.toml");
            if target_toml.exists() && !force {
                Err(format!(
                    "{} already exists (pass --force to overwrite)",
                    target_toml.display()
                ))
            } else {
                init_workspace(Path::new(&venture_name), &venture_name, &template)
                    .map(|_| json!({ "initialized": venture_name, "template": template }))
                    .map_err(|e| format!("Init failed: {e}"))
            }
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
                let prompt = if args.len() > 3 {
                    args[3..].join(" ")
                } else {
                    String::new()
                };
                if prompt.trim().is_empty() {
                    Err("Usage: avos mission start <prompt>".to_string())
                } else {
                    send_rpc(&socket_path, "mission.create", json!({ "prompt": prompt }))
                }
            }
            Some("list") => send_rpc(&socket_path, "mission.list", json!({})),
            Some("get") => match args.get(3) {
                Some(id) if !id.is_empty() => {
                    send_rpc(&socket_path, "mission.get", json!({ "mission_id": id }))
                }
                _ => Err("Usage: avos mission get <mission_id>".to_string()),
            },
            _ => Err("Usage: avos mission <start|list|get>".to_string()),
        },
        Some("loop") => match args.get(2).map(String::as_str) {
            Some("list") => send_rpc(&socket_path, "loop.list", json!({})),
            Some("run") => {
                let target = args.get(3).cloned().unwrap_or_default();
                if target.is_empty() || target.starts_with('-') {
                    Err(
                        "Usage: avos loop run <path_or_name> [--approve] [--exec] [--model <m>] [--endpoint <url>]"
                            .to_string(),
                    )
                } else {
                    let approved = args.iter().any(|a| a == "--approve");
                    let exec_cmds = args.iter().any(|a| a == "--exec");
                    let llm_model = flag_value(&args, "--model");
                    let llm_endpoint = flag_value(&args, "--endpoint");
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
                                        "execute_commands": exec_cmds,
                                        "llm_model": llm_model,
                                        "llm_endpoint": llm_endpoint,
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
                                "execute_commands": exec_cmds,
                                "llm_model": llm_model,
                                "llm_endpoint": llm_endpoint,
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
        _ => Err(
            "AVOS CLI v1.0.0 — Run `avos --help` for usage: avos [tui] | <init|health|login|mission|autonomy|loop>"
                .to_string(),
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
