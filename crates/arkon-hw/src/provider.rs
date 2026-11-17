/// Abstraction over the Linux virtual filesystems (/proc, /sys).
///
/// The real implementation reads from the actual paths. Tests inject a
/// `MockProvider` that serves pre-recorded fixture strings without touching
/// the host filesystem.
use crate::error::{ProbeError, ProbeResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub trait SysfsProvider: Send + Sync {
    /// Read a file and return its contents as a trimmed string.
    fn read_file(&self, path: &Path) -> ProbeResult<String>;

    /// Return the names of the immediate children of a directory.
    fn list_dir(&self, path: &Path) -> ProbeResult<Vec<String>>;

    /// Return true if the path exists (file or directory).
    fn exists(&self, path: &Path) -> bool;
}

// ── Real provider ────────────────────────────────────────────────────────────

/// Reads from the real filesystem.  Used in production.
pub struct RealProvider {
    /// An optional prefix prepended to every path, used only in integration
    /// tests that mount a tmpfs tree at a known location.
    pub root: PathBuf,
}

impl RealProvider {
    pub fn new() -> Self {
        Self { root: PathBuf::from("/") }
    }

    fn full(&self, path: &Path) -> PathBuf {
        // Avoid double-slash at root
        if self.root == Path::new("/") {
            path.to_owned()
        } else {
            self.root.join(path.strip_prefix("/").unwrap_or(path))
        }
    }
}

impl Default for RealProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SysfsProvider for RealProvider {
    fn read_file(&self, path: &Path) -> ProbeResult<String> {
        let full = self.full(path);
        let bytes = std::fs::read(&full).map_err(|e| ProbeError::read(&full, e))?;
        let s = String::from_utf8(bytes).map_err(|_| ProbeError::utf8(&full))?;
        Ok(s.trim().to_owned())
    }

    fn list_dir(&self, path: &Path) -> ProbeResult<Vec<String>> {
        let full = self.full(path);
        let entries = std::fs::read_dir(&full)
            .map_err(|e| ProbeError::read_dir(&full, e))?;
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| ProbeError::read_dir(&full, e))?;
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_owned());
            }
        }
        Ok(names)
    }

    fn exists(&self, path: &Path) -> bool {
        self.full(path).exists()
    }
}

// ── Mock provider ─────────────────────────────────────────────────────────────

/// In-memory provider for unit tests.
///
/// Files are keyed by their absolute path string.  Directories are inferred:
/// any key whose parent matches the queried path contributes its file name to
/// `list_dir`.
#[derive(Default)]
pub struct MockProvider {
    files: HashMap<String, String>,
}

impl MockProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a file at `path` with the given `content`.
    pub fn add_file(&mut self, path: impl Into<String>, content: impl Into<String>) -> &mut Self {
        self.files.insert(path.into(), content.into());
        self
    }
}

impl SysfsProvider for MockProvider {
    fn read_file(&self, path: &Path) -> ProbeResult<String> {
        let key = path.to_string_lossy().replace('\\', "/");
        self.files
            .get(key.as_str())
            .map(|s| s.trim().to_owned())
            .ok_or_else(|| {
                ProbeError::read(
                    path,
                    std::io::Error::new(std::io::ErrorKind::NotFound, "mock: not registered"),
                )
            })
    }

    fn list_dir(&self, path: &Path) -> ProbeResult<Vec<String>> {
        let dir = path.to_string_lossy().replace('\\', "/");
        let dir = dir.trim_end_matches('/');
        let mut names = std::collections::BTreeSet::new();
        for key in self.files.keys() {
            let k = key.trim_end_matches('/');
            // Check if key is a direct child of dir
            if let Some(rest) = k.strip_prefix(dir) {
                let rest = rest.trim_start_matches('/');
                if !rest.is_empty() {
                    // Take only the first path component
                    let component = rest.split('/').next().unwrap();
                    if !component.is_empty() {
                        names.insert(component.to_owned());
                    }
                }
            }
        }
        if names.is_empty() && !self.files.keys().any(|k| k.starts_with(dir)) {
            return Err(ProbeError::read_dir(
                path,
                std::io::Error::new(std::io::ErrorKind::NotFound, "mock: directory not found"),
            ));
        }
        Ok(names.into_iter().collect())
    }

    fn exists(&self, path: &Path) -> bool {
        let key = path.to_string_lossy().replace('\\', "/");
        let key = key.trim_end_matches('/');
        // A path exists if it is a registered file or is the prefix of one
        self.files.keys().any(|k| {
            let k = k.trim_end_matches('/');
            k == key || k.starts_with(&format!("{}/", key))
        })
    }
}
