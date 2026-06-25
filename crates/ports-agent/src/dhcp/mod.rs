use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::core::system;

struct Backend {
    binary: &'static str,
    test_args: &'static [&'static str],
    service: &'static str,
    suffix: &'static str,
}

fn backend_for(name: &str) -> Backend {
    if name == "kea" {
        Backend {
            binary: "kea-dhcp4",
            test_args: &["-t"],
            service: "kea-dhcp4",
            suffix: "conf",
        }
    } else {
        Backend {
            binary: "dnsmasq",
            test_args: &["--test", "--conf-file"],
            service: "dnsmasq",
            suffix: "conf",
        }
    }
}

pub fn plan(backend: &str, config: &str) -> Value {
    validate(&backend_for(backend), config)
}

fn validate(backend: &Backend, config: &str) -> Value {
    if !system::which(backend.binary) {
        return json!({ "valid": false, "output": format!("{} is not installed", backend.binary) });
    }
    let tmp = format!("/tmp/ports-{}-{}.{}", backend.binary, std::process::id(), backend.suffix);
    if fs::write(&tmp, config).is_err() {
        return json!({ "valid": false, "output": "failed to write temporary config" });
    }
    let mut cmd = Command::new(backend.binary);
    cmd.args(backend.test_args).arg(&tmp);
    let output = cmd.output();
    let _ = fs::remove_file(&tmp);
    match output {
        Ok(o) => {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            json!({ "valid": o.status.success(), "output": text.trim() })
        }
        Err(e) => json!({ "valid": false, "output": format!("{} test failed: {e}", backend.binary) }),
    }
}

pub fn apply(backend: &str, config_path: &str, config: &str) -> Result<Value, String> {
    let backend = backend_for(backend);
    let validation = validate(&backend, config);
    if !validation.get("valid").and_then(|v| v.as_bool()).unwrap_or(false) {
        let output = validation.get("output").and_then(|v| v.as_str()).unwrap_or("");
        return Err(format!("config validation failed: {output}"));
    }
    if let Some(parent) = Path::new(config_path).parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(config_path, config).map_err(|e| e.to_string())?;
    let reloaded = reload(&backend);
    Ok(json!({
        "status": "applied",
        "configPath": config_path,
        "backend": backend.binary,
        "reloaded": reloaded,
    }))
}

fn reload(backend: &Backend) -> bool {
    if !system::which("systemctl") {
        return false;
    }
    Command::new("systemctl")
        .args(["reload-or-restart", backend.service])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
