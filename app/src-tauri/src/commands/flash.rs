use tauri::Emitter;

#[tauri::command]
pub async fn patch_image(
    image_path: String,
    values: std::collections::HashMap<String, String>,
) -> Result<(), String> {
    let mut patcher = image_patcher::ImagePatcher::open(&image_path)
        .map_err(|e| e.to_string())?;

    let manifest = patcher
        .read_manifest()
        .map_err(|e| e.to_string())?;

    if let Some(manifest) = manifest {
        for step in &manifest.steps {
            patcher
                .apply_writes(step, &values)
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn flash_image(
    window: tauri::Window,
    image_path: String,
    drive_path: String,
) -> Result<(), String> {
    let flasher = flasher::Flasher::new(&image_path, &drive_path);

    flasher
        .flash(move |progress| {
            let _ = window.emit("flash-progress", &progress);
        })
        .map_err(|e| e.to_string())
}
