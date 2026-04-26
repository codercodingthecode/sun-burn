use thiserror::Error;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal_strength: i32,
    pub secured: bool,
    pub frequency_ghz: Option<f32>,
}

#[derive(Debug, Error)]
pub enum WifiError {
    #[error("failed to run subprocess: {0}")]
    Subprocess(#[from] std::io::Error),
    #[error("failed to parse output: {0}")]
    Parse(String),
}

pub fn scan_networks() -> Result<Vec<WifiNetwork>, WifiError> {
    let mut networks = scan_networks_impl()?;
    dedup_by_ssid(&mut networks);
    Ok(networks)
}

/// Keep the entry with the strongest signal for each SSID.
fn dedup_by_ssid(networks: &mut Vec<WifiNetwork>) {
    use std::collections::HashMap;

    let mut best: HashMap<String, WifiNetwork> = HashMap::new();
    for net in networks.drain(..) {
        let entry = best.entry(net.ssid.clone()).or_insert_with(|| net.clone());
        if net.signal_strength > entry.signal_strength {
            *entry = net;
        }
    }

    networks.extend(best.into_values());
    networks.sort_by(|a, b| b.signal_strength.cmp(&a.signal_strength));
}

// ── macOS ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn scan_networks_impl() -> Result<Vec<WifiNetwork>, WifiError> {
    use std::process::Command;

    let output = Command::new(
        "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport",
    )
    .arg("-s")
    .output()?;

    let text = String::from_utf8_lossy(&output.stdout);
    parse_airport_output(&text)
}

#[cfg(target_os = "macos")]
fn parse_airport_output(text: &str) -> Result<Vec<WifiNetwork>, WifiError> {
    // airport -s output looks like:
    //                             SSID BSSID             RSSI CHANNEL HT CC SECURITY (auth/unicast/group)
    //                         MyNet01 aa:bb:cc:dd:ee:ff  -65  6       Y  US WPA2(PSK/AES/AES)
    //
    // SSID is right-justified into the first 32 chars, followed by a space and the rest.
    // We detect the header line to find column offsets, then parse subsequent lines.

    let mut lines = text.lines();

    // Find the header line (contains "SSID" and "BSSID")
    let header_line = loop {
        match lines.next() {
            None => return Ok(Vec::new()),
            Some(l) if l.contains("SSID") && l.contains("BSSID") => break l,
            _ => continue,
        }
    };

    // Locate column start offsets from the header
    let bssid_col = header_line.find("BSSID").unwrap_or(33);
    let rssi_col = header_line.find("RSSI").unwrap_or(bssid_col + 18);
    let security_col = header_line.find("SECURITY").unwrap_or(rssi_col + 30);

    let mut networks = Vec::new();

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }

        // SSID occupies everything up to bssid_col, right-padded with spaces
        if line.len() < bssid_col {
            continue;
        }

        let ssid = line[..bssid_col].trim().to_string();
        if ssid.is_empty() {
            continue;
        }

        // RSSI: parse the number that appears at rssi_col
        let rssi_str = if line.len() > rssi_col {
            line[rssi_col..].split_whitespace().next().unwrap_or("0")
        } else {
            "0"
        };
        let signal_strength: i32 = rssi_str.parse().unwrap_or(0);

        // Security: last column
        let security = if line.len() > security_col {
            line[security_col..].split_whitespace().next().unwrap_or("NONE")
        } else {
            "NONE"
        };
        let secured = !security.eq_ignore_ascii_case("NONE");

        // Channel → rough frequency mapping
        // Channels 1-14 → 2.4 GHz, 36+ → 5 GHz
        let channel_str = {
            let after_rssi = if line.len() > rssi_col { &line[rssi_col..] } else { "" };
            let mut parts = after_rssi.split_whitespace();
            parts.next(); // skip RSSI
            parts.next().unwrap_or("").to_string()
        };
        let frequency_ghz = channel_str
            .split(',')
            .next()
            .and_then(|c| c.trim().parse::<u32>().ok())
            .map(|ch| if ch <= 14 { 2.4_f32 } else { 5.0_f32 });

        networks.push(WifiNetwork {
            ssid,
            signal_strength,
            secured,
            frequency_ghz,
        });
    }

    Ok(networks)
}

// ── Linux ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn scan_networks_impl() -> Result<Vec<WifiNetwork>, WifiError> {
    // Try nmcli first, fall back to iwlist
    match try_nmcli() {
        Ok(nets) if !nets.is_empty() => return Ok(nets),
        Ok(_) => {}
        Err(_) => {}
    }
    try_iwlist()
}

#[cfg(target_os = "linux")]
fn try_nmcli() -> Result<Vec<WifiNetwork>, WifiError> {
    use std::process::Command;

    let output = Command::new("nmcli")
        .args(["-t", "-f", "SSID,SIGNAL,SECURITY", "device", "wifi", "list"])
        .output()?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut networks = Vec::new();

    for line in text.lines() {
        // nmcli -t separates fields with ':' but SSIDs may contain ':'
        // Format: SSID:SIGNAL:SECURITY
        // We split from the right to isolate SECURITY and SIGNAL
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() < 3 {
            continue;
        }
        // With -t and these 3 fields: parts[0]=SSID, parts[1]=SIGNAL, parts[2]=SECURITY
        // But SSID itself may have been split — nmcli escapes ':' as '\:'
        // splitn(3,...) keeps any ':' in the security field together; SSID comes first.
        let ssid = parts[0].trim().replace("\\:", ":").to_string();
        if ssid.is_empty() {
            continue;
        }

        let signal: i32 = parts[1].trim().parse().unwrap_or(0);
        // Convert 0-100 percentage to approximate dBm: dBm ≈ signal/2 - 100
        let signal_strength = signal / 2 - 100;

        let security = parts[2].trim();
        let secured = security != "--" && !security.is_empty();

        networks.push(WifiNetwork {
            ssid,
            signal_strength,
            secured,
            frequency_ghz: None,
        });
    }

    Ok(networks)
}

#[cfg(target_os = "linux")]
fn try_iwlist() -> Result<Vec<WifiNetwork>, WifiError> {
    use std::process::Command;

    // Find the first wireless interface
    let iw_output = Command::new("iwlist").args(["scan"]).output()?;

    let text = String::from_utf8_lossy(&iw_output.stdout);
    let mut networks = Vec::new();
    let mut current_ssid: Option<String> = None;
    let mut current_signal: i32 = 0;
    let mut current_secured = false;
    let mut current_freq: Option<f32> = None;

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("Cell ") {
            // Save previous network if any
            if let Some(ssid) = current_ssid.take() {
                if !ssid.is_empty() {
                    networks.push(WifiNetwork {
                        ssid,
                        signal_strength: current_signal,
                        secured: current_secured,
                        frequency_ghz: current_freq,
                    });
                }
            }
            current_signal = 0;
            current_secured = false;
            current_freq = None;
        } else if trimmed.starts_with("ESSID:") {
            let raw = trimmed.trim_start_matches("ESSID:").trim().trim_matches('"');
            current_ssid = Some(raw.to_string());
        } else if trimmed.starts_with("Signal level=") || trimmed.contains("Signal level=") {
            // "Signal level=-65 dBm" or "Quality=40/70  Signal level=-70 dBm"
            if let Some(pos) = trimmed.find("Signal level=") {
                let rest = &trimmed[pos + "Signal level=".len()..];
                let val_str = rest.split_whitespace().next().unwrap_or("0");
                current_signal = val_str.trim_end_matches("dBm").parse().unwrap_or(0);
            }
        } else if trimmed.starts_with("Frequency:") {
            // "Frequency:2.437 GHz" or "Frequency:5.180 GHz"
            let rest = trimmed.trim_start_matches("Frequency:").trim();
            if let Some(v) = rest.split_whitespace().next() {
                current_freq = v.parse::<f32>().ok();
            }
        } else if trimmed.starts_with("Encryption key:") {
            current_secured = trimmed.contains("on");
        }
    }

    // Don't forget the last one
    if let Some(ssid) = current_ssid.take() {
        if !ssid.is_empty() {
            networks.push(WifiNetwork {
                ssid,
                signal_strength: current_signal,
                secured: current_secured,
                frequency_ghz: current_freq,
            });
        }
    }

    Ok(networks)
}

// ── Windows ───────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn scan_networks_impl() -> Result<Vec<WifiNetwork>, WifiError> {
    use std::process::Command;

    let output = Command::new("netsh")
        .args(["wlan", "show", "networks", "mode=bssid"])
        .output()?;

    let text = String::from_utf8_lossy(&output.stdout);
    parse_netsh_output(&text)
}

#[cfg(target_os = "windows")]
fn parse_netsh_output(text: &str) -> Result<Vec<WifiNetwork>, WifiError> {
    // netsh output groups networks with blank-line separated blocks like:
    //
    // SSID 1 : MyNetwork
    //  Network type            : Infrastructure
    //  Authentication          : WPA2-Personal
    //  Encryption              : CCMP
    //  BSSID 1                 : aa:bb:cc:dd:ee:ff
    //   Signal             : 78%
    //   Channel            : 6

    let mut networks = Vec::new();
    let mut current_ssid: Option<String> = None;
    let mut current_signal: i32 = 0;
    let mut current_secured = false;

    for line in text.lines() {
        let line = line.trim();

        if line.starts_with("SSID") && line.contains(':') && !line.starts_with("BSSID") {
            // Save previous if present
            if let Some(ssid) = current_ssid.take() {
                if !ssid.is_empty() {
                    networks.push(WifiNetwork {
                        ssid,
                        signal_strength: current_signal,
                        secured: current_secured,
                        frequency_ghz: None,
                    });
                }
            }
            current_signal = 0;
            current_secured = false;
            let val = line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
            current_ssid = Some(val);
        } else if line.starts_with("Signal") && line.contains(':') {
            // "Signal             : 78%"
            let val = line.splitn(2, ':').nth(1).unwrap_or("").trim().trim_end_matches('%');
            let pct: i32 = val.parse().unwrap_or(0);
            // Convert percentage to approximate dBm
            current_signal = pct / 2 - 100;
        } else if line.starts_with("Authentication") && line.contains(':') {
            let val = line.splitn(2, ':').nth(1).unwrap_or("").trim();
            current_secured = !val.eq_ignore_ascii_case("Open") && !val.is_empty();
        }
    }

    // Last block
    if let Some(ssid) = current_ssid.take() {
        if !ssid.is_empty() {
            networks.push(WifiNetwork {
                ssid,
                signal_strength: current_signal,
                secured: current_secured,
                frequency_ghz: None,
            });
        }
    }

    Ok(networks)
}

// ── Fallback ──────────────────────────────────────────────────────────────────

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn scan_networks_impl() -> Result<Vec<WifiNetwork>, WifiError> {
    Ok(Vec::new())
}
