use thiserror::Error;

/// Errors from the planner and validator.
/// Scan errors are represented as blockers on the StorageReport, not as Err.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum StorageError {
    // ── Plan construction ──────────────────────────────────────────────────
    #[error("disk '{0}' not found in scan")]
    DiskNotFound(String),

    #[error("region '{0}' not found on disk '{1}'")]
    RegionNotFound(String, String),

    #[error("disk '{0}' is blocked: {1}")]
    DiskBlocked(String, String),

    #[error("disk '{0}' has no partition table; GPT is required")]
    NoPartitionTable(String),

    #[error("disk '{0}' uses an MBR partition table; GPT is required for free-space mode")]
    MbrNotSupportedForFreeSpace(String),

    #[error("disk '{0}' is too small ({1} bytes available, {2} bytes required)")]
    DiskTooSmall(String, u64, u64),

    #[error("region '{0}' is too small ({1} bytes available, {2} bytes required)")]
    RegionTooSmall(String, u64, u64),

    #[error("sector size {0} is not supported (must be 512 or 4096 bytes)")]
    UnsupportedSectorSize(u32),

    #[error("disk '{0}' is the installation medium and cannot be a target")]
    IsInstallMedium(String),

    #[error("no eligible ARKON installation found on disk '{0}' for reinstall mode")]
    NoArkonInstallFound(String),

    #[error("custom root size {0} bytes is too small (minimum {1} bytes)")]
    RootTooSmall(u64, u64),

    #[error("custom root size {0} bytes exceeds available space {1} bytes")]
    RootTooLarge(u64, u64),

    #[error("scan_id mismatch: plan was built from scan '{0}' but current scan is '{1}'")]
    ScanIdMismatch(String, String),

    #[error("unsupported planning mode for this disk configuration: {0}")]
    UnsupportedMode(String),

    // ── Validator ─────────────────────────────────────────────────────────
    #[error("validator rule P{0} failed: {1}")]
    ValidationFailed(u8, String),

    #[error("plan hash mismatch: expected '{0}', got '{1}'")]
    PlanHashMismatch(String, String),
}

pub type StorageResult<T> = Result<T, StorageError>;
