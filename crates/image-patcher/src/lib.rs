use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use thiserror::Error;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum PatcherError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("invalid disk image: {0}")]
    InvalidImage(String),

    #[error("FAT filesystem error: {0}")]
    Fat(String),

    #[error("manifest error: {0}")]
    Manifest(#[from] manifest::ManifestError),
}


// ── MBR helpers ───────────────────────────────────────────────────────────────

/// Read the first partition's LBA start from the MBR and return the byte
/// offset into the image file.
fn boot_partition_offset(file: &mut File) -> Result<u64, PatcherError> {
    // MBR partition table starts at offset 446; each entry is 16 bytes.
    // Bytes 8-11 of the first entry are the LBA start (little-endian u32).
    file.seek(SeekFrom::Start(446 + 8))?;
    let mut buf = [0u8; 4];
    file.read_exact(&mut buf)?;
    let lba_start = u32::from_le_bytes(buf);
    if lba_start == 0 {
        return Err(PatcherError::InvalidImage(
            "MBR first partition entry has LBA start == 0".into(),
        ));
    }
    Ok(lba_start as u64 * 512)
}

// ── OffsetFile ─────────────────────────────────────────────────────────────────
//
// Wraps a `File` and exposes a window [base_offset, base_offset + len) as a
// seekable, readable, writable stream starting at position 0.  `fatfs`
// requires `Read + Write + Seek` with no generic I/O traits, so we provide
// the full set even though reads/writes do the real work.

struct OffsetFile {
    file: File,
    base: u64,
    pos: u64,
}

impl OffsetFile {
    fn new(file: File, base: u64) -> Self {
        OffsetFile { file, base, pos: 0 }
    }
}

impl Read for OffsetFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.seek(SeekFrom::Start(self.base + self.pos))?;
        let n = self.file.read(buf)?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl Write for OffsetFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.seek(SeekFrom::Start(self.base + self.pos))?;
        let n = self.file.write(buf)?;
        self.pos += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl Seek for OffsetFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.pos = match pos {
            SeekFrom::Start(n) => n,
            SeekFrom::Current(n) => {
                (self.pos as i64 + n).max(0) as u64
            }
            SeekFrom::End(n) => {
                // We don't know the partition length here; let the underlying
                // file resolve this and adjust back to a partition-relative pos.
                let abs = self.file.seek(SeekFrom::End(n))?;
                if abs < self.base {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "seek before partition start",
                    ));
                }
                abs - self.base
            }
        };
        Ok(self.pos)
    }
}

// ── Template substitution ─────────────────────────────────────────────────────

fn substitute(template: &str, values: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in values {
        result = result.replace(&format!("{{{{{}}}}}", key), value);
    }
    result
}

// ── ImagePatcher ──────────────────────────────────────────────────────────────

pub struct ImagePatcher {
    path: PathBuf,
    boot_offset: u64,
}

impl ImagePatcher {
    /// Open a raw `.img` file and locate its FAT32 boot partition.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PatcherError> {
        let path = path.as_ref().to_path_buf();
        let mut file = File::open(&path)?;
        let boot_offset = boot_partition_offset(&mut file)?;
        Ok(ImagePatcher { path, boot_offset })
    }

    // ── private helpers ───────────────────────────────────────────────────

    fn open_ro(&self) -> Result<fatfs::FileSystem<OffsetFile>, PatcherError> {
        let file = File::open(&self.path)?;
        let cursor = OffsetFile::new(file, self.boot_offset);
        let fs = fatfs::FileSystem::new(cursor, fatfs::FsOptions::new())?;
        Ok(fs)
    }

    fn open_rw(&self) -> Result<fatfs::FileSystem<OffsetFile>, PatcherError> {
        let file = OpenOptions::new().read(true).write(true).open(&self.path)?;
        let cursor = OffsetFile::new(file, self.boot_offset);
        let fs = fatfs::FileSystem::new(cursor, fatfs::FsOptions::new())?;
        Ok(fs)
    }

    // ── public API ────────────────────────────────────────────────────────

    /// Read a file from the boot (FAT32) partition.
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, PatcherError> {
        let fs = self.open_ro()?;
        let root = fs.root_dir();
        let mut file = root.open_file(path).map_err(|e| {
            PatcherError::Fat(format!("cannot open '{}': {}", path, e))
        })?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        Ok(data)
    }

    /// Write a file to the boot partition (creates or overwrites).
    pub fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), PatcherError> {
        let fs = self.open_rw()?;
        let root = fs.root_dir();
        let mut file = root.create_file(path).map_err(|e| {
            PatcherError::Fat(format!("cannot create '{}': {}", path, e))
        })?;
        file.truncate()?;
        file.write_all(data)?;
        Ok(())
    }

    /// Read `sunburn.json` from the boot partition if it exists.
    pub fn read_manifest(&self) -> Result<Option<manifest::Manifest>, PatcherError> {
        match self.read_file("sunburn.json") {
            Ok(data) => {
                let json = String::from_utf8(data).map_err(|_| {
                    PatcherError::InvalidImage("sunburn.json is not valid UTF-8".into())
                })?;
                let m = manifest::parse(&json)?;
                Ok(Some(m))
            }
            Err(PatcherError::Fat(_)) => {
                // File not found — basic-mode image
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// Apply write rules from a [`manifest::Step`] by substituting `{{field_id}}`
    /// placeholders in each template and writing the result to the boot partition.
    pub fn apply_writes(
        &mut self,
        step: &manifest::Step,
        values: &HashMap<String, String>,
    ) -> Result<(), PatcherError> {
        for rule in &step.writes {
            let content = substitute(&rule.template, values);
            self.write_file(&rule.path, content.as_bytes())?;
        }
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitute_replaces_placeholders() {
        let mut values = HashMap::new();
        values.insert("ssid".into(), "MyNetwork".into());
        values.insert("password".into(), "s3cr3t".into());
        let result = substitute("ssid={{ssid}}\npassword={{password}}", &values);
        assert_eq!(result, "ssid=MyNetwork\npassword=s3cr3t");
    }

    #[test]
    fn substitute_leaves_unknown_placeholders() {
        let values = HashMap::new();
        let result = substitute("key={{unknown}}", &values);
        assert_eq!(result, "key={{unknown}}");
    }
}
