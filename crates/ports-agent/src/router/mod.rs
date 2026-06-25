use serde_json::{json, Value};

use ports_common::{msg, AgentConfig, RpcRequest, RpcResponse};

use crate::core::{system, telemetry, AGENT_VERSION};
use crate::db::Store;
use crate::firewall::{provider_for, PortForward};

fn state_path(config: &AgentConfig) -> String {
    format!("{}/state.json", config.agent.data_dir)
}

pub fn dispatch(req: &RpcRequest, config: &AgentConfig) -> RpcResponse {
    let rid = req.request_id.clone();
    match req.kind.as_str() {
        msg::HELLO => RpcResponse::ok(msg::HELLO_RESULT, rid, json!({ "agentVersion": AGENT_VERSION })),
        msg::NODE_INFO => {
            RpcResponse::ok(msg::NODE_INFO_RESULT, rid, system::node_info(&config.node.id, &config.node.name))
        }
        msg::DETECT_INTERFACES => RpcResponse::ok(
            msg::DETECT_INTERFACES_RESULT,
            rid,
            json!({ "interfaces": system::detect_interfaces() }),
        ),
        msg::TELEMETRY_REQUEST => RpcResponse::ok(msg::TELEMETRY_SNAPSHOT, rid, telemetry::snapshot()),
        msg::FIREWALL_PLAN => firewall_plan(req, config, rid),
        msg::FIREWALL_APPLY_RULE => firewall_apply(req, config, rid),
        msg::FIREWALL_DELETE_RULE => firewall_delete(req, config, rid),
        msg::FIREWALL_BACKUP => firewall_backup(config, rid),
        msg::FIREWALL_RESTORE => firewall_restore(req, config, rid),
        msg::DOCKER_DETECT => RpcResponse::ok(
            msg::DOCKER_DETECT_RESULT,
            rid,
            json!({ "dockerAvailable": system::detect_docker(), "containers": system::docker_containers() }),
        ),
        msg::COMPOSE_DETECT => RpcResponse::ok(
            msg::COMPOSE_DETECT_RESULT,
            rid,
            json!({ "projects": system::compose_projects() }),
        ),
        msg::LOGS_TAIL => {
            let lines = req.payload.get("lines").and_then(|v| v.as_u64()).unwrap_or(200) as usize;
            RpcResponse::ok(msg::LOGS_TAIL_RESULT, rid, json!({ "lines": system::tail_logs(lines) }))
        }
        other => RpcResponse::error(rid, format!("unknown command {other:?}")),
    }
}

fn firewall_plan(req: &RpcRequest, config: &AgentConfig, rid: Option<String>) -> RpcResponse {
    let pf = match PortForward::from_payload(&req.payload) {
        Ok(pf) => pf,
        Err(e) => return RpcResponse::error(rid, e.to_string()),
    };
    let plan = provider_for(config).plan(&pf);
    let payload = serde_json::to_value(&plan).unwrap_or(Value::Null);
    RpcResponse::ok(msg::FIREWALL_PLAN_RESULT, rid, payload)
}

fn firewall_apply(req: &RpcRequest, config: &AgentConfig, rid: Option<String>) -> RpcResponse {
    let pf = match PortForward::from_payload(&req.payload) {
        Ok(pf) => pf,
        Err(e) => return RpcResponse::error(rid, e.to_string()),
    };
    match provider_for(config).apply(&pf) {
        Ok(()) => {
            let mut store = Store::open(&state_path(config));
            let _ = store.record_rule(&pf.rule_id, req.payload.clone());
            RpcResponse::ok(
                msg::FIREWALL_APPLY_RESULT,
                rid,
                json!({ "ruleId": pf.rule_id, "status": "applied" }),
            )
        }
        Err(e) => RpcResponse::error(rid, e.to_string()),
    }
}

fn firewall_delete(req: &RpcRequest, config: &AgentConfig, rid: Option<String>) -> RpcResponse {
    let rule_id = match req.payload.get("ruleId").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return RpcResponse::error(rid, "missing ruleId"),
    };
    match provider_for(config).delete(&rule_id) {
        Ok(()) => {
            let mut store = Store::open(&state_path(config));
            let _ = store.remove_rule(&rule_id);
            RpcResponse::ok(
                msg::FIREWALL_DELETE_RESULT,
                rid,
                json!({ "ruleId": rule_id, "status": "deleted" }),
            )
        }
        Err(e) => RpcResponse::error(rid, e.to_string()),
    }
}

fn firewall_backup(config: &AgentConfig, rid: Option<String>) -> RpcResponse {
    match provider_for(config).backup() {
        Ok(path) => {
            let mut store = Store::open(&state_path(config));
            let _ = store.set_last_backup(&path);
            RpcResponse::ok(msg::FIREWALL_BACKUP_RESULT, rid, json!({ "path": path }))
        }
        Err(e) => RpcResponse::error(rid, e.to_string()),
    }
}

fn firewall_restore(req: &RpcRequest, config: &AgentConfig, rid: Option<String>) -> RpcResponse {
    let path = match req.payload.get("path").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => return RpcResponse::error(rid, "missing path"),
    };
    match provider_for(config).restore(&path) {
        Ok(()) => RpcResponse::ok(msg::FIREWALL_RESTORE_RESULT, rid, json!({ "status": "restored" })),
        Err(e) => RpcResponse::error(rid, e.to_string()),
    }
}
