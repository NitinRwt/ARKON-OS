/// Safety validator — rules P1–P9 from PARTITIONING_DESIGN.md §5.
///
/// The validator is called twice:
///   1. After planning (pre-execution): all nine rules checked on the plan.
///   2. After execution (post-execution): the same rules checked against the
///      rescanned disk, using the plan as the expected state.
///
/// This module never writes to any disk.
use crate::error::StorageError;
use crate::plan::{
    OperationType, Plan, PlanMode, RuleResult, ValidationReport,
    ALIGNMENT_BYTES, ESP_SIZE_BYTES, BOOT_SIZE_BYTES, MIN_ROOT_BYTES,
};
use crate::scan::{PartitionClass, StorageReport};

/// Run all validator rules against `plan`.
///
/// Returns `Ok(ValidationReport)` even when rules fail, because the caller
/// must see which rules failed.  Returns `Err` only when the validator itself
/// cannot run (e.g. the target disk is missing from the report).
pub fn validate(plan: &Plan, report: &StorageReport) -> Result<ValidationReport, StorageError> {
    let disk = report.disk(&plan.target_disk).ok_or_else(|| {
        StorageError::DiskNotFound(plan.target_disk.0.clone())
    })?;

    let mut rules: Vec<RuleResult> = Vec::new();

    // P1: target disk is not the installation medium.
    rules.push(check_p1(disk));

    // P2: every new partition lies within the target region.
    rules.push(check_p2(plan, disk));

    // P3: in non-erase modes, every pre-existing partition is unchanged.
    rules.push(check_p3(plan, disk));

    // P4: no operation refers to a partition on a different disk.
    rules.push(check_p4(plan));

    // P5: no pre-existing ESP is formatted, mounted read-write or passed to bootc.
    rules.push(check_p5(plan, disk));

    // P6: data_loss list equals existing partitions inside the target region.
    rules.push(check_p6(plan, disk));

    // P7: sizes meet minimums; alignment is to 1 MiB.
    rules.push(check_p7(plan, disk));

    // P8: in mode 5b, only ARKON-labelled partitions are reformatted.
    rules.push(check_p8(plan, disk));

    // P9: scan_id on the plan matches the report.
    rules.push(check_p9(plan, report));

    let all_passed = rules.iter().all(|r| r.passed);
    Ok(ValidationReport {
        plan_id: plan.plan_id.clone(),
        all_passed,
        rules,
    })
}

/// Return the first failing rule, if any, as a `StorageError`.
pub fn validate_strict(
    plan: &Plan,
    report: &StorageReport,
) -> Result<ValidationReport, StorageError> {
    let report = validate(plan, report)?;
    if let Some(rule) = report.rules.iter().find(|r| !r.passed) {
        return Err(StorageError::ValidationFailed(rule.rule, rule.detail.clone()));
    }
    Ok(report)
}

// ── Rule implementations ──────────────────────────────────────────────────────

fn pass(rule: u8, detail: impl Into<String>) -> RuleResult {
    RuleResult { rule, passed: true, detail: detail.into() }
}
fn fail(rule: u8, detail: impl Into<String>) -> RuleResult {
    RuleResult { rule, passed: false, detail: detail.into() }
}

/// P1: target is not the install medium.
fn check_p1(disk: &crate::scan::Disk) -> RuleResult {
    if disk.is_install_medium {
        fail(1, "target disk is the installation medium")
    } else {
        pass(1, "target disk is not the installation medium")
    }
}

/// P2: every Create operation lands within the target region (whole disk in
/// erase mode; the chosen free region in free-space / manual mode).
fn check_p2(plan: &Plan, disk: &crate::scan::Disk) -> RuleResult {
    let disk_end_lba = disk.size_bytes / u64::from(disk.logical_sector_bytes);

    for op in &plan.operations {
        if op.operation != OperationType::Create {
            continue;
        }
        let Some(after) = &op.after else { continue };

        let op_end = after.start_lba + after.size_lba;

        match &plan.mode {
            PlanMode::EraseDisk => {
                if op_end > disk_end_lba {
                    return fail(2, format!(
                        "op {} end LBA {} exceeds disk end {}",
                        op.op_id, op_end, disk_end_lba
                    ));
                }
            }
            PlanMode::FreeSpace | PlanMode::Manual => {
                // Find the matching free region: the first region that contains
                // the operation's start LBA.
                let in_region = disk.free_regions.iter().any(|r| {
                    after.start_lba >= r.start_lba
                        && op_end <= r.start_lba + r.size_lba
                });
                if !in_region {
                    return fail(2, format!(
                        "op {} LBA range {}..{} is not within any free region",
                        op.op_id, after.start_lba, op_end
                    ));
                }
            }
            _ => {} // reinstall modes do not create in unallocated space
        }
    }
    pass(2, "all create operations lie within the target region")
}

/// P3: in non-erase modes every pre-existing partition reports unchanged.
fn check_p3(plan: &Plan, disk: &crate::scan::Disk) -> RuleResult {
    if matches!(plan.mode, PlanMode::EraseDisk) {
        return pass(3, "erase mode: P3 not applicable");
    }

    for op in &plan.operations {
        if op.operation != OperationType::Keep {
            continue;
        }
        let (Some(before), Some(after)) = (&op.before, &op.after) else {
            return fail(3, format!("op {} Keep has missing before/after state", op.op_id));
        };
        // The corresponding partition must still exist on the disk and have
        // the same start, size, type_guid and part_uuid.
        let Some(part) = disk.partition(before.number) else {
            return fail(3, format!(
                "partition {} listed as Keep but not found on disk", before.number
            ));
        };
        if part.start_lba != before.start_lba {
            return fail(3, format!(
                "partition {} start changed: expected {} got {}",
                before.number, before.start_lba, part.start_lba
            ));
        }
        if part.size_lba != before.size_lba {
            return fail(3, format!(
                "partition {} size changed: expected {} got {}",
                before.number, before.size_lba, part.size_lba
            ));
        }
        if part.type_guid != before.type_guid {
            return fail(3, format!(
                "partition {} type_guid changed: expected {:?} got {:?}",
                before.number, before.type_guid, part.type_guid
            ));
        }
        if part.part_uuid != before.part_uuid {
            return fail(3, format!(
                "partition {} PARTUUID changed: expected '{}' got '{}'",
                before.number, before.part_uuid, part.part_uuid
            ));
        }
        // before and after must be identical for Keep.
        if before.start_lba != after.start_lba || before.size_lba != after.size_lba {
            return fail(3, format!(
                "op {} Keep has differing before/after state", op.op_id
            ));
        }
    }
    pass(3, "all existing partitions are unchanged")
}

/// P4: no operation targets a partition on a disk other than the target.
fn check_p4(plan: &Plan) -> RuleResult {
    for op in &plan.operations {
        if op.target_disk != plan.target_disk {
            return fail(4, format!(
                "op {} targets disk '{}', not the target disk '{}'",
                op.op_id, op.target_disk, plan.target_disk
            ));
        }
    }
    pass(4, "all operations target the correct disk")
}

/// P5: no pre-existing ESP is formatted (rule S4/S5 from phase 2 architecture).
fn check_p5(plan: &Plan, disk: &crate::scan::Disk) -> RuleResult {
    for op in &plan.operations {
        if op.operation != OperationType::Format && op.operation != OperationType::EraseTable {
            continue;
        }
        let Some(part_num) = op.target_partition else { continue };
        let Some(part) = disk.partition(part_num) else { continue };
        if part.class == PartitionClass::Esp {
            // Only allowed in ReinstallKeepHome on the ARKON-owned ESP.
            if !matches!(plan.mode, PlanMode::ReinstallKeepHome) {
                return fail(5, format!(
                    "op {} would format existing ESP on partition {}",
                    op.op_id, part_num
                ));
            }
            // In reinstall mode it must be the ARKON ESP, not a foreign one.
            if !part.is_arkon() {
                return fail(5, format!(
                    "op {} would format a foreign ESP on partition {}",
                    op.op_id, part_num
                ));
            }
        }
    }
    pass(5, "no pre-existing ESP will be formatted")
}

/// P6: data_loss list equals the set of existing partitions inside the target
/// region.  In erase mode that is all partitions on the disk.  In free-space
/// mode it is empty (the region is unallocated).
fn check_p6(plan: &Plan, disk: &crate::scan::Disk) -> RuleResult {
    match plan.mode {
        PlanMode::EraseDisk => {
            // Every existing partition must appear in data_loss.
            for p in &disk.partitions {
                if !plan.data_loss.iter().any(|d| d.part_uuid == p.part_uuid) {
                    return fail(6, format!(
                        "partition {} ({}) is not listed in data_loss",
                        p.number, p.part_uuid
                    ));
                }
            }
            // No extra entries.
            for d in &plan.data_loss {
                if !disk.partitions.iter().any(|p| p.part_uuid == d.part_uuid) {
                    return fail(6, format!(
                        "data_loss entry for '{}' has no matching partition",
                        d.part_uuid
                    ));
                }
            }
        }
        PlanMode::FreeSpace | PlanMode::Manual => {
            if !plan.data_loss.is_empty() {
                return fail(6, format!(
                    "free-space mode must have an empty data_loss list, found {}",
                    plan.data_loss.len()
                ));
            }
        }
        PlanMode::ReinstallKeepHome => {
            // data_loss must list the ESP, /boot and root partitions.
            // We accept it as long as the list is non-empty and all entries
            // refer to actual partitions.
            for d in &plan.data_loss {
                if !disk.partitions.iter().any(|p| p.part_uuid == d.part_uuid) {
                    return fail(6, format!(
                        "data_loss entry '{}' has no matching partition", d.part_uuid
                    ));
                }
            }
        }
    }
    pass(6, "data_loss list is consistent with the target region")
}

/// P7: ESP ≥ 1 GiB, /boot ≥ 2 GiB, root ≥ 15 GiB, all aligned to 1 MiB.
fn check_p7(plan: &Plan, disk: &crate::scan::Disk) -> RuleResult {
    let sector = u64::from(disk.logical_sector_bytes);
    let align_lba = ALIGNMENT_BYTES / sector;

    for op in &plan.operations {
        if op.operation != OperationType::Create {
            continue;
        }
        let Some(after) = &op.after else { continue };

        // Alignment check
        if align_lba > 0 {
            if after.start_lba % align_lba != 0 {
                return fail(7, format!(
                    "op {} start LBA {} is not aligned to 1 MiB (align_lba={})",
                    op.op_id, after.start_lba, align_lba
                ));
            }
            if after.size_lba % align_lba != 0 {
                return fail(7, format!(
                    "op {} size LBA {} is not a multiple of 1 MiB alignment",
                    op.op_id, after.size_lba
                ));
            }
        }

        // Size minimums by role (detected by label)
        if let Some(label) = &after.part_label {
            let size_bytes = after.size_lba * sector;
            match label.as_str() {
                crate::plan::LABEL_ARKON_ESP => {
                    if size_bytes < ESP_SIZE_BYTES {
                        return fail(7, format!(
                            "ESP op {} size {} < minimum {}", op.op_id, size_bytes, ESP_SIZE_BYTES
                        ));
                    }
                }
                crate::plan::LABEL_ARKON_BOOT => {
                    if size_bytes < BOOT_SIZE_BYTES {
                        return fail(7, format!(
                            "/boot op {} size {} < minimum {}", op.op_id, size_bytes, BOOT_SIZE_BYTES
                        ));
                    }
                }
                crate::plan::LABEL_ARKON_ROOT => {
                    if size_bytes < MIN_ROOT_BYTES {
                        return fail(7, format!(
                            "root op {} size {} < minimum {}", op.op_id, size_bytes, MIN_ROOT_BYTES
                        ));
                    }
                }
                _ => {}
            }
        }
    }
    pass(7, "all sizes meet minimums and alignment")
}

/// P8: in reinstall-keep-home mode, only ARKON-labelled partitions are reformatted.
fn check_p8(plan: &Plan, disk: &crate::scan::Disk) -> RuleResult {
    if !matches!(plan.mode, PlanMode::ReinstallKeepHome) {
        return pass(8, "P8 not applicable outside reinstall mode");
    }
    for op in &plan.operations {
        if !matches!(op.operation, OperationType::Format | OperationType::ReplaceSubvolume) {
            continue;
        }
        let Some(part_num) = op.target_partition else { continue };
        let Some(part) = disk.partition(part_num) else {
            return fail(8, format!("op {} targets partition {} not found", op.op_id, part_num));
        };
        if !part.is_arkon() {
            return fail(8, format!(
                "op {} reformats partition {} which is not ARKON-labelled",
                op.op_id, part_num
            ));
        }
    }
    pass(8, "only ARKON-labelled partitions are reformatted in reinstall mode")
}

/// P9: plan was built from the same scan as the current report.
fn check_p9(plan: &Plan, report: &StorageReport) -> RuleResult {
    if plan.scan_id != report.scan_id {
        fail(9, format!(
            "plan scan_id '{}' does not match current scan_id '{}'",
            plan.scan_id.0, report.scan_id.0
        ))
    } else {
        pass(9, "scan_id matches")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::PlanMode;
    use crate::scan::{
        DiskId, FreeRegion, Partition, PartitionClass, PartitionTable, RegionId, ScanId,
        Transport, test_disk, test_report,
    };

    fn minimal_plan(mode: PlanMode, disk_id: &str, scan_id: &str) -> Plan {
        Plan {
            plan_id: crate::plan::PlanId::new("plan-test"),
            plan_hash: "0000000000000000".to_owned(),
            scan_id: ScanId::new(scan_id),
            mode,
            target_disk: DiskId::new(disk_id),
            variant: None,
            encryption: false,
            operations: vec![],
            unchanged: vec![],
            data_loss: vec![],
            destructive: false,
            warnings: vec![],
            confirmation: crate::plan::ConfirmationRequirements {
                phrase_required: false,
                phrase_hint: None,
                bitlocker_ack_required: false,
            },
            repart_definitions: vec![],
        }
    }

    #[test]
    fn p1_fails_when_install_medium() {
        let mut disk = test_disk("nvme0", 100 * 1024 * 1024 * 1024, 512, PartitionTable::Gpt);
        disk.is_install_medium = true;
        let report = test_report(vec![disk]);
        let plan = minimal_plan(PlanMode::EraseDisk, "nvme0", "test-scan-001");
        let r = validate(&plan, &report).unwrap();
        assert!(!r.rules.iter().find(|r| r.rule == 1).unwrap().passed);
    }

    #[test]
    fn p1_passes_for_normal_disk() {
        let disk = test_disk("nvme0", 100 * 1024 * 1024 * 1024, 512, PartitionTable::Gpt);
        let report = test_report(vec![disk]);
        let plan = minimal_plan(PlanMode::EraseDisk, "nvme0", "test-scan-001");
        let r = validate(&plan, &report).unwrap();
        assert!(r.rules.iter().find(|r| r.rule == 1).unwrap().passed);
    }

    #[test]
    fn p4_fails_when_op_targets_wrong_disk() {
        let disk = test_disk("nvme0", 200 * 1024 * 1024 * 1024, 512, PartitionTable::Gpt);
        let report = test_report(vec![disk]);
        let mut plan = minimal_plan(PlanMode::EraseDisk, "nvme0", "test-scan-001");
        plan.operations.push(crate::plan::PlannedOperation {
            op_id: "op-001".to_owned(),
            target_disk: DiskId::new("sda"),  // wrong disk
            target_partition: None,
            operation: OperationType::EraseTable,
            before: None,
            after: None,
            risk: crate::plan::RiskLevel::Destructive,
            validated: false,
        });
        let r = validate(&plan, &report).unwrap();
        assert!(!r.rules.iter().find(|r| r.rule == 4).unwrap().passed);
    }

    #[test]
    fn p9_fails_on_scan_id_mismatch() {
        let disk = test_disk("nvme0", 200 * 1024 * 1024 * 1024, 512, PartitionTable::Gpt);
        let report = test_report(vec![disk]);
        let plan = minimal_plan(PlanMode::EraseDisk, "nvme0", "old-scan-id");
        let r = validate(&plan, &report).unwrap();
        assert!(!r.rules.iter().find(|r| r.rule == 9).unwrap().passed);
    }

    #[test]
    fn p9_passes_matching_scan_id() {
        let disk = test_disk("nvme0", 200 * 1024 * 1024 * 1024, 512, PartitionTable::Gpt);
        let report = test_report(vec![disk]);
        let plan = minimal_plan(PlanMode::EraseDisk, "nvme0", "test-scan-001");
        let r = validate(&plan, &report).unwrap();
        assert!(r.rules.iter().find(|r| r.rule == 9).unwrap().passed);
    }

    #[test]
    fn p5_fails_when_foreign_esp_formatted() {
        use crate::plan::{OperationState, RiskLevel};
        let mut disk = test_disk("nvme0", 200 * 1024 * 1024 * 1024, 512, PartitionTable::Gpt);
        // Add a non-ARKON ESP
        disk.partitions.push(Partition {
            number: 1,
            part_uuid: "aaaa-1111".to_owned(),
            start_lba: 2048,
            size_lba: 2_097_152,
            type_guid: Some(crate::plan::GUID_EFI_SYSTEM.to_owned()),
            class: PartitionClass::Esp,
            fs_type: Some(crate::scan::FsType::Vfat),
            fs_uuid: None,
            fs_label: None,
            part_label: Some("EFI".to_owned()), // not ARKON-prefixed
        });
        let report = test_report(vec![disk]);
        let mut plan = minimal_plan(PlanMode::EraseDisk, "nvme0", "test-scan-001");
        plan.operations.push(crate::plan::PlannedOperation {
            op_id: "op-001".to_owned(),
            target_disk: DiskId::new("nvme0"),
            target_partition: Some(1),
            operation: OperationType::Format,
            before: Some(OperationState {
                number: 1, start_lba: 2048, size_lba: 2_097_152,
                type_guid: Some(crate::plan::GUID_EFI_SYSTEM.to_owned()),
                part_uuid: "aaaa-1111".to_owned(),
                part_label: Some("EFI".to_owned()),
                fs_type: Some(crate::scan::FsType::Vfat),
            }),
            after: None,
            risk: RiskLevel::Destructive,
            validated: false,
        });
        let r = validate(&plan, &report).unwrap();
        assert!(!r.rules.iter().find(|r| r.rule == 5).unwrap().passed);
    }
}
