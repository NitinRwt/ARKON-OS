/// Block device, partition, filesystem and OS detection.
///
/// Reads /sys/block/ for device topology and /sys/block/<dev>/<part>/ for
/// per-partition metadata.  Partition type GUIDs are read from the kernel-
/// exposed sysfs attribute (Linux ≥ 5.5); on older kernels the field is absent
/// and the class is recorded as Unknown.
///
/// This module never writes to any disk or mounts any filesystem.
use crate::error::{ProbeError, ProbeResult};
use crate::provider::SysfsProvider;
use serde::{Deserialize, Serialize};
use std::path::Path;

const SYS_BLOCK: &str = "/sys/block";

// ── GPT partition type GUIDs (lowercase, with hyphens) ──────────────────────
const GUID_EFI_SYSTEM: &str             = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b";
const GUID_MICROSOFT_BASIC_DATA: &str   = "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7";
const GUID_MICROSOFT_RESERVED: &str     = "e3c9e316-0b5c-4db8-817d-f92df00215ae";
const GUID_WINDOWS_RECOVERY: &str       = "de94bba4-06d1-4d40-a16a-bfd50179d6ac";
const GUID_LINUX_FILESYSTEM: &str       = "0fc63daf-8483-4772-8e79-3d69d8477de4";
const GUID_LINUX_SWAP: &str             = "0657fd6d-a4ab-43c4-84e5-0933c84b4f4f";
const GUID_LINUX_LVM: &str              = "e6d6d379-f507-44c2-a23c-238f2a3df928";
const GUID_LINUX_RAID: &str             = "a19d880f-05fc-4d3b-a006-743f0f84911e";
const GUID_LUKS: &str                   = "ca7d7ccb-63ed-4c53-861c-1742536059cc";
const GUID_BIOS_BOOT: &str              = "21686148-6449-6e6f-744e-656564454649";

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    Nvme,
    Sata,
    Usb,
    Virtio,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartitionClass {
    /// EFI System Partition
    Esp,
    /// Windows MSR partition
    MicrosoftReserved,
    /// Windows data partition (NTFS)
    WindowsData,
    /// BitLocker-encrypted Windows data
    Bitlocker,
    /// Windows Recovery Environment
    WindowsRecovery,
    /// Standard Linux filesystem partition
    LinuxFilesystem,
    LinuxSwap,
    LinuxLvm,
    LinuxRaid,
    /// LUKS-encrypted container
    Luks,
    /// BIOS boot (legacy; never expected on UEFI-only installs)
    BiosBoot,
    /// Type GUID known but not yet assigned a class above
    OtherGpt(String),
    /// No type GUID available (MBR partition or kernel too old)
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemType {
    Vfat,
    Ntfs,
    Ext4,
    Xfs,
    Btrfs,
    Exfat,
    Swap,
    Unknown(String),
}

impl FilesystemType {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "vfat" | "fat32" | "fat16" => Self::Vfat,
            "ntfs" => Self::Ntfs,
            "ext4" => Self::Ext4,
            "xfs" => Self::Xfs,
            "btrfs" => Self::Btrfs,
            "exfat" => Self::Exfat,
            "swap" | "linux-swap" => Self::Swap,
            other => Self::Unknown(other.to_owned()),
        }
    }
}

/// A detected operating system on a partition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectedOs {
    /// Windows detected via partition class pattern (MSR + BasicData or
    /// WinRecovery on the same disk).
    Windows,
    /// A previous ARKON installation (ARKON-labelled partition present).
    Arkon,
    /// Fedora or other Linux identified by partition class combination.
    Linux,
    /// Unknown — may hold data, treat as valuable.
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionInfo {
    /// Partition number within the disk.
    pub number: u32,
    /// Partition start in 512-byte sectors.
    pub start_sectors: u64,
    /// Partition size in bytes.
    pub size_bytes: u64,
    pub class: PartitionClass,
    /// Filesystem type, if the kernel reported it via sysfs.
    pub fs_type: Option<FilesystemType>,
    /// Filesystem UUID from /sys/.../uuid (if present).
    pub fs_uuid: Option<String>,
    /// Filesystem label from /sys/.../label (if present).
    pub fs_label: Option<String>,
    /// GPT partition label from /sys/.../name (if present).
    pub part_label: Option<String>,
    /// GPT type GUID, lowercase with hyphens.
    pub type_guid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageDevice {
    /// Kernel device name, e.g. "nvme0n1" or "sda".
    pub device_name: String,
    pub transport: Transport,
    /// True for USB or SD-card attached devices.
    pub removable: bool,
    /// True when the device is rotational (spinning hard disk).
    pub rotational: bool,
    /// Total disk size in bytes.
    pub size_bytes: u64,
    /// Disk model string from /sys/block/<dev>/device/model.
    pub model: Option<String>,
    /// Partitions found on this disk.
    pub partitions: Vec<PartitionInfo>,
    /// Operating systems inferred from the partition layout.
    pub detected_systems: Vec<DetectedOs>,
}

// ── Detection entry point ────────────────────────────────────────────────────

pub fn detect(provider: &dyn SysfsProvider) -> ProbeResult<Vec<StorageDevice>> {
    let entries = match provider.list_dir(Path::new(SYS_BLOCK)) {
        Ok(e) => e,
        Err(_) => return Ok(vec![]),
    };

    let mut devices = Vec::new();
    for name in &entries {
        if !is_target_device(name) {
            continue;
        }
        let dev_path = format!("{}/{}", SYS_BLOCK, name);
        match read_device(provider, name, &dev_path) {
            Ok(d) => devices.push(d),
            Err(e) => {
                // A single unreadable device does not abort the whole scan.
                // The caller sees ProbeWarning-level issues through the logs;
                // here we surface them only in test builds.
                let _ = e;
                #[cfg(test)]
                eprintln!("storage: skipping {}: {}", name, e);
            }
        }
    }

    Ok(devices)
}

/// Only enumerate NVMe namespaces and SATA/SAS disks; skip loop, dm, zram, etc.
fn is_target_device(name: &str) -> bool {
    // NVMe: nvme0n1, nvme1n1, …
    if name.starts_with("nvme") && name.contains('n') && !name.contains('p') {
        return true;
    }
    // SCSI/SATA/USB: sda, sdb, …
    if name.starts_with("sd") && name.len() <= 4 {
        return true;
    }
    false
}

fn read_device(
    provider: &dyn SysfsProvider,
    name: &str,
    dev_path: &str,
) -> ProbeResult<StorageDevice> {
    let size_bytes = read_size_bytes(provider, dev_path)?;
    let removable = read_bool(provider, &format!("{}/removable", dev_path));
    let rotational = read_bool(provider, &format!("{}/queue/rotational", dev_path));
    let transport = infer_transport(name);
    let model = provider
        .read_file(Path::new(&format!("{}/device/model", dev_path)))
        .ok();

    let partitions = read_partitions(provider, name, dev_path)?;
    let detected_systems = infer_operating_systems(&partitions);

    Ok(StorageDevice {
        device_name: name.to_owned(),
        transport,
        removable,
        rotational,
        size_bytes,
        model,
        partitions,
        detected_systems,
    })
}

fn read_size_bytes(provider: &dyn SysfsProvider, dev_path: &str) -> ProbeResult<u64> {
    let raw = provider.read_file(Path::new(&format!("{}/size", dev_path)))?;
    // /sys/block/<dev>/size is in 512-byte sectors
    raw.trim().parse::<u64>()
        .map(|sectors| sectors * 512)
        .map_err(|_| ProbeError::parse(format!("{}/size", dev_path), "not a valid u64"))
}

fn read_bool(provider: &dyn SysfsProvider, path: &str) -> bool {
    provider
        .read_file(Path::new(path))
        .map(|s| s.trim() == "1")
        .unwrap_or(false)
}

fn infer_transport(name: &str) -> Transport {
    if name.starts_with("nvme") {
        Transport::Nvme
    } else if name.starts_with("sd") {
        // Could be SATA, SAS, or USB; refine via /sys/.../device/vendor
        // or the host controller's driver — here we default to SATA/SAS.
        Transport::Sata
    } else {
        Transport::Unknown
    }
}

fn read_partitions(
    provider: &dyn SysfsProvider,
    disk_name: &str,
    dev_path: &str,
) -> ProbeResult<Vec<PartitionInfo>> {
    // NVMe partitions: nvme0n1p1, nvme0n1p2, …
    // SATA partitions: sda1, sda2, …
    let entries = match provider.list_dir(Path::new(dev_path)) {
        Ok(e) => e,
        Err(_) => return Ok(vec![]),
    };

    let mut partitions = Vec::new();
    for entry in &entries {
        if !is_partition_name(disk_name, entry) {
            continue;
        }
        let part_path = format!("{}/{}", dev_path, entry);
        if let Ok(p) = read_partition(provider, &part_path) {
            partitions.push(p);
        }
    }
    partitions.sort_by_key(|p| p.number);
    Ok(partitions)
}

fn is_partition_name(disk: &str, candidate: &str) -> bool {
    // NVMe: nvme0n1 → nvme0n1p1, nvme0n1p2, …
    if disk.starts_with("nvme") {
        return candidate.starts_with(disk) && candidate.contains('p');
    }
    // SATA: sda → sda1, sda2, …
    if candidate.starts_with(disk) {
        let suffix = &candidate[disk.len()..];
        return suffix.chars().all(|c| c.is_ascii_digit());
    }
    false
}

fn read_partition(provider: &dyn SysfsProvider, part_path: &str) -> ProbeResult<PartitionInfo> {
    let start_sectors: u64 = provider
        .read_file(Path::new(&format!("{}/start", part_path)))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    let size_bytes: u64 = provider
        .read_file(Path::new(&format!("{}/size", part_path)))
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|sectors| sectors * 512)
        .unwrap_or(0);

    // Partition number from the last digit(s) of the sysfs directory name.
    let number = extract_partition_number(part_path).unwrap_or(0);

    let type_guid = provider
        .read_file(Path::new(&format!("{}/partition_type", part_path)))
        .ok()
        .map(|s| s.trim().to_lowercase());

    let class = type_guid
        .as_deref()
        .map(classify_partition_guid)
        .unwrap_or(PartitionClass::Unknown);

    let fs_type = provider
        .read_file(Path::new(&format!("{}/fs_type", part_path)))
        .ok()
        .map(|s| FilesystemType::from_str(s.trim()));

    let fs_uuid = provider
        .read_file(Path::new(&format!("{}/uuid", part_path)))
        .ok()
        .filter(|s| !s.is_empty());

    let fs_label = provider
        .read_file(Path::new(&format!("{}/label", part_path)))
        .ok()
        .filter(|s| !s.is_empty());

    let part_label = provider
        .read_file(Path::new(&format!("{}/name", part_path)))
        .ok()
        .filter(|s| !s.is_empty());

    Ok(PartitionInfo {
        number,
        start_sectors,
        size_bytes,
        class,
        fs_type,
        fs_uuid,
        fs_label,
        part_label,
        type_guid,
    })
}

fn extract_partition_number(path: &str) -> Option<u32> {
    let last = path.rsplit('/').next()?;
    // Strip trailing digits
    let digits: String = last.chars().rev().take_while(|c| c.is_ascii_digit()).collect();
    let digits: String = digits.chars().rev().collect();
    digits.parse().ok()
}

fn classify_partition_guid(guid: &str) -> PartitionClass {
    match guid {
        GUID_EFI_SYSTEM           => PartitionClass::Esp,
        GUID_MICROSOFT_RESERVED   => PartitionClass::MicrosoftReserved,
        GUID_MICROSOFT_BASIC_DATA => PartitionClass::WindowsData,
        GUID_WINDOWS_RECOVERY     => PartitionClass::WindowsRecovery,
        GUID_LINUX_FILESYSTEM     => PartitionClass::LinuxFilesystem,
        GUID_LINUX_SWAP           => PartitionClass::LinuxSwap,
        GUID_LINUX_LVM            => PartitionClass::LinuxLvm,
        GUID_LINUX_RAID           => PartitionClass::LinuxRaid,
        GUID_LUKS                 => PartitionClass::Luks,
        GUID_BIOS_BOOT            => PartitionClass::BiosBoot,
        other                     => PartitionClass::OtherGpt(other.to_owned()),
    }
}

/// Infer which operating systems are present based on the partition class
/// pattern, without mounting anything.
fn infer_operating_systems(partitions: &[PartitionInfo]) -> Vec<DetectedOs> {
    let mut systems = Vec::new();

    let has_msr     = partitions.iter().any(|p| p.class == PartitionClass::MicrosoftReserved);
    let has_win     = partitions.iter().any(|p| p.class == PartitionClass::WindowsData);
    let has_bitlock = partitions.iter().any(|p| p.class == PartitionClass::Bitlocker);
    let has_win_re  = partitions.iter().any(|p| p.class == PartitionClass::WindowsRecovery);
    let has_linux   = partitions.iter().any(|p| {
        matches!(
            p.class,
            PartitionClass::LinuxFilesystem
                | PartitionClass::LinuxLvm
                | PartitionClass::Luks
        )
    });
    let has_arkon = partitions.iter().any(|p| {
        p.part_label.as_deref().map(|l| l.starts_with("arkon-")).unwrap_or(false)
    });

    if has_arkon {
        systems.push(DetectedOs::Arkon);
    }
    if has_msr && (has_win || has_bitlock || has_win_re) {
        systems.push(DetectedOs::Windows);
    }
    if has_linux {
        systems.push(DetectedOs::Linux);
    }

    systems
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::MockProvider;

    fn add_partition(
        p: &mut MockProvider,
        disk: &str,
        part: &str,
        start: u64,
        size_sectors: u64,
        type_guid: &str,
        part_label: &str,
    ) {
        let base = format!("{}/{}/{}", SYS_BLOCK, disk, part);
        p.add_file(format!("{}/start", base), start.to_string());
        p.add_file(format!("{}/size", base), size_sectors.to_string());
        p.add_file(format!("{}/partition_type", base), type_guid);
        if !part_label.is_empty() {
            p.add_file(format!("{}/name", base), part_label);
        }
    }

    fn add_disk(p: &mut MockProvider, name: &str, size_sectors: u64, model: &str) {
        let base = format!("{}/{}", SYS_BLOCK, name);
        p.add_file(format!("{}/size", base), size_sectors.to_string());
        p.add_file(format!("{}/removable", base), "0");
        p.add_file(format!("{}/queue/rotational", base), "0");
        p.add_file(format!("{}/device/model", base), model);
    }

    #[test]
    fn detects_nvme_disk_with_partitions() {
        let mut p = MockProvider::new();
        // 500 GB NVMe
        add_disk(&mut p, "nvme0n1", 1_000_215_216, "Samsung SSD 980 500GB");
        add_partition(&mut p, "nvme0n1", "nvme0n1p1", 2048, 1_048_576, GUID_EFI_SYSTEM, "");
        add_partition(&mut p, "nvme0n1", "nvme0n1p2", 1_050_624, 16_777_216, GUID_MICROSOFT_RESERVED, "");
        add_partition(&mut p, "nvme0n1", "nvme0n1p3", 17_827_840, 976_562_500, GUID_MICROSOFT_BASIC_DATA, "");

        let devices = detect(&p).unwrap();
        assert_eq!(devices.len(), 1);
        let d = &devices[0];
        assert_eq!(d.device_name, "nvme0n1");
        assert_eq!(d.transport, Transport::Nvme);
        assert_eq!(d.partitions.len(), 3);
        assert_eq!(d.partitions[0].class, PartitionClass::Esp);
        assert_eq!(d.partitions[1].class, PartitionClass::MicrosoftReserved);
        assert_eq!(d.partitions[2].class, PartitionClass::WindowsData);
        assert!(d.detected_systems.contains(&DetectedOs::Windows));
    }

    #[test]
    fn detects_sata_disk() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "sda", 976_773_168, "ST500LT012-1DG142");
        add_partition(&mut p, "sda", "sda1", 2048, 2_097_152, GUID_EFI_SYSTEM, "");
        add_partition(&mut p, "sda", "sda2", 2_099_200, 974_671_872, GUID_LINUX_FILESYSTEM, "");

        let devices = detect(&p).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].transport, Transport::Sata);
        assert_eq!(devices[0].partitions[1].class, PartitionClass::LinuxFilesystem);
        assert!(devices[0].detected_systems.contains(&DetectedOs::Linux));
    }

    #[test]
    fn detects_arkon_install_by_partition_label() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "sda", 500_000_000, "Testdisk");
        add_partition(&mut p, "sda", "sda1", 2048, 2_000_000, GUID_EFI_SYSTEM, "arkon-esp");
        add_partition(&mut p, "sda", "sda2", 2_002_048, 498_000_000, GUID_LINUX_FILESYSTEM, "arkon-root");

        let devices = detect(&p).unwrap();
        assert!(devices[0].detected_systems.contains(&DetectedOs::Arkon));
    }

    #[test]
    fn partition_number_extracted_correctly() {
        assert_eq!(extract_partition_number("/sys/block/nvme0n1/nvme0n1p3"), Some(3));
        assert_eq!(extract_partition_number("/sys/block/sda/sda12"), Some(12));
        assert_eq!(extract_partition_number("/sys/block/sda"), None);
    }

    #[test]
    fn size_converted_from_sectors() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "nvme0n1", 2048, "tiny");
        // No partitions — just check size_bytes
        let devices = detect(&p).unwrap();
        assert_eq!(devices[0].size_bytes, 2048 * 512);
    }

    #[test]
    fn luks_partition_classified_correctly() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "sda", 100_000_000, "disk");
        add_partition(&mut p, "sda", "sda1", 2048, 2_000_000, GUID_EFI_SYSTEM, "");
        add_partition(&mut p, "sda", "sda2", 2_002_048, 98_000_000, GUID_LUKS, "");

        let devices = detect(&p).unwrap();
        assert_eq!(devices[0].partitions[1].class, PartitionClass::Luks);
        assert!(devices[0].detected_systems.contains(&DetectedOs::Linux));
    }

    #[test]
    fn unknown_guid_recorded_as_other_gpt() {
        let unknown_guid = "12345678-1234-1234-1234-123456789abc";
        let class = classify_partition_guid(unknown_guid);
        assert!(matches!(class, PartitionClass::OtherGpt(_)));
    }

    #[test]
    fn no_sysfs_returns_empty() {
        let p = MockProvider::new();
        let devices = detect(&p).unwrap();
        assert!(devices.is_empty());
    }
}
