use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::{FirstBootError, FirstBootResult};

/// The eleven ordered setup steps.
/// See ARKON OS Phase 2, Step 9 requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SetupStep {
    CreateUser,
    ConfigureHostname,
    ConfigureTimezone,
    ConfigureNetwork,
    VerifyGraphicsDriver,
    VerifyNvidiaGpu,
    VerifyVulkan,
    VerifyAudio,
    VerifyUpdates,
    VerifyBootDeployment,
    GenerateHealthReport,
}

impl SetupStep {
    pub fn label(&self) -> &'static str {
        match self {
            Self::CreateUser           => "Create user account",
            Self::ConfigureHostname    => "Configure hostname",
            Self::ConfigureTimezone    => "Configure timezone",
            Self::ConfigureNetwork     => "Configure network",
            Self::VerifyGraphicsDriver => "Verify graphics driver",
            Self::VerifyNvidiaGpu      => "Verify NVIDIA GPU",
            Self::VerifyVulkan         => "Verify Vulkan support",
            Self::VerifyAudio          => "Verify audio services",
            Self::VerifyUpdates        => "Verify system update configuration",
            Self::VerifyBootDeployment => "Verify boot deployment",
            Self::GenerateHealthReport => "Generate health report",
        }
    }

    /// Whether failure of this step halts the entire setup process.
    /// Health-check steps are always non-fatal (diagnostic only).
    pub fn is_fatal_on_failure(&self) -> bool {
        matches!(self, Self::CreateUser)
    }

    /// Whether this step is safe to retry after a previous failure.
    pub fn is_retryable(&self) -> bool {
        !matches!(self, Self::CreateUser)
    }

    pub fn all() -> &'static [SetupStep] {
        &[
            Self::CreateUser, Self::ConfigureHostname, Self::ConfigureTimezone,
            Self::ConfigureNetwork, Self::VerifyGraphicsDriver, Self::VerifyNvidiaGpu,
            Self::VerifyVulkan, Self::VerifyAudio, Self::VerifyUpdates,
            Self::VerifyBootDeployment, Self::GenerateHealthReport,
        ]
    }
}

/// Completion status for one setup step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StepStatus {
    /// Not yet attempted.
    Pending,
    /// Completed successfully and must not be re-executed.
    Complete,
    /// Completed with a non-fatal warning.
    Warning { message: String },
    /// Failed; may be retried depending on the step.
    Failed { error: String },
    /// Explicitly skipped (e.g. NVIDIA check on a non-NVIDIA machine).
    Skipped { reason: String },
}

impl StepStatus {
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete | Self::Warning { .. } | Self::Skipped { .. })
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }
}

/// The setup journal — persisted between reboots for restart safety.
/// Lives at `/var/lib/arkon-firstboot/journal.json` on the installed system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupJournal {
    pub steps: HashMap<SetupStep, StepStatus>,
    /// Whether the whole sequence has finished.
    pub finished: bool,
    /// Whether the handoff file was present and valid.
    pub handoff_present: bool,
    /// Error recorded if the whole run was aborted (e.g. user creation failed).
    pub abort_reason: Option<String>,
}

impl SetupJournal {
    /// Create a fresh journal with every step Pending.
    pub fn new() -> Self {
        let mut steps = HashMap::new();
        for &step in SetupStep::all() {
            steps.insert(step, StepStatus::Pending);
        }
        Self { steps, finished: false, handoff_present: false, abort_reason: None }
    }

    pub fn status(&self, step: SetupStep) -> &StepStatus {
        self.steps.get(&step).unwrap_or(&StepStatus::Pending)
    }

    pub fn set_status(&mut self, step: SetupStep, status: StepStatus) {
        self.steps.insert(step, status);
    }

    /// Record a successful completion; returns an error if already complete.
    pub fn complete(&mut self, step: SetupStep) -> FirstBootResult<()> {
        if self.status(step).is_complete() {
            return Err(FirstBootError::AlreadyComplete(step.label().into()));
        }
        self.steps.insert(step, StepStatus::Complete);
        Ok(())
    }

    /// All steps that are still pending (or failed and retryable).
    pub fn pending_steps(&self) -> Vec<SetupStep> {
        SetupStep::all().iter().copied()
            .filter(|&s| {
                let st = self.status(s);
                st.is_pending() || (st.is_failed() && s.is_retryable())
            })
            .collect()
    }

    /// Count of steps that completed (with or without warnings).
    pub fn completed_count(&self) -> usize {
        self.steps.values().filter(|s| s.is_complete()).count()
    }

    /// Count of steps that failed.
    pub fn failed_count(&self) -> usize {
        self.steps.values().filter(|s| s.is_failed()).count()
    }

    pub fn to_json(&self) -> FirstBootResult<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| FirstBootError::JournalError(e.to_string()))
    }

    pub fn from_json(json: &str) -> FirstBootResult<Self> {
        serde_json::from_str(json)
            .map_err(|e| FirstBootError::JournalError(e.to_string()))
    }
}

impl Default for SetupJournal {
    fn default() -> Self { Self::new() }
}
