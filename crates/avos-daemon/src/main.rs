use avos_daemon::config::load_daemon_config;
use avos_daemon::{LiteDaemonState, serve_unix_socket};
use avos_license::verify_offline_key;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn main() {
    let license_key = std::env::var("AVOS_LICENSE_KEY").unwrap_or_default();
    let edition = verify_offline_key(license_key.as_bytes());
    let socket_path = PathBuf::from(
        std::env::var("AVOS_SOCKET_PATH").unwrap_or_else(|_| "/tmp/avos.sock".to_string()),
    );
    let config_path = PathBuf::from(
        std::env::var("AVOS_CONFIG_PATH").unwrap_or_else(|_| "avos.toml".to_string()),
    );
    let config = load_daemon_config(&config_path);

    println!(
        "// AVOS-DAEMON [{}] ({:?}, autonomy=L{}) // Listening on {}",
        config.runtime_mode,
        edition,
        config.autonomy_level,
        socket_path.display()
    );

    let state = Arc::new(Mutex::new(LiteDaemonState::with_config(edition, config)));
    if let Err(e) = serve_unix_socket(&socket_path, state) {
        eprintln!(
            "Failed to start avos-daemon on {}: {e}",
            socket_path.display()
        );
        std::process::exit(1);
    }
}
