use avos_license::{
    DEFAULT_AVOS_PUBLIC_KEY, LicenseEdition, verify_offline_key, verify_offline_key_with_pubkey,
};
use std::env;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("avos-license — Offline Ed25519 license verifier");
        println!("Usage: avos-license [--file <path> | <license_token>]");
        println!("Reads AVOS_LICENSE_KEY or ./license.avoskey when no argument is provided.");
        return;
    }

    let raw_key = if let Some(pos) = args.iter().position(|a| a == "--file" || a == "-f") {
        let Some(path) = args.get(pos + 1) else {
            eprintln!("Error: --file requires a path argument");
            std::process::exit(1);
        };
        fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("Failed to read license file {path}: {e}");
            std::process::exit(1);
        })
    } else if let Some(arg) = args.get(1) {
        let p = Path::new(arg);
        if p.exists() {
            fs::read_to_string(p).unwrap_or_else(|e| {
                eprintln!("Failed to read license file {arg}: {e}");
                std::process::exit(1);
            })
        } else {
            arg.clone()
        }
    } else if let Ok(env_key) = env::var("AVOS_LICENSE_KEY") {
        env_key
    } else if Path::new("license.avoskey").exists() {
        fs::read_to_string("license.avoskey").unwrap_or_default()
    } else {
        String::new()
    };

    if raw_key.trim().is_empty() {
        let edition = verify_offline_key(b"");
        println!("// AVOS-LICENSE // Active edition: {:?}", edition);
        return;
    }

    match verify_offline_key_with_pubkey(raw_key.as_bytes(), &DEFAULT_AVOS_PUBLIC_KEY, now_unix()) {
        Ok(claims) => {
            println!(
                "// AVOS-LICENSE // Active edition: {:?} (issued_to={}, max_seats={})",
                claims.edition, claims.issued_to, claims.max_seats
            );
        }
        Err(err) => {
            eprintln!(
                "// AVOS-LICENSE // Verification failed ({:?}); falling back to {:?}",
                err,
                LicenseEdition::CommunityFree
            );
            std::process::exit(1);
        }
    }
}
