pub mod nftables;

use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::Value;

use ports_common::AgentConfig;

#[derive(Debug, Clone)]
pub struct PortForward {
    pub rule_id: String,
    pub protocol: String,
    pub external_interface: String,
    pub external_port: u16,
    pub internal_ip: String,
    pub internal_port: u16,
    pub public_ip: Option<String>,
    pub comment: Option<String>,
}

impl PortForward {
    pub fn from_payload(payload: &Value) -> Result<Self> {
        let get_str = |key: &str| payload.get(key).and_then(|v| v.as_str()).map(|s| s.to_string());
        let get_u16 = |key: &str| payload.get(key).and_then(|v| v.as_u64()).map(|n| n as u16);

        Ok(Self {
            rule_id: get_str("ruleId").ok_or_else(|| anyhow!("missing ruleId"))?,
            protocol: get_str("protocol").unwrap_or_else(|| "tcp".to_string()).to_lowercase(),
            external_interface: get_str("externalInterface")
                .ok_or_else(|| anyhow!("missing externalInterface"))?,
            external_port: get_u16("externalPort").ok_or_else(|| anyhow!("missing externalPort"))?,
            internal_ip: get_str("internalIp").ok_or_else(|| anyhow!("missing internalIp"))?,
            internal_port: get_u16("internalPort").ok_or_else(|| anyhow!("missing internalPort"))?,
            public_ip: get_str("publicIp"),
            comment: get_str("comment"),
        })
    }

    pub fn protocols(&self) -> Vec<String> {
        if self.protocol.contains('+') {
            vec!["tcp".to_string(), "udp".to_string()]
        } else {
            vec![self.protocol.clone()]
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct Plan {
    pub changes: Vec<String>,
    pub warnings: Vec<String>,
}

pub trait FirewallProvider {
    fn plan(&self, pf: &PortForward) -> Plan;
    fn apply(&self, pf: &PortForward) -> Result<()>;
    fn delete(&self, rule_id: &str) -> Result<()>;
    fn backup(&self) -> Result<String>;
    fn restore(&self, path: &str) -> Result<()>;
}

pub fn provider_for(config: &AgentConfig) -> Box<dyn FirewallProvider> {
    Box::new(nftables::Nftables::new(
        config.firewall.table_name.clone(),
        config.firewall.dry_run,
        config.firewall.backup_before_apply,
    ))
}
