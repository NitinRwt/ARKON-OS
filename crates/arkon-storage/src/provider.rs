/// SysfsProvider: abstraction over /proc and /sys for testability.
///
/// Mirrors the same trait in arkon-hw so both crates can share fixture helpers.
/// The real provider reads the actual filesystem; the mock serves pre-registered
/// strings without touching the host.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub trait SysfsProvider: Send + Sync {
    fn read_file(&self, path: &Path) -> Result<String, std::io::Error>;
    fn list_dir(&self, path: &Path) -> Result<Vec<String>, std::io::Error>;
    fn exists(&self, path: &Path) -> bool;
}

// ── Real provider ─────────────────────────────────────────────────────────────

pub struct RealProvider {
    pub root: PathBuf,
}

impl RealProvider {
    pub fn new() -> Self {
        Self { root: PathBuf::from("/") }
    }

    fn full(&self, path: &Path) -> PathBuf {
        if self.root == Path::new("/") {
            path.to_owned()
        } else {
            self.root.join(path.strip_prefix("/").unwrap_or(path))
        }
    }
}

impl Default for RealProvider {
    fn default() -> Self { Self::new() }
}

impl SysfsProvider for RealProvider {
    fn read_file(&self, path: &Path) -> Result<String, std::io::Error> {
        let full = self.full(path);
        let bytes = std::fs::read(&full)?;
        let s = String::from_utf8(bytes)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "not UTF-8"))?;
        Ok(s.trim().to_owned())
    }

    fn list_dir(&self, path: &Path) -> Result<Vec<String>, std::io::Error> {
        let full = self.full(path);
        let mut names = Vec::new();
        for entry in std::fs::read_dir(&full)? {
            let entry = entry?;
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

#[derive(Default)]
pub struct MockProvider {
    files: HashMap<String, String>,
}

impl MockProvider {
    pub fn new() -> Self { Self::default() }

    pub fn add_file(&mut self, path: impl Into<String>, content: impl Into<String>) -> &mut Self {
        self.files.insert(path.into(), content.into());
        self
    }
}

impl SysfsProvider for MockProvider {
    fn read_file(&self, path: &Path) -> Result<String, std::io::Error> {
        let key = path.to_string_lossy().replace('\\', "/");
        self.files
            .get(key.as_str())
            .map(|s| s.trim().to_owned())
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::NotFound, "mock: not registered",
            ))
    }

    fn list_dir(&self, path: &Path) -> Result<Vec<String>, std::io::Error> {
        let dir = path.to_string_lossy().replace('\\', "/");
        let dir = dir.trim_end_matches('/');
        let mut names = std::collections::BTreeSet::new();
        for key in self.files.keys() {
            let k = key.trim_end_matches('/');
            if let Some(rest) = k.strip_prefix(dir) {
                let rest = rest.trim_start_matches('/');
                if !rest.is_empty() {
                    if let Some(component) = rest.split('/').next() {
                        if !component.is_empty() {
                            names.insert(component.to_owned());
                        }
                    }
                }
            }
        }
        if names.is_empty() && !self.files.keys().any(|k| k.starts_with(dir)) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound, "mock: directory not found",
            ));
        }
        Ok(names.into_iter().collect())
    }

    fn exists(&self, path: &Path) -> bool {
        let key = path.to_string_lossy().replace('\\', "/");
        let key = key.trim_end_matches('/');
        self.files.keys().any(|k| {
            let k = k.trim_end_matches('/');
            k == key || k.starts_with(&format!("{}/", key))
        })
    }
}
