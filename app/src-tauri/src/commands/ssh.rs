use std::fs;

#[derive(serde::Serialize)]
pub struct SshKey {
    pub name: String,
    pub content: String,
}

#[tauri::command]
pub async fn list_ssh_keys() -> Result<Vec<SshKey>, String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let ssh_dir = std::path::Path::new(&home).join(".ssh");

    if !ssh_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut keys = Vec::new();
    let entries = fs::read_dir(&ssh_dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("pub") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if let Ok(content) = fs::read_to_string(&path) {
            let content = content.trim().to_string();
            if !content.is_empty() {
                keys.push(SshKey { name, content });
            }
        }
    }
    keys.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(keys)
}
