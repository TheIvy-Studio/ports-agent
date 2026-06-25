use serde_json::{json, Value};

pub fn conntrack(limit: usize) -> Value {
    let content = std::fs::read_to_string("/proc/net/nf_conntrack")
        .or_else(|_| std::fs::read_to_string("/proc/net/ip_conntrack"))
        .unwrap_or_default();

    let mut connections = Vec::new();
    for line in content.lines().take(limit) {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 4 {
            continue;
        }
        let proto = proto_name(&tokens);
        let mut src = None;
        let mut dst = None;
        let mut sport = None;
        let mut dport = None;
        let mut state = None;
        let mut bytes: u64 = 0;
        for t in &tokens {
            if src.is_none() {
                src = t.strip_prefix("src=").map(|s| s.to_string());
            }
            if dst.is_none() {
                dst = t.strip_prefix("dst=").map(|s| s.to_string());
            }
            if sport.is_none() {
                sport = t.strip_prefix("sport=").and_then(|s| s.parse::<u32>().ok());
            }
            if dport.is_none() {
                dport = t.strip_prefix("dport=").and_then(|s| s.parse::<u32>().ok());
            }
            if let Some(b) = t.strip_prefix("bytes=").and_then(|s| s.parse::<u64>().ok()) {
                bytes += b;
            }
            if state.is_none() && is_state(t) {
                state = Some(t.to_string());
            }
        }
        connections.push(json!({
            "protocol": proto,
            "src": src,
            "dst": dst,
            "sport": sport,
            "dport": dport,
            "state": state,
            "bytes": bytes,
        }));
    }

    let count = connections.len();
    json!({ "connections": connections, "count": count })
}

fn proto_name(tokens: &[&str]) -> String {
    tokens.get(2).map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string())
}

fn is_state(token: &str) -> bool {
    token.len() > 2
        && !token.contains('=')
        && token.chars().all(|c| c.is_ascii_uppercase() || c == '_')
}
