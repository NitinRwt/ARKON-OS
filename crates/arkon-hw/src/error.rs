use thiserror::Error;
use std::path::PathBuf;

#[derive(Debug, Error)]
pub enum ProbeError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("cannot list directory {path}: {source}")]
    ReadDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("file {path} contains invalid UTF-8")]
    Utf8 { path: PathBuf },

    #[error("file {path} has unexpected format: {detail}")]
    ParseError { path: PathBuf, detail: String },

    #[error("required interface not present: {detail}")]
    NotPresent { detail: String },

    #[error("hardware report validation failed: {detail}")]
    Validation { detail: String },
}

impl ProbeError {
    pub fn read(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Read { path: path.into(), source }
    }
    pub fn read_dir(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::ReadDir { path: path.into(), source }
    }
    pub fn utf8(path: impl Into<PathBuf>) -> Self {
        Self::Utf8 { path: path.into() }
    }
    pub fn parse(path: impl Into<PathBuf>, detail: impl Into<String>) -> Self {
        Self::ParseError { path: path.into(), detail: detail.into() }
    }
    pub fn not_present(detail: impl Into<String>) -> Self {
        Self::NotPresent { detail: detail.into() }
    }
    pub fn validation(detail: impl Into<String>) -> Self {
        Self::Validation { detail: detail.into() }
    }
}

pub type ProbeResult<T> = Result<T, ProbeError>;
