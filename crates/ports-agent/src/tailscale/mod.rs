use serde_json::{json, Value};
use std::process::Command;

use crate::core::system;

pub fn status() -> Value {
    if !system::which("tailscale") {
        return json!({ "installed": false });
    }
    let output = match Command::new("tailscale").args(["status", "--json"]).output() {
        Ok(o) if o.status.success() => o.stdout,
        _ => return json!({ "installed": true, "error": "tailscale status failed" }),
    };
    let parsed: Value = serde_json::from_slice(&output).unwrap_or(Value::Null);
    let self_node = parsed.get("Self");
    json!({
        "installed": true,
        "tailnet": parsed.get("MagicDNSSuffix").and_then(|v| v.as_str()),
        "hostname": self_node.and_then(|v| v.get("HostName")).and_then(|v| v.as_str()),
        "tailscaleIpV4": self_node.and_then(|v| v.get("TailscaleIPs")).and_then(|v| v.as_array()).and_then(|a| a.first()).and_then(|v| v.as_str()),
        "tailscaleIpV6": self_node.and_then(|v| v.get("TailscaleIPs")).and_then(|v| v.as_array()).and_then(|a| a.get(1)).and_then(|v| v.as_str()),
        "online": self_node.and_then(|v| v.get("Online")).and_then(|v| v.as_bool()).unwrap_or(false),
        "derp": self_node.and_then(|v| v.get("DERP")).and_then(|v| v.as_str()),
        "raw": parsed,
    })
}
