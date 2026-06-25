use std::time::Duration;

use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use ports_common::{msg, AgentConfig, RpcRequest, RpcResponse};

use crate::core::{keys, telemetry, AGENT_VERSION};
use crate::router::dispatch;

pub async fn run(config: AgentConfig) -> Result<()> {
    let url = config.backend.websocket_url.clone();
    if url.is_empty() {
        return Err(anyhow!("backend.websocket_url is empty"));
    }
    loop {
        match connect_once(&config, &url).await {
            Ok(()) => tracing::warn!("connection closed; reconnecting in 5s"),
            Err(e) => tracing::error!("connection error: {e}; reconnecting in 5s"),
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn connect_once(config: &AgentConfig, url: &str) -> Result<()> {
    let (ws, _) = connect_async(url).await?;
    let (mut write, mut read) = ws.split();

    let signature = keys::sign_b64(&config.auth.key_path, config.node.id.as_bytes())?;
    let hello = json!({
        "type": msg::HELLO,
        "payload": {
            "nodeId": config.node.id,
            "agentVersion": AGENT_VERSION,
            "signature": signature,
        }
    });
    write.send(Message::Text(hello.to_string())).await?;
    tracing::info!(node = %config.node.name, "connected to backend");

    let mut ticker = tokio::time::interval(Duration::from_secs(15));
    loop {
        tokio::select! {
            incoming = read.next() => match incoming {
                Some(Ok(Message::Text(text))) => {
                    if let Some(reply) = handle_text(&text, config) {
                        write.send(Message::Text(reply)).await?;
                    }
                }
                Some(Ok(Message::Ping(payload))) => write.send(Message::Pong(payload)).await?,
                Some(Ok(Message::Close(_))) | None => return Ok(()),
                Some(Ok(_)) => {}
                Some(Err(e)) => return Err(e.into()),
            },
            _ = ticker.tick() => {
                if config.features.traffic {
                    let frame = json!({
                        "type": msg::TELEMETRY_SNAPSHOT,
                        "payload": telemetry::snapshot(),
                    });
                    write.send(Message::Text(frame.to_string())).await?;
                }
            }
        }
    }
}

fn handle_text(text: &str, config: &AgentConfig) -> Option<String> {
    let response = match serde_json::from_str::<RpcRequest>(text) {
        Ok(req) => dispatch(&req, config),
        Err(e) => RpcResponse::error(None, format!("invalid request: {e}")),
    };
    serde_json::to_string(&response).ok()
}
