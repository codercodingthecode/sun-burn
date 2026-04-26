#[derive(serde::Serialize, serde::Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: i32,
    pub secured: bool,
}

#[tauri::command]
pub async fn scan_wifi_networks() -> Result<Vec<WifiNetwork>, String> {
    wifi_scanner::scan_networks()
        .map_err(|e| e.to_string())
        .map(|networks| {
            networks
                .into_iter()
                .map(|n| WifiNetwork {
                    ssid: n.ssid,
                    signal: n.signal_strength,
                    secured: n.secured,
                })
                .collect()
        })
}
