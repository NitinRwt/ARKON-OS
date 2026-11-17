use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum BootError {
    #[error("not a UEFI system; installation requires UEFI firmware")]
    NotUefi,

    #[error("firmware boot entry changed unexpectedly: {label} (number {number:#06x})")]
    EntryChanged { number: u16, label: String },

    #[error("firmware boot entry was removed unexpectedly: {label} (number {number:#06x})")]
    EntryRemoved { number: u16, label: String },

    #[error("unexpected new firmware entries were added: {count} extra entries")]
    UnexpectedNewEntries { count: usize },

    #[error("ESP is missing required file: {path}")]
    EspFileMissing { path: String },

    #[error("bootloader {selected} is not supported by image {image_ref}")]
    BootloaderIncompatible { selected: String, image_ref: String },

    /// Never disable Secure Boot silently — this error is raised when code
    /// attempts a path that would do so (project charter, SECURE_BOOT_DESIGN.md §1).
    #[error("Secure Boot must not be disabled; the installed system supports Secure Boot on")]
    SecureBootDisableRefused,

    #[error("MOK enrolment cannot proceed: {reason}")]
    MokEnrolmentFailed { reason: String },

    #[error("plan validation failed: {0}")]
    ValidationFailed(String),

    #[error("I/O error reading {path}: {detail}")]
    Io { path: String, detail: String },

    #[error("parse error for {item}: {detail}")]
    Parse { item: String, detail: String },

    #[error("not present: {item}")]
    NotPresent { item: String },
}

pub type BootResult<T> = Result<T, BootError>;
