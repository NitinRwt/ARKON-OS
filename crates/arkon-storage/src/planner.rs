/// Partition planner: turns a PlanRequest + StorageReport into a Plan.
///
/// This module never writes to any disk.  It performs arithmetic on partition
/// layouts, generates systemd-repart definition files and assembles the Plan.
/// The returned Plan is NOT yet confirmed safe; the validator must be run next.
use crate::error::{StorageError, StorageResult};
use crate::plan::{
    ArkonRole, ConfirmationRequirements, DataLossItem, OperationState, OperationType,
    Plan, PlanId, PlanMode, PlanRequest, PlanWarning, PlannedOperation, PlannedPartition,
    RepartDefinition, RiskLevel, UnchangedEntry,
    ALIGNMENT_BYTES, BOOT_SIZE_BYTES, ESP_SIZE_BYTES, GUID_EFI_SYSTEM, GUID_LINUX_FILESYSTEM,
    GUID_LUKS, LABEL_ARKON_BOOT, LABEL_ARKON_ESP, LABEL_ARKON_ROOT, MIN_DISK_BYTES,
    MIN_FREE_REGION_BYTES, MIN_ROOT_BYTES,
};
use crate::scan::{DetectedSystem, Disk, FsType, PartitionClass, StorageReport, Transport};

static PLAN_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn next_plan_id() -> PlanId {
    let n = PLAN_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    PlanId::new(format!("plan-{:04}", n))
}

fn next_op_id(seq: &mut u32) -> String {
    *seq += 1;
    format!("op-{:03}", seq)
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Build a plan from a request and a scan report.
///
/// Returns `Err` for hard failures (wrong disk, too small, etc.).
/// Non-fatal issues appear as `Plan.warnings`.
pub fn build_plan(request: &PlanRequest, report: &StorageReport) -> StorageResult<Plan> {
    // Scan ID must match.
    if request.scan_id != report.scan_id {
        return Err(StorageError::ScanIdMismatch(
            request.scan_id.0.clone(),
            report.scan_id.0.clone(),
        ));
    }

    let disk = report.disk(&request.target_disk).ok_or_else(|| {
        StorageError::DiskNotFound(request.target_disk.0.clone())
    })?;

    // Hard blockers prevent planning entirely.
    if !disk.blockers.is_empty() {
        let detail = disk
            .blockers
            .iter()
            .map(|b| format!("{:?}", b))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(StorageError::DiskBlocked(disk.id.0.clone(), detail));
    }

    validate_sector_size(disk)?;

    match request.mode {
        PlanMode::EraseDisk       => plan_erase_disk(request, disk, report),
        PlanMode::FreeSpace       => plan_free_space(request, disk, report),
        PlanMode::Manual          => plan_manual(request, disk, report),
        PlanMode::ReinstallKeepHome => plan_reinstall_keep_home(request, disk, report),
    }
}

// ── Mode 1: erase the whole disk ──────────────────────────────────────────────

fn plan_erase_disk(
    req: &PlanRequest,
    disk: &Disk,
    report: &StorageReport,
) -> StorageResult<Plan> {
    if disk.size_bytes < MIN_DISK_BYTES {
        return Err(StorageError::DiskTooSmall(
            disk.id.0.clone(), disk.size_bytes, MIN_DISK_BYTES,
        ));
    }

    let mut seq = 0u32;
    let mut ops: Vec<PlannedOperation> = Vec::new();
    let mut data_loss: Vec<DataLossItem> = Vec::new();
    let mut warnings: Vec<PlanWarning> = Vec::new();

    // Record existing partitions as data loss.
    for p in &disk.partitions {
        let os_name = detected_os_name_for_partition(disk, p.number);
        data_loss.push(DataLossItem {
            part_uuid: p.part_uuid.clone(),
            partition_number: p.number,
            description: partition_description(p),
            detected_os: os_name,
        });
    }

    // Op 1: erase the partition table.
    ops.push(PlannedOperation {
        op_id: next_op_id(&mut seq),
        target_disk: disk.id.clone(),
        target_partition: None,
        operation: OperationType::EraseTable,
        before: None,
        after: None,
        risk: RiskLevel::Destructive,
        validated: false,
    });

    let root_size = disk.size_bytes
        .saturating_sub(ESP_SIZE_BYTES)
        .saturating_sub(BOOT_SIZE_BYTES)
        .saturating_sub(ALIGNMENT_BYTES * 4); // leading gap + inter-partition gaps

    if root_size < MIN_ROOT_BYTES {
        return Err(StorageError::DiskTooSmall(disk.id.0.clone(), disk.size_bytes, MIN_DISK_BYTES));
    }

    // Ops 2–4: create the three partitions.
    let sector = u64::from(disk.logical_sector_bytes);
    let align_lba = ALIGNMENT_BYTES / sector;

    let esp_start_lba = align_lba; // 1 MiB gap from start
    let esp_size_lba  = ESP_SIZE_BYTES / sector;
    let boot_start_lba = align_up(esp_start_lba + esp_size_lba, align_lba);
    let boot_size_lba  = BOOT_SIZE_BYTES / sector;
    let root_start_lba = align_up(boot_start_lba + boot_size_lba, align_lba);
    let root_size_lba  = align_down(
        (disk.size_bytes / sector) - root_start_lba - align_lba,
        align_lba,
    );

    // Create ESP
    ops.push(make_create_op(
        next_op_id(&mut seq), disk.id.clone(), 1,
        esp_start_lba, esp_size_lba,
        GUID_EFI_SYSTEM, LABEL_ARKON_ESP, FsType::Vfat,
        PartitionClass::Esp, ArkonRole::Esp, false,
    ));
    // Create /boot
    ops.push(make_create_op(
        next_op_id(&mut seq), disk.id.clone(), 2,
        boot_start_lba, boot_size_lba,
        GUID_LINUX_FILESYSTEM, LABEL_ARKON_BOOT, FsType::Ext4,
        PartitionClass::LinuxFs, ArkonRole::Boot, false,
    ));
    // Create root (optionally LUKS)
    let root_guid   = if req.encryption { GUID_LUKS } else { GUID_LINUX_FILESYSTEM };
    let root_class  = if req.encryption { PartitionClass::Luks } else { PartitionClass::LinuxFs };
    let root_fstype = if req.encryption { FsType::Unknown } else { FsType::Btrfs };
    ops.push(make_create_op(
        next_op_id(&mut seq), disk.id.clone(), 3,
        root_start_lba, root_size_lba,
        root_guid, LABEL_ARKON_ROOT, root_fstype,
        root_class, ArkonRole::Root, req.encryption,
    ));

    disk_warnings(disk, report, &mut warnings);

    // All partitions on other disks are unchanged.
    let unchanged = collect_unchanged_other_disks(report, &disk.id);

    let repart = generate_repart_erase(
        esp_start_lba, esp_size_lba,
        boot_start_lba, boot_size_lba,
        root_start_lba, root_size_lba,
        req.encryption,
    );

    let confirmation = ConfirmationRequirements {
        phrase_required: true,
        phrase_hint: Some(phrase_from_disk(disk)),
        bitlocker_ack_required: report.bitlocker_present,
    };

    let plan = Plan {
        plan_id: next_plan_id(),
        plan_hash: String::new(), // filled in by finalize_plan
        scan_id: req.scan_id.clone(),
        mode: PlanMode::EraseDisk,
        target_disk: disk.id.clone(),
        variant: req.variant.clone(),
        encryption: req.encryption,
        operations: ops,
        unchanged,
        data_loss,
        destructive: true,
        warnings,
        confirmation,
        repart_definitions: repart,
    };
    Ok(finalize_plan(plan))
}

// ── Mode 3b / free-space ──────────────────────────────────────────────────────

fn plan_free_space(
    req: &PlanRequest,
    disk: &Disk,
    report: &StorageReport,
) -> StorageResult<Plan> {
    let region_id = req.region_id.as_ref().ok_or_else(|| {
        StorageError::UnsupportedMode("free_space requires a region_id".to_owned())
    })?;

    let region = disk.free_region(region_id).ok_or_else(|| {
        StorageError::RegionNotFound(region_id.0.clone(), disk.id.0.clone())
    })?;

    let sector = u64::from(disk.logical_sector_bytes);
    let region_bytes = region.size_bytes(disk.logical_sector_bytes);

    if region_bytes < MIN_FREE_REGION_BYTES {
        return Err(StorageError::RegionTooSmall(
            region_id.0.clone(), region_bytes, MIN_FREE_REGION_BYTES,
        ));
    }

    // Reject MBR for free-space mode.
    if disk.table == crate::scan::PartitionTable::Mbr {
        return Err(StorageError::MbrNotSupportedForFreeSpace(disk.id.0.clone()));
    }

    let mut seq = 0u32;
    let mut ops: Vec<PlannedOperation> = Vec::new();
    let mut warnings: Vec<PlanWarning> = Vec::new();

    let align_lba = ALIGNMENT_BYTES / sector;
    let esp_start_lba  = align_up(region.start_lba, align_lba);
    let esp_size_lba   = ESP_SIZE_BYTES / sector;
    let boot_start_lba = align_up(esp_start_lba + esp_size_lba, align_lba);
    let boot_size_lba  = BOOT_SIZE_BYTES / sector;
    let root_start_lba = align_up(boot_start_lba + boot_size_lba, align_lba);
    let region_end_lba = region.start_lba + region.size_lba;
    let root_size_lba  = align_down(
        region_end_lba.saturating_sub(root_start_lba).saturating_sub(align_lba),
        align_lba,
    );

    if root_size_lba * sector < MIN_ROOT_BYTES {
        return Err(StorageError::RegionTooSmall(
            region_id.0.clone(), region_bytes, MIN_FREE_REGION_BYTES,
        ));
    }

    // All existing partitions are Keep operations.
    let next_part_num = disk.partitions.iter().map(|p| p.number).max().unwrap_or(0) + 1;
    for p in &disk.partitions {
        ops.push(PlannedOperation {
            op_id: next_op_id(&mut seq),
            target_disk: disk.id.clone(),
            target_partition: Some(p.number),
            operation: OperationType::Keep,
            before: Some(partition_state(p)),
            after:  Some(partition_state(p)),
            risk: RiskLevel::None,
            validated: false,
        });
    }

    ops.push(make_create_op(
        next_op_id(&mut seq), disk.id.clone(), next_part_num,
        esp_start_lba, esp_size_lba,
        GUID_EFI_SYSTEM, LABEL_ARKON_ESP, FsType::Vfat,
        PartitionClass::Esp, ArkonRole::Esp, false,
    ));
    ops.push(make_create_op(
        next_op_id(&mut seq), disk.id.clone(), next_part_num + 1,
        boot_start_lba, boot_size_lba,
        GUID_LINUX_FILESYSTEM, LABEL_ARKON_BOOT, FsType::Ext4,
        PartitionClass::LinuxFs, ArkonRole::Boot, false,
    ));
    let root_guid   = if req.encryption { GUID_LUKS } else { GUID_LINUX_FILESYSTEM };
    let root_class  = if req.encryption { PartitionClass::Luks } else { PartitionClass::LinuxFs };
    let root_fstype = if req.encryption { FsType::Unknown } else { FsType::Btrfs };
    ops.push(make_create_op(
        next_op_id(&mut seq), disk.id.clone(), next_part_num + 2,
        root_start_lba, root_size_lba,
        root_guid, LABEL_ARKON_ROOT, root_fstype,
        root_class, ArkonRole::Root, req.encryption,
    ));

    disk_warnings(disk, report, &mut warnings);

    let unchanged = collect_all_unchanged(report, &disk.id, &disk.partitions);

    let repart = generate_repart_free_space(
        esp_start_lba, esp_size_lba,
        boot_start_lba, boot_size_lba,
        root_start_lba, root_size_lba,
        req.encryption,
    );

    // Free-space mode: no phrase, but confirm button required.
    let confirmation = ConfirmationRequirements {
        phrase_required: false,
        phrase_hint: None,
        bitlocker_ack_required: report.bitlocker_present,
    };

    let plan = Plan {
        plan_id: next_plan_id(),
        plan_hash: String::new(),
        scan_id: req.scan_id.clone(),
        mode: PlanMode::FreeSpace,
        target_disk: disk.id.clone(),
        variant: req.variant.clone(),
        encryption: req.encryption,
        operations: ops,
        unchanged,
        data_loss: vec![],
        destructive: false,
        warnings,
        confirmation,
        repart_definitions: repart,
    };
    Ok(finalize_plan(plan))
}

// ── Mode 2: manual (constrained) ─────────────────────────────────────────────

fn plan_manual(
    req: &PlanRequest,
    disk: &Disk,
    report: &StorageReport,
) -> StorageResult<Plan> {
    let custom_root = req.root_size_bytes.ok_or_else(|| {
        StorageError::UnsupportedMode("manual mode requires root_size_bytes".to_owned())
    })?;

    if custom_root < MIN_ROOT_BYTES {
        return Err(StorageError::RootTooSmall(custom_root, MIN_ROOT_BYTES));
    }

    // Delegate to free_space with the root size overridden after.
    // We build a free-space plan then adjust the root partition.
    let mut plan = plan_free_space(req, disk, report)?;
    plan.mode = PlanMode::Manual;

    // Adjust the root Create operation size.
    let sector = u64::from(disk.logical_sector_bytes);
    let custom_root_lba = align_down(custom_root / sector, ALIGNMENT_BYTES / sector);

    for op in &mut plan.operations {
        if let Some(after) = &mut op.after {
            if after.part_label.as_deref() == Some(LABEL_ARKON_ROOT) {
                // Validate the custom size does not exceed what was allocated.
                let available_lba = after.size_lba;
                if custom_root_lba > available_lba {
                    return Err(StorageError::RootTooLarge(
                        custom_root,
                        available_lba * sector,
                    ));
                }
                after.size_lba = custom_root_lba;
            }
        }
    }

    Ok(finalize_plan(plan))
}

// ── Mode 5b: reinstall keeping home ──────────────────────────────────────────

fn plan_reinstall_keep_home(
    req: &PlanRequest,
    disk: &Disk,
    report: &StorageReport,
) -> StorageResult<Plan> {
    // Find an existing ARKON installation on this disk.
    let arkon = disk.detected_systems.iter().find_map(|s| {
        if let DetectedSystem::Arkon { esp_number, boot_number, root_number } = s {
            Some((*esp_number, *boot_number, *root_number))
        } else {
            None
        }
    });
    let (esp_num, boot_num, root_num) = arkon.ok_or_else(|| {
        StorageError::NoArkonInstallFound(disk.id.0.clone())
    })?;

    let mut seq = 0u32;
    let mut ops: Vec<PlannedOperation> = Vec::new();
    let mut data_loss: Vec<DataLossItem> = Vec::new();
    let mut warnings: Vec<PlanWarning> = Vec::new();

    for p in &disk.partitions {
        let is_esp  = p.number == esp_num;
        let is_boot = boot_num.map_or(false, |n| p.number == n);
        let is_root = p.number == root_num;

        if is_esp || is_boot {
            // Reformat: keep the partition, wipe and recreate the filesystem.
            ops.push(PlannedOperation {
                op_id: next_op_id(&mut seq),
                target_disk: disk.id.clone(),
                target_partition: Some(p.number),
                operation: OperationType::Format,
                before: Some(partition_state(p)),
                after: Some(OperationState {
                    number: p.number,
                    start_lba: p.start_lba,
                    size_lba: p.size_lba,
                    type_guid: p.type_guid.clone(),
                    part_uuid: p.part_uuid.clone(),
                    part_label: p.part_label.clone(),
                    fs_type: if is_esp { Some(FsType::Vfat) } else { Some(FsType::Ext4) },
                }),
                risk: RiskLevel::Destructive,
                validated: false,
            });
            data_loss.push(DataLossItem {
                part_uuid: p.part_uuid.clone(),
                partition_number: p.number,
                description: format!("{} filesystem will be recreated", if is_esp { "ESP" } else { "/boot" }),
                detected_os: None,
            });
        } else if is_root {
            ops.push(PlannedOperation {
                op_id: next_op_id(&mut seq),
                target_disk: disk.id.clone(),
                target_partition: Some(p.number),
                operation: OperationType::ReplaceSubvolume,
                before: Some(partition_state(p)),
                after: Some(partition_state(p)), // partition unchanged, subvolume replaced
                risk: RiskLevel::Destructive,
                validated: false,
            });
            data_loss.push(DataLossItem {
                part_uuid: p.part_uuid.clone(),
                partition_number: p.number,
                description: "ARKON root subvolume will be replaced; home subvolume is kept".to_owned(),
                detected_os: Some("ARKON".to_owned()),
            });
        } else {
            ops.push(PlannedOperation {
                op_id: next_op_id(&mut seq),
                target_disk: disk.id.clone(),
                target_partition: Some(p.number),
                operation: OperationType::Keep,
                before: Some(partition_state(p)),
                after: Some(partition_state(p)),
                risk: RiskLevel::None,
                validated: false,
            });
        }
    }

    disk_warnings(disk, report, &mut warnings);

    let unchanged = collect_all_unchanged(report, &disk.id, &[]);

    let confirmation = ConfirmationRequirements {
        phrase_required: true,
        phrase_hint: Some(phrase_from_disk(disk)),
        bitlocker_ack_required: false,
    };

    let plan = Plan {
        plan_id: next_plan_id(),
        plan_hash: String::new(),
        scan_id: req.scan_id.clone(),
        mode: PlanMode::ReinstallKeepHome,
        target_disk: disk.id.clone(),
        variant: req.variant.clone(),
        encryption: req.encryption,
        operations: ops,
        unchanged,
        data_loss,
        destructive: true,
        warnings,
        confirmation,
        repart_definitions: vec![],
    };
    Ok(finalize_plan(plan))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn validate_sector_size(disk: &Disk) -> StorageResult<()> {
    if disk.logical_sector_bytes != 512 && disk.logical_sector_bytes != 4096 {
        return Err(StorageError::UnsupportedSectorSize(disk.logical_sector_bytes));
    }
    Ok(())
}

fn align_up(lba: u64, align: u64) -> u64 {
    if align == 0 { return lba; }
    ((lba + align - 1) / align) * align
}

fn align_down(lba: u64, align: u64) -> u64 {
    if align == 0 { return lba; }
    (lba / align) * align
}

fn make_create_op(
    op_id: String,
    disk_id: crate::scan::DiskId,
    number: u32,
    start_lba: u64,
    size_lba: u64,
    type_guid: &str,
    part_label: &str,
    fs_type: FsType,
    class: PartitionClass,
    role: ArkonRole,
    encrypted: bool,
) -> PlannedOperation {
    let _ = (class, role, encrypted); // used in after state
    PlannedOperation {
        op_id,
        target_disk: disk_id,
        target_partition: None,
        operation: OperationType::Create,
        before: None,
        after: Some(OperationState {
            number,
            start_lba,
            size_lba,
            type_guid: Some(type_guid.to_owned()),
            part_uuid: String::new(), // assigned by partitioner
            part_label: Some(part_label.to_owned()),
            fs_type: Some(fs_type),
        }),
        risk: RiskLevel::Low,
        validated: false,
    }
}

fn partition_state(p: &crate::scan::Partition) -> OperationState {
    OperationState {
        number: p.number,
        start_lba: p.start_lba,
        size_lba: p.size_lba,
        type_guid: p.type_guid.clone(),
        part_uuid: p.part_uuid.clone(),
        part_label: p.part_label.clone(),
        fs_type: p.fs_type.clone(),
    }
}

fn partition_description(p: &crate::scan::Partition) -> String {
    let label = p.part_label.as_deref().unwrap_or("");
    let fs = p.fs_type.as_ref().map(|f| format!("{:?}", f)).unwrap_or_default();
    if !label.is_empty() {
        format!("partition {} '{}' ({})", p.number, label, fs)
    } else {
        format!("partition {} ({})", p.number, fs)
    }
}

fn detected_os_name_for_partition(disk: &Disk, part_num: u32) -> Option<String> {
    for sys in &disk.detected_systems {
        match sys {
            DetectedSystem::Windows { partition_numbers, .. }
                if partition_numbers.contains(&part_num) =>
            {
                return Some("Windows".to_owned());
            }
            DetectedSystem::Arkon { esp_number, boot_number, root_number }
                if *esp_number == part_num
                    || boot_number.map_or(false, |n| n == part_num)
                    || *root_number == part_num =>
            {
                return Some("ARKON".to_owned());
            }
            DetectedSystem::Linux { name, partition_numbers }
                if partition_numbers.contains(&part_num) =>
            {
                return Some(name.clone().unwrap_or_else(|| "Linux".to_owned()));
            }
            _ => {}
        }
    }
    None
}

fn phrase_from_disk(disk: &Disk) -> String {
    let model = disk.model.as_deref().unwrap_or("disk");
    let tail = disk.serial_tail.as_deref().unwrap_or("????");
    format!("erase {} {}", model.split_whitespace().next().unwrap_or("disk"), tail)
}

fn disk_warnings(disk: &Disk, report: &StorageReport, warnings: &mut Vec<PlanWarning>) {
    if disk.rotational { warnings.push(PlanWarning::TargetIsRotational); }
    if disk.transport == Transport::Usb { warnings.push(PlanWarning::TargetIsUsb); }
    if report.bitlocker_present { warnings.push(PlanWarning::BitlockerPresent); }
    warnings.push(PlanWarning::ProvisionedSizesAreEstimates);
}

fn collect_unchanged_other_disks(
    report: &StorageReport,
    target: &crate::scan::DiskId,
) -> Vec<UnchangedEntry> {
    report.disks.iter()
        .filter(|d| &d.id != target)
        .flat_map(|d| d.partitions.iter().map(move |p| UnchangedEntry {
            disk_id: d.id.clone(),
            partition_number: p.number,
            part_uuid: p.part_uuid.clone(),
            description: partition_description(p),
        }))
        .collect()
}

fn collect_all_unchanged(
    report: &StorageReport,
    target: &crate::scan::DiskId,
    kept_partitions: &[crate::scan::Partition],
) -> Vec<UnchangedEntry> {
    let mut entries = collect_unchanged_other_disks(report, target);
    for p in kept_partitions {
        entries.push(UnchangedEntry {
            disk_id: target.clone(),
            partition_number: p.number,
            part_uuid: p.part_uuid.clone(),
            description: partition_description(p),
        });
    }
    entries
}

/// Hash the plan content to produce a stable plan_hash.
fn plan_hash(plan: &Plan) -> String {
    // FNV-1a over the JSON of the plan (plan_hash field replaced with empty).
    // This is not a cryptographic hash; it is used for confirmation binding
    // only, where the front end has no way to forge it without also having the
    // plan content.
    let json = serde_json::to_string(plan).unwrap_or_default();
    let mut hash: u64 = 14_695_981_039_346_656_037;
    for byte in json.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    format!("{:016x}", hash)
}

fn finalize_plan(mut plan: Plan) -> Plan {
    plan.plan_hash = plan_hash(&plan);
    // Mark all operations as validated = false; the validator will update them.
    plan
}

// ── systemd-repart definition generation ─────────────────────────────────────

fn generate_repart_erase(
    esp_start: u64, esp_size: u64,
    boot_start: u64, boot_size: u64,
    root_start: u64, root_size: u64,
    encrypted: bool,
) -> Vec<RepartDefinition> {
    vec![
        RepartDefinition {
            filename: "10-esp.conf".to_owned(),
            content: format!(
                "[Partition]\nType=esp\nFormat=vfat\nSizeMinBytes={esp_size}\nSizeMaxBytes={esp_size}\nLabel={label}\n",
                esp_size = ESP_SIZE_BYTES,
                label = crate::plan::LABEL_ARKON_ESP,
            ),
        },
        RepartDefinition {
            filename: "20-boot.conf".to_owned(),
            content: format!(
                "[Partition]\nType=linux-generic\nFormat=ext4\nSizeMinBytes={boot_size}\nSizeMaxBytes={boot_size}\nLabel={label}\n",
                boot_size = BOOT_SIZE_BYTES,
                label = crate::plan::LABEL_ARKON_BOOT,
            ),
        },
        RepartDefinition {
            filename: "30-root.conf".to_owned(),
            content: if encrypted {
                format!(
                    "[Partition]\nType=linux-generic\nEncrypt=key-file\nSizeMinBytes={root_size}\nLabel={label}\n",
                    root_size = root_size * 512,
                    label = crate::plan::LABEL_ARKON_ROOT,
                )
            } else {
                format!(
                    "[Partition]\nType=linux-generic\nFormat=btrfs\nSizeMinBytes={root_size}\nLabel={label}\n",
                    root_size = root_size * 512,
                    label = crate::plan::LABEL_ARKON_ROOT,
                )
            },
        },
    ]
}

fn generate_repart_free_space(
    _esp_start: u64, _esp_size: u64,
    _boot_start: u64, _boot_size: u64,
    _root_start: u64, root_size: u64,
    encrypted: bool,
) -> Vec<RepartDefinition> {
    // Free-space mode uses the same definitions but systemd-repart will place
    // them in the available gap rather than erasing first.
    generate_repart_erase(0, 0, 0, 0, 0, root_size, encrypted)
}
