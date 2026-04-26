use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Serialize)]
pub struct FlashProgress {
    pub bytes_written: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
}

#[derive(Debug, Error)]
pub enum FlashError {
    #[error("source image not found: {0}")]
    SourceNotFound(PathBuf),

    #[error("destination device not found: {0}")]
    DestinationNotFound(String),

    #[error("permission denied accessing {0}")]
    PermissionDenied(String),

    #[error("unmount failed: {0}")]
    UnmountFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct Flasher {
    source_path: PathBuf,
    dest_path: String,
}

/// Convert a macOS block-device path to its raw (character-device) equivalent.
/// `/dev/diskN` → `/dev/rdiskN`; other paths are returned unchanged.
pub(crate) fn to_raw_device(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("/dev/disk") {
        format!("/dev/rdisk{}", stripped)
    } else {
        path.to_string()
    }
}

impl Flasher {
    pub fn new(source_path: impl AsRef<Path>, dest_path: impl Into<String>) -> Self {
        Self {
            source_path: source_path.as_ref().to_path_buf(),
            dest_path: dest_path.into(),
        }
    }

    pub fn flash<F>(&self, on_progress: F) -> Result<(), FlashError>
    where
        F: Fn(FlashProgress) + Send + 'static,
    {
        if !self.source_path.exists() {
            return Err(FlashError::SourceNotFound(self.source_path.clone()));
        }

        let total_bytes = self.source_path.metadata()?.len();

        #[cfg(target_os = "macos")]
        {
            self.flash_macos(total_bytes, on_progress)
        }

        #[cfg(target_os = "linux")]
        {
            self.flash_linux(total_bytes, on_progress)
        }

        #[cfg(target_os = "windows")]
        {
            self.flash_windows(total_bytes, on_progress)
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            Err(FlashError::Io(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "unsupported platform",
            )))
        }
    }

    #[cfg(target_os = "macos")]
    fn flash_macos<F>(&self, total_bytes: u64, on_progress: F) -> Result<(), FlashError>
    where
        F: Fn(FlashProgress) + Send + 'static,
    {
        // Unmount the disk before writing
        let disk = &self.dest_path;
        let unmount_output = std::process::Command::new("diskutil")
            .args(["unmountDisk", disk])
            .output()
            .map_err(|e| FlashError::UnmountFailed(e.to_string()))?;

        if !unmount_output.status.success() {
            let stderr = String::from_utf8_lossy(&unmount_output.stderr).into_owned();
            return Err(FlashError::UnmountFailed(stderr));
        }

        // Convert /dev/diskN to /dev/rdiskN for raw (faster) access
        let raw_dest = to_raw_device(disk);

        let dest_exists = Path::new(&raw_dest).exists();
        if !dest_exists {
            return Err(FlashError::DestinationNotFound(raw_dest.clone()));
        }

        let mut dest = OpenOptions::new()
            .write(true)
            .open(&raw_dest)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    FlashError::PermissionDenied(raw_dest.clone())
                } else {
                    FlashError::Io(e)
                }
            })?;

        self.write_with_progress(total_bytes, &mut dest, on_progress)?;

        // Eject after writing
        let _ = std::process::Command::new("diskutil")
            .args(["eject", disk])
            .output();

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn flash_linux<F>(&self, total_bytes: u64, on_progress: F) -> Result<(), FlashError>
    where
        F: Fn(FlashProgress) + Send + 'static,
    {
        // Sync before writing
        let _ = std::process::Command::new("sync").output();

        let dest_path = &self.dest_path;
        if !Path::new(dest_path).exists() {
            return Err(FlashError::DestinationNotFound(dest_path.clone()));
        }

        let mut dest = OpenOptions::new()
            .write(true)
            .open(dest_path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    FlashError::PermissionDenied(dest_path.clone())
                } else {
                    FlashError::Io(e)
                }
            })?;

        self.write_with_progress(total_bytes, &mut dest, on_progress)?;

        // Sync after writing
        let _ = std::process::Command::new("sync").output();

        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn flash_windows<F>(&self, total_bytes: u64, on_progress: F) -> Result<(), FlashError>
    where
        F: Fn(FlashProgress) + Send + 'static,
    {
        let dest_path = &self.dest_path;

        // Windows requires exact 512-byte aligned chunks for raw physical drive access
        let mut dest = OpenOptions::new()
            .write(true)
            .open(dest_path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    FlashError::PermissionDenied(dest_path.clone())
                } else if e.kind() == std::io::ErrorKind::NotFound {
                    FlashError::DestinationNotFound(dest_path.clone())
                } else {
                    FlashError::Io(e)
                }
            })?;

        self.write_with_progress_chunked(total_bytes, &mut dest, 512 * 1024, on_progress)?;

        Ok(())
    }

    /// Write source to dest in 1MB chunks, reporting progress every 500ms.
    fn write_with_progress<W, F>(
        &self,
        total_bytes: u64,
        dest: &mut W,
        on_progress: F,
    ) -> Result<(), FlashError>
    where
        W: Write,
        F: Fn(FlashProgress),
    {
        self.write_with_progress_chunked(total_bytes, dest, 1024 * 1024, on_progress)
    }

    fn write_with_progress_chunked<W, F>(
        &self,
        total_bytes: u64,
        dest: &mut W,
        chunk_size: usize,
        on_progress: F,
    ) -> Result<(), FlashError>
    where
        W: Write,
        F: Fn(FlashProgress),
    {
        let mut src = File::open(&self.source_path)?;
        let mut buf = vec![0u8; chunk_size];
        let mut bytes_written: u64 = 0;

        let mut last_progress = Instant::now();
        let mut bytes_since_last = 0u64;
        let progress_interval = Duration::from_millis(500);

        loop {
            let n = src.read(&mut buf)?;
            if n == 0 {
                break;
            }
            dest.write_all(&buf[..n])?;
            bytes_written += n as u64;
            bytes_since_last += n as u64;

            let elapsed = last_progress.elapsed();
            if elapsed >= progress_interval {
                let speed_bps = if elapsed.as_secs_f64() > 0.0 {
                    (bytes_since_last as f64 / elapsed.as_secs_f64()) as u64
                } else {
                    0
                };
                on_progress(FlashProgress {
                    bytes_written,
                    total_bytes,
                    speed_bps,
                });
                last_progress = Instant::now();
                bytes_since_last = 0;
            }
        }

        // Final progress report
        on_progress(FlashProgress {
            bytes_written,
            total_bytes,
            speed_bps: 0,
        });

        dest.flush()?;
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    // ── to_raw_device ─────────────────────────────────────────────────────────

    #[test]
    fn raw_device_converts_disk_to_rdisk() {
        assert_eq!(to_raw_device("/dev/disk4"), "/dev/rdisk4");
        assert_eq!(to_raw_device("/dev/disk0"), "/dev/rdisk0");
        assert_eq!(to_raw_device("/dev/disk12"), "/dev/rdisk12");
    }

    #[test]
    fn raw_device_leaves_non_disk_paths_unchanged() {
        assert_eq!(to_raw_device("/dev/sdb"), "/dev/sdb");
        assert_eq!(to_raw_device(r"\\.\PHYSICALDRIVE1"), r"\\.\PHYSICALDRIVE1");
        assert_eq!(to_raw_device("/dev/rdisk4"), "/dev/rdisk4");
    }

    // ── Flasher::new ──────────────────────────────────────────────────────────

    #[test]
    fn flasher_new_stores_paths() {
        let f = Flasher::new("/tmp/image.img", "/dev/sdb");
        assert_eq!(f.source_path, std::path::PathBuf::from("/tmp/image.img"));
        assert_eq!(f.dest_path, "/dev/sdb");
    }

    #[test]
    fn flasher_new_dest_as_string() {
        let dest = String::from("/dev/disk4");
        let f = Flasher::new("/tmp/foo.img", dest);
        assert_eq!(f.dest_path, "/dev/disk4");
    }

    // ── FlashProgress serde ───────────────────────────────────────────────────

    #[test]
    fn flash_progress_serializes_correctly() {
        let p = FlashProgress {
            bytes_written: 1024 * 1024,
            total_bytes: 4 * 1024 * 1024,
            speed_bps: 512 * 1024,
        };
        let json = serde_json::to_string(&p).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(v["bytes_written"], 1024 * 1024u64);
        assert_eq!(v["total_bytes"], 4 * 1024 * 1024u64);
        assert_eq!(v["speed_bps"], 512 * 1024u64);
    }

    #[test]
    fn flash_progress_zero_values_serialize() {
        let p = FlashProgress { bytes_written: 0, total_bytes: 0, speed_bps: 0 };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"bytes_written\":0"));
        assert!(json.contains("\"speed_bps\":0"));
    }

    // ── FlashError display ────────────────────────────────────────────────────

    #[test]
    fn flash_error_source_not_found_display() {
        let err = FlashError::SourceNotFound(PathBuf::from("/tmp/missing.img"));
        let msg = err.to_string();
        assert!(msg.contains("missing.img"), "display: {}", msg);
        assert!(msg.contains("source image not found"), "display: {}", msg);
    }

    #[test]
    fn flash_error_destination_not_found_display() {
        let err = FlashError::DestinationNotFound("/dev/disk99".into());
        let msg = err.to_string();
        assert!(msg.contains("disk99"), "display: {}", msg);
    }

    #[test]
    fn flash_error_permission_denied_display() {
        let err = FlashError::PermissionDenied("/dev/sda".into());
        let msg = err.to_string();
        assert!(msg.contains("permission denied"), "display: {}", msg);
        assert!(msg.contains("/dev/sda"), "display: {}", msg);
    }

    #[test]
    fn flash_error_unmount_failed_display() {
        let err = FlashError::UnmountFailed("disk4: busy".into());
        let msg = err.to_string();
        assert!(msg.contains("unmount failed"), "display: {}", msg);
        assert!(msg.contains("busy"), "display: {}", msg);
    }

    // ── write_with_progress_chunked (via a real temp file) ────────────────────

    #[test]
    fn write_progress_reports_final_callback() {
        use std::io::Write as _;

        // Build a source temp file with known content
        let mut src_tmp = tempfile::NamedTempFile::new().unwrap();
        let payload = vec![0xABu8; 64 * 1024]; // 64 KiB
        src_tmp.write_all(&payload).unwrap();
        src_tmp.flush().unwrap();

        let flasher = Flasher::new(src_tmp.path(), "/dev/null");
        let total = src_tmp.path().metadata().unwrap().len();

        let calls: Arc<Mutex<Vec<FlashProgress>>> = Arc::new(Mutex::new(Vec::new()));
        let calls2 = Arc::clone(&calls);

        let mut dest = std::io::sink();
        flasher
            .write_with_progress(total, &mut dest, move |p| {
                calls2.lock().unwrap().push(p);
            })
            .unwrap();

        let recorded = calls.lock().unwrap();
        // There must be at least the final progress callback
        assert!(!recorded.is_empty());
        // The last callback must report all bytes written
        let last = recorded.last().unwrap();
        assert_eq!(last.bytes_written, total);
        assert_eq!(last.total_bytes, total);
    }

    #[test]
    fn write_progress_speed_field_present() {
        use std::io::Write as _;

        // Write enough data that we might get a mid-run speed reading;
        // at minimum the final report has speed_bps=0 by design.
        let mut src_tmp = tempfile::NamedTempFile::new().unwrap();
        src_tmp.write_all(&vec![0u8; 1024]).unwrap();
        src_tmp.flush().unwrap();
        let total = src_tmp.path().metadata().unwrap().len();

        let flasher = Flasher::new(src_tmp.path(), "/dev/null");
        let mut dest = std::io::sink();

        let saw_progress = Arc::new(Mutex::new(false));
        let saw2 = Arc::clone(&saw_progress);

        flasher
            .write_with_progress(total, &mut dest, move |p| {
                // speed_bps is a u64 — just check the field is accessible
                let _ = p.speed_bps;
                *saw2.lock().unwrap() = true;
            })
            .unwrap();
        assert!(*saw_progress.lock().unwrap());
    }

    #[test]
    fn flash_fails_when_source_missing() {
        let f = Flasher::new("/nonexistent/path/image.img", "/dev/null");
        let err = f.flash(|_| {}).unwrap_err();
        assert!(matches!(err, FlashError::SourceNotFound(_)));
    }
}
