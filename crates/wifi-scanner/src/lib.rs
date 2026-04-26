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
pub(crate) fn dedup_by_ssid(networks: &mut Vec<WifiNetwork>) {
    use std::collections::HashMap;

    let mut best: HashMap<String, WifiNetwork> = HashMap::new();
    for net in networks.drain(..) {
        let entry = best.entry(net.ssid.clone()).or_insert_with(|| net.clone());
        if net.signal_strength > entry.signal_strength {
            *entry = net;
        }
    }

    networks.extend(best.into_values());
    networks.sort_by_key(|n| std::cmp::Reverse(n.signal_strength));
}

// ── macOS ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn scan_networks_impl() -> Result<Vec<WifiNetwork>, WifiError> {
    let results = wifi_scan::scan().map_err(|e| WifiError::Parse(e.to_string()))?;
    let networks = results
        .into_iter()
        .filter(|w| !w.ssid.is_empty())
        .map(|w| WifiNetwork {
            ssid: w.ssid,
            signal_strength: w.signal_level,
            secured: !w.security.is_empty() && w.security != vec![wifi_scan::WifiSecurity::Open],
            frequency_ghz: None,
        })
        .collect();
    Ok(networks)
}

#[cfg(target_os = "macos")]
pub(crate) fn parse_system_profiler_output(text: &str) -> Result<Vec<WifiNetwork>, WifiError> {
    // system_profiler SPAirPortDataType output (macOS 15+):
    //
    //   Current Network Information:
    //     MyNetwork:
    //       Security: WPA2 Personal
    //       Signal / Noise: -54 dBm / -88 dBm
    //       Channel: 157 (5GHz, 80MHz)
    //   Other Local Wi-Fi Networks:
    //     AnotherNet:
    //       Security: WPA2 Personal
    //       Signal / Noise: -67 dBm / -94 dBm
    //       Channel: 6 (2GHz, 40MHz)
    //
    // Network names: lines ending with ':' that have no value after the colon,
    // at exactly 4 spaces indent inside a network block.

    let mut networks = Vec::new();
    let mut in_networks = false;
    let mut current_ssid: Option<String> = None;
    let mut current_signal: i32 = 0;
    let mut current_secured = true;
    let mut current_freq: Option<f32> = None;

    for line in text.lines() {
        let trimmed = line.trim();

        // Enter network listing sections
        if trimmed == "Current Network Information:" || trimmed == "Other Local Wi-Fi Networks:" {
            in_networks = true;
            continue;
        }

        // Exit network section when we hit a non-indented or shallowly-indented line
        if in_networks && !line.starts_with("      ") && !trimmed.is_empty() {
            if let Some(ssid) = current_ssid.take() {
                networks.push(WifiNetwork { ssid, signal_strength: current_signal, secured: current_secured, frequency_ghz: current_freq });
            }
            in_networks = false;
            continue;
        }

        if !in_networks {
            continue;
        }

        // Network name: indented, ends with ':', no value after colon
        if line.starts_with("            ") && !line.starts_with("              ") && trimmed.ends_with(':') {
            if let Some(ssid) = current_ssid.take() {
                networks.push(WifiNetwork { ssid, signal_strength: current_signal, secured: current_secured, frequency_ghz: current_freq });
            }
            let name = trimmed.trim_end_matches(':').to_string();
            if !name.is_empty() {
                current_ssid = Some(name);
                current_signal = 0;
                current_secured = true;
                current_freq = None;
            }
            continue;
        }

        if current_ssid.is_none() {
            continue;
        }

        // Signal / Noise: -54 dBm / -88 dBm
        if trimmed.starts_with("Signal / Noise:") {
            if let Some(rest) = trimmed.strip_prefix("Signal / Noise:") {
                let dbm = rest.trim().split_whitespace().next().unwrap_or("0");
                current_signal = dbm.parse().unwrap_or(0);
            }
            continue;
        }

        // Security: WPA2 Personal / None / Open
        if trimmed.starts_with("Security:") {
            let sec = trimmed.trim_start_matches("Security:").trim();
            current_secured = !sec.eq_ignore_ascii_case("none") && !sec.eq_ignore_ascii_case("open");
            continue;
        }

        // Channel: 157 (5GHz, 80MHz) or Channel: 6 (2GHz, 40MHz)
        if trimmed.starts_with("Channel:") {
            let ch_str = trimmed.trim_start_matches("Channel:").trim();
            if ch_str.contains("5GHz") {
                current_freq = Some(5.0);
            } else if ch_str.contains("2GHz") {
                current_freq = Some(2.4);
            }
            continue;
        }
    }

    // Flush last network
    if let Some(ssid) = current_ssid.take() {
        networks.push(WifiNetwork { ssid, signal_strength: current_signal, secured: current_secured, frequency_ghz: current_freq });
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
    parse_nmcli_output(&text)
}

#[cfg(target_os = "linux")]
pub(crate) fn parse_nmcli_output(text: &str) -> Result<Vec<WifiNetwork>, WifiError> {
    let mut networks = Vec::new();

    for line in text.lines() {
        // nmcli -t escapes ':' in SSIDs as '\:'. Replace escaped colons with a
        // placeholder before splitting so they don't get treated as field delimiters.
        const PLACEHOLDER: &str = "\x00";
        let sanitized = line.replace("\\:", PLACEHOLDER);
        let parts: Vec<&str> = sanitized.splitn(3, ':').collect();
        if parts.len() < 3 {
            continue;
        }

        let ssid = parts[0].trim().replace(PLACEHOLDER, ":").to_string();
        if ssid.is_empty() {
            continue;
        }

        let signal: i32 = parts[1].trim().parse().unwrap_or(0);
        let signal_strength = signal / 2 - 100;

        let security = parts[2].trim().replace(PLACEHOLDER, ":");
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
pub(crate) fn parse_netsh_output(text: &str) -> Result<Vec<WifiNetwork>, WifiError> {
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── macOS system_profiler parsing ────────────────────────────────────────

    #[cfg(target_os = "macos")]
    fn mock_profiler_output() -> &'static str {
        concat!(
            "Wi-Fi:\n\n",
            "      Interfaces:\n",
            "        en0:\n",
            "          Status: Connected\n",
            "          Current Network Information:\n",
            "            HomeNetwork:\n",
            "              PHY Mode: 802.11ax\n",
            "              Channel: 157 (5GHz, 80MHz)\n",
            "              Security: WPA2 Personal\n",
            "              Signal / Noise: -45 dBm / -90 dBm\n",
            "          Other Local Wi-Fi Networks:\n",
            "            OpenCafe:\n",
            "              Channel: 6 (2GHz, 40MHz)\n",
            "              Security: None\n",
            "              Signal / Noise: -60 dBm / -88 dBm\n",
            "            Office5G:\n",
            "              Channel: 36 (5GHz, 80MHz)\n",
            "              Security: WPA3 Personal\n",
            "              Signal / Noise: -70 dBm / -92 dBm\n",
        )
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn profiler_parses_secured_and_open() {
        let nets = parse_system_profiler_output(mock_profiler_output()).unwrap();
        assert_eq!(nets.len(), 3);
        let home = nets.iter().find(|n| n.ssid == "HomeNetwork").unwrap();
        assert_eq!(home.signal_strength, -45);
        assert!(home.secured);
        assert_eq!(home.frequency_ghz, Some(5.0));
        let cafe = nets.iter().find(|n| n.ssid == "OpenCafe").unwrap();
        assert_eq!(cafe.signal_strength, -60);
        assert!(!cafe.secured);
        assert_eq!(cafe.frequency_ghz, Some(2.4));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn profiler_empty_returns_empty() {
        let nets = parse_system_profiler_output("").unwrap();
        assert!(nets.is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn profiler_wpa3_is_secured() {
        let nets = parse_system_profiler_output(mock_profiler_output()).unwrap();
        let office = nets.iter().find(|n| n.ssid == "Office5G").unwrap();
        assert!(office.secured);
        assert_eq!(office.frequency_ghz, Some(5.0));
    }

    // ── Linux nmcli parsing ───────────────────────────────────────────────────

    #[cfg(target_os = "linux")]
    #[test]
    fn nmcli_parses_secured_and_open() {
        let input = "HomeNetwork:85:WPA2\nOpenCafe:60:--\n";
        let nets = parse_nmcli_output(input).unwrap();
        assert_eq!(nets.len(), 2);

        let home = nets.iter().find(|n| n.ssid == "HomeNetwork").unwrap();
        // signal 85 → 85/2 - 100 = -58
        assert_eq!(home.signal_strength, 85 / 2 - 100);
        assert!(home.secured);

        let cafe = nets.iter().find(|n| n.ssid == "OpenCafe").unwrap();
        assert!(!cafe.secured);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn nmcli_empty_ssid_skipped() {
        let input = ":50:WPA2\nRealNet:70:WPA2\n";
        let nets = parse_nmcli_output(input).unwrap();
        assert_eq!(nets.len(), 1);
        assert_eq!(nets[0].ssid, "RealNet");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn nmcli_ssid_with_colon_unescaped() {
        // nmcli escapes ':' in SSIDs as '\:'
        let input = "My\\:Net:80:WPA2\n";
        let nets = parse_nmcli_output(input).unwrap();
        assert_eq!(nets[0].ssid, "My:Net");
    }

    // ── Windows netsh parsing ─────────────────────────────────────────────────

    #[cfg(target_os = "windows")]
    #[test]
    fn netsh_parses_secured_and_open() {
        let input = "\
SSID 1                 : HomeNetwork\r\n\
Authentication         : WPA2-Personal\r\n\
Signal                 : 85%\r\n\
\r\n\
SSID 2                 : OpenCafe\r\n\
Authentication         : Open\r\n\
Signal                 : 60%\r\n";

        let nets = parse_netsh_output(input).unwrap();
        assert_eq!(nets.len(), 2);

        let home = nets.iter().find(|n| n.ssid == "HomeNetwork").unwrap();
        // 85% → 85/2-100 = -58
        assert_eq!(home.signal_strength, 85 / 2 - 100);
        assert!(home.secured);

        let cafe = nets.iter().find(|n| n.ssid == "OpenCafe").unwrap();
        assert!(!cafe.secured);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn netsh_empty_input_returns_empty() {
        let nets = parse_netsh_output("").unwrap();
        assert!(nets.is_empty());
    }

    // ── dedup_by_ssid ─────────────────────────────────────────────────────────

    #[test]
    fn dedup_keeps_stronger_signal() {
        let mut nets = vec![
            WifiNetwork { ssid: "Home".into(), signal_strength: -70, secured: true,  frequency_ghz: None },
            WifiNetwork { ssid: "Home".into(), signal_strength: -45, secured: true,  frequency_ghz: None },
            WifiNetwork { ssid: "Home".into(), signal_strength: -60, secured: false, frequency_ghz: None },
        ];
        dedup_by_ssid(&mut nets);
        assert_eq!(nets.len(), 1);
        assert_eq!(nets[0].ssid, "Home");
        assert_eq!(nets[0].signal_strength, -45);
    }

    #[test]
    fn dedup_preserves_distinct_ssids() {
        let mut nets = vec![
            WifiNetwork { ssid: "Net1".into(), signal_strength: -50, secured: true,  frequency_ghz: None },
            WifiNetwork { ssid: "Net2".into(), signal_strength: -60, secured: false, frequency_ghz: None },
        ];
        dedup_by_ssid(&mut nets);
        assert_eq!(nets.len(), 2);
    }

    #[test]
    fn dedup_sorts_by_signal_descending() {
        let mut nets = vec![
            WifiNetwork { ssid: "Weak".into(),   signal_strength: -80, secured: false, frequency_ghz: None },
            WifiNetwork { ssid: "Strong".into(), signal_strength: -30, secured: true,  frequency_ghz: None },
            WifiNetwork { ssid: "Mid".into(),    signal_strength: -55, secured: true,  frequency_ghz: None },
        ];
        dedup_by_ssid(&mut nets);
        assert_eq!(nets[0].ssid, "Strong");
        assert_eq!(nets[1].ssid, "Mid");
        assert_eq!(nets[2].ssid, "Weak");
    }

    #[test]
    fn dedup_empty_input_stays_empty() {
        let mut nets: Vec<WifiNetwork> = Vec::new();
        dedup_by_ssid(&mut nets);
        assert!(nets.is_empty());
    }
}
