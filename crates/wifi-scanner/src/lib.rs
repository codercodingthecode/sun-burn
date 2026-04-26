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
//
// macOS 15+ enforces Location permission before disclosing SSIDs via any
// CoreWLAN or system_profiler API (CWNetwork.ssid returns nil, system_profiler
// returns "<redacted>").  The only way to get real SSIDs without Location
// permission is `networksetup -listpreferredwirelessnetworks <iface>`, which
// returns the user's saved/known networks.  This is the right list for the
// "pick a network to configure your device for" use-case anyway.

#[cfg(target_os = "macos")]
fn scan_networks_impl() -> Result<Vec<WifiNetwork>, WifiError> {
    use std::process::Command;

    // Find the WiFi interface (usually en0, but discover it dynamically).
    let iface = find_wifi_interface().unwrap_or_else(|| "en0".to_string());

    let output = Command::new("networksetup")
        .args(["-listpreferredwirelessnetworks", &iface])
        .output()?;

    let text = String::from_utf8_lossy(&output.stdout);
    parse_preferred_networks_output(&text)
}

/// Parses the WiFi interface name from `networksetup -listallhardwareports`.
/// Returns `None` if no Wi-Fi interface is found.
#[cfg(target_os = "macos")]
pub(crate) fn find_wifi_interface() -> Option<String> {
    use std::process::Command;

    let output = Command::new("networksetup")
        .args(["-listallhardwareports"])
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout);
    parse_wifi_interface_from_hardware_ports(&text)
}

/// Pure parser for `networksetup -listallhardwareports` output.
/// Looks for the block whose Hardware Port is "Wi-Fi" and returns its Device.
#[cfg(target_os = "macos")]
pub(crate) fn parse_wifi_interface_from_hardware_ports(text: &str) -> Option<String> {
    let mut in_wifi_block = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Hardware Port:") {
            let port = trimmed["Hardware Port:".len()..].trim();
            in_wifi_block = port.eq_ignore_ascii_case("wi-fi")
                || port.eq_ignore_ascii_case("airport");
        } else if in_wifi_block && trimmed.starts_with("Device:") {
            return Some(trimmed["Device:".len()..].trim().to_string());
        }
    }
    None
}

/// Parses `networksetup -listpreferredwirelessnetworks <iface>` output.
///
/// Output format:
///   Preferred networks on en0:
///   \t<ssid1>
///   \t<ssid2>
///   ...
///
/// Each SSID is tab-indented.  The first line is a header.
/// Signal strength is unknown (0) because macOS redacts RSSI from this
/// command — the list is already sorted by preference/recency.
#[cfg(target_os = "macos")]
pub(crate) fn parse_preferred_networks_output(text: &str) -> Result<Vec<WifiNetwork>, WifiError> {
    let mut networks = Vec::new();
    for line in text.lines() {
        // Header line: "Preferred networks on en0:" — skip it.
        if line.starts_with("Preferred networks") {
            continue;
        }
        // Each SSID is tab-indented.
        let ssid = line.trim_start_matches('\t').trim();
        if ssid.is_empty() {
            continue;
        }
        networks.push(WifiNetwork {
            ssid: ssid.to_string(),
            // RSSI is unavailable without Location permission.
            // Use 0 as a neutral sentinel; the UI should suppress the dBm label for 0.
            signal_strength: 0,
            // Security info is also unavailable from this command.
            // Default to secured=true as a conservative assumption for the UI.
            secured: true,
            frequency_ghz: None,
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

    // ── macOS preferred-networks parsing ─────────────────────────────────────

    #[cfg(target_os = "macos")]
    #[test]
    fn preferred_networks_parses_list() {
        let input = "Preferred networks on en0:\n\tHomeNetwork\n\tOpenCafe\n\tOffice5G\n";
        let nets = parse_preferred_networks_output(input).unwrap();
        assert_eq!(nets.len(), 3);
        assert_eq!(nets[0].ssid, "HomeNetwork");
        assert_eq!(nets[1].ssid, "OpenCafe");
        assert_eq!(nets[2].ssid, "Office5G");
        // Signal is 0 (RSSI unavailable without Location)
        assert_eq!(nets[0].signal_strength, 0);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn preferred_networks_empty_returns_empty() {
        let nets = parse_preferred_networks_output("").unwrap();
        assert!(nets.is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn preferred_networks_skips_empty_lines() {
        let input = "Preferred networks on en0:\n\tMyNet\n\n\tOtherNet\n";
        let nets = parse_preferred_networks_output(input).unwrap();
        assert_eq!(nets.len(), 2);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn hardware_ports_finds_wifi_interface() {
        let input = concat!(
            "Hardware Port: Ethernet\nDevice: en1\nEthernet Address: aa:bb:cc:dd:ee:ff\n\n",
            "Hardware Port: Wi-Fi\nDevice: en0\nEthernet Address: f4:d4:88:79:c9:d5\n\n",
            "Hardware Port: Bluetooth PAN\nDevice: en3\n",
        );
        let iface = parse_wifi_interface_from_hardware_ports(input);
        assert_eq!(iface, Some("en0".to_string()));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn hardware_ports_airport_alias_works() {
        let input = "Hardware Port: AirPort\nDevice: en0\nEthernet Address: f4:d4:88:79:c9:d5\n";
        let iface = parse_wifi_interface_from_hardware_ports(input);
        assert_eq!(iface, Some("en0".to_string()));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn hardware_ports_no_wifi_returns_none() {
        let input = "Hardware Port: Ethernet\nDevice: en1\nEthernet Address: aa:bb:cc:dd:ee:ff\n";
        let iface = parse_wifi_interface_from_hardware_ports(input);
        assert!(iface.is_none());
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
