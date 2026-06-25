use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::core::system;

pub fn plan(_corefile: &str) -> Value {
    if !system::which("coredns") {
        return json!({ "valid": false, "output": "coredns is not installed" });
    }
    json!({ "valid": true, "output": "coredns present" })
}

pub fn apply(payload: &Value) -> Result<Value, String> {
    let corefile_path = payload.get("corefilePath").and_then(|v| v.as_str()).unwrap_or("");
    let corefile = payload.get("corefile").and_then(|v| v.as_str()).unwrap_or("");
    let zones_dir = payload.get("zonesDir").and_then(|v| v.as_str()).unwrap_or("/etc/coredns/zones");
    let zone_name = payload.get("zoneName").and_then(|v| v.as_str()).unwrap_or("");
    let zonefile = payload.get("zonefile").and_then(|v| v.as_str()).unwrap_or("");
    if corefile_path.is_empty() || zone_name.is_empty() {
        return Err("missing corefilePath or zoneName".to_string());
    }

    if let Some(parent) = Path::new(corefile_path).parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(zones_dir).map_err(|e| e.to_string())?;
    fs::write(corefile_path, corefile).map_err(|e| e.to_string())?;
    let zone_path = format!("{zones_dir}/{zone_name}.db");
    fs::write(&zone_path, zonefile).map_err(|e| e.to_string())?;

    let reloaded = reload();
    Ok(json!({
        "status": "applied",
        "corefilePath": corefile_path,
        "zonePath": zone_path,
        "reloaded": reloaded,
    }))
}

fn reload() -> bool {
    if !system::which("systemctl") {
        return false;
    }
    Command::new("systemctl")
        .args(["reload-or-restart", "coredns"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
