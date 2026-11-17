use std::collections::HashMap;
use crate::error::{BootError, BootResult};
use crate::firmware::FirmwareEntry;

/// Abstraction over EFI variable reads and ESP filesystem access.
///
/// `RealProvider` reads from `/sys/firmware/efi/efivars/` on a live Linux
/// system.  `MockProvider` injects structured data for unit and integration
/// tests, making the library fully testable on any host OS.
pub trait BootProvider {
    fn list_entries(&self) -> BootResult<Vec<FirmwareEntry>>;
    fn boot_order(&self) -> BootResult<Vec<u16>>;
    fn boot_next(&self) -> BootResult<Option<u16>>;
    fn esp_file_exists(&self, path: &str) -> bool;
    fn list_esp_dir(&self, path: &str) -> BootResult<Vec<String>>;
}

// ── MockProvider ──────────────────────────────────────────────────────────────

pub struct MockProvider {
    entries: Vec<FirmwareEntry>,
    boot_order: Vec<u16>,
    boot_next: Option<u16>,
    esp_files: HashMap<String, bool>,
    esp_dirs: HashMap<String, Vec<String>>,
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            entries: vec![],
            boot_order: vec![],
            boot_next: None,
            esp_files: HashMap::new(),
            esp_dirs: HashMap::new(),
        }
    }

    pub fn with_entry(mut self, entry: FirmwareEntry) -> Self {
        let n = entry.number;
        self.entries.push(entry);
        if !self.boot_order.contains(&n) {
            self.boot_order.push(n);
        }
        self
    }

    pub fn with_boot_order(mut self, order: Vec<u16>) -> Self {
        self.boot_order = order;
        self
    }

    pub fn with_boot_next(mut self, next: u16) -> Self {
        self.boot_next = Some(next);
        self
    }

    /// Register a file as present on the mock ESP.
    /// Path is normalised to lowercase forward slashes.
    pub fn with_esp_file(mut self, path: &str) -> Self {
        let norm = path.replace('\\', "/").to_lowercase();
        self.esp_files.insert(norm, true);
        self
    }

    pub fn with_esp_dir(mut self, path: &str, entries: Vec<String>) -> Self {
        let norm = path.replace('\\', "/").to_lowercase();
        self.esp_dirs.insert(norm, entries);
        self
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl BootProvider for MockProvider {
    fn list_entries(&self) -> BootResult<Vec<FirmwareEntry>> {
        Ok(self.entries.clone())
    }

    fn boot_order(&self) -> BootResult<Vec<u16>> {
        Ok(self.boot_order.clone())
    }

    fn boot_next(&self) -> BootResult<Option<u16>> {
        Ok(self.boot_next)
    }

    fn esp_file_exists(&self, path: &str) -> bool {
        let norm = path.replace('\\', "/").to_lowercase();
        self.esp_files.get(&norm).copied().unwrap_or(false)
    }

    fn list_esp_dir(&self, path: &str) -> BootResult<Vec<String>> {
        let norm = path.replace('\\', "/").to_lowercase();
        Ok(self.esp_dirs.get(&norm).cloned().unwrap_or_default())
    }
}

// ── RealProvider ──────────────────────────────────────────────────────────────

/// Reads EFI variables from `/sys/firmware/efi/efivars/` and the ESP from a
/// mounted path.  Only usable on a Linux system with the efivars filesystem
/// mounted; use `MockProvider` for all other contexts.
pub struct RealProvider {
    esp_mount: String,
}

impl RealProvider {
    pub fn new(esp_mount: impl Into<String>) -> Self {
        Self { esp_mount: esp_mount.into() }
    }
}

impl BootProvider for RealProvider {
    fn list_entries(&self) -> BootResult<Vec<FirmwareEntry>> {
        // TODO: parse /sys/firmware/efi/efivars/BootXXXX-8be4df61-... on Linux.
        // The binary EFI_LOAD_OPTION format is: 4-byte attributes, 2-byte
        // FilePathListLength, UCS-2 description, EFI_DEVICE_PATH_PROTOCOL list.
        Err(BootError::NotPresent {
            item: "RealProvider EFI variable parsing not yet implemented".into(),
        })
    }

    fn boot_order(&self) -> BootResult<Vec<u16>> {
        Err(BootError::NotPresent {
            item: "RealProvider EFI variable parsing not yet implemented".into(),
        })
    }

    fn boot_next(&self) -> BootResult<Option<u16>> {
        Err(BootError::NotPresent {
            item: "RealProvider EFI variable parsing not yet implemented".into(),
        })
    }

    fn esp_file_exists(&self, path: &str) -> bool {
        let full = format!(
            "{}/{}",
            self.esp_mount.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        std::path::Path::new(&full).exists()
    }

    fn list_esp_dir(&self, path: &str) -> BootResult<Vec<String>> {
        let full = format!(
            "{}/{}",
            self.esp_mount.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        std::fs::read_dir(&full)
            .map_err(|e| BootError::Io { path: full.clone(), detail: e.to_string() })?
            .map(|entry| {
                entry
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .map_err(|e| BootError::Io { path: full.clone(), detail: e.to_string() })
            })
            .collect()
    }
}
