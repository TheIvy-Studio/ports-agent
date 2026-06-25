use std::io::{BufRead, Write};

use anyhow::Result;

use ports_common::{AgentConfig, RpcRequest, RpcResponse};

use crate::router::dispatch;

pub fn run(config: &AgentConfig) -> Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<RpcRequest>(&line) {
            Ok(req) => dispatch(&req, config),
            Err(e) => RpcResponse::error(None, format!("invalid request: {e}")),
        };
        writeln!(out, "{}", serde_json::to_string(&response)?)?;
        out.flush()?;
    }
    Ok(())
}
