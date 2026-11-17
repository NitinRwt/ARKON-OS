/// Scan-time types: the StorageReport and everything in it.
///
/// These are produced by the scanner (not yet implemented) and consumed by the
/// planner and validator.  In tests, StorageReport values are built directly.
use serde::{Deserialize, Serialize};

// ── Stable identifiers ────────────────────────────────────────────────────────

/// Stable disk identifier — a /dev/disk/by-id/… path in production.
/// In tests, any unique string is accepted.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiskId(pub String);

impl DiskId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DiskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable free-region identifier within a disk.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegionId(pub String);

impl RegionId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Opaque scan identifier.  Changes whenever any disk's layout changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanId(pub String);

impl ScanId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

// ── Enumerations ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartitionTable {
    Gpt,
    Mbr,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    Nvme,
    Sata,
    Usb,
    Mmc,
    Other,
}

/// Partition classification following STORAGE_ARCHITECTURE.md §4.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartitionClass {
    Esp,
    Msr,
    WindowsData,
    Bitlocker,
    WindowsRecovery,
    LinuxFs,
    Luks,
    Lvm,
    SoftwareRaid,
    Zfs,
    Bcache,
    Swap,
    Arkon,
    Unknown,
}

/// Filesystem type for a partition (advisory — not all can be detected
/// without mounting).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FsType {
    Vfat,
    Ext4,
    Xfs,
    Btrfs,
    Ntfs,
    Exfat,
    Swap,
    Unknown,
}

/// An operating system detected on a disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectedSystem {
    Windows { partition_numbers: Vec<u32>, bitlocker: bool },
    Arkon { esp_number: u32, boot_number: Option<u32>, root_number: u32 },
    Linux { name: Option<String>, partition_numbers: Vec<u32> },
    Unknown { partition_numbers: Vec<u32> },
}

/// A condition that prevents a disk from being used as an installation target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskBlocker {
    IsInstallMedium,
    TooSmall { size_bytes: u64, required_bytes: u64 },
    UnsupportedSectorSize { logical_bytes: u32 },
    HasLvm,
    HasSoftwareRaid,
    HasZfs,
    HasBcache,
    HasFirmwareRaid,
    Removable,
}

/// A non-fatal condition the user should be warned about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskWarning {
    Rotational,
    UsbTransport,
    HasWindows,
    HasBitlocker,
    HasUnknownPartitions,
    SmallFreeRegion { region_id: String, size_bytes: u64 },
    MbrTable,
}

// ── Core scan types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partition {
    pub number: u32,
    /// GPT PARTUUID (lowercase, with hyphens).  Empty string for MBR partitions.
    pub part_uuid: String,
    /// LBA start (in logical sectors).
    pub start_lba: u64,
    /// LBA count (in logical sectors).
    pub size_lba: u64,
    /// GPT type GUID (lowercase, with hyphens), if known.
    pub type_guid: Option<String>,
    pub class: PartitionClass,
    pub fs_type: Option<FsType>,
    pub fs_uuid: Option<String>,
    pub fs_label: Option<String>,
    /// GPT partition label.
    pub part_label: Option<String>,
}

impl Partition {
    /// Size in bytes, given the disk's logical sector size.
    pub fn size_bytes(&self, logical_sector: u32) -> u64 {
        self.size_lba * u64::from(logical_sector)
    }

    /// True when this partition carries an ARKON role label.
    pub fn is_arkon(&self) -> bool {
        self.part_label
            .as_deref()
            .map(|l| l.starts_with("ARKON-"))
            .unwrap_or(false)
            || self.class == PartitionClass::Arkon
    }

    /// True when the partition holds data the user likely cares about.
    pub fn is_valuable(&self) -> bool {
        matches!(
            self.class,
            PartitionClass::WindowsData
                | PartitionClass::Bitlocker
                | PartitionClass::WindowsRecovery
                | PartitionClass::Luks
                | PartitionClass::LinuxFs
                | PartitionClass::Lvm
                | PartitionClass::SoftwareRaid
                | PartitionClass::Zfs
                | PartitionClass::Bcache
                | PartitionClass::Arkon
                | PartitionClass::Unknown
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeRegion {
    pub region_id: RegionId,
    /// LBA start.
    pub start_lba: u64,
    /// LBA count.
    pub size_lba: u64,
}

impl FreeRegion {
    pub fn size_bytes(&self, logical_sector: u32) -> u64 {
        self.size_lba * u64::from(logical_sector)
    }
}

/// EFI firmware boot entry as read from the firmware.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareEntry {
    pub number: u16,
    pub label: String,
    pub part_uuid: Option<String>,
    pub loader_path: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Disk {
    pub id: DiskId,
    pub model: Option<String>,
    /// Last four characters of the serial number, for user display.
    pub serial_tail: Option<String>,
    pub transport: Transport,
    pub size_bytes: u64,
    pub logical_sector_bytes: u32,
    pub physical_sector_bytes: u32,
    pub table: PartitionTable,
    pub removable: bool,
    pub rotational: bool,
    pub is_install_medium: bool,
    pub partitions: Vec<Partition>,
    pub free_regions: Vec<FreeRegion>,
    pub detected_systems: Vec<DetectedSystem>,
    pub blockers: Vec<DiskBlocker>,
    pub warnings: Vec<DiskWarning>,
}

impl Disk {
    /// True when this disk can be an installation target at all (no hard blockers).
    pub fn is_eligible(&self) -> bool {
        self.blockers.is_empty()
    }

    /// Return a partition by number.
    pub fn partition(&self, number: u32) -> Option<&Partition> {
        self.partitions.iter().find(|p| p.number == number)
    }

    /// True when BitLocker is present on this disk.
    pub fn has_bitlocker(&self) -> bool {
        self.partitions.iter().any(|p| p.class == PartitionClass::Bitlocker)
            || self.detected_systems.iter().any(|s| {
                matches!(s, DetectedSystem::Windows { bitlocker: true, .. })
            })
    }

    /// True when BitLocker is present anywhere on this disk or any other disk
    /// in the same report (checked at StorageReport level).
    pub fn free_region(&self, id: &RegionId) -> Option<&FreeRegion> {
        self.free_regions.iter().find(|r| &r.region_id == id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageReport {
    /// Changes whenever any disk's layout changes.
    pub scan_id: ScanId,
    pub disks: Vec<Disk>,
    pub firmware_entries: Vec<FirmwareEntry>,
    /// True when any BitLocker volume is present on any disk.
    pub bitlocker_present: bool,
}

impl StorageReport {
    pub fn disk(&self, id: &DiskId) -> Option<&Disk> {
        self.disks.iter().find(|d| &d.id == id)
    }
}

// ── Builder helpers for tests ─────────────────────────────────────────────────

/// Build a minimal Disk for use in tests.
pub fn test_disk(
    id: &str,
    size_bytes: u64,
    logical_sector: u32,
    table: PartitionTable,
) -> Disk {
    Disk {
        id: DiskId::new(id),
        model: Some(format!("Test Disk {}", id)),
        serial_tail: Some("TEST".to_owned()),
        transport: Transport::Nvme,
        size_bytes,
        logical_sector_bytes: logical_sector,
        physical_sector_bytes: logical_sector,
        table,
        removable: false,
        rotational: false,
        is_install_medium: false,
        partitions: vec![],
        free_regions: vec![],
        detected_systems: vec![],
        blockers: vec![],
        warnings: vec![],
    }
}

pub fn test_report(disks: Vec<Disk>) -> StorageReport {
    StorageReport {
        scan_id: ScanId::new("test-scan-001"),
        disks,
        firmware_entries: vec![],
        bitlocker_present: false,
    }
}
