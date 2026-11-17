/// arkon-storage — storage scanner, partition planner and safety validator.
///
/// Entry points:
///   - [`scan`]              — populate a StorageReport from /sys/block
///   - [`plan`]              — build a partition plan from a request and a scan report
///   - [`plan_and_validate`] — plan + run all validator rules
///   - [`plan_dry_run`]      — plan + validate + produce a simulation report
pub mod dryrun;
pub mod error;
pub mod plan;
pub mod planner;
pub mod provider;
pub mod scan;
pub mod scanner;
pub mod validator;

pub use dryrun::{DryRunReport, DryRunStep, SizeSummary, dry_run};
pub use error::{StorageError, StorageResult};
pub use plan::{
    Plan, PlanId, PlanMode, PlanRequest, PlanWarning, PlannedOperation, ValidationReport,
    MIN_DISK_BYTES, MIN_FREE_REGION_BYTES, MIN_ROOT_BYTES,
    ESP_SIZE_BYTES, BOOT_SIZE_BYTES,
};
pub use planner::build_plan;
pub use provider::{MockProvider, RealProvider, SysfsProvider};
pub use scan::{
    Disk, DiskId, FreeRegion, PartitionClass, PartitionTable, RegionId, ScanId,
    StorageReport, Transport,
};
pub use scanner::scan;
pub use validator::{validate, validate_strict};

/// Build a partition plan from `request` and `report`.
///
/// Does not validate; call `validate` or `plan_and_validate` for that.
pub fn plan(request: &PlanRequest, report: &StorageReport) -> StorageResult<Plan> {
    build_plan(request, report)
}

/// Build a plan and run the full validator.  Returns the plan and the
/// validation report.  The plan is returned even when validation fails, so the
/// caller can inspect which rules failed.
pub fn plan_and_validate(
    request: &PlanRequest,
    report: &StorageReport,
) -> StorageResult<(Plan, ValidationReport)> {
    let p = build_plan(request, report)?;
    let v = validate(&p, report)?;
    Ok((p, v))
}

/// Build a plan, validate it, and run a dry-run simulation.
pub fn plan_dry_run(
    request: &PlanRequest,
    report: &StorageReport,
) -> StorageResult<DryRunReport> {
    let p = build_plan(request, report)?;
    dry_run(&p, report)
}
