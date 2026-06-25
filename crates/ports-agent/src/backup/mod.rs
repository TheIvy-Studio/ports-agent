use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn create(scope: &str, paths: &[String]) -> Result<Value, String> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let backup_dir = format!("/var/lib/ports/backups/{scope}/{ts}");
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    let mut checksums = serde_json::Map::new();
    for path in paths {
        if Path::new(path).exists() {
            let dest = format!("{backup_dir}/{}", path.replace('/', "__"));
            let _ = Command::new("cp").args(["-a", path, &dest]).status();
            if let Ok(meta) = fs::metadata(&dest) {
                checksums.insert(path.clone(), json!({"size": meta.len()}));
            }
        }
    }

    Ok(json!({
        "scope": scope,
        "remotePath": backup_dir,
        "paths": paths,
        "checksums": checksums,
    }))
}

pub fn restore(_scope: &str, remote_path: &str, paths: &[String]) -> Result<Value, String> {
    for path in paths {
        let src = format!("{remote_path}/{}", path.replace('/', "__"));
        if Path::new(&src).exists() {
            let parent = Path::new(path).parent().map(|p| p.to_string_lossy().to_string());
            if let Some(p) = parent {
                let _ = Command::new("mkdir").args(["-p", &p]).status();
            }
            Command::new("cp").args(["-a", &src, path]).status().map_err(|e| e.to_string())?;
        }
    }
    Ok(json!({ "status": "restored" }))
}
