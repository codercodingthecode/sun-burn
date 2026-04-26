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

pub(crate) fn substitute(template: &str, values: &HashMap<String, String>) -> String {
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

    // ── substitute ────────────────────────────────────────────────────────────

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

    #[test]
    fn substitute_multiple_occurrences() {
        let mut values = HashMap::new();
        values.insert("name".into(), "world".into());
        let result = substitute("hello {{name}}, hello {{name}}!", &values);
        assert_eq!(result, "hello world, hello world!");
    }

    // ── FAT image helpers ─────────────────────────────────────────────────────

    /// Build a minimal raw disk image in a tempfile:
    ///   - 1 MiB total
    ///   - MBR pointing partition 1 at LBA 1 (byte offset 512)
    ///   - FAT12 filesystem filling the rest
    fn build_test_image() -> tempfile::NamedTempFile {
        use std::io::{Seek, SeekFrom, Write};

        const IMG_SIZE: usize = 1024 * 1024; // 1 MiB
        const SECTOR: usize = 512;
        const PART_LBA: u32 = 1; // partition starts at sector 1

        // 1. Create temp file and zero-fill
        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let zeros = vec![0u8; IMG_SIZE];
        tmp.write_all(&zeros).unwrap();

        // 2. Write MBR partition entry at offset 446 + 8 (LBA start field)
        //    Entry layout (16 bytes total, offsets relative to entry start):
        //      0:  status (0x80 = bootable)
        //      1-3: CHS start (ignore for LBA mode)
        //      4:  partition type (0x0B = FAT32 CHS, good enough for fatfs)
        //      5-7: CHS end (ignore)
        //      8-11: LBA start (little-endian u32)
        //      12-15: sector count (little-endian u32)
        let part_sectors = ((IMG_SIZE / SECTOR) - PART_LBA as usize) as u32;
        let mut entry = [0u8; 16];
        entry[0] = 0x80; // bootable
        entry[4] = 0x0B; // FAT32 CHS
        entry[8..12].copy_from_slice(&PART_LBA.to_le_bytes());
        entry[12..16].copy_from_slice(&part_sectors.to_le_bytes());

        tmp.seek(SeekFrom::Start(446)).unwrap();
        tmp.write_all(&entry).unwrap();

        // 3. Format the partition region as FAT using fatfs
        //    We open the temp file again via OffsetFile and call FileSystem::new
        //    with format=true (FsOptions::new() formats when the BPB is absent/zeroed).
        tmp.seek(SeekFrom::Start(0)).unwrap();
        let raw = tmp.reopen().expect("reopen");

        let offset_file = OffsetFile::new(raw, PART_LBA as u64 * SECTOR as u64);
        fatfs::format_volume(
            offset_file,
            fatfs::FormatVolumeOptions::new().volume_label(*b"SUNBURN    "),
        )
        .expect("format_volume");

        tmp
    }

    /// Open an `ImagePatcher` against the given temp-file path.
    fn open_patcher(tmp: &tempfile::NamedTempFile) -> ImagePatcher {
        ImagePatcher::open(tmp.path()).expect("ImagePatcher::open")
    }

    // ── read_file / write_file ────────────────────────────────────────────────

    #[test]
    fn write_then_read_file() {
        let tmp = build_test_image();
        let mut patcher = open_patcher(&tmp);

        patcher.write_file("hello.txt", b"hello world").unwrap();

        let data = patcher.read_file("hello.txt").unwrap();
        assert_eq!(data, b"hello world");
    }

    #[test]
    fn read_file_missing_returns_error() {
        let tmp = build_test_image();
        let patcher = open_patcher(&tmp);
        let err = patcher.read_file("no_such_file.txt").unwrap_err();
        // Should be a Fat error
        assert!(matches!(err, PatcherError::Fat(_)));
    }

    #[test]
    fn write_file_overwrites_existing() {
        let tmp = build_test_image();
        let mut patcher = open_patcher(&tmp);

        patcher.write_file("data.txt", b"first").unwrap();
        patcher.write_file("data.txt", b"second").unwrap();

        let data = patcher.read_file("data.txt").unwrap();
        assert_eq!(data, b"second");
    }

    // ── read_manifest ─────────────────────────────────────────────────────────

    #[test]
    fn read_manifest_returns_none_when_absent() {
        let tmp = build_test_image();
        let patcher = open_patcher(&tmp);
        let result = patcher.read_manifest().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn read_manifest_returns_some_when_present() {
        let tmp = build_test_image();
        let mut patcher = open_patcher(&tmp);

        let manifest_json = r#"{
            "version": "1",
            "name": "Test Image",
            "steps": [{
                "id": "net",
                "title": "Network",
                "fields": [],
                "writes": []
            }]
        }"#;
        patcher.write_file("sunburn.json", manifest_json.as_bytes()).unwrap();

        let m = patcher.read_manifest().unwrap().expect("should be Some");
        assert_eq!(m.name, "Test Image");
    }

    // ── apply_writes ──────────────────────────────────────────────────────────

    #[test]
    fn apply_writes_substitutes_and_writes() {
        let tmp = build_test_image();
        let mut patcher = open_patcher(&tmp);

        let step = manifest::Step {
            id: "net".into(),
            title: "Network".into(),
            fields: vec![],
            writes: vec![manifest::WriteRule {
                path: "wpa.conf".into(),
                template: "network={\n  ssid=\"{{ssid}}\"\n  psk=\"{{psk}}\"\n}".into(),
            }],
        };

        let mut values = HashMap::new();
        values.insert("ssid".into(), "MySSID".into());
        values.insert("psk".into(), "MyPass".into());

        patcher.apply_writes(&step, &values).unwrap();

        let data = patcher.read_file("wpa.conf").unwrap();
        let text = String::from_utf8(data).unwrap();
        assert!(text.contains("ssid=\"MySSID\""));
        assert!(text.contains("psk=\"MyPass\""));
    }

    #[test]
    fn apply_writes_multiple_rules() {
        let tmp = build_test_image();
        let mut patcher = open_patcher(&tmp);

        let step = manifest::Step {
            id: "s".into(),
            title: "S".into(),
            fields: vec![],
            writes: vec![
                manifest::WriteRule { path: "a.txt".into(), template: "A={{val}}".into() },
                manifest::WriteRule { path: "b.txt".into(), template: "B={{val}}".into() },
            ],
        };

        let mut values = HashMap::new();
        values.insert("val".into(), "42".into());

        patcher.apply_writes(&step, &values).unwrap();

        assert_eq!(patcher.read_file("a.txt").unwrap(), b"A=42");
        assert_eq!(patcher.read_file("b.txt").unwrap(), b"B=42");
    }

    // ── OffsetFile ────────────────────────────────────────────────────────────

    #[test]
    fn offset_file_seek_start_and_current() {
        use std::io::{Read, Seek, SeekFrom, Write};

        let tmp_file = tempfile::tempfile().unwrap();
        let mut of = OffsetFile::new(tmp_file, 0);

        // Write some bytes
        of.write_all(b"ABCDE").unwrap();

        // Seek back to start and read
        of.seek(SeekFrom::Start(0)).unwrap();
        let mut buf = [0u8; 5];
        of.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"ABCDE");

        // SeekFrom::Current moves relative to current pos (now 5)
        of.seek(SeekFrom::Current(-3)).unwrap();
        let mut buf2 = [0u8; 1];
        of.read_exact(&mut buf2).unwrap();
        assert_eq!(&buf2, b"C");
    }
}
