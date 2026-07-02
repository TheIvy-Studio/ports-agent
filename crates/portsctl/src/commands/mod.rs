use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};

use ports_agent::core::{keys, system, AGENT_VERSION};
use ports_agent::db::Store;
use ports_agent::firewall::{provider_for, PortForward};
use ports_agent::transport::stdio;
use ports_common::config::{paths, BackendSection, NodeSection};
use ports_common::AgentConfig;

use crate::core;
use crate::core::client::{self, EnrollRequest};

#[derive(Parser)]
#[command(name = "portsctl", version, about = "Manage a Ports node")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Login {
        backend_url: Option<String>,
        token: Option<String>,
        #[arg(long, default_value = "reverse")]
        mode: String,
        #[arg(long)]
        ssh_host: Option<String>,
        #[arg(long, default_value_t = 22)]
        ssh_port: u16,
        #[arg(long, default_value = "ports")]
        ssh_user: String,
        #[arg(long)]
        tailscale_ip: Option<String>,
    },
    Logout,
    Status,
    Config,
    Node {
        #[command(subcommand)]
        action: NodeAction,
    },
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
    Check,
    DetectInterfaces,
    Firewall {
        #[command(subcommand)]
        action: FirewallAction,
    },
    Logs {
        #[arg(long, default_value_t = 100)]
        lines: u32,
    },
    #[command(hide = true)]
    AgentRpc,
}

#[derive(Subcommand)]
enum NodeAction {
    Info,
    Rename { name: String },
}

#[derive(Subcommand)]
enum AgentAction {
    Start,
    Stop,
    Restart,
}

#[derive(Subcommand)]
enum FirewallAction {
    Plan,
    Apply,
    Backup,
    Restore { path: String },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Login { backend_url, token, mode, ssh_host, ssh_port, ssh_user, tailscale_ip } => {
            login(backend_url, token, mode, ssh_host, ssh_port, ssh_user, tailscale_ip)
        }
        Commands::Logout => logout(),
        Commands::Status => status(),
        Commands::Config => show_config(),
        Commands::Node { action } => match action {
            NodeAction::Info => node_info(),
            NodeAction::Rename { name } => node_rename(name),
        },
        Commands::Agent { action } => match action {
            AgentAction::Start => core::systemctl("start"),
            AgentAction::Stop => core::systemctl("stop"),
            AgentAction::Restart => core::systemctl("restart"),
        },
        Commands::Check => check(),
        Commands::DetectInterfaces => {
            println!("{}", serde_json::to_string_pretty(&system::detect_interfaces())?);
            Ok(())
        }
        Commands::Firewall { action } => firewall(action),
        Commands::Logs { lines } => logs(lines),
        Commands::AgentRpc => stdio::run(&core::load_config()?),
    }
}

fn derive_ws(backend_url: &str) -> String {
    let base = backend_url.trim_end_matches('/');
    let base = if let Some(rest) = base.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = base.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        base.to_string()
    };
    format!("{base}/api/agent/connect")
}

fn login(
    backend_url: Option<String>,
    token: Option<String>,
    mode: String,
    ssh_host: Option<String>,
    ssh_port: u16,
    ssh_user: String,
    tailscale_ip: Option<String>,
) -> Result<()> {
    let backend_url = backend_url
        .ok_or_else(|| anyhow!("usage: portsctl login <backend-url> <enrollment-token> [--mode ssh|reverse|tailscale]"))?;
    let token = token.ok_or_else(|| anyhow!("missing enrollment token"))?;
    if mode != "reverse" && mode != "ssh" && mode != "tailscale" {
        return Err(anyhow!("mode must be 'reverse', 'ssh' or 'tailscale'"));
    }
    if mode == "ssh" && ssh_host.is_none() {
        return Err(anyhow!("--ssh-host is required for ssh mode"));
    }
    if mode == "tailscale" && tailscale_ip.is_none() {
        return Err(anyhow!("--tailscale-ip is required for tailscale mode"));
    }

    core::ensure_dirs()?;
    let public_key = keys::generate_and_store(&paths::node_key(), &paths::node_pub())?;

    let request = EnrollRequest {
        token,
        mode: mode.clone(),
        name: None,
        hostname: system::hostname(),
        os: system::os(),
        arch: system::arch(),
        agent_version: AGENT_VERSION.to_string(),
        firewall_backend: system::firewall_backend(),
        public_key: Some(public_key),
        interfaces: system::detect_interfaces(),
        proxmox_detected: system::detect_proxmox(),
        ssh_host,
        ssh_port,
        ssh_user,
        tailscale_ip_v4: tailscale_ip,
    };

    let response = client::enroll(&backend_url, &request)?;
    let websocket_url = response.websocket_url.clone().unwrap_or_else(|| derive_ws(&backend_url));

    let config = AgentConfig {
        node: NodeSection { id: response.node_id.clone(), name: response.name.clone(), mode: mode.clone() },
        backend: BackendSection { url: backend_url.trim_end_matches('/').to_string(), websocket_url },
        auth: Default::default(),
        agent: Default::default(),
        firewall: Default::default(),
        features: Default::default(),
    };
    config
        .save(&paths::config_file())
        .map_err(|e| anyhow!("cannot write {}: {e}", paths::config_file()))?;

    println!("Enrolled as node {} ({})", response.name, response.node_id);

    if mode == "reverse" {
        if let Err(e) = core::systemctl("enable") {
            eprintln!("warning: {e}");
        }
        if let Err(e) = core::systemctl("restart") {
            eprintln!("warning: {e}");
        }
        println!("Reverse agent enabled and started.");
    } else {
        if let Some(key) = response.backend_public_key {
            install_authorized_key(&key)?;
        }
        if mode == "tailscale" {
            println!("Tailscale SSH mode configured. Backend reaches this node via Tailscale IP.");
        } else {
            println!("SSH mode configured. Backend reaches this node via `portsctl agent-rpc`.");
        }
    }
    Ok(())
}

fn install_authorized_key(key: &str) -> Result<()> {
    let line = format!(
        "command=\"/usr/bin/portsctl agent-rpc\",no-port-forwarding,no-X11-forwarding,\
         no-agent-forwarding,no-pty {key}\n"
    );
    let ssh_dir = "/home/ports/.ssh";
    if std::fs::create_dir_all(ssh_dir).is_ok() {
        let path = format!("{ssh_dir}/authorized_keys");
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        if !existing.contains(key) {
            std::fs::write(&path, format!("{existing}{line}"))?;
        }
        println!("Installed restricted backend key into {path}");
    } else {
        println!("Add this restricted entry to the ports user authorized_keys:\n{line}");
    }
    Ok(())
}

fn logout() -> Result<()> {
    let _ = core::systemctl("stop");
    let _ = core::systemctl("disable");
    if std::path::Path::new(&paths::config_file()).exists() {
        std::fs::remove_file(paths::config_file())?;
    }
    println!("Logged out: stopped agent and removed {}", paths::config_file());
    Ok(())
}

fn status() -> Result<()> {
    let config = core::load_config()?;
    println!("node id:          {}", config.node.id);
    println!("node name:        {}", config.node.name);
    println!("mode:             {}", config.node.mode);
    println!("backend:          {}", config.backend.url);
    if config.node.mode == "reverse" {
        println!("websocket:        {}", config.backend.websocket_url);
    }
    println!("service:          {}", core::service_active());
    println!("firewall backend: {}", system::firewall_backend());
    println!("proxmox detected: {}", system::detect_proxmox());
    println!("interfaces:       {}", system::detect_interfaces().len());
    Ok(())
}

fn show_config() -> Result<()> {
    let text = std::fs::read_to_string(paths::config_file())
        .map_err(|e| anyhow!("cannot read {}: {e}", paths::config_file()))?;
    print!("{text}");
    Ok(())
}

fn node_info() -> Result<()> {
    let (id, name) = core::load_config().map(|c| (c.node.id, c.node.name)).unwrap_or_default();
    println!("{}", serde_json::to_string_pretty(&system::node_info(&id, &name))?);
    Ok(())
}

fn node_rename(name: String) -> Result<()> {
    let mut config = core::load_config()?;
    config.node.name = name.clone();
    config.save(&paths::config_file()).map_err(|e| anyhow!("cannot write config: {e}"))?;
    println!("Renamed node to {name} (local config only; update the panel for the backend record).");
    Ok(())
}

fn check() -> Result<()> {
    let config = core::load_config().ok();
    let mark = |ok: bool| if ok { "ok" } else { "FAIL" };

    println!("[{}] config present", mark(config.is_some()));
    println!("[{}] node key present", mark(std::path::Path::new(&paths::node_key()).exists()));
    println!("[{}] nft available", mark(system::which("nft")));
    let ip_forward = std::fs::read_to_string("/proc/sys/net/ipv4/ip_forward")
        .map(|s| s.trim() == "1")
        .unwrap_or(false);
    println!("[{}] ip_forward enabled", mark(ip_forward));
    println!("[{}] service active", mark(core::service_active() == "active"));
    if let Some(cfg) = &config {
        println!("[{}] backend reachable ({})", mark(client::health(&cfg.backend.url)), cfg.backend.url);
    }
    Ok(())
}

fn firewall(action: FirewallAction) -> Result<()> {
    let config = core::load_config()?;
    let provider = provider_for(&config);
    match action {
        FirewallAction::Plan => {
            let store = Store::open(&format!("{}/state.json", config.agent.data_dir));
            println!("managed rules: {}", store.state().applied_rules.len());
            for (rule_id, payload) in &store.state().applied_rules {
                if let Ok(pf) = PortForward::from_payload(payload) {
                    let plan = provider.plan(&pf);
                    println!("- {rule_id}:");
                    for change in plan.changes {
                        println!("    {change}");
                    }
                    for warning in plan.warnings {
                        println!("    ! {warning}");
                    }
                }
            }
            Ok(())
        }
        FirewallAction::Apply => {
            let store = Store::open(&format!("{}/state.json", config.agent.data_dir));
            let mut applied = 0;
            for payload in store.state().applied_rules.values() {
                if let Ok(pf) = PortForward::from_payload(payload) {
                    provider.apply(&pf)?;
                    applied += 1;
                }
            }
            println!("reapplied {applied} rule(s)");
            Ok(())
        }
        FirewallAction::Backup => {
            let path = provider.backup()?;
            println!("backup written to {path}");
            Ok(())
        }
        FirewallAction::Restore { path } => {
            provider.restore(&path)?;
            println!("restored from {path}");
            Ok(())
        }
    }
}

fn logs(lines: u32) -> Result<()> {
    let mut cmd = std::process::Command::new("journalctl");
    if core::user_mode() {
        cmd.arg("--user");
    }
    let status = cmd
        .args(["-u", paths::SERVICE_NAME, "-n", &lines.to_string(), "--no-pager"])
        .status()?;
    if !status.success() {
        return Err(anyhow!("journalctl failed"));
    }
    Ok(())
}
