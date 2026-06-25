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
