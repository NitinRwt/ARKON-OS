/// Plan types: everything the planner produces and the validator consumes.
use crate::scan::{DiskId, FsType, PartitionClass, RegionId, ScanId};
use serde::{Deserialize, Serialize};

// ── Plan request ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanMode {
    /// Erase the entire target disk and install fresh.
    EraseDisk,
    /// Install in unallocated space on the target disk.
    FreeSpace,
    /// Like FreeSpace but the user chose a specific root size.
    Manual,
    /// Replace the OS on an existing ARKON install; keep the home subvolume.
    ReinstallKeepHome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRequest {
    pub mode: PlanMode,
    /// Must equal the scan_id the report was built from.
    pub scan_id: ScanId,
    pub target_disk: DiskId,
    /// Required for FreeSpace and Manual modes.
    pub region_id: Option<RegionId>,
    /// Required for Manual mode; must be ≥ MIN_ROOT_BYTES.
    pub root_size_bytes: Option<u64>,
    pub encryption: bool,
    /// Image variant name, e.g. "arkon" or "arkon-nvidia".
    pub variant: Option<String>,
}

// ── Plan output ───────────────────────────────────────────────────────────────

/// What a planned partition will look like after the operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedPartition {
    pub number: u32,
    pub start_lba: u64,
    pub size_lba: u64,
    pub type_guid: String,
    pub part_label: String,
    pub fs_type: FsType,
    pub class: PartitionClass,
    /// True when this partition is encrypted (LUKS2 container).
    pub encrypted: bool,
    /// ARKON role for the target system.
    pub arkon_role: Option<ArkonRole>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArkonRole {
    Esp,
    Boot,
    Root,
}

/// The kind of operation and its risk level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    /// Create a new partition in unallocated space.
    Create,
    /// Erase the partition table of the target disk.
    EraseTable,
    /// Keep an existing partition unchanged.
    Keep,
    /// Reformat an existing partition (data lost).
    Format,
    /// Replace the Btrfs root subvolume, keeping the home subvolume.
    ReplaceSubvolume,
}

/// Risk classification for a planned operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// No data is lost and no existing structure is modified.
    None,
    /// Existing data may be disrupted if the operation fails midway.
    Low,
    /// Data is permanently destroyed by this operation.
    Destructive,
}

/// One atomic disk operation within a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedOperation {
    /// Unique within the plan, e.g. "op-001".
    pub op_id: String,
    pub target_disk: DiskId,
    /// Existing partition number this operation targets, or None for new ones.
    pub target_partition: Option<u32>,
    pub operation: OperationType,
    /// State before this operation (None for create on empty space or erase).
    pub before: Option<OperationState>,
    /// Expected state after this operation.
    pub after: Option<OperationState>,
    pub risk: RiskLevel,
    /// Whether this operation has passed pre-execution validation.
    pub validated: bool,
}

/// A snapshot of a partition's key identifiers, used to detect changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationState {
    pub number: u32,
    pub start_lba: u64,
    pub size_lba: u64,
    pub type_guid: Option<String>,
    pub part_uuid: String,
    pub part_label: Option<String>,
    pub fs_type: Option<FsType>,
}

/// An item that will be lost as a result of this plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLossItem {
    pub part_uuid: String,
    pub partition_number: u32,
    pub description: String,
    pub detected_os: Option<String>,
}

/// What the user must do to confirm this plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationRequirements {
    /// Destructive plans require a typed phrase.
    pub phrase_required: bool,
    /// A short, human-readable phrase derived from the target disk.
    pub phrase_hint: Option<String>,
    /// True when BitLocker was detected anywhere.
    pub bitlocker_ack_required: bool,
}

/// A validated, executable partition plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub plan_id: PlanId,
    /// FNV-1a hash of the plan content (excluding this field), used for
    /// confirmation binding.
    pub plan_hash: String,
    pub scan_id: ScanId,
    pub mode: PlanMode,
    pub target_disk: DiskId,
    pub variant: Option<String>,
    pub encryption: bool,
    pub operations: Vec<PlannedOperation>,
    /// Every existing partition on every disk that this plan leaves unchanged.
    pub unchanged: Vec<UnchangedEntry>,
    pub data_loss: Vec<DataLossItem>,
    pub destructive: bool,
    pub warnings: Vec<PlanWarning>,
    pub confirmation: ConfirmationRequirements,
    /// systemd-repart definition file contents, keyed by filename.
    /// These are passed to the executor when the plan is confirmed.
    pub repart_definitions: Vec<RepartDefinition>,
}

/// A partition confirmed unchanged by this plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnchangedEntry {
    pub disk_id: DiskId,
    pub partition_number: u32,
    pub part_uuid: String,
    pub description: String,
}

/// A generated systemd-repart partition definition file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepartDefinition {
    /// Filename, e.g. "10-esp.conf".
    pub filename: String,
    /// File content (INI format).
    pub content: String,
}

/// A non-fatal warning attached to the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanWarning {
    BitlockerPresent,
    TargetIsRotational,
    TargetIsUsb,
    NvidiaTuringDriverRequired,
    MinimumSizeBand { size_bytes: u64 },
    ProvisionedSizesAreEstimates,
}

/// Opaque plan identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanId(pub String);

impl PlanId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for PlanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Result returned by the validator for a single rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleResult {
    pub rule: u8,
    pub passed: bool,
    pub detail: String,
}

/// Full validation output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub plan_id: PlanId,
    pub all_passed: bool,
    pub rules: Vec<RuleResult>,
}

// ── Layout constants ──────────────────────────────────────────────────────────

/// Minimum disk size (64 GiB, provisional until Phase 1 image measurements).
pub const MIN_DISK_BYTES: u64 = 64 * 1024 * 1024 * 1024;

/// ESP size: 1 GiB.
pub const ESP_SIZE_BYTES: u64 = 1 * 1024 * 1024 * 1024;

/// /boot size: 2 GiB.
pub const BOOT_SIZE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Minimum root partition size: 15 GiB (provisional).
pub const MIN_ROOT_BYTES: u64 = 15 * 1024 * 1024 * 1024;

/// Minimum free region for free-space mode: ESP + /boot + min root.
pub const MIN_FREE_REGION_BYTES: u64 = ESP_SIZE_BYTES + BOOT_SIZE_BYTES + MIN_ROOT_BYTES;

/// Partition alignment: 1 MiB.
pub const ALIGNMENT_BYTES: u64 = 1024 * 1024;

// GPT type GUIDs (lowercase, with hyphens)
pub const GUID_EFI_SYSTEM: &str         = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b";
pub const GUID_LINUX_FILESYSTEM: &str   = "0fc63daf-8483-4772-8e79-3d69d8477de4";
pub const GUID_LUKS: &str               = "ca7d7ccb-63ed-4c53-861c-1742536059cc";

// ARKON partition labels
pub const LABEL_ARKON_ESP: &str  = "ARKON-ESP";
pub const LABEL_ARKON_BOOT: &str = "ARKON-BOOT";
pub const LABEL_ARKON_ROOT: &str = "ARKON-ROOT";
