use thiserror::Error;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Drive {
    pub path: String,
    pub display_name: String,
    pub size_bytes: u64,
    pub removable: bool,
}

#[derive(Debug, Error)]
pub enum EnumeratorError {
    #[error("failed to run subprocess: {0}")]
    Subprocess(#[from] std::io::Error),
    #[error("failed to parse output: {0}")]
    Parse(String),
}

pub fn list_drives() -> Result<Vec<Drive>, EnumeratorError> {
    list_drives_impl()
}

// ── macOS ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn list_drives_impl() -> Result<Vec<Drive>, EnumeratorError> {
    use plist::Value;
    use std::process::Command;

    let output = Command::new("diskutil")
        .args(["list", "-plist", "external"])
        .output()?;

    let plist_bytes = output.stdout.as_slice();
    let value = Value::from_reader(std::io::Cursor::new(plist_bytes))
        .map_err(|e| EnumeratorError::Parse(e.to_string()))?;

    let mut drives = Vec::new();

    let dict = value
        .as_dictionary()
        .ok_or_else(|| EnumeratorError::Parse("expected plist dictionary".into()))?;

    let all_disks = dict
        .get("AllDisksAndPartitions")
        .and_then(|v| v.as_array())
        .ok_or_else(|| EnumeratorError::Parse("missing AllDisksAndPartitions".into()))?;

    for disk in all_disks {
        let disk_dict = match disk.as_dictionary() {
            Some(d) => d,
            None => continue,
        };

        let device_id = match disk_dict.get("DeviceIdentifier").and_then(|v| v.as_string()) {
            Some(id) => id,
            None => continue,
        };

        let path = format!("/dev/{}", device_id);

        let size_bytes = disk_dict
            .get("Size")
            .and_then(|v| v.as_unsigned_integer())
            .unwrap_or(0);

        let media_name = disk_dict
            .get("MediaName")
            .and_then(|v| v.as_string())
            .unwrap_or("")
            .to_string();

        let display_name = if media_name.is_empty() {
            format!("{} ({:.0} GB)", device_id, size_bytes as f64 / 1e9)
        } else {
            format!("{} ({:.0} GB)", media_name, size_bytes as f64 / 1e9)
        };

        drives.push(Drive {
            path,
            display_name,
            size_bytes,
            removable: true,
        });
    }

    Ok(drives)
}

// ── Linux ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn list_drives_impl() -> Result<Vec<Drive>, EnumeratorError> {
    use std::process::Command;

    let output = Command::new("lsblk")
        .args(["-J", "-b", "-o", "NAME,SIZE,RM,MODEL,TYPE"])
        .output()?;

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| EnumeratorError::Parse(e.to_string()))?;

    let blockdevices = json["blockdevices"]
        .as_array()
        .ok_or_else(|| EnumeratorError::Parse("missing blockdevices".into()))?;

    let mut drives = Vec::new();

    for dev in blockdevices {
        let dev_type = dev["type"].as_str().unwrap_or("");
        if dev_type != "disk" {
            continue;
        }

        // rm can be a boolean or the string "1"
        let removable = match &dev["rm"] {
            serde_json::Value::Bool(b) => *b,
            serde_json::Value::String(s) => s == "1",
            serde_json::Value::Number(n) => n.as_u64().unwrap_or(0) == 1,
            _ => false,
        };

        if !removable {
            continue;
        }

        let name = dev["name"].as_str().unwrap_or("").to_string();
        if name.is_empty() {
            continue;
        }

        let path = format!("/dev/{}", name);

        let size_bytes = match &dev["size"] {
            serde_json::Value::Number(n) => n.as_u64().unwrap_or(0),
            serde_json::Value::String(s) => s.parse::<u64>().unwrap_or(0),
            _ => 0,
        };

        let model = dev["model"].as_str().unwrap_or("").trim().to_string();
        let display_name = if model.is_empty() {
            format!("{} ({:.0} GB)", name, size_bytes as f64 / 1e9)
        } else {
            format!("{} ({:.0} GB)", model, size_bytes as f64 / 1e9)
        };

        drives.push(Drive {
            path,
            display_name,
            size_bytes,
            removable: true,
        });
    }

    Ok(drives)
}

// ── Windows ───────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn list_drives_impl() -> Result<Vec<Drive>, EnumeratorError> {
    use std::process::Command;

    let output = Command::new("wmic")
        .args([
            "diskdrive",
            "where",
            "MediaType='Removable Media' OR InterfaceType='USB'",
            "get",
            "DeviceID,Size,Model",
            "/format:csv",
        ])
        .output()?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut drives = Vec::new();

    // CSV format: Node,DeviceID,Model,Size  (header + blank line + rows)
    // wmic /format:csv puts an extra blank line before data rows
    let mut header: Option<Vec<String>> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let cols: Vec<String> = line.split(',').map(|s| s.trim().to_string()).collect();

        if header.is_none() {
            // First non-blank line is the header
            header = Some(cols);
            continue;
        }

        let hdr = header.as_ref().unwrap();
        let get = |name: &str| -> String {
            hdr.iter()
                .zip(cols.iter())
                .find(|(h, _)| h.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };

        let device_id = get("DeviceID");
        let model = get("Model");
        let size_str = get("Size");

        if device_id.is_empty() {
            continue;
        }

        let size_bytes: u64 = size_str.parse().unwrap_or(0);
        let display_name = if model.is_empty() {
            format!("{} ({:.0} GB)", device_id, size_bytes as f64 / 1e9)
        } else {
            format!("{} ({:.0} GB)", model, size_bytes as f64 / 1e9)
        };

        drives.push(Drive {
            path: device_id,
            display_name,
            size_bytes,
            removable: true,
        });
    }

    Ok(drives)
}

// ── Fallback for unsupported platforms ───────────────────────────────────────

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn list_drives_impl() -> Result<Vec<Drive>, EnumeratorError> {
    Ok(Vec::new())
}
