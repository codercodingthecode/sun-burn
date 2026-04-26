#[derive(serde::Serialize, serde::Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal_strength: i32,
    pub secured: bool,
    pub frequency_ghz: Option<f32>,
}

#[tauri::command]
pub async fn scan_wifi_networks() -> Result<Vec<WifiNetwork>, String> {
    wifi_scanner::scan_networks()
        .map_err(|e| e.to_string())
        .map(|networks| {
            networks
                .into_iter()
                .filter(|n| !n.ssid.is_empty())
                .map(|n| WifiNetwork {
                    ssid: n.ssid,
                    signal_strength: n.signal_strength,
                    secured: n.secured,
                    frequency_ghz: n.frequency_ghz,
                })
                .collect()
        })
}
