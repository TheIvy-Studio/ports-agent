use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub mod paths {
    use std::path::PathBuf;

    fn base_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("PORTS_CONFIG_DIR") {
            return PathBuf::from(dir);
        }
        if let Ok(home) = std::env::var("HOME") {
            let user_dir = PathBuf::from(home).join(".config/ports");
            if user_dir.join("config/agent.yaml").exists() {
                return user_dir;
            }
        }
        PathBuf::from("/etc/ports")
    }

    pub fn base() -> String {
        base_dir().to_string_lossy().into_owned()
    }

    pub fn config_dir() -> String {
        base_dir().join("config").to_string_lossy().into_owned()
    }

    pub fn config_file() -> String {
        base_dir().join("config/agent.yaml").to_string_lossy().into_owned()
    }

    pub fn key_dir() -> String {
        base_dir().join("keys").to_string_lossy().into_owned()
    }

    pub fn node_key() -> String {
        base_dir().join("keys/node.key").to_string_lossy().into_owned()
    }

    pub fn node_pub() -> String {
        base_dir().join("keys/node.pub").to_string_lossy().into_owned()
    }

    pub fn data_dir() -> String {
        if let Ok(dir) = std::env::var("PORTS_DATA_DIR") {
            dir
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".local/share/ports").to_string_lossy().into_owned()
        } else {
            "/var/lib/ports".to_string()
        }
    }

    pub fn state_file() -> String {
        PathBuf::from(data_dir()).join("state.json").to_string_lossy().into_owned()
    }

    pub fn log_dir() -> String {
        if let Ok(dir) = std::env::var("PORTS_LOG_DIR") {
            dir
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".local/state/ports").to_string_lossy().into_owned()
        } else {
            "/var/log/ports".to_string()
        }
    }

    pub const SERVICE_NAME: &str = "ports-agent";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSection {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_mode")]
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendSection {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub websocket_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSection {
    #[serde(default = "default_key_path")]
    pub key_path: String,
    #[serde(default = "default_pub_path")]
    pub public_key_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSection {
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallSection {
    #[serde(default = "default_firewall_backend")]
    pub backend: String,
    #[serde(default = "default_table_name")]
    pub table_name: String,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default = "default_true")]
    pub backup_before_apply: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesSection {
    #[serde(default = "default_true")]
    pub firewall: bool,
    #[serde(default = "default_true")]
    pub traffic: bool,
    #[serde(default = "default_true")]
    pub conntrack: bool,
    #[serde(default)]
    pub proxmox_local: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub node: NodeSection,
    pub backend: BackendSection,
    #[serde(default)]
    pub auth: AuthSection,
    #[serde(default)]
    pub agent: AgentSection,
    #[serde(default)]
    pub firewall: FirewallSection,
    #[serde(default)]
    pub features: FeaturesSection,
}

impl AgentConfig {
    pub fn load(path: &str) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        serde_yaml::from_str(&text)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn save(&self, path: &str) -> std::io::Result<()> {
        if let Some(parent) = PathBuf::from(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_yaml::to_string(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, text)
    }
}

fn default_mode() -> String {
    "reverse".to_string()
}
fn default_key_path() -> String {
    paths::node_key()
}
fn default_pub_path() -> String {
    paths::node_pub()
}
fn default_data_dir() -> String {
    paths::data_dir()
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_firewall_backend() -> String {
    "nftables".to_string()
}
fn default_table_name() -> String {
    "ports_agent".to_string()
}
fn default_true() -> bool {
    true
}

impl Default for AuthSection {
    fn default() -> Self {
        Self { key_path: default_key_path(), public_key_path: default_pub_path() }
    }
}
impl Default for AgentSection {
    fn default() -> Self {
        Self { data_dir: default_data_dir(), log_level: default_log_level() }
    }
}
impl Default for FirewallSection {
    fn default() -> Self {
        Self {
            backend: default_firewall_backend(),
            table_name: default_table_name(),
            dry_run: false,
            backup_before_apply: true,
        }
    }
}
impl Default for FeaturesSection {
    fn default() -> Self {
        Self { firewall: true, traffic: true, conntrack: true, proxmox_local: false }
    }
}
