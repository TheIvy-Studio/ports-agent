use std::process::Command;

use anyhow::{anyhow, Result};

use ports_common::config::paths;

use super::{FirewallProvider, Plan, PortForward};

pub struct Nftables {
    table: String,
    dry_run: bool,
    backup_before_apply: bool,
}

impl Nftables {
    pub fn new(table: String, dry_run: bool, backup_before_apply: bool) -> Self {
        Self { table, dry_run, backup_before_apply }
    }

    fn setup_commands(&self) -> Vec<Vec<String>> {
        let t = &self.table;
        vec![
            vec!["add".into(), "table".into(), "ip".into(), t.clone()],
            vec![
                "add".into(), "chain".into(), "ip".into(), t.clone(), "prerouting".into(),
                "{".into(), "type".into(), "nat".into(), "hook".into(), "prerouting".into(),
                "priority".into(), "-100".into(), ";".into(), "}".into(),
            ],
            vec![
                "add".into(), "chain".into(), "ip".into(), t.clone(), "postrouting".into(),
                "{".into(), "type".into(), "nat".into(), "hook".into(), "postrouting".into(),
                "priority".into(), "100".into(), ";".into(), "}".into(),
            ],
        ]
    }

    fn rule_commands(&self, pf: &PortForward) -> Vec<Vec<String>> {
        let t = &self.table;
        let mut commands = Vec::new();
        for proto in pf.protocols() {
            commands.push(vec![
                "add".into(), "rule".into(), "ip".into(), t.clone(), "prerouting".into(),
                "iif".into(), pf.external_interface.clone(),
                proto.clone(), "dport".into(), pf.external_port.to_string(),
                "dnat".into(), "to".into(),
                format!("{}:{}", pf.internal_ip, pf.internal_port),
                "comment".into(), format!("\"{}\"", pf.rule_id),
            ]);
            commands.push(vec![
                "add".into(), "rule".into(), "ip".into(), t.clone(), "postrouting".into(),
                "ip".into(), "daddr".into(), pf.internal_ip.clone(),
                proto, "dport".into(), pf.internal_port.to_string(),
                "masquerade".into(),
                "comment".into(), format!("\"{}\"", pf.rule_id),
            ]);
        }
        commands
    }

    fn run(&self, args: &[String]) -> Result<String> {
        let output = Command::new("nft").args(args).output()?;
        if !output.status.success() {
            return Err(anyhow!(
                "nft {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn ip_forward_enabled() -> bool {
        std::fs::read_to_string("/proc/sys/net/ipv4/ip_forward")
            .map(|s| s.trim() == "1")
            .unwrap_or(false)
    }

    fn rule_handle(&self, chain: &str, rule_id: &str) -> Result<Option<u64>> {
        let listing = self.run(&[
            "-a".into(), "list".into(), "chain".into(), "ip".into(), self.table.clone(),
            chain.into(),
        ])?;
        let needle = format!("comment \"{rule_id}\"");
        for line in listing.lines() {
            if line.contains(&needle) {
                if let Some(idx) = line.find("# handle ") {
                    let handle = line[idx + "# handle ".len()..].trim();
                    if let Ok(parsed) = handle.parse::<u64>() {
                        return Ok(Some(parsed));
                    }
                }
            }
        }
        Ok(None)
    }

    fn delete_existing(&self, rule_id: &str) -> Result<()> {
        for chain in ["prerouting", "postrouting"] {
            while let Some(handle) = self.rule_handle(chain, rule_id)? {
                self.run(&[
                    "delete".into(), "rule".into(), "ip".into(), self.table.clone(),
                    chain.into(), "handle".into(), handle.to_string(),
                ])?;
            }
        }
        Ok(())
    }
}

impl FirewallProvider for Nftables {
    fn plan(&self, pf: &PortForward) -> Plan {
        let mut plan = Plan::default();
        for cmd in self.setup_commands() {
            plan.changes.push(format!("nft {}", cmd.join(" ")));
        }
        for cmd in self.rule_commands(pf) {
            plan.changes.push(format!("nft {}", cmd.join(" ")));
        }
        if !Self::ip_forward_enabled() {
            plan.warnings
                .push("net.ipv4.ip_forward is disabled; forwarding will not work".into());
        }
        if !super::super::core::system::which("nft") {
            plan.warnings.push("nft binary not found on PATH".into());
        }
        plan
    }

    fn apply(&self, pf: &PortForward) -> Result<()> {
        if self.dry_run {
            return Ok(());
        }
        if self.backup_before_apply {
            self.backup()?;
        }
        for cmd in self.setup_commands() {
            let _ = self.run(&cmd);
        }
        self.delete_existing(&pf.rule_id)?;
        for cmd in self.rule_commands(pf) {
            self.run(&cmd)?;
        }
        Ok(())
    }

    fn delete(&self, rule_id: &str) -> Result<()> {
        if self.dry_run {
            return Ok(());
        }
        self.delete_existing(rule_id)
    }

    fn backup(&self) -> Result<String> {
        let dir = format!("{}/backups", paths::data_dir());
        std::fs::create_dir_all(&dir)?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = format!("{dir}/ruleset-{stamp}.nft");
        let ruleset = self.run(&["list".into(), "ruleset".into()])?;
        std::fs::write(&path, ruleset)?;
        Ok(path)
    }

    fn restore(&self, path: &str) -> Result<()> {
        if self.dry_run {
            return Ok(());
        }
        self.run(&["-f".into(), path.to_string()])?;
        Ok(())
    }
}
