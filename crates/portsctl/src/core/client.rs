use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollRequest {
    pub token: String,
    pub mode: String,
    pub name: Option<String>,
    pub hostname: String,
    pub os: String,
    pub arch: String,
    pub agent_version: String,
    pub firewall_backend: String,
    pub public_key: Option<String>,
    pub interfaces: Vec<Value>,
    pub proxmox_detected: bool,
    pub ssh_host: Option<String>,
    pub ssh_port: u16,
    pub ssh_user: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollResponse {
    pub node_id: String,
    pub name: String,
    pub websocket_url: Option<String>,
    pub backend_public_key: Option<String>,
}

pub fn enroll(backend_url: &str, request: &EnrollRequest) -> Result<EnrollResponse> {
    let url = format!("{}/api/agent/enroll", backend_url.trim_end_matches('/'));
    let response = reqwest::blocking::Client::new()
        .post(&url)
        .json(request)
        .send()
        .with_context(|| format!("POST {url}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        anyhow::bail!("enrollment failed ({status}): {body}");
    }
    Ok(response.json()?)
}

pub fn health(backend_url: &str) -> bool {
    let base = backend_url.trim_end_matches('/');
    let client = reqwest::blocking::Client::new();
    for path in ["/health", "/api/health"] {
        let url = format!("{base}{path}");
        let ok = client
            .get(&url)
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false);
        if ok {
            return true;
        }
    }
    false
}
