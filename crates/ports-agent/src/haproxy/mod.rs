use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::core::system;

pub fn validate(config: &str) -> Value {
    if !system::which("haproxy") {
        return json!({ "valid": false, "output": "haproxy is not installed" });
    }
    let tmp = format!("/tmp/ports-haproxy-{}.cfg", std::process::id());
    if fs::write(&tmp, config).is_err() {
        return json!({ "valid": false, "output": "failed to write temporary config" });
    }
    let output = Command::new("haproxy").args(["-c", "-f", &tmp]).output();
    let _ = fs::remove_file(&tmp);
    match output {
        Ok(o) => {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            json!({ "valid": o.status.success(), "output": text.trim() })
        }
        Err(e) => json!({ "valid": false, "output": format!("haproxy check failed: {e}") }),
    }
}

pub fn apply(config_path: &str, config: &str) -> Result<Value, String> {
    let validation = validate(config);
    if !validation.get("valid").and_then(|v| v.as_bool()).unwrap_or(false) {
        let output = validation.get("output").and_then(|v| v.as_str()).unwrap_or("");
        return Err(format!("config validation failed: {output}"));
    }
    if let Some(parent) = Path::new(config_path).parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(config_path, config).map_err(|e| e.to_string())?;
    let reloaded = reload();
    Ok(json!({
        "status": "applied",
        "configPath": config_path,
        "reloaded": reloaded,
    }))
}

fn reload() -> bool {
    if !system::which("systemctl") {
        return false;
    }
    Command::new("systemctl")
        .args(["reload-or-restart", "haproxy"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
