use anyhow::Result;
use ports_agent::core::logging;
use ports_agent::transport;
use ports_common::{config::paths, AgentConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let config = AgentConfig::load(&paths::config_file())
        .map_err(|e| anyhow::anyhow!("failed to load {}: {e}", paths::config_file()))?;
    logging::init(&config.agent.log_level);
    tracing::info!(node = %config.node.name, mode = %config.node.mode, "starting ports-agent");

    match config.node.mode.as_str() {
        "reverse" => transport::reverse::run(config).await,
        other => {
            tracing::warn!(
                "mode {other:?} does not run a daemon; backend reaches this node via \
                 `portsctl agent-rpc` over SSH"
            );
            Ok(())
        }
    }
}
