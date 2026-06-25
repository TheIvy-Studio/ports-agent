use serde_json::{json, Value};
use std::fs;
use std::process::Command;

use crate::core::system;

pub fn issue(payload: &Value) -> Result<Value, String> {
    run(payload, false)
}

pub fn renew(payload: &Value) -> Result<Value, String> {
    run(payload, true)
}

fn run(payload: &Value, renew: bool) -> Result<Value, String> {
    if !system::which("lego") {
        return Err("lego is not installed".to_string());
    }
    let domains: Vec<String> = payload
        .get("domains")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    if domains.is_empty() {
        return Err("no domains provided".to_string());
    }
    let email = payload.get("email").and_then(|v| v.as_str()).unwrap_or("");
    let challenge = payload.get("challengeType").and_then(|v| v.as_str()).unwrap_or("http-01");
    let base = payload.get("path").and_then(|v| v.as_str()).unwrap_or("/etc/ports-agent/certs");
    let server = payload.get("server").and_then(|v| v.as_str());

    fs::create_dir_all(base).map_err(|e| e.to_string())?;

    let mut cmd = Command::new("lego");
    cmd.arg("--accept-tos");
    if !email.is_empty() {
        cmd.args(["--email", email]);
    }
    cmd.args(["--path", base]);
    for d in &domains {
        cmd.args(["--domains", d]);
    }
    if let Some(s) = server {
        cmd.args(["--server", s]);
    }
    if challenge == "dns-01" {
        let provider = payload
            .get("dnsProvider")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "dns provider required for dns-01".to_string())?;
        cmd.args(["--dns", provider]);
        if let Some(env) = payload.get("dnsEnv").and_then(|v| v.as_object()) {
            for (k, v) in env {
                if let Some(val) = v.as_str() {
                    cmd.env(k, val);
                }
            }
        }
    } else {
        cmd.args(["--http", "--http.port", ":80"]);
    }
    cmd.arg(if renew { "renew" } else { "run" });

    let output = cmd.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!("lego failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }

    let primary = &domains[0];
    let cert_path = format!("{base}/certificates/{primary}.crt");
    let key_path = format!("{base}/certificates/{primary}.key");
    let domain_dir = format!("{base}/{primary}");
    fs::create_dir_all(&domain_dir).map_err(|e| e.to_string())?;
    let haproxy_pem = format!("{domain_dir}/haproxy.pem");
    let crt = fs::read_to_string(&cert_path).map_err(|e| e.to_string())?;
    let key = fs::read_to_string(&key_path).map_err(|e| e.to_string())?;
    fs::write(&haproxy_pem, format!("{crt}{key}")).map_err(|e| e.to_string())?;

    Ok(json!({
        "certPath": cert_path,
        "keyPath": key_path,
        "haproxyPemPath": haproxy_pem,
        "issuer": "lego",
        "expiresAt": cert_expiry(&cert_path),
    }))
}

fn cert_expiry(path: &str) -> Option<String> {
    if !system::which("openssl") {
        return None;
    }
    let output = Command::new("openssl")
        .args(["x509", "-enddate", "-noout", "-in", path])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .strip_prefix("notAfter=")
        .map(|s| s.to_string())
}
