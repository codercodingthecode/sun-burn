#[derive(serde::Serialize, serde::Deserialize)]
pub struct Drive {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub removable: bool,
}

#[tauri::command]
pub async fn list_removable_drives() -> Result<Vec<Drive>, String> {
    device_enumerator::list_drives()
        .map_err(|e| e.to_string())
        .map(|drives| {
            drives
                .into_iter()
                .map(|d| Drive {
                    path: d.path,
                    name: d.display_name,
                    size_bytes: d.size_bytes,
                    removable: d.removable,
                })
                .collect()
        })
}
