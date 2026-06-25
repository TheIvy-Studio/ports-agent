use serde_json::{json, Value};
use std::process::Command;

use crate::core::system;

pub fn scan(kinds: &[String]) -> Value {
    let mut resources = Vec::new();
    let all = kinds.is_empty() || kinds.contains(&"all".to_string());

    if all || kinds.contains(&"system".to_string()) {
        resources.push(scan_system());
    }
    if all || kinds.contains(&"network".to_string()) {
        resources.extend(scan_network());
    }
    if all || kinds.contains(&"listening_ports".to_string()) {
        resources.extend(scan_listening_ports());
    }
    if all || kinds.contains(&"docker".to_string()) {
        resources.extend(scan_docker());
    }
    if all || kinds.contains(&"tailscale".to_string()) {
        resources.extend(scan_tailscale());
    }
    if all || kinds.contains(&"firewall".to_string()) {
        resources.extend(scan_firewall());
    }

    json!({ "resources": resources })
}

fn scan_system() -> Value {
    json!({
        "kind": "system",
        "source": "/etc/os-release",
        "name": system::hostname(),
        "confidence": "high",
        "status": "detected",
        "raw": {
            "hostname": system::hostname(),
            "os": system::os(),
            "arch": system::arch(),
            "firewallBackend": system::firewall_backend(),
            "proxmoxDetected": system::detect_proxmox(),
        },
        "normalized": {
            "hostname": system::hostname(),
            "os": system::os(),
            "arch": system::arch(),
        }
    })
}

fn scan_network() -> Vec<Value> {
    system::detect_interfaces()
        .into_iter()
        .map(|iface| {
            let name = iface.get("name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
            json!({
                "kind": "interface",
                "source": "iproute",
                "name": name,
                "confidence": "high",
                "status": "detected",
                "raw": iface,
                "normalized": {
                    "name": name,
                    "type": iface.get("type").and_then(|v| v.as_str()).unwrap_or("lan"),
                    "ip": iface.get("ip").and_then(|v| v.as_str()).unwrap_or(""),
                    "status": iface.get("status").and_then(|v| v.as_str()).unwrap_or("down"),
                }
            })
        })
        .collect()
}

fn scan_listening_ports() -> Vec<Value> {
    let output = Command::new("ss")
        .args(["-tulpn", "-H"])
        .output();
    let text = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };

    let mut result = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }
        let proto = parts[0];
        let local = parts[4];
        let process = parts.get(6).unwrap_or(&"-");
        result.push(json!({
            "kind": "listening_port",
            "source": "ss",
            "name": format!("{}/{}", local, proto),
            "confidence": "medium",
            "status": "detected",
            "raw": { "line": line },
            "normalized": {
                "protocol": proto,
                "local": local,
                "process": process,
            }
        }));
    }
    result
}

fn scan_docker() -> Vec<Value> {
    system::docker_containers()
        .into_iter()
        .map(|c| {
            let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
            json!({
                "kind": "docker_container",
                "source": "docker",
                "name": name,
                "confidence": "high",
                "status": "detected",
                "raw": c,
                "normalized": {
                    "id": c.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                    "image": c.get("image").and_then(|v| v.as_str()).unwrap_or(""),
                    "ports": c.get("ports").and_then(|v| v.as_str()).unwrap_or(""),
                }
            })
        })
        .collect()
}

fn scan_tailscale() -> Vec<Value> {
    if !system::which("tailscale") {
        return Vec::new();
    }
    let output = match Command::new("tailscale").args(["status", "--json"]).output() {
        Ok(o) if o.status.success() => o.stdout,
        _ => return Vec::new(),
    };
    let parsed: Value = serde_json::from_slice(&output).unwrap_or(Value::Null);
    let mut result = Vec::new();
    if let Some(self_node) = parsed.get("Self") {
        result.push(json!({
            "kind": "tailscale_node",
            "source": "tailscale status",
            "name": self_node.get("HostName").and_then(|v| v.as_str()).unwrap_or("unknown"),
            "confidence": "high",
            "status": "detected",
            "raw": parsed,
            "normalized": {
                "tailnet": parsed.get("MagicDNSSuffix").and_then(|v| v.as_str()).unwrap_or(""),
                "hostname": self_node.get("HostName").and_then(|v| v.as_str()).unwrap_or(""),
                "tailscaleIpV4": self_node.get("TailscaleIPs").and_then(|v| v.as_array()).and_then(|a| a.first()).and_then(|v| v.as_str()),
                "derp": self_node.get("DERP").and_then(|v| v.as_str()).unwrap_or(""),
            }
        }));
    }
    result
}

fn scan_firewall() -> Vec<Value> {
    let mut result = Vec::new();
    if system::which("nft") {
        if let Ok(output) = Command::new("nft").args(["--json", "list", "ruleset"]).output() {
            if output.status.success() {
                result.push(json!({
                    "kind": "firewall_ruleset",
                    "source": "nft",
                    "name": "nftables ruleset",
                    "confidence": "high",
                    "status": "detected",
                    "raw": serde_json::from_slice::<Value>(&output.stdout).unwrap_or(Value::Null),
                    "normalized": { "backend": "nftables" }
                }));
            }
        }
    }
    if system::which("iptables-save") {
        if let Ok(output) = Command::new("iptables-save").output() {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                result.push(json!({
                    "kind": "firewall_ruleset",
                    "source": "iptables-save",
                    "name": "iptables ruleset",
                    "confidence": "medium",
                    "status": "detected",
                    "raw": { "ruleset": text },
                    "normalized": { "backend": "iptables" }
                }));
            }
        }
    }
    result
}
