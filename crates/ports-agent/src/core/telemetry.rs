use serde_json::{json, Value};

pub fn snapshot() -> Value {
    json!({
        "interfaces": interface_counters(),
        "traffic": interface_counters(),
        "connections": connection_count(),
        "firewallCounters": [],
    })
}

fn interface_counters() -> Vec<Value> {
    let mut result = Vec::new();
    let content = match std::fs::read_to_string("/proc/net/dev") {
        Ok(c) => c,
        Err(_) => return result,
    };
    for line in content.lines().skip(2) {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        let fields: Vec<&str> = rest.split_whitespace().collect();
        if fields.len() < 16 {
            continue;
        }
        let rx_bytes: u64 = fields[0].parse().unwrap_or(0);
        let tx_bytes: u64 = fields[8].parse().unwrap_or(0);
        result.push(json!({
            "interface": name,
            "rxBytes": rx_bytes,
            "txBytes": tx_bytes,
        }));
    }
    result
}

fn connection_count() -> Value {
    let count = std::fs::read_to_string("/proc/sys/net/netfilter/nf_conntrack_count")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0);
    json!([{ "type": "conntrack", "count": count }])
}
