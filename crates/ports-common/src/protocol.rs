use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod msg {
    pub const HELLO: &str = "hello";
    pub const NODE_INFO: &str = "node.info";
    pub const DETECT_INTERFACES: &str = "interfaces.detect";
    pub const FIREWALL_PLAN: &str = "firewall.plan";
    pub const FIREWALL_APPLY_RULE: &str = "firewall.apply_rule";
    pub const FIREWALL_DELETE_RULE: &str = "firewall.delete_rule";
    pub const FIREWALL_BACKUP: &str = "firewall.backup";
    pub const FIREWALL_RESTORE: &str = "firewall.restore";
    pub const TELEMETRY_REQUEST: &str = "telemetry.request";
    pub const DOCKER_DETECT: &str = "docker.detect";
    pub const COMPOSE_DETECT: &str = "compose.detect";
    pub const LOGS_TAIL: &str = "logs.tail";

    pub const DISCOVERY_SCAN: &str = "discovery.scan";
    pub const DISCOVERY_IMPORT: &str = "discovery.import";
    pub const BACKUP_CREATE: &str = "backup.create";
    pub const BACKUP_RESTORE: &str = "backup.restore";
    pub const TAILSCALE_STATUS: &str = "tailscale.status";
    pub const TAILSCALE_UP: &str = "tailscale.up";
    pub const DHCP_PLAN: &str = "dhcp.plan";
    pub const DHCP_APPLY: &str = "dhcp.apply";
    pub const HAPROXY_VALIDATE: &str = "haproxy.validate";
    pub const HAPROXY_RELOAD: &str = "haproxy.reload";
    pub const CERT_ISSUE: &str = "cert.issue";
    pub const CERT_RENEW: &str = "cert.renew";

    pub const HELLO_RESULT: &str = "hello_result";
    pub const NODE_INFO_RESULT: &str = "node.info_result";
    pub const DETECT_INTERFACES_RESULT: &str = "interfaces.detect_result";
    pub const FIREWALL_PLAN_RESULT: &str = "firewall.plan_result";
    pub const FIREWALL_APPLY_RESULT: &str = "firewall.apply_result";
    pub const FIREWALL_DELETE_RESULT: &str = "firewall.delete_result";
    pub const FIREWALL_BACKUP_RESULT: &str = "firewall.backup_result";
    pub const FIREWALL_RESTORE_RESULT: &str = "firewall.restore_result";
    pub const DOCKER_DETECT_RESULT: &str = "docker.detect_result";
    pub const COMPOSE_DETECT_RESULT: &str = "compose.detect_result";
    pub const LOGS_TAIL_RESULT: &str = "logs.tail_result";

    pub const DISCOVERY_SCAN_RESULT: &str = "discovery.scan_result";
    pub const DISCOVERY_IMPORT_RESULT: &str = "discovery.import_result";
    pub const BACKUP_CREATE_RESULT: &str = "backup.create_result";
    pub const BACKUP_RESTORE_RESULT: &str = "backup.restore_result";
    pub const TAILSCALE_STATUS_RESULT: &str = "tailscale.status_result";
    pub const TAILSCALE_UP_RESULT: &str = "tailscale.up_result";
    pub const DHCP_PLAN_RESULT: &str = "dhcp.plan_result";
    pub const DHCP_APPLY_RESULT: &str = "dhcp.apply_result";
    pub const HAPROXY_VALIDATE_RESULT: &str = "haproxy.validate_result";
    pub const HAPROXY_RELOAD_RESULT: &str = "haproxy.reload_result";
    pub const CERT_ISSUE_RESULT: &str = "cert.issue_result";
    pub const CERT_RENEW_RESULT: &str = "cert.renew_result";

    pub const TELEMETRY_SNAPSHOT: &str = "telemetry.snapshot";
    pub const ERROR: &str = "error";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "requestId", default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "requestId", skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub ok: bool,
    #[serde(default)]
    pub payload: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl RpcResponse {
    pub fn ok(kind: &str, request_id: Option<String>, payload: Value) -> Self {
        Self { kind: kind.to_string(), request_id, ok: true, payload, error: None }
    }

    pub fn error(request_id: Option<String>, message: impl Into<String>) -> Self {
        Self {
            kind: msg::ERROR.to_string(),
            request_id,
            ok: false,
            payload: Value::Null,
            error: Some(message.into()),
        }
    }
}
