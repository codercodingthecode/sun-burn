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
        let raw_dest = if let Some(stripped) = disk.strip_prefix("/dev/disk") {
            format!("/dev/rdisk{}", stripped)
        } else {
            disk.clone()
        };

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
