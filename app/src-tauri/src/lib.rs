pub mod commands;

use commands::{devices, flash, manifest, wifi};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            manifest::read_manifest,
            wifi::scan_wifi_networks,
            devices::list_removable_drives,
            flash::patch_image,
            flash::flash_image,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
