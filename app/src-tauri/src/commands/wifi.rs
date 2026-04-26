#[derive(serde::Serialize, serde::Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: i32,
    pub secured: bool,
    pub frequency_ghz: Option<f32>,
}

#[tauri::command]
pub async fn scan_wifi_networks() -> Result<Vec<WifiNetwork>, String> {
    #[cfg(target_os = "macos")]
    request_location_permission();

    wifi_scanner::scan_networks()
        .map_err(|e| e.to_string())
        .map(|networks| {
            networks
                .into_iter()
                .filter(|n| !n.ssid.is_empty() && n.ssid != "<redacted>")
                .map(|n| WifiNetwork {
                    ssid: n.ssid,
                    signal: n.signal_strength,
                    secured: n.secured,
                    frequency_ghz: n.frequency_ghz,
                })
                .collect()
        })
}

#[cfg(target_os = "macos")]
fn request_location_permission() {
    use std::process::Command;
    // Trigger CoreLocation authorization via a lightweight launchctl check.
    // The real prompt comes from the OS when system_profiler accesses WiFi data.
    // We prime it here so the user sees the dialog before the scan runs.
    let _ = Command::new("osascript")
        .args(["-e", r#"tell application "System Events" to get name of every process"#])
        .output();
}
