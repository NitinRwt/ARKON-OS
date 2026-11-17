/// Integration tests covering all eight required disk scenarios plus extras.
///
/// Every test uses `MockReport` helpers — no real disk access.  The planner and
/// validator run against the constructed StorageReport.
use arkon_storage::plan::{
    PlanMode, PlanRequest, BOOT_SIZE_BYTES, ESP_SIZE_BYTES, MIN_DISK_BYTES,
    MIN_FREE_REGION_BYTES, MIN_ROOT_BYTES,
};
use arkon_storage::scan::{
    DetectedSystem, Disk, DiskBlocker, DiskWarning, FreeRegion, FsType, Partition,
    PartitionClass, PartitionTable, RegionId, ScanId, Transport, test_disk, test_report,
};
use arkon_storage::{StorageError, plan_and_validate, plan_dry_run};

// ── Helpers ───────────────────────────────────────────────────────────────────

const GIB: u64 = 1024 * 1024 * 1024;
const SECTOR: u32 = 512;

fn empty_gpt_disk(id: &str, size_gib: u64) -> Disk {
    let mut d = test_disk(id, size_gib * GIB, SECTOR, PartitionTable::Gpt);
    // Single free region covering the whole disk (minus GPT overhead).
    d.free_regions.push(FreeRegion {
        region_id: RegionId::new("r0"),
        start_lba: 2048,
        size_lba: (size_gib * GIB / u64::from(SECTOR)) - 4096,
    });
    d
}

fn windows_disk(id: &str, size_gib: u64) -> Disk {
    let mut d = test_disk(id, size_gib * GIB, SECTOR, PartitionTable::Gpt);
    d.partitions = vec![
        Partition {
            number: 1,
            part_uuid: "11111111-1111-1111-1111-111111111111".to_owned(),
            start_lba: 2048,
            size_lba: 2_097_152,   // 1 GiB ESP
            type_guid: Some("c12a7328-f81f-11d2-ba4b-00a0c93ec93b".to_owned()),
            class: PartitionClass::Esp,
            fs_type: Some(FsType::Vfat),
            fs_uuid: None,
            fs_label: None,
            part_label: Some("EFI System".to_owned()),
        },
        Partition {
            number: 2,
            part_uuid: "22222222-2222-2222-2222-222222222222".to_owned(),
            start_lba: 2_099_200,
            size_lba: 32_768,   // 16 MiB MSR
            type_guid: Some("e3c9e316-0b5c-4db8-817d-f92df00215ae".to_owned()),
            class: PartitionClass::Msr,
            fs_type: None,
            fs_uuid: None,
            fs_label: None,
            part_label: None,
        },
        Partition {
            number: 3,
            part_uuid: "33333333-3333-3333-3333-333333333333".to_owned(),
            start_lba: 2_131_968,
            size_lba: 150_000_000,   // ~71.5 GiB Windows data
            type_guid: Some("ebd0a0a2-b9e5-4433-87c0-68b6b72699c7".to_owned()),
            class: PartitionClass::WindowsData,
            fs_type: Some(FsType::Ntfs),
            fs_uuid: None,
            fs_label: None,
            part_label: Some("Windows".to_owned()),
        },
        Partition {
            number: 4,
            part_uuid: "44444444-4444-4444-4444-444444444444".to_owned(),
            start_lba: 152_131_968,
            size_lba: 2_097_152,   // 1 GiB WinRE
            type_guid: Some("de94bba4-06d1-4d40-a16a-bfd50179d6ac".to_owned()),
            class: PartitionClass::WindowsRecovery,
            fs_type: Some(FsType::Ntfs),
            fs_uuid: None,
            fs_label: None,
            part_label: Some("Recovery".to_owned()),
        },
    ];
    d.detected_systems.push(DetectedSystem::Windows {
        partition_numbers: vec![1, 2, 3, 4],
        bitlocker: false,
    });
    d.warnings.push(DiskWarning::HasWindows);
    d
}

fn linux_disk(id: &str, size_gib: u64) -> Disk {
    let mut d = test_disk(id, size_gib * GIB, SECTOR, PartitionTable::Gpt);
    d.partitions = vec![
        Partition {
            number: 1,
            part_uuid: "aaaa1111-aaaa-aaaa-aaaa-aaaa11111111".to_owned(),
            start_lba: 2048,
            size_lba: 2_097_152,
            type_guid: Some("c12a7328-f81f-11d2-ba4b-00a0c93ec93b".to_owned()),
            class: PartitionClass::Esp,
            fs_type: Some(FsType::Vfat),
            fs_uuid: None,
            fs_label: None,
            part_label: Some("EFI".to_owned()),
        },
        Partition {
            number: 2,
            part_uuid: "bbbb2222-bbbb-bbbb-bbbb-bbbb22222222".to_owned(),
            start_lba: 2_099_200,
            size_lba: 100_000_000,
            type_guid: Some("0fc63daf-8483-4772-8e79-3d69d8477de4".to_owned()),
            class: PartitionClass::LinuxFs,
            fs_type: Some(FsType::Btrfs),
            fs_uuid: None,
            fs_label: None,
            part_label: Some("fedora-root".to_owned()),
        },
    ];
    d.detected_systems.push(DetectedSystem::Linux {
        name: Some("Fedora Linux".to_owned()),
        partition_numbers: vec![1, 2],
    });
    d
}

fn arkon_disk(id: &str, size_gib: u64) -> Disk {
    let mut d = test_disk(id, size_gib * GIB, SECTOR, PartitionTable::Gpt);
    d.partitions = vec![
        Partition {
            number: 1,
            part_uuid: "aaa00001-0000-0000-0000-000000000001".to_owned(),
            start_lba: 2048,
            size_lba: 2_097_152,
            type_guid: Some("c12a7328-f81f-11d2-ba4b-00a0c93ec93b".to_owned()),
            class: PartitionClass::Esp,
            fs_type: Some(FsType::Vfat),
            fs_uuid: None,
            fs_label: None,
            part_label: Some("ARKON-ESP".to_owned()),
        },
        Partition {
            number: 2,
            part_uuid: "aaa00002-0000-0000-0000-000000000002".to_owned(),
            start_lba: 2_099_200,
            size_lba: 4_194_304,
            type_guid: Some("0fc63daf-8483-4772-8e79-3d69d8477de4".to_owned()),
            class: PartitionClass::Arkon,
            fs_type: Some(FsType::Ext4),
            fs_uuid: None,
            fs_label: None,
            part_label: Some("ARKON-BOOT".to_owned()),
        },
        Partition {
            number: 3,
            part_uuid: "aaa00003-0000-0000-0000-000000000003".to_owned(),
            start_lba: 6_293_504,
            size_lba: (size_gib * GIB / u64::from(SECTOR)) - 6_293_504 - 2048,
            type_guid: Some("0fc63daf-8483-4772-8e79-3d69d8477de4".to_owned()),
            class: PartitionClass::Arkon,
            fs_type: Some(FsType::Btrfs),
            fs_uuid: None,
            fs_label: None,
            part_label: Some("ARKON-ROOT".to_owned()),
        },
    ];
    d.detected_systems.push(DetectedSystem::Arkon {
        esp_number: 1,
        boot_number: Some(2),
        root_number: 3,
    });
    d
}

fn req(mode: PlanMode, disk_id: &str, scan_id: &str) -> PlanRequest {
    PlanRequest {
        mode,
        scan_id: ScanId::new(scan_id),
        target_disk: arkon_storage::DiskId::new(disk_id),
        region_id: None,
        root_size_bytes: None,
        encryption: false,
        variant: Some("arkon".to_owned()),
    }
}

// ── Scenario 1: empty disk — clean install ────────────────────────────────────

#[test]
fn scenario_empty_disk_erase_plan() {
    let disk = empty_gpt_disk("nvme0", 200);
    let report = test_report(vec![disk]);
    let r = req(PlanMode::EraseDisk, "nvme0", "test-scan-001");
    let (plan, validation) = plan_and_validate(&r, &report).unwrap();
    assert!(validation.all_passed, "{:?}", validation.rules);
    assert_eq!(plan.mode, PlanMode::EraseDisk);
    assert!(plan.destructive);
    // EraseTable + 3 Create ops
    assert_eq!(plan.operations.len(), 4);
    // No existing data → data_loss is empty
    assert!(plan.data_loss.is_empty());
}

#[test]
fn scenario_empty_disk_dry_run() {
    let disk = empty_gpt_disk("nvme0", 200);
    let report = test_report(vec![disk]);
    let r = req(PlanMode::EraseDisk, "nvme0", "test-scan-001");
    let dr = plan_dry_run(&r, &report).unwrap();
    assert!(!dr.has_validation_errors);
    assert!(dr.size_summary.esp_size_bytes.unwrap() >= ESP_SIZE_BYTES);
    assert!(dr.size_summary.boot_size_bytes.unwrap() >= BOOT_SIZE_BYTES);
    assert!(dr.size_summary.root_size_bytes.unwrap() >= MIN_ROOT_BYTES);
}

// ── Scenario 2: Windows-only disk ────────────────────────────────────────────

#[test]
fn scenario_windows_disk_erase_names_windows_in_data_loss() {
    let disk = windows_disk("nvme0", 200);
    let report = test_report(vec![disk]);
    let r = req(PlanMode::EraseDisk, "nvme0", "test-scan-001");
    let (plan, validation) = plan_and_validate(&r, &report).unwrap();
    // P3 not applicable in erase mode; all rules should pass.
    assert!(validation.all_passed, "{:?}", validation.rules);
    // Every existing partition must appear in data_loss.
    assert_eq!(plan.data_loss.len(), 4);
    // At least one item names Windows.
    assert!(plan.data_loss.iter().any(|d| d.detected_os.as_deref() == Some("Windows")));
    // Confirmation phrase required (destructive).
    assert!(plan.confirmation.phrase_required);
}

#[test]
fn scenario_windows_disk_erase_generates_repart_definitions() {
    let disk = windows_disk("nvme0", 200);
    let report = test_report(vec![disk]);
    let r = req(PlanMode::EraseDisk, "nvme0", "test-scan-001");
    let (plan, _) = plan_and_validate(&r, &report).unwrap();
    assert_eq!(plan.repart_definitions.len(), 3);
    let filenames: Vec<&str> = plan.repart_definitions.iter().map(|d| d.filename.as_str()).collect();
    assert!(filenames.contains(&"10-esp.conf"));
    assert!(filenames.contains(&"20-boot.conf"));
    assert!(filenames.contains(&"30-root.conf"));
}

// ── Scenario 3: existing Linux disk ──────────────────────────────────────────

#[test]
fn scenario_linux_disk_erase_names_linux_in_data_loss() {
    let disk = linux_disk("sda", 500);
    let report = test_report(vec![disk]);
    let r = req(PlanMode::EraseDisk, "sda", "test-scan-001");
    let (plan, _) = plan_and_validate(&r, &report).unwrap();
    assert_eq!(plan.data_loss.len(), 2);
    assert!(plan.data_loss.iter().any(|d| d.detected_os.as_deref() == Some("Fedora Linux")));
}

// ── Scenario 4: multiple NVMe drives ─────────────────────────────────────────

#[test]
fn scenario_multiple_nvme_unchanged_disk_in_unchanged_list() {
    let disk0 = empty_gpt_disk("nvme0", 500);
    let disk1 = windows_disk("nvme1", 200);
    let report = test_report(vec![disk0, disk1]);
    // Install on nvme0
    let r = req(PlanMode::EraseDisk, "nvme0", "test-scan-001");
    let (plan, validation) = plan_and_validate(&r, &report).unwrap();
    assert!(validation.all_passed, "{:?}", validation.rules);
    // nvme1's partitions must all be in unchanged[].
    let unchanged_disks: Vec<_> = plan.unchanged.iter().map(|u| u.disk_id.0.as_str()).collect();
    assert!(unchanged_disks.iter().all(|&d| d == "nvme1"),
        "unexpected disk in unchanged: {:?}", unchanged_disks);
    assert_eq!(plan.unchanged.len(), 4); // 4 partitions on the Windows nvme1
}

// ── Scenario 5: insufficient storage ─────────────────────────────────────────

#[test]
fn scenario_insufficient_storage_rejected() {
    // 10 GiB is below the 64 GiB minimum.
    let disk = empty_gpt_disk("nvme0", 10);
    let report = test_report(vec![disk]);
    let r = req(PlanMode::EraseDisk, "nvme0", "test-scan-001");
    let err = plan_and_validate(&r, &report).unwrap_err();
    assert!(matches!(err, StorageError::DiskTooSmall(_, _, _)), "got: {:?}", err);
}

#[test]
fn scenario_free_region_too_small_rejected() {
    let mut disk = test_disk("nvme0", 200 * GIB, SECTOR, PartitionTable::Gpt);
    // Only 10 GiB free — not enough.
    disk.free_regions.push(FreeRegion {
        region_id: RegionId::new("r0"),
        start_lba: 2048,
        size_lba: (10 * GIB) / u64::from(SECTOR),
    });
    let report = test_report(vec![disk]);
    let mut r = req(PlanMode::FreeSpace, "nvme0", "test-scan-001");
    r.region_id = Some(RegionId::new("r0"));
    let err = plan_and_validate(&r, &report).unwrap_err();
    assert!(matches!(err, StorageError::RegionTooSmall(_, _, _)), "got: {:?}", err);
}

// ── Scenario 6: unsupported filesystem / sector size ─────────────────────────

#[test]
fn scenario_unsupported_sector_size_rejected() {
    let mut disk = test_disk("nvme0", 200 * GIB, 1024, PartitionTable::Gpt); // 1024-byte sectors
    // No blocker added yet — sector size check is in planner
    let report = test_report(vec![disk]);
    let r = req(PlanMode::EraseDisk, "nvme0", "test-scan-001");
    let err = plan_and_validate(&r, &report).unwrap_err();
    assert!(matches!(err, StorageError::UnsupportedSectorSize(1024)), "got: {:?}", err);
}

#[test]
fn scenario_4k_sector_disk_accepted() {
    let disk = test_disk("nvme0", 200 * GIB, 4096, PartitionTable::Gpt);
    let report = test_report(vec![disk]);
    let r = req(PlanMode::EraseDisk, "nvme0", "test-scan-001");
    // 4096-byte sectors are supported.
    let result = plan_and_validate(&r, &report);
    assert!(result.is_ok(), "unexpected error: {:?}", result.err());
}

// ── Scenario 7: encrypted (LUKS) partitions ──────────────────────────────────

#[test]
fn scenario_encrypted_plan_uses_luks_type() {
    let disk = empty_gpt_disk("nvme0", 200);
    let report = test_report(vec![disk]);
    let mut r = req(PlanMode::EraseDisk, "nvme0", "test-scan-001");
    r.encryption = true;
    let (plan, _) = plan_and_validate(&r, &report).unwrap();
    assert!(plan.encryption);
    // Root partition must be LUKS type in its definition.
    let root_def = plan.repart_definitions.iter().find(|d| d.filename == "30-root.conf").unwrap();
    assert!(root_def.content.contains("Encrypt=key-file"), "content: {}", root_def.content);
}

#[test]
fn scenario_disk_with_luks_partition_has_warning() {
    let mut disk = test_disk("nvme0", 200 * GIB, SECTOR, PartitionTable::Gpt);
    disk.partitions.push(Partition {
        number: 1,
        part_uuid: "luks-uuid-1234".to_owned(),
        start_lba: 2048,
        size_lba: 100_000_000,
        type_guid: Some("ca7d7ccb-63ed-4c53-861c-1742536059cc".to_owned()),
        class: PartitionClass::Luks,
        fs_type: None,
        fs_uuid: None,
        fs_label: None,
        part_label: None,
    });
    let report = test_report(vec![disk]);
    let r = req(PlanMode::EraseDisk, "nvme0", "test-scan-001");
    let (plan, _) = plan_and_validate(&r, &report).unwrap();
    // LUKS partition means unknown data — it should be in data_loss.
    assert!(plan.data_loss.iter().any(|d| d.part_uuid == "luks-uuid-1234"));
}

// ── Scenario 8: malformed / blocked partition table ───────────────────────────

#[test]
fn scenario_install_medium_blocked() {
    let mut disk = empty_gpt_disk("nvme0", 200);
    disk.is_install_medium = true;
    disk.blockers.push(DiskBlocker::IsInstallMedium);
    let report = test_report(vec![disk]);
    let r = req(PlanMode::EraseDisk, "nvme0", "test-scan-001");
    let err = plan_and_validate(&r, &report).unwrap_err();
    assert!(matches!(err, StorageError::DiskBlocked(_, _)), "got: {:?}", err);
}

#[test]
fn scenario_lvm_disk_blocked() {
    let mut disk = test_disk("sda", 200 * GIB, SECTOR, PartitionTable::Gpt);
    disk.blockers.push(DiskBlocker::HasLvm);
    let report = test_report(vec![disk]);
    let r = req(PlanMode::EraseDisk, "sda", "test-scan-001");
    let err = plan_and_validate(&r, &report).unwrap_err();
    assert!(matches!(err, StorageError::DiskBlocked(_, _)));
}

#[test]
fn scenario_no_partition_table_reported_ok_in_erase_mode() {
    // A disk with no partition table (empty / just wiped) is valid for erase mode.
    let disk = test_disk("nvme0", 200 * GIB, SECTOR, PartitionTable::None);
    let report = test_report(vec![disk]);
    let r = req(PlanMode::EraseDisk, "nvme0", "test-scan-001");
    // Should succeed — erase mode will recreate the GPT.
    let result = plan_and_validate(&r, &report);
    assert!(result.is_ok(), "unexpected error: {:?}", result.err());
}

#[test]
fn scenario_mbr_disk_rejected_for_free_space() {
    let mut disk = test_disk("sda", 200 * GIB, SECTOR, PartitionTable::Mbr);
    disk.free_regions.push(FreeRegion {
        region_id: RegionId::new("r0"),
        start_lba: 2048,
        size_lba: (200 * GIB) / u64::from(SECTOR) - 4096,
    });
    let report = test_report(vec![disk]);
    let mut r = req(PlanMode::FreeSpace, "sda", "test-scan-001");
    r.region_id = Some(RegionId::new("r0"));
    let err = plan_and_validate(&r, &report).unwrap_err();
    assert!(matches!(err, StorageError::MbrNotSupportedForFreeSpace(_)));
}

// ── Dual-boot preservation ────────────────────────────────────────────────────

#[test]
fn scenario_dual_boot_free_space_keeps_all_existing_partitions() {
    let mut disk = test_disk("nvme0", 500 * GIB, SECTOR, PartitionTable::Gpt);
    // Windows occupies 300 GiB; 150 GiB free.
    disk.partitions = windows_disk("nvme0", 500).partitions;
    let free_start_lba = 154_229_120u64; // after Windows
    disk.free_regions.push(FreeRegion {
        region_id: RegionId::new("r-free"),
        start_lba: free_start_lba,
        size_lba: (150 * GIB) / u64::from(SECTOR),
    });

    let report = test_report(vec![disk]);
    let mut r = req(PlanMode::FreeSpace, "nvme0", "test-scan-001");
    r.region_id = Some(RegionId::new("r-free"));

    let (plan, validation) = plan_and_validate(&r, &report).unwrap();
    assert!(validation.all_passed, "{:?}", validation.rules);

    // All 4 existing Windows partitions must appear in plan.unchanged.
    let unchanged_parts: Vec<u32> = plan.unchanged.iter().map(|u| u.partition_number).collect();
    for part_num in 1..=4 {
        assert!(unchanged_parts.contains(&part_num),
            "partition {} not in unchanged list", part_num);
    }
    // No data loss in free-space mode.
    assert!(plan.data_loss.is_empty());
    // Not destructive.
    assert!(!plan.destructive);
    // No phrase required.
    assert!(!plan.confirmation.phrase_required);
}

#[test]
fn scenario_reinstall_keep_home_on_arkon_disk() {
    let disk = arkon_disk("nvme0", 200);
    let report = test_report(vec![disk]);
    let r = req(PlanMode::ReinstallKeepHome, "nvme0", "test-scan-001");
    let (plan, validation) = plan_and_validate(&r, &report).unwrap();
    // P8 requires only ARKON-labelled partitions are reformatted.
    assert!(validation.all_passed, "{:?}", validation.rules);
    assert!(plan.destructive);
    // data_loss must name the ARKON partitions.
    assert!(!plan.data_loss.is_empty());
}

#[test]
fn scenario_reinstall_on_non_arkon_disk_errors() {
    let disk = linux_disk("sda", 500);
    let report = test_report(vec![disk]);
    let r = req(PlanMode::ReinstallKeepHome, "sda", "test-scan-001");
    let err = plan_and_validate(&r, &report).unwrap_err();
    assert!(matches!(err, StorageError::NoArkonInstallFound(_)));
}

// ── Scan ID mismatch ──────────────────────────────────────────────────────────

#[test]
fn scenario_stale_plan_rejected_by_validator() {
    let disk = empty_gpt_disk("nvme0", 200);
    let report = test_report(vec![disk]);
    // Build the plan...
    let r = req(PlanMode::EraseDisk, "nvme0", "test-scan-001");
    let (plan, _) = plan_and_validate(&r, &report).unwrap();
    // ...but then validate against a report with a different scan_id.
    let mut new_report = report.clone();
    new_report.scan_id = ScanId::new("test-scan-002");
    let v = arkon_storage::validate(&plan, &new_report).unwrap();
    let p9 = v.rules.iter().find(|r| r.rule == 9).unwrap();
    assert!(!p9.passed, "P9 should fail on scan_id mismatch");
}

// ── Manual mode ───────────────────────────────────────────────────────────────

#[test]
fn scenario_manual_root_size_respected() {
    let mut disk = test_disk("nvme0", 500 * GIB, SECTOR, PartitionTable::Gpt);
    disk.free_regions.push(FreeRegion {
        region_id: RegionId::new("r0"),
        start_lba: 2048,
        size_lba: (500 * GIB) / u64::from(SECTOR) - 4096,
    });
    let report = test_report(vec![disk]);
    let r = PlanRequest {
        mode: PlanMode::Manual,
        scan_id: ScanId::new("test-scan-001"),
        target_disk: arkon_storage::DiskId::new("nvme0"),
        region_id: Some(RegionId::new("r0")),
        root_size_bytes: Some(50 * GIB),
        encryption: false,
        variant: None,
    };
    let (plan, _) = plan_and_validate(&r, &report).unwrap();
    // Root partition size must not exceed 50 GiB.
    let root_op = plan.operations.iter().find(|op| {
        op.after.as_ref()
            .and_then(|a| a.part_label.as_deref())
            == Some("ARKON-ROOT")
    });
    let root_size = root_op.unwrap().after.as_ref().unwrap().size_lba
        * u64::from(SECTOR);
    assert!(root_size <= 50 * GIB, "root_size {} > 50 GiB", root_size);
    assert!(root_size >= arkon_storage::MIN_ROOT_BYTES, "root_size too small");
}

#[test]
fn scenario_manual_root_too_small_rejected() {
    let mut disk = test_disk("nvme0", 500 * GIB, SECTOR, PartitionTable::Gpt);
    disk.free_regions.push(FreeRegion {
        region_id: RegionId::new("r0"),
        start_lba: 2048,
        size_lba: (500 * GIB) / u64::from(SECTOR) - 4096,
    });
    let report = test_report(vec![disk]);
    let r = PlanRequest {
        mode: PlanMode::Manual,
        scan_id: ScanId::new("test-scan-001"),
        target_disk: arkon_storage::DiskId::new("nvme0"),
        region_id: Some(RegionId::new("r0")),
        root_size_bytes: Some(1 * GIB), // too small
        encryption: false,
        variant: None,
    };
    let err = plan_and_validate(&r, &report).unwrap_err();
    assert!(matches!(err, StorageError::RootTooSmall(_, _)));
}

// ── Disk not found ────────────────────────────────────────────────────────────

#[test]
fn scenario_unknown_disk_id_rejected() {
    let disk = empty_gpt_disk("nvme0", 200);
    let report = test_report(vec![disk]);
    let r = req(PlanMode::EraseDisk, "nonexistent-disk", "test-scan-001");
    let err = plan_and_validate(&r, &report).unwrap_err();
    assert!(matches!(err, StorageError::DiskNotFound(_)));
}

// ── P5: existing foreign ESP must not be formatted ───────────────────────────

#[test]
fn scenario_erase_does_not_format_existing_partitions_before_erase_op() {
    // In erase mode the EraseTable op is first; then Create ops create new
    // partitions.  P5 must not trip on the new ARKON ESP that is created.
    let disk = windows_disk("nvme0", 200);
    let report = test_report(vec![disk]);
    let r = req(PlanMode::EraseDisk, "nvme0", "test-scan-001");
    let (plan, validation) = plan_and_validate(&r, &report).unwrap();
    let p5 = validation.rules.iter().find(|r| r.rule == 5).unwrap();
    assert!(p5.passed, "P5 failed: {}", p5.detail);
}

// ── Bitlocker detection ───────────────────────────────────────────────────────

#[test]
fn scenario_bitlocker_disk_requires_acknowledgement() {
    let mut disk = windows_disk("nvme0", 200);
    // Replace windows data partition with BitLocker
    for p in &mut disk.partitions {
        if p.number == 3 {
            p.class = PartitionClass::Bitlocker;
        }
    }
    disk.detected_systems.clear();
    disk.detected_systems.push(DetectedSystem::Windows {
        partition_numbers: vec![1, 2, 3, 4],
        bitlocker: true,
    });
    let mut report = test_report(vec![disk]);
    report.bitlocker_present = true;

    let r = req(PlanMode::EraseDisk, "nvme0", "test-scan-001");
    let (plan, _) = plan_and_validate(&r, &report).unwrap();
    assert!(plan.confirmation.bitlocker_ack_required);
    assert!(plan.warnings.contains(&arkon_storage::plan::PlanWarning::BitlockerPresent));
}
