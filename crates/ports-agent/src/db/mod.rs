use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub applied_rules: BTreeMap<String, Value>,
    #[serde(default)]
    pub last_backup: Option<String>,
}

pub struct Store {
    path: String,
    state: State,
}

impl Store {
    pub fn open(path: &str) -> Self {
        let state = std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { path: path.to_string(), state }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn record_rule(&mut self, rule_id: &str, payload: Value) -> Result<()> {
        self.state.applied_rules.insert(rule_id.to_string(), payload);
        self.persist()
    }

    pub fn remove_rule(&mut self, rule_id: &str) -> Result<()> {
        self.state.applied_rules.remove(rule_id);
        self.persist()
    }

    pub fn set_last_backup(&mut self, path: &str) -> Result<()> {
        self.state.last_backup = Some(path.to_string());
        self.persist()
    }

    fn persist(&self) -> Result<()> {
        if let Some(parent) = Path::new(&self.path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, serde_json::to_string_pretty(&self.state)?)?;
        Ok(())
    }
}
