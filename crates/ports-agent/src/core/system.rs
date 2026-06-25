use std::process::Command;

use serde_json::{json, Value};

pub fn os() -> String {
    std::env::consts::OS.to_string()
}

pub fn arch() -> String {
    std::env::consts::ARCH.to_string()
}

pub fn hostname() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

pub fn firewall_backend() -> String {
    if which("nft") {
        "nftables".to_string()
    } else if which("iptables") {
        "iptables".to_string()
    } else {
        "none".to_string()
    }
}

pub fn detect_proxmox() -> bool {
    std::path::Path::new("/etc/pve").exists() || which("pveversion")
}

pub fn detect_docker() -> bool {
    which("docker")
}

pub fn docker_containers() -> Vec<Value> {
    let output = match Command::new("docker")
        .args(["ps", "--format", "json", "--no-trunc"])
        .output()
    {
        Ok(o) if o.status.success() => o.stdout,
        _ => return Vec::new(),
    };
    let text = String::from_utf8_lossy(&output);
    let mut result = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(parsed) = serde_json::from_str::<Value>(line) else { continue };
        let id = parsed.get("ID").or_else(|| parsed.get("Id")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let name = parsed.get("Names").or_else(|| parsed.get("name")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let image = parsed.get("Image").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let status = parsed.get("State").or_else(|| parsed.get("Status")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        let labels = parsed.get("Labels").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let compose_project = labels.split(',').find(|s| s.starts_with("com.docker.compose.project=")).map(|s| s.split('=').nth(1).unwrap_or("").to_string()).filter(|s| !s.is_empty());
        let compose_service = labels.split(',').find(|s| s.starts_with("com.docker.compose.service=")).map(|s| s.split('=').nth(1).unwrap_or("").to_string()).filter(|s| !s.is_empty());
        let ports = parsed.get("Ports").and_then(|v| v.as_str()).unwrap_or("").to_string();
        result.push(json!({
            "id": id,
            "name": name,
            "image": image,
            "status": status,
            "composeProject": compose_project,
            "composeService": compose_service,
            "ports": ports,
        }));
    }
    result
}

pub fn tail_logs(lines: usize) -> Vec<Value> {
    let output = Command::new("journalctl")
        .args([
            "--user",
            "-u",
            "ports-agent",
            "-n",
            &lines.to_string(),
            "--no-pager",
        ])
        .output();
    let text = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => String::new(),
    };
    text.lines()
        .map(|line| {
            json!({
                "message": line,
                "level": infer_level(line),
                "time": None::<String>,
            })
        })
        .collect()
}

fn infer_level(line: &str) -> &str {
    let lower = line.to_lowercase();
    if lower.contains("error") || lower.contains("panic") || lower.contains("failed") {
        "error"
    } else if lower.contains("warn") {
        "warn"
    } else if lower.contains("debug") {
        "debug"
    } else {
        "info"
    }
}

pub fn compose_projects() -> Vec<Value> {
    if !which("docker") {
        return Vec::new();
    }
    let output = match Command::new("docker")
        .args(["compose", "ls", "--format", "json"])
        .output()
    {
        Ok(o) if o.status.success() => o.stdout,
        _ => return Vec::new(),
    };
    let text = String::from_utf8_lossy(&output);
    let parsed: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    let array = parsed.as_array().cloned().unwrap_or_default();
    array
        .into_iter()
        .map(|item| {
            let name = item.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let status = item.get("Status").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let config_file = item.get("ConfigFiles").and_then(|v| v.as_str()).unwrap_or("").to_string();
            json!({
                "id": name,
                "name": name,
                "status": status,
                "configFiles": config_file,
            })
        })
        .collect()
}

pub fn which(binary: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            if std::path::Path::new(dir).join(binary).exists() {
                return true;
            }
        }
    }
    for dir in ["/usr/sbin", "/sbin", "/usr/bin", "/bin"] {
        if std::path::Path::new(dir).join(binary).exists() {
            return true;
        }
    }
    false
}

fn classify(name: &str, kind: Option<&str>) -> &'static str {
    if name == "lo" {
        "loopback"
    } else if kind == Some("bridge") || name.starts_with("vmbr") || name.starts_with("br") {
        "bridge"
    } else if kind == Some("vlan") || name.contains('.') || name.starts_with("vlan") {
        "vlan"
    } else {
        "lan"
    }
}

pub fn detect_interfaces() -> Vec<Value> {
    if let Some(parsed) = detect_interfaces_iproute() {
        return parsed;
    }
    detect_interfaces_sysfs()
}

fn detect_interfaces_iproute() -> Option<Vec<Value>> {
    let output = Command::new("ip").args(["-j", "-d", "addr", "show"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let parsed: Value = serde_json::from_slice(&output.stdout).ok()?;
    let array = parsed.as_array()?;
    let mut result = Vec::new();
    for item in array {
        let name = item.get("ifname").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if name.is_empty() {
            continue;
        }
        let kind = item
            .get("linkinfo")
            .and_then(|v| v.get("info_kind"))
            .and_then(|v| v.as_str());
        let operstate = item.get("operstate").and_then(|v| v.as_str()).unwrap_or("UNKNOWN");
        let status = if operstate.eq_ignore_ascii_case("up") { "up" } else { "down" };
        let mut ip = String::new();
        let mut addresses = Vec::new();
        if let Some(addrs) = item.get("addr_info").and_then(|v| v.as_array()) {
            for addr in addrs {
                if addr.get("family").and_then(|v| v.as_str()) == Some("inet") {
                    if let Some(local) = addr.get("local").and_then(|v| v.as_str()) {
                        addresses.push(json!({
                            "family": "inet",
                            "local": local,
                            "prefixlen": addr.get("prefixlen").and_then(|v| v.as_u64()),
                        }));
                        ip = local.to_string();
                    }
                } else if addr.get("family").and_then(|v| v.as_str()) == Some("inet6") {
                    if let Some(local) = addr.get("local").and_then(|v| v.as_str()) {
                        addresses.push(json!({
                            "family": "inet6",
                            "local": local,
                            "prefixlen": addr.get("prefixlen").and_then(|v| v.as_u64()),
                        }));
                    }
                }
            }
        }
        result.push(json!({
            "id": name,
            "name": name,
            "type": classify(&name, kind),
            "ip": ip,
            "status": status,
            "ifindex": item.get("ifindex").and_then(|v| v.as_u64()),
            "mac": item.get("address").and_then(|v| v.as_str()),
            "broadcast": item.get("broadcast").and_then(|v| v.as_str()),
            "mtu": item.get("mtu").and_then(|v| v.as_u64()),
            "qdisc": item.get("qdisc").and_then(|v| v.as_str()),
            "master": item.get("master").and_then(|v| v.as_str()),
            "linkType": item.get("link_type").and_then(|v| v.as_str()).or(kind),
            "flags": item.get("flags").and_then(|v| v.as_array()).cloned().unwrap_or_default(),
            "addresses": addresses,
        }));
    }
    Some(result)
}

fn detect_interfaces_sysfs() -> Vec<Value> {
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let operstate = std::fs::read_to_string(entry.path().join("operstate"))
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            let status = if operstate == "up" { "up" } else { "down" };
            result.push(json!({
                "id": name,
                "name": name,
                "type": classify(&name, None),
                "ip": "",
                "status": status,
            }));
        }
    }
    result
}

pub fn node_info(node_id: &str, node_name: &str) -> Value {
    json!({
        "nodeId": node_id,
        "name": node_name,
        "hostname": hostname(),
        "os": os(),
        "arch": arch(),
        "agentVersion": super::AGENT_VERSION,
        "firewallBackend": firewall_backend(),
        "proxmoxDetected": detect_proxmox(),
        "interfaces": detect_interfaces(),
    })
}
