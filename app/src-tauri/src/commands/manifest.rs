#[tauri::command]
pub async fn read_manifest(image_path: String) -> Result<Option<serde_json::Value>, String> {
    let patcher = image_patcher::ImagePatcher::open(&image_path)
        .map_err(|e| e.to_string())?;

    match patcher.read_manifest().map_err(|e| e.to_string())? {
        None => Ok(None),
        Some(manifest) => {
            let value = serde_json::to_value(&manifest).map_err(|e| e.to_string())?;
            Ok(Some(value))
        }
    }
}
