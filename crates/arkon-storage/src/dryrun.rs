/// Dry-run mode: simulate a plan against a StorageReport without touching any
/// disk and return a detailed report of what would happen.
///
/// The dry-run never invokes the partitioner; it validates the plan and
/// produces a human-readable summary of every operation.
use crate::error::StorageResult;
use crate::plan::{OperationType, Plan, PlanMode, RiskLevel, ValidationReport};
use crate::scan::StorageReport;
use crate::validator::validate;
use serde::{Deserialize, Serialize};

/// One line of dry-run output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DryRunStep {
    pub op_id: String,
    pub risk: RiskLevel,
    pub summary: String,
    /// True when this step would be undone by a rollback (all steps before
    /// the point of no return are considered reversible).
    pub reversible: bool,
}

/// Full dry-run result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DryRunReport {
    pub plan_id: String,
    pub mode: PlanMode,
    pub target_disk: String,
    pub validation: ValidationReport,
    pub steps: Vec<DryRunStep>,
    pub data_loss_summary: Vec<String>,
    pub size_summary: SizeSummary,
    /// True when at least one rule failed validation.
    pub has_validation_errors: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeSummary {
    pub disk_size_bytes: u64,
    pub esp_size_bytes: Option<u64>,
    pub boot_size_bytes: Option<u64>,
    pub root_size_bytes: Option<u64>,
    pub free_after_bytes: Option<u64>,
}

/// Run the full dry-run: validate the plan and produce a simulation report.
pub fn dry_run(plan: &Plan, report: &StorageReport) -> StorageResult<DryRunReport> {
    let validation = validate(plan, report)?;
    let disk = report.disk(&plan.target_disk).ok_or_else(|| {
        crate::error::StorageError::DiskNotFound(plan.target_disk.0.clone())
    })?;

    let mut steps: Vec<DryRunStep> = Vec::new();
    let mut esp_bytes: Option<u64> = None;
    let mut boot_bytes: Option<u64> = None;
    let mut root_bytes: Option<u64> = None;

    let sector = u64::from(disk.logical_sector_bytes);
    // The point of no return is the first EraseTable or any Format.
    let por_index = plan
        .operations
        .iter()
        .position(|op| {
            matches!(
                op.operation,
                OperationType::EraseTable | OperationType::Format | OperationType::ReplaceSubvolume
            )
        })
        .unwrap_or(plan.operations.len());

    for (i, op) in plan.operations.iter().enumerate() {
        let reversible = i < por_index;
        let summary = describe_op(op, sector, &mut esp_bytes, &mut boot_bytes, &mut root_bytes);
        steps.push(DryRunStep {
            op_id: op.op_id.clone(),
            risk: op.risk.clone(),
            summary,
            reversible,
        });
    }

    let data_loss_summary = plan
        .data_loss
        .iter()
        .map(|d| {
            if let Some(os) = &d.detected_os {
                format!("[{}] {} ({})", d.partition_number, d.description, os)
            } else {
                format!("[{}] {}", d.partition_number, d.description)
            }
        })
        .collect();

    // Rough free space after installation
    let used = esp_bytes.unwrap_or(0)
        + boot_bytes.unwrap_or(0)
        + root_bytes.unwrap_or(0);
    let free_after = disk.size_bytes.checked_sub(used);

    let has_validation_errors = !validation.all_passed;

    Ok(DryRunReport {
        plan_id: plan.plan_id.0.clone(),
        mode: plan.mode.clone(),
        target_disk: plan.target_disk.0.clone(),
        validation,
        steps,
        data_loss_summary,
        size_summary: SizeSummary {
            disk_size_bytes: disk.size_bytes,
            esp_size_bytes: esp_bytes,
            boot_size_bytes: boot_bytes,
            root_size_bytes: root_bytes,
            free_after_bytes: free_after,
        },
        has_validation_errors,
    })
}

fn describe_op(
    op: &crate::plan::PlannedOperation,
    sector: u64,
    esp_bytes: &mut Option<u64>,
    boot_bytes: &mut Option<u64>,
    root_bytes: &mut Option<u64>,
) -> String {
    let risk_tag = match op.risk {
        RiskLevel::None        => "[safe]",
        RiskLevel::Low         => "[low risk]",
        RiskLevel::Destructive => "[DESTRUCTIVE]",
    };

    match &op.operation {
        OperationType::EraseTable => {
            format!("{} erase partition table on {}", risk_tag, op.target_disk)
        }
        OperationType::Create => {
            if let Some(after) = &op.after {
                let size = after.size_lba * sector;
                let label = after.part_label.as_deref().unwrap_or("?");
                let gib = size as f64 / (1024.0 * 1024.0 * 1024.0);
                match label {
                    crate::plan::LABEL_ARKON_ESP  => { *esp_bytes  = Some(size); }
                    crate::plan::LABEL_ARKON_BOOT => { *boot_bytes = Some(size); }
                    crate::plan::LABEL_ARKON_ROOT => { *root_bytes = Some(size); }
                    _ => {}
                }
                format!(
                    "{} create partition {} '{}' at LBA {} ({:.1} GiB, {:?})",
                    risk_tag, after.number, label, after.start_lba, gib,
                    after.fs_type,
                )
            } else {
                format!("{} create partition (details unavailable)", risk_tag)
            }
        }
        OperationType::Keep => {
            if let Some(before) = &op.before {
                format!("[safe] keep partition {} unchanged", before.number)
            } else {
                "[safe] keep partition unchanged".to_owned()
            }
        }
        OperationType::Format => {
            if let Some(before) = &op.before {
                format!(
                    "{} reformat partition {} '{}'",
                    risk_tag, before.number,
                    before.part_label.as_deref().unwrap_or("?")
                )
            } else {
                format!("{} reformat partition", risk_tag)
            }
        }
        OperationType::ReplaceSubvolume => {
            if let Some(before) = &op.before {
                format!(
                    "{} replace root subvolume on partition {} (home subvolume kept)",
                    risk_tag, before.number
                )
            } else {
                format!("{} replace root subvolume", risk_tag)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::PlanRequest;
    use crate::planner::build_plan;
    use crate::scan::{PartitionTable, test_disk, test_report};

    #[test]
    fn dry_run_erase_produces_steps() {
        let disk = test_disk(
            "nvme0",
            200 * 1024 * 1024 * 1024, // 200 GiB
            512,
            PartitionTable::Gpt,
        );
        let report = test_report(vec![disk]);
        let req = PlanRequest {
            mode: crate::plan::PlanMode::EraseDisk,
            scan_id: report.scan_id.clone(),
            target_disk: report.disks[0].id.clone(),
            region_id: None,
            root_size_bytes: None,
            encryption: false,
            variant: Some("arkon".to_owned()),
        };
        let plan = build_plan(&req, &report).unwrap();
        let dr = dry_run(&plan, &report).unwrap();
        // Should have EraseTable + 3 Create ops
        assert_eq!(dr.steps.len(), 4);
        assert!(dr.steps.iter().any(|s| s.summary.contains("erase partition table")));
        assert!(dr.size_summary.esp_size_bytes.is_some());
        assert!(dr.size_summary.root_size_bytes.is_some());
    }

    #[test]
    fn dry_run_validation_errors_flagged() {
        let mut disk = test_disk("nvme0", 200 * 1024 * 1024 * 1024, 512, PartitionTable::Gpt);
        disk.is_install_medium = true;
        let report = test_report(vec![disk]);
        // Build the plan bypassing the blocker check by using a custom minimal plan
        use crate::plan::{PlanId, PlanMode};
        use crate::scan::ScanId;
        let plan = crate::plan::Plan {
            plan_id: PlanId::new("test"),
            plan_hash: "0".to_owned(),
            scan_id: ScanId::new("test-scan-001"),
            mode: PlanMode::EraseDisk,
            target_disk: report.disks[0].id.clone(),
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
        };
        let dr = dry_run(&plan, &report).unwrap();
        assert!(dr.has_validation_errors);
    }
}
