pub mod client;

use std::process::Command;

use anyhow::{anyhow, Result};

use ports_common::{config::paths, AgentConfig};

pub fn load_config() -> Result<AgentConfig> {
    AgentConfig::load(&paths::config_file())
        .map_err(|e| anyhow!("cannot read {}: {e} (run `portsctl login` first)", paths::config_file()))
}

pub fn ensure_dirs() -> Result<()> {
    for dir in [paths::config_dir(), paths::key_dir(), paths::data_dir(), paths::log_dir()] {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}

pub fn user_mode() -> bool {
    std::env::var("PORTS_USER_SERVICE").is_ok_and(|v| v == "1")
        || std::path::Path::new(&format!("{}/.config/systemd/user/ports-agent.service", std::env::var("HOME").unwrap_or_default())).exists()
}

pub fn systemctl(action: &str) -> Result<()> {
    let mut cmd = Command::new("systemctl");
    if user_mode() {
        cmd.arg("--user");
    }
    let status = cmd.arg(action).arg(paths::SERVICE_NAME).status()?;
    if !status.success() {
        return Err(anyhow!("systemctl {action} {} failed", paths::SERVICE_NAME));
    }
    Ok(())
}

pub fn service_active() -> String {
    let mut cmd = Command::new("systemctl");
    if user_mode() {
        cmd.arg("--user");
    }
    cmd.args(["is-active", paths::SERVICE_NAME])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}
