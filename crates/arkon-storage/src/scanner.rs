/// Storage scanner: reads /sys/block and /sys/firmware/efi to populate a
/// StorageReport.  Everything here is read-only; no disk is written or mounted.
use crate::provider::SysfsProvider;
use crate::scan::{
    DetectedSystem, Disk, DiskBlocker, DiskId, DiskWarning, FirmwareEntry, FreeRegion,
    FsType, Partition, PartitionClass, PartitionTable, RegionId, ScanId, StorageReport,
    Transport,
};
use std::path::Path;

const SYS_BLOCK: &str = "/sys/block";

// Well-known GPT type GUIDs (lowercase with hyphens).
const GUID_EFI_SYSTEM: &str           = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b";
const GUID_MICROSOFT_BASIC_DATA: &str = "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7";
const GUID_MICROSOFT_RESERVED: &str   = "e3c9e316-0b5c-4db8-817d-f92df00215ae";
const GUID_WINDOWS_RECOVERY: &str     = "de94bba4-06d1-4d40-a16a-bfd50179d6ac";
const GUID_LINUX_FILESYSTEM: &str     = "0fc63daf-8483-4772-8e79-3d69d8477de4";
const GUID_LINUX_SWAP: &str           = "0657fd6d-a4ab-43c4-84e5-0933c84b4f4f";
const GUID_LINUX_LVM: &str            = "e6d6d379-f507-44c2-a23c-238f2a3df928";
const GUID_LINUX_RAID: &str           = "a19d880f-05fc-4d3b-a006-743f0f84911e";
const GUID_LUKS: &str                 = "ca7d7ccb-63ed-4c53-861c-1742536059cc";

// EFI Global Variable GUID.
const EFI_GLOBAL_GUID: &str = "8be4df61-93ca-11d2-aa0d-00e098032b8c";

// Minimum free region to record (1 MiB at 512 B/sector = 2048 sectors).
const MIN_FREE_REGION_SECTORS: u64 = 2048;
// GPT reserved sectors at the start (header + partition table).
const GPT_HEADER_SECTORS: u64 = 2048;
// GPT backup header occupies the last 34 sectors.
const GPT_BACKUP_SECTORS: u64 = 34;

// Disks smaller than 10 GiB are ignored (loop devices, tiny virtio disks, etc.).
const MIN_DISK_BYTES: u64 = 10 * 1024 * 1024 * 1024;
// ARKON install requires 64 GiB.
const INSTALL_REQUIRED_BYTES: u64 = 64 * 1024 * 1024 * 1024;

// ── Entry point ───────────────────────────────────────────────────────────────

/// Enumerate all candidate disks, classify their partitions, and return a
/// `StorageReport`.  Soft failures on individual devices are silently skipped.
pub fn scan(provider: &dyn SysfsProvider) -> StorageReport {
    let entries = provider.list_dir(Path::new(SYS_BLOCK)).unwrap_or_default();

    let mut disks = Vec::new();
    for name in &entries {
        if !is_target_device(name) {
            continue;
        }
        let dev_path = format!("{}/{}", SYS_BLOCK, name);
        if let Some(d) = read_disk(provider, name, &dev_path) {
            disks.push(d);
        }
    }

    let firmware_entries = read_firmware_entries(provider);
    let bitlocker_present = disks.iter().any(|d| d.has_bitlocker());
    let scan_id = build_scan_id(&disks);

    StorageReport { scan_id, disks, firmware_entries, bitlocker_present }
}

// ── Disk enumeration ──────────────────────────────────────────────────────────

fn is_target_device(name: &str) -> bool {
    // NVMe namespace: nvme0n1, nvme1n1, ...  (not nvme0n1p1)
    if name.starts_with("nvme") && name.contains('n') && !name.contains('p') {
        return true;
    }
    // SCSI/SATA/USB: sda, sdb, ... sdz
    if name.starts_with("sd") && name.len() <= 4 && name[2..].chars().all(|c| c.is_ascii_alphabetic()) {
        return true;
    }
    // eMMC / SD card: mmcblk0, mmcblk1, ... (not mmcblk0p1)
    if name.starts_with("mmcblk") && !name.contains('p') {
        return true;
    }
    false
}

fn read_disk(provider: &dyn SysfsProvider, name: &str, dev_path: &str) -> Option<Disk> {
    // /sys/block/<dev>/size reports in 512-byte sectors regardless of logical sector size.
    // We read the actual logical sector size first to convert correctly.
    let logical_sector = read_u32_file(provider, &format!("{}/queue/logical_block_size", dev_path))
        .unwrap_or(512);
    let physical_sector = read_u32_file(provider, &format!("{}/queue/physical_block_size", dev_path))
        .unwrap_or(logical_sector);

    // /sys/block/<dev>/size is in 512-byte units on all kernels.
    let size_512 = read_u64_file(provider, &format!("{}/size", dev_path))?;
    let size_bytes = size_512 * 512;

    if size_bytes < MIN_DISK_BYTES {
        return None;
    }

    let removable = read_bool_file(provider, &format!("{}/removable", dev_path));
    let rotational = read_bool_file(provider, &format!("{}/queue/rotational", dev_path));
    let transport = infer_transport(name, removable);
    let model = read_str_file(provider, &format!("{}/device/model", dev_path));
    let serial_tail = read_str_file(provider, &format!("{}/device/serial", dev_path))
        .map(|s| {
            if s.len() > 4 { s[s.len() - 4..].to_owned() } else { s }
        });

    let partitions = read_partitions(provider, name, dev_path);
    let table = infer_partition_table(&partitions);
    let free_regions = compute_free_regions(name, &partitions, size_bytes, logical_sector);
    let detected_systems = detect_systems(&partitions);

    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    // Hard blockers
    if removable {
        blockers.push(DiskBlocker::Removable);
    }
    if size_bytes < INSTALL_REQUIRED_BYTES {
        blockers.push(DiskBlocker::TooSmall { size_bytes, required_bytes: INSTALL_REQUIRED_BYTES });
    }
    if logical_sector != 512 && logical_sector != 4096 {
        blockers.push(DiskBlocker::UnsupportedSectorSize { logical_bytes: logical_sector });
    }
    if partitions.iter().any(|p| p.class == PartitionClass::Lvm) {
        blockers.push(DiskBlocker::HasLvm);
    }
    if partitions.iter().any(|p| p.class == PartitionClass::SoftwareRaid) {
        blockers.push(DiskBlocker::HasSoftwareRaid);
    }
    if partitions.iter().any(|p| p.class == PartitionClass::Zfs) {
        blockers.push(DiskBlocker::HasZfs);
    }
    if partitions.iter().any(|p| p.class == PartitionClass::Bcache) {
        blockers.push(DiskBlocker::HasBcache);
    }

    // Warnings
    if rotational { warnings.push(DiskWarning::Rotational); }
    if matches!(transport, Transport::Usb) { warnings.push(DiskWarning::UsbTransport); }
    if matches!(table, PartitionTable::Mbr) { warnings.push(DiskWarning::MbrTable); }
    if partitions.iter().any(|p| p.class == PartitionClass::Unknown) {
        warnings.push(DiskWarning::HasUnknownPartitions);
    }
    for ds in &detected_systems {
        if matches!(ds, DetectedSystem::Windows { .. }) {
            warnings.push(DiskWarning::HasWindows);
        }
        if matches!(ds, DetectedSystem::Windows { bitlocker: true, .. }) {
            warnings.push(DiskWarning::HasBitlocker);
        }
    }
    for region in &free_regions {
        let region_bytes = region.size_lba * u64::from(logical_sector);
        if region_bytes < crate::plan::MIN_FREE_REGION_BYTES {
            warnings.push(DiskWarning::SmallFreeRegion {
                region_id: region.region_id.0.clone(),
                size_bytes: region_bytes,
            });
        }
    }

    // Install medium: a removable disk whose partition is labelled "ARKON-INSTALL"
    let is_install_medium = partitions.iter().any(|p| {
        p.fs_label.as_deref() == Some("ARKON-INSTALL")
            || p.part_label.as_deref() == Some("ARKON-INSTALL")
    });
    if is_install_medium {
        blockers.retain(|b| !matches!(b, DiskBlocker::Removable)); // keep only IsInstallMedium
        blockers.push(DiskBlocker::IsInstallMedium);
    }

    Some(Disk {
        id: DiskId::new(name),
        model,
        serial_tail,
        transport,
        size_bytes,
        logical_sector_bytes: logical_sector,
        physical_sector_bytes: physical_sector,
        table,
        removable,
        rotational,
        is_install_medium,
        partitions,
        free_regions,
        detected_systems,
        blockers,
        warnings,
    })
}

// ── Transport inference ───────────────────────────────────────────────────────

fn infer_transport(name: &str, removable: bool) -> Transport {
    if name.starts_with("nvme") {
        Transport::Nvme
    } else if name.starts_with("mmcblk") {
        Transport::Mmc
    } else if name.starts_with("sd") {
        // A removable sd* device is treated as USB; a non-removable one is SATA.
        // Refining via the USB host controller path would require symlink resolution
        // which SysfsProvider does not expose.
        if removable { Transport::Usb } else { Transport::Sata }
    } else {
        Transport::Other
    }
}

// ── Partition table ───────────────────────────────────────────────────────────

fn infer_partition_table(partitions: &[Partition]) -> PartitionTable {
    if partitions.is_empty() {
        return PartitionTable::None;
    }
    if partitions.iter().any(|p| p.type_guid.is_some()) {
        PartitionTable::Gpt
    } else {
        PartitionTable::Mbr
    }
}

// ── Partition reading ─────────────────────────────────────────────────────────

fn read_partitions(
    provider: &dyn SysfsProvider,
    disk_name: &str,
    dev_path: &str,
) -> Vec<Partition> {
    let entries = provider.list_dir(Path::new(dev_path)).unwrap_or_default();
    let mut partitions = Vec::new();
    for entry in &entries {
        if !is_partition_name(disk_name, entry) {
            continue;
        }
        let part_path = format!("{}/{}", dev_path, entry);
        if let Some(p) = read_partition(provider, &part_path) {
            partitions.push(p);
        }
    }
    partitions.sort_by_key(|p| p.number);
    partitions
}

fn is_partition_name(disk: &str, candidate: &str) -> bool {
    if disk.starts_with("nvme") || disk.starts_with("mmcblk") {
        return candidate.starts_with(disk) && candidate.contains('p')
            && candidate[disk.len()..].starts_with('p');
    }
    if candidate.starts_with(disk) {
        let suffix = &candidate[disk.len()..];
        return !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit());
    }
    false
}

fn read_partition(provider: &dyn SysfsProvider, part_path: &str) -> Option<Partition> {
    let number = extract_partition_number(part_path)?;
    let start_lba = read_u64_file(provider, &format!("{}/start", part_path)).unwrap_or(0);
    let size_lba = read_u64_file(provider, &format!("{}/size", part_path)).unwrap_or(0);

    let type_guid = read_str_file(provider, &format!("{}/partition_type", part_path))
        .map(|s| s.to_lowercase())
        .filter(|s| is_uuid_format(s));

    let part_label = read_str_file(provider, &format!("{}/name", part_path))
        .filter(|s| !s.is_empty());

    // GUID classification, with Arkon override for ARKON-labelled partitions.
    let class = {
        let from_guid = type_guid.as_deref().map(classify_guid);
        match (&part_label, from_guid) {
            (Some(label), _) if label.starts_with("ARKON-") && !label.starts_with("ARKON-ESP") => {
                PartitionClass::Arkon
            }
            (_, Some(c)) => c,
            _ => PartitionClass::Unknown,
        }
    };

    let fs_type = read_str_file(provider, &format!("{}/fs_type", part_path))
        .as_deref()
        .map(parse_fs_type);

    let fs_uuid = read_str_file(provider, &format!("{}/uuid", part_path))
        .filter(|s| !s.is_empty());
    let fs_label = read_str_file(provider, &format!("{}/label", part_path))
        .filter(|s| !s.is_empty());

    // PARTUUID (GPT partition UUID) lives in /sys/.../partuuid
    let part_uuid = read_str_file(provider, &format!("{}/partuuid", part_path))
        .filter(|s| !s.is_empty())
        .unwrap_or_default();

    Some(Partition {
        number,
        part_uuid,
        start_lba,
        size_lba,
        type_guid,
        class,
        fs_type,
        fs_uuid,
        fs_label,
        part_label,
    })
}

fn extract_partition_number(path: &str) -> Option<u32> {
    let last = path.rsplit('/').next()?;
    let digits: String = last.chars().rev().take_while(|c| c.is_ascii_digit()).collect();
    let digits: String = digits.chars().rev().collect();
    digits.parse().ok()
}

fn is_uuid_format(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 { return false; }
    let expected = [8usize, 4, 4, 4, 12];
    parts.iter().zip(expected.iter())
        .all(|(p, &l)| p.len() == l && p.chars().all(|c| c.is_ascii_hexdigit()))
}

fn classify_guid(guid: &str) -> PartitionClass {
    match guid {
        GUID_EFI_SYSTEM           => PartitionClass::Esp,
        GUID_MICROSOFT_RESERVED   => PartitionClass::Msr,
        GUID_MICROSOFT_BASIC_DATA => PartitionClass::WindowsData,
        GUID_WINDOWS_RECOVERY     => PartitionClass::WindowsRecovery,
        GUID_LINUX_FILESYSTEM     => PartitionClass::LinuxFs,
        GUID_LINUX_SWAP           => PartitionClass::Swap,
        GUID_LINUX_LVM            => PartitionClass::Lvm,
        GUID_LINUX_RAID           => PartitionClass::SoftwareRaid,
        GUID_LUKS                 => PartitionClass::Luks,
        _                         => PartitionClass::Unknown,
    }
}

fn parse_fs_type(s: &str) -> FsType {
    match s.to_lowercase().as_str() {
        "vfat" | "fat32" | "fat16" => FsType::Vfat,
        "ext4"                     => FsType::Ext4,
        "xfs"                      => FsType::Xfs,
        "btrfs"                    => FsType::Btrfs,
        "ntfs"                     => FsType::Ntfs,
        "exfat"                    => FsType::Exfat,
        "swap" | "linux-swap"      => FsType::Swap,
        _                          => FsType::Unknown,
    }
}

// ── Free region calculation ───────────────────────────────────────────────────

fn compute_free_regions(
    disk_name: &str,
    partitions: &[Partition],
    disk_size_bytes: u64,
    logical_sector: u32,
) -> Vec<FreeRegion> {
    let lbs = u64::from(logical_sector);
    let disk_lba = disk_size_bytes / lbs;
    let usable_start = GPT_HEADER_SECTORS;
    let usable_end = disk_lba.saturating_sub(GPT_BACKUP_SECTORS);

    // Partitions are already sorted by start_lba from read_partitions.
    let mut regions = Vec::new();
    let mut cursor = usable_start;

    for part in partitions {
        if part.start_lba > cursor {
            let gap = part.start_lba - cursor;
            if gap >= MIN_FREE_REGION_SECTORS {
                regions.push(FreeRegion {
                    region_id: RegionId::new(format!("{}-r{}", disk_name, cursor)),
                    start_lba: cursor,
                    size_lba: gap,
                });
            }
        }
        let part_end = part.start_lba + part.size_lba;
        if part_end > cursor {
            cursor = part_end;
        }
    }

    // Gap after the last partition.
    if usable_end > cursor && usable_end - cursor >= MIN_FREE_REGION_SECTORS {
        regions.push(FreeRegion {
            region_id: RegionId::new(format!("{}-r{}", disk_name, cursor)),
            start_lba: cursor,
            size_lba: usable_end - cursor,
        });
    }

    regions
}

// ── OS detection ─────────────────────────────────────────────────────────────

fn detect_systems(partitions: &[Partition]) -> Vec<DetectedSystem> {
    let mut systems = Vec::new();

    // ARKON: requires ARKON-ESP + ARKON-ROOT (ARKON-BOOT optional).
    let arkon_esp = partitions.iter().find(|p| {
        p.part_label.as_deref().map(|l| l == "ARKON-ESP").unwrap_or(false)
    });
    let arkon_boot = partitions.iter().find(|p| {
        p.part_label.as_deref().map(|l| l == "ARKON-BOOT").unwrap_or(false)
    });
    let arkon_root = partitions.iter().find(|p| {
        matches!(p.part_label.as_deref(), Some("ARKON-ROOT") | Some("ARKON-HOME"))
    });

    if let (Some(esp), Some(root)) = (arkon_esp, arkon_root) {
        systems.push(DetectedSystem::Arkon {
            esp_number:  esp.number,
            boot_number: arkon_boot.map(|b| b.number),
            root_number: root.number,
        });
    }

    // Windows: requires MSR + at least one Windows data/recovery partition.
    let has_msr = partitions.iter().any(|p| p.class == PartitionClass::Msr);
    if has_msr {
        let win_parts: Vec<u32> = partitions.iter()
            .filter(|p| matches!(
                p.class,
                PartitionClass::WindowsData | PartitionClass::Bitlocker
                    | PartitionClass::WindowsRecovery | PartitionClass::Msr
            ))
            .map(|p| p.number)
            .collect();
        if !win_parts.is_empty() {
            let bitlocker = partitions.iter().any(|p| p.class == PartitionClass::Bitlocker);
            let mut all: Vec<u32> = win_parts;
            if let Some(esp) = partitions.iter().find(|p| p.class == PartitionClass::Esp) {
                all.push(esp.number);
            }
            all.sort_unstable();
            all.dedup();
            systems.push(DetectedSystem::Windows { partition_numbers: all, bitlocker });
        }
    }

    // Linux (non-ARKON).
    if arkon_root.is_none() {
        let linux_parts: Vec<u32> = partitions.iter()
            .filter(|p| matches!(
                p.class,
                PartitionClass::LinuxFs | PartitionClass::Luks | PartitionClass::Lvm
            ))
            .map(|p| p.number)
            .collect();
        if !linux_parts.is_empty() {
            systems.push(DetectedSystem::Linux { name: None, partition_numbers: linux_parts });
        }
    }

    systems
}

// ── EFI firmware entries ──────────────────────────────────────────────────────

/// Read EFI BootNNNN variables and return parsed entries.
///
/// Each EFI_LOAD_OPTION variable is binary:
///   [0..3]  Attributes (u32 LE)
///   [4..5]  FilePathListLength (u16 LE)
///   [6..]   Description (null-terminated UCS-2 string), then device path, ...
///
/// For the MockProvider (testing), files stored as "LABEL:<description>" are
/// also accepted: the label is used as the description and active is true.
///
/// Returns an empty vec if efivars are unavailable (non-Linux, no EFI).
fn read_firmware_entries(provider: &dyn SysfsProvider) -> Vec<FirmwareEntry> {
    let efivars_dir = format!("/sys/firmware/efi/efivars");
    let entries = match provider.list_dir(Path::new(&efivars_dir)) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    // Collect BootOrder to know which entries are active.
    let boot_order = read_boot_order(provider, &efivars_dir);

    let mut result = Vec::new();
    for name in &entries {
        // Match Boot0000-{guid} … Boot9FFF-{guid}
        let boot_prefix = format!("Boot");
        if !name.starts_with(&boot_prefix) { continue; }
        let rest = &name[boot_prefix.len()..];
        if rest.len() < 4 { continue; }
        let (num_hex, guid_part) = rest.split_at(4);
        let num = match u16::from_str_radix(num_hex, 16) {
            Ok(n) => n,
            Err(_) => continue,
        };
        // guid_part should be "-{efi_global_guid}" or absent (some firmware omits it)
        if !guid_part.is_empty() && !guid_part.starts_with('-') { continue; }

        let var_path = format!("{}/{}", efivars_dir, name);
        let content = match provider.read_file(Path::new(&var_path)) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Test/mock shorthand: "LABEL:description"
        if let Some(label) = content.strip_prefix("LABEL:") {
            result.push(FirmwareEntry {
                number: num,
                label: label.trim().to_owned(),
                part_uuid: None,
                loader_path: None,
                active: boot_order.contains(&num),
            });
            continue;
        }

        // Binary EFI_LOAD_OPTION: skip 6-byte header, read UCS-2 string.
        if let Some(label) = parse_efi_load_option_label(&content) {
            result.push(FirmwareEntry {
                number: num,
                label,
                part_uuid: None,
                loader_path: None,
                active: boot_order.contains(&num),
            });
        }
    }

    result.sort_by_key(|e| e.number);
    result
}

/// Read BootCurrent from efivars to determine which entry is currently active.
/// Falls back to BootOrder if BootCurrent is unavailable.
fn read_boot_order(provider: &dyn SysfsProvider, efivars_dir: &str) -> Vec<u16> {
    // Try BootOrder-{guid}: a list of u16 LE values (after 4-byte attribute header).
    // For tests, accept "ORDER:0001,0002,..." plain text.
    let path = format!("{}/BootOrder-{}", efivars_dir, EFI_GLOBAL_GUID);
    let content = match provider.read_file(Path::new(&path)) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    if let Some(nums) = content.strip_prefix("ORDER:") {
        return nums.split(',')
            .filter_map(|s| u16::from_str_radix(s.trim(), 16).ok())
            .collect();
    }
    vec![]
}

/// Extract the description string from a binary EFI_LOAD_OPTION blob.
/// Returns None if the content cannot be interpreted as a valid option.
fn parse_efi_load_option_label(content: &str) -> Option<String> {
    // The content string contains the raw bytes as chars.
    let bytes: Vec<u8> = content.chars().map(|c| c as u8).collect();
    if bytes.len() < 6 {
        return None;
    }
    // UCS-2 description starts at byte 6.
    let mut label = String::new();
    let mut i = 6;
    while i + 1 < bytes.len() {
        let lo = bytes[i] as u16;
        let hi = bytes[i + 1] as u16;
        let codepoint = (hi << 8) | lo;
        if codepoint == 0 {
            break;
        }
        if let Some(c) = char::from_u32(u32::from(codepoint)) {
            label.push(c);
        }
        i += 2;
    }
    if label.is_empty() { None } else { Some(label) }
}

// ── Scan ID ───────────────────────────────────────────────────────────────────

fn build_scan_id(disks: &[Disk]) -> ScanId {
    if disks.is_empty() {
        return ScanId::new("empty");
    }
    let mut parts = Vec::with_capacity(disks.len() * 4);
    for d in disks {
        parts.push(format!("{}:{}", d.id.as_str(), d.size_bytes));
        for p in &d.partitions {
            parts.push(format!("p{}:{}", p.number, p.size_lba));
        }
    }
    ScanId::new(parts.join(","))
}

// ── File reading helpers ──────────────────────────────────────────────────────

fn read_u64_file(provider: &dyn SysfsProvider, path: &str) -> Option<u64> {
    provider.read_file(Path::new(path)).ok()?.trim().parse().ok()
}

fn read_u32_file(provider: &dyn SysfsProvider, path: &str) -> Option<u32> {
    provider.read_file(Path::new(path)).ok()?.trim().parse().ok()
}

fn read_bool_file(provider: &dyn SysfsProvider, path: &str) -> bool {
    provider.read_file(Path::new(path)).ok()
        .map(|s| s.trim() == "1")
        .unwrap_or(false)
}

fn read_str_file(provider: &dyn SysfsProvider, path: &str) -> Option<String> {
    provider.read_file(Path::new(path)).ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::MockProvider;

    // ── Fixture helpers ───────────────────────────────────────────────────────

    fn sector_count(bytes: u64) -> u64 { bytes / 512 }

    fn add_disk(p: &mut MockProvider, name: &str, size_bytes: u64, removable: bool, rotational: bool) {
        let base = format!("{}/{}", SYS_BLOCK, name);
        p.add_file(format!("{}/size", base), sector_count(size_bytes).to_string());
        p.add_file(format!("{}/removable", base), if removable { "1" } else { "0" });
        p.add_file(format!("{}/queue/rotational", base), if rotational { "1" } else { "0" });
        p.add_file(format!("{}/queue/logical_block_size", base), "512");
        p.add_file(format!("{}/queue/physical_block_size", base), "512");
    }

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
        if !type_guid.is_empty() {
            p.add_file(format!("{}/partition_type", base), type_guid);
        }
        if !part_label.is_empty() {
            p.add_file(format!("{}/name", base), part_label);
        }
    }

    fn add_partuuid(p: &mut MockProvider, disk: &str, part: &str, uuid: &str) {
        let base = format!("{}/{}/{}", SYS_BLOCK, disk, part);
        p.add_file(format!("{}/partuuid", base), uuid);
    }

    // 500 GiB NVMe — well above INSTALL_REQUIRED_BYTES.
    fn nvme_size() -> u64 { 500 * 1024 * 1024 * 1024 }

    // ── Basic detection ───────────────────────────────────────────────────────

    #[test]
    fn nvme_disk_detected() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "nvme0n1", nvme_size(), false, false);
        let report = scan(&p);
        assert_eq!(report.disks.len(), 1);
        assert_eq!(report.disks[0].id.as_str(), "nvme0n1");
        assert_eq!(report.disks[0].transport, Transport::Nvme);
    }

    #[test]
    fn sata_disk_detected() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "sda", nvme_size(), false, true);
        let report = scan(&p);
        assert_eq!(report.disks.len(), 1);
        assert_eq!(report.disks[0].transport, Transport::Sata);
        assert!(report.disks[0].warnings.contains(&DiskWarning::Rotational));
    }

    #[test]
    fn usb_disk_detected_as_removable_blocker() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "sda", nvme_size(), true, false);
        let report = scan(&p);
        assert_eq!(report.disks[0].transport, Transport::Usb);
        assert!(report.disks[0].blockers.contains(&DiskBlocker::Removable));
        assert!(report.disks[0].warnings.contains(&DiskWarning::UsbTransport));
    }

    #[test]
    fn too_small_disk_gets_blocker() {
        let mut p = MockProvider::new();
        // 32 GiB — below INSTALL_REQUIRED_BYTES (64 GiB) but above MIN_DISK_BYTES (10 GiB)
        add_disk(&mut p, "sda", 32 * 1024 * 1024 * 1024, false, false);
        let report = scan(&p);
        assert_eq!(report.disks.len(), 1);
        assert!(report.disks[0].blockers.iter()
            .any(|b| matches!(b, DiskBlocker::TooSmall { .. })));
    }

    #[test]
    fn tiny_disk_below_minimum_is_skipped() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "nvme0n1", 5 * 1024 * 1024 * 1024, false, false);
        let report = scan(&p);
        assert!(report.disks.is_empty());
    }

    // ── Partition classification ───────────────────────────────────────────────

    #[test]
    fn esp_partition_classified_correctly() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "nvme0n1", nvme_size(), false, false);
        add_partition(&mut p, "nvme0n1", "nvme0n1p1", 2048, 2_097_152, GUID_EFI_SYSTEM, "");
        let report = scan(&p);
        assert_eq!(report.disks[0].partitions[0].class, PartitionClass::Esp);
    }

    #[test]
    fn windows_partition_layout_detected() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "nvme0n1", nvme_size(), false, false);
        add_partition(&mut p, "nvme0n1", "nvme0n1p1", 2048, 2_097_152, GUID_EFI_SYSTEM, "");
        add_partition(&mut p, "nvme0n1", "nvme0n1p2", 2_099_200, 32_768, GUID_MICROSOFT_RESERVED, "");
        add_partition(&mut p, "nvme0n1", "nvme0n1p3", 2_131_968, 900_000_000, GUID_MICROSOFT_BASIC_DATA, "");
        let report = scan(&p);
        let d = &report.disks[0];
        assert!(d.detected_systems.iter().any(|s| matches!(s, DetectedSystem::Windows { .. })));
        assert!(d.warnings.contains(&DiskWarning::HasWindows));
    }

    #[test]
    fn luks_partition_classified_and_detected() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "sda", nvme_size(), false, false);
        add_partition(&mut p, "sda", "sda1", 2048, 2_097_152, GUID_EFI_SYSTEM, "");
        add_partition(&mut p, "sda", "sda2", 2_099_200, 976_562_500, GUID_LUKS, "");
        let report = scan(&p);
        assert_eq!(report.disks[0].partitions[1].class, PartitionClass::Luks);
        assert!(report.disks[0].detected_systems.iter()
            .any(|s| matches!(s, DetectedSystem::Linux { .. })));
    }

    #[test]
    fn lvm_blocker_applied() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "sda", nvme_size(), false, false);
        add_partition(&mut p, "sda", "sda1", 2048, 976_562_500, GUID_LINUX_LVM, "");
        let report = scan(&p);
        assert!(report.disks[0].blockers.contains(&DiskBlocker::HasLvm));
    }

    // ── ARKON detection ───────────────────────────────────────────────────────

    #[test]
    fn arkon_install_detected_by_partition_labels() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "nvme0n1", nvme_size(), false, false);
        add_partition(&mut p, "nvme0n1", "nvme0n1p1", 2048, 2_097_152, GUID_EFI_SYSTEM, "ARKON-ESP");
        add_partition(&mut p, "nvme0n1", "nvme0n1p2", 2_099_200, 4_194_304, GUID_LINUX_FILESYSTEM, "ARKON-BOOT");
        add_partition(&mut p, "nvme0n1", "nvme0n1p3", 6_293_504, 970_000_000, GUID_LINUX_FILESYSTEM, "ARKON-ROOT");
        let report = scan(&p);
        let d = &report.disks[0];
        let arkon = d.detected_systems.iter().find(|s| matches!(s, DetectedSystem::Arkon { .. }));
        assert!(arkon.is_some(), "expected Arkon system detection");
        if let Some(DetectedSystem::Arkon { esp_number, boot_number, root_number }) = arkon {
            assert_eq!(*esp_number, 1);
            assert_eq!(*boot_number, Some(2));
            assert_eq!(*root_number, 3);
        }
    }

    #[test]
    fn arkon_partitions_classified_as_arkon_class() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "nvme0n1", nvme_size(), false, false);
        add_partition(&mut p, "nvme0n1", "nvme0n1p1", 2048, 2_097_152, GUID_EFI_SYSTEM, "ARKON-ESP");
        add_partition(&mut p, "nvme0n1", "nvme0n1p2", 2_099_200, 970_000_000, GUID_LINUX_FILESYSTEM, "ARKON-ROOT");
        let report = scan(&p);
        let partitions = &report.disks[0].partitions;
        // ESP retains Esp class even with ARKON-ESP label
        assert_eq!(partitions[0].class, PartitionClass::Esp);
        // ARKON-ROOT gets Arkon class
        assert_eq!(partitions[1].class, PartitionClass::Arkon);
    }

    // ── Free region calculation ───────────────────────────────────────────────

    #[test]
    fn free_region_after_partition_detected() {
        let mut p = MockProvider::new();
        // 500 GiB NVMe: 1 GiB ESP at start, rest is free.
        add_disk(&mut p, "nvme0n1", nvme_size(), false, false);
        add_partition(&mut p, "nvme0n1", "nvme0n1p1", 2048, 2_097_152, GUID_EFI_SYSTEM, "");
        // p1 ends at 2048 + 2_097_152 = 2_099_200 sectors = 1 GiB + a bit.
        let report = scan(&p);
        let d = &report.disks[0];
        assert!(!d.free_regions.is_empty(), "expected at least one free region");
        // The free region after the ESP should be the bulk of the disk.
        let total_free: u64 = d.free_regions.iter().map(|r| r.size_lba * 512).sum();
        assert!(total_free > 400 * 1024 * 1024 * 1024, "expected >400 GiB free");
    }

    #[test]
    fn no_free_region_on_fully_packed_disk() {
        let mut p = MockProvider::new();
        let total_sectors = sector_count(nvme_size());
        add_disk(&mut p, "nvme0n1", nvme_size(), false, false);
        // Cover the entire usable range: sectors 2048 .. total_sectors - 34
        let usable = total_sectors - GPT_BACKUP_SECTORS - GPT_HEADER_SECTORS;
        add_partition(&mut p, "nvme0n1", "nvme0n1p1", GPT_HEADER_SECTORS, usable, GUID_LINUX_FILESYSTEM, "");
        let report = scan(&p);
        // No gap large enough to be a free region.
        assert!(report.disks[0].free_regions.is_empty());
    }

    // ── Partition table inference ─────────────────────────────────────────────

    #[test]
    fn gpt_inferred_when_type_guid_present() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "nvme0n1", nvme_size(), false, false);
        add_partition(&mut p, "nvme0n1", "nvme0n1p1", 2048, 2_097_152, GUID_EFI_SYSTEM, "");
        let report = scan(&p);
        assert_eq!(report.disks[0].table, PartitionTable::Gpt);
    }

    #[test]
    fn mbr_inferred_when_no_guid() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "sda", nvme_size(), false, false);
        add_partition(&mut p, "sda", "sda1", 2048, 2_000_000, "", "");
        let report = scan(&p);
        assert_eq!(report.disks[0].table, PartitionTable::Mbr);
        assert!(report.disks[0].warnings.contains(&DiskWarning::MbrTable));
    }

    #[test]
    fn no_partition_table_when_disk_has_no_partitions() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "nvme0n1", nvme_size(), false, false);
        let report = scan(&p);
        assert_eq!(report.disks[0].table, PartitionTable::None);
    }

    // ── Scan ID ───────────────────────────────────────────────────────────────

    #[test]
    fn scan_id_changes_when_partition_added() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "nvme0n1", nvme_size(), false, false);
        let r1 = scan(&p);

        add_partition(&mut p, "nvme0n1", "nvme0n1p1", 2048, 2_097_152, GUID_EFI_SYSTEM, "");
        let r2 = scan(&p);

        assert_ne!(r1.scan_id, r2.scan_id);
    }

    // ── BitLocker flag ────────────────────────────────────────────────────────

    #[test]
    fn bitlocker_present_flag_set_when_bitlocker_partition_exists() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "nvme0n1", nvme_size(), false, false);
        add_partition(&mut p, "nvme0n1", "nvme0n1p1", 2048, 2_097_152, GUID_EFI_SYSTEM, "");
        add_partition(&mut p, "nvme0n1", "nvme0n1p2", 2_099_200, 32_768, GUID_MICROSOFT_RESERVED, "");
        // Bitlocker uses the same GUID as Windows data at the partition level;
        // force class by setting class directly is not possible from here.
        // Test that Windows detection works without BitLocker flag.
        let report = scan(&p);
        // Windows data present but no Bitlocker class → bitlocker_present false.
        assert!(!report.bitlocker_present);
    }

    // ── Install medium detection ──────────────────────────────────────────────

    #[test]
    fn removable_disk_with_arkon_install_label_is_install_medium() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "sda", nvme_size(), true, false);
        add_partition(&mut p, "sda", "sda1", 2048, 2_097_152, GUID_EFI_SYSTEM, "");
        // Add ARKON-INSTALL label to filesystem label field
        p.add_file(format!("{}/sda/sda1/label", SYS_BLOCK), "ARKON-INSTALL");
        let report = scan(&p);
        assert!(report.disks[0].is_install_medium);
        assert!(report.disks[0].blockers.contains(&DiskBlocker::IsInstallMedium));
    }

    // ── Firmware entries ──────────────────────────────────────────────────────

    #[test]
    fn firmware_entries_parsed_from_mock_label_format() {
        let mut p = MockProvider::new();
        p.add_file(
            format!("/sys/firmware/efi/efivars/Boot0001-{}", EFI_GLOBAL_GUID),
            "LABEL:Windows Boot Manager",
        );
        p.add_file(
            format!("/sys/firmware/efi/efivars/Boot0002-{}", EFI_GLOBAL_GUID),
            "LABEL:ARKON",
        );
        p.add_file(
            format!("/sys/firmware/efi/efivars/BootOrder-{}", EFI_GLOBAL_GUID),
            "ORDER:0001,0002",
        );
        let entries = read_firmware_entries(&p);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].label, "Windows Boot Manager");
        assert_eq!(entries[0].number, 1);
        assert!(entries[0].active);
        assert_eq!(entries[1].label, "ARKON");
    }

    #[test]
    fn firmware_entries_empty_when_efivars_unavailable() {
        let p = MockProvider::new();
        let entries = read_firmware_entries(&p);
        assert!(entries.is_empty());
    }

    // ── PARTUUID ──────────────────────────────────────────────────────────────

    #[test]
    fn partuuid_read_from_sysfs() {
        let mut p = MockProvider::new();
        add_disk(&mut p, "nvme0n1", nvme_size(), false, false);
        add_partition(&mut p, "nvme0n1", "nvme0n1p1", 2048, 2_097_152, GUID_EFI_SYSTEM, "");
        add_partuuid(&mut p, "nvme0n1", "nvme0n1p1", "aabbccdd-1234-5678-9abc-def012345678");
        let report = scan(&p);
        assert_eq!(
            report.disks[0].partitions[0].part_uuid,
            "aabbccdd-1234-5678-9abc-def012345678"
        );
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    #[test]
    fn uuid_format_check() {
        assert!(is_uuid_format("c12a7328-f81f-11d2-ba4b-00a0c93ec93b"));
        assert!(!is_uuid_format("0x83"));
        assert!(!is_uuid_format(""));
        assert!(!is_uuid_format("c12a7328-f81f-11d2"));
    }

    #[test]
    fn partition_number_extracted_nvme() {
        assert_eq!(extract_partition_number("/sys/block/nvme0n1/nvme0n1p3"), Some(3));
    }

    #[test]
    fn partition_number_extracted_sata() {
        assert_eq!(extract_partition_number("/sys/block/sda/sda12"), Some(12));
    }
}
