use serde::{Deserialize, Serialize};

/// One of the twelve execution steps defined in INSTALLATION_STATE_MACHINE.md §4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ExecutionStep {
    /// X1 — Inhibit sleep/shutdown; pin disk by identifier.
    X1Inhibit,
    /// X2 — Save partition table and firmware entries to log.
    X2Snapshot,
    /// X3 — Partition, format, encrypt. POINT OF NO RETURN in erase modes.
    X3Partition,
    /// X4 — Rescan and compare with the plan.
    X4Verify,
    /// X5 — Unlock, create subvolumes, mount.
    X5Mount,
    /// X6 — Check payload digest; deploy with bootc.
    X6Deploy,
    /// X7 — Read back the deployment.
    X7Readback,
    /// X8 — Write the first-boot handoff.
    X8Handoff,
    /// X9 — Compare firmware entries; apply boot order choice.
    X9Firmware,
    /// X10 — Queue MOK enrolment if applicable.
    X10Mok,
    /// X11 — Write ARKON marker; unmount; flush.
    X11Finalise,
    /// X12 — Final verification: remount read-only, check marker.
    X12Check,
}

impl ExecutionStep {
    /// Human-readable description shown in the progress UI.
    pub fn label(&self) -> &'static str {
        match self {
            Self::X1Inhibit   => "Preparing installation environment",
            Self::X2Snapshot  => "Saving current disk and firmware state",
            Self::X3Partition => "Partitioning and formatting",
            Self::X4Verify    => "Verifying partition layout",
            Self::X5Mount     => "Mounting new partitions",
            Self::X6Deploy    => "Deploying ARKON OS",
            Self::X7Readback  => "Verifying deployment",
            Self::X8Handoff   => "Writing first-boot configuration",
            Self::X9Firmware  => "Registering boot entry",
            Self::X10Mok      => "Queuing Secure Boot certificate",
            Self::X11Finalise => "Finalising installation",
            Self::X12Check    => "Running final verification",
        }
    }

    /// Whether cancellation is still allowed before this step starts.
    /// Cancel is refused once X3 starts.
    pub fn cancel_allowed_before(&self) -> bool {
        matches!(self, Self::X1Inhibit | Self::X2Snapshot)
    }

    /// Approximate fraction of total work completed at the start of this step.
    pub fn approx_fraction(&self) -> f32 {
        match self {
            Self::X1Inhibit   => 0.00,
            Self::X2Snapshot  => 0.05,
            Self::X3Partition => 0.10,
            Self::X4Verify    => 0.30,
            Self::X5Mount     => 0.35,
            Self::X6Deploy    => 0.40,
            Self::X7Readback  => 0.85,
            Self::X8Handoff   => 0.88,
            Self::X9Firmware  => 0.91,
            Self::X10Mok      => 0.94,
            Self::X11Finalise => 0.97,
            Self::X12Check    => 0.99,
        }
    }

    pub fn all() -> &'static [ExecutionStep] {
        &[
            Self::X1Inhibit, Self::X2Snapshot, Self::X3Partition, Self::X4Verify,
            Self::X5Mount, Self::X6Deploy, Self::X7Readback, Self::X8Handoff,
            Self::X9Firmware, Self::X10Mok, Self::X11Finalise, Self::X12Check,
        ]
    }
}

/// One of the ten failure classes defined in INSTALLATION_STATE_MACHINE.md §5.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureClass {
    /// F-PRE — Preflight or scan problem; disk unchanged.
    FPre,
    /// F-PLAN — Validator rejected the plan; disk unchanged.
    FPlan,
    /// F-STALE — Disk changed between plan and confirmation.
    FStale,
    /// F-PART — Partitioner failed (X3).
    FPart,
    /// F-VERIFY — Post-execution comparison failed (X4). Unexpected change detected.
    FVerify,
    /// F-DEPLOY — bootc failed (X6 or X7); ARKON partitions exist but no bootable system.
    FDeploy,
    /// F-HANDOFF — Could not write the handoff (X8); ARKON installed but first boot interactive.
    FHandoff,
    /// F-FIRMWARE — Firmware entry could not be created or existing entry differs (X9).
    FFirmware,
    /// F-MOK — MOK enrolment could not be queued (X10).
    FMok,
    /// F-POWER — Power lost during execution.
    FPower,
}

impl FailureClass {
    pub fn code(&self) -> &'static str {
        match self {
            Self::FPre      => "F-PRE",
            Self::FPlan     => "F-PLAN",
            Self::FStale    => "F-STALE",
            Self::FPart     => "F-PART",
            Self::FVerify   => "F-VERIFY",
            Self::FDeploy   => "F-DEPLOY",
            Self::FHandoff  => "F-HANDOFF",
            Self::FFirmware => "F-FIRMWARE",
            Self::FMok      => "F-MOK",
            Self::FPower    => "F-POWER",
        }
    }

    /// Whether the failure is fatal (no automatic recovery).
    pub fn is_fatal(&self) -> bool {
        matches!(self, Self::FVerify | Self::FPower)
    }

    /// Whether the user may request cleanup.
    pub fn cleanup_offered(&self) -> bool {
        matches!(self, Self::FPart | Self::FDeploy | Self::FHandoff | Self::FFirmware)
    }
}

/// Disk condition description that must accompany every failure report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiskCondition {
    /// No changes were made to the disk.
    Unchanged,
    /// Target disk has incomplete new partitions; existing OS partitions verified intact.
    IncompleteNewPartitions,
    /// An unexpected change was detected; manual inspection required.
    UnexpectedChangeDetected,
    /// ARKON partitions exist but hold no bootable system.
    PartitionsExistNoSystem,
    /// ARKON is installed; first boot will be interactive.
    InstalledInteractiveFirstBoot,
    /// ARKON is installed but no firmware boot entry.
    InstalledNoBootEntry,
}

impl DiskCondition {
    pub fn description(&self) -> &'static str {
        match self {
            Self::Unchanged                   => "The disk was not modified.",
            Self::IncompleteNewPartitions     => "New partitions may partly exist; existing partitions are intact.",
            Self::UnexpectedChangeDetected    => "An unexpected change was detected. Manual inspection is required.",
            Self::PartitionsExistNoSystem     => "ARKON partitions were created but contain no bootable system.",
            Self::InstalledInteractiveFirstBoot => "ARKON is installed. First boot will launch interactive setup.",
            Self::InstalledNoBootEntry        => "ARKON is installed but was not registered with the firmware.",
        }
    }
}

/// A failure that ended or degraded the installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallFailure {
    pub class: FailureClass,
    pub step: Option<ExecutionStep>,
    pub disk_condition: DiskCondition,
    pub other_os_affected: Option<bool>,
    pub detail: String,
    pub user_action: String,
}

/// Live progress snapshot sent to the front end during Executing state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallProgress {
    pub step: ExecutionStep,
    /// Progress fraction [0.0, 1.0].  Never decreases; never reaches 1.0 until X12 completes.
    pub fraction: f32,
    /// Message code for localised string lookup.
    pub message_code: String,
}

impl InstallProgress {
    pub fn at_step(step: ExecutionStep) -> Self {
        Self {
            fraction: step.approx_fraction(),
            message_code: format!("progress.{}", step.label().to_lowercase().replace(' ', "_")),
            step,
        }
    }
}
