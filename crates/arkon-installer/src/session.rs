use serde::{Deserialize, Serialize};
use crate::config::SystemConfig;
use crate::error::{InstallerError, InstallerResult};
use crate::progress::{ExecutionStep, FailureClass, InstallFailure, DiskCondition};
use crate::screens::Screen;

/// The twelve states of the installer state machine.
/// See INSTALLATION_STATE_MACHINE.md §1 for the full transition diagram.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Start,
    Preflight,
    Blocked,
    Scanned,
    Planned,
    Configured,
    Confirmed,
    Executing,
    Verified,
    Complete,
    Failed,
    CleaningUp,
}

impl SessionState {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Start      => "start",
            Self::Preflight  => "preflight",
            Self::Blocked    => "blocked",
            Self::Scanned    => "scanned",
            Self::Planned    => "planned",
            Self::Configured => "configured",
            Self::Confirmed  => "confirmed",
            Self::Executing  => "executing",
            Self::Verified   => "verified",
            Self::Complete   => "complete",
            Self::Failed     => "failed",
            Self::CleaningUp => "cleaning_up",
        }
    }

    /// Whether disk writes are possible in this state.
    pub fn writes_possible(&self) -> bool {
        matches!(self, Self::Executing | Self::Verified | Self::CleaningUp)
    }

    /// Whether the user may cancel in this state (before X3 is a runtime check).
    pub fn cancel_allowed(&self) -> bool {
        !matches!(self, Self::Executing | Self::Verified | Self::Complete | Self::CleaningUp)
    }
}

/// Reason the preflight or scan was blocked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockReason {
    LegacyBios,
    InsufficientMemory { available_mib: u64, required_mib: u64 },
    NoEligibleDisk,
    DamagedMedium,
    UnsupportedCpu,
}

impl BlockReason {
    pub fn message(&self) -> String {
        match self {
            Self::LegacyBios => "This machine booted in legacy BIOS mode. ARKON OS requires UEFI firmware.".into(),
            Self::InsufficientMemory { available_mib, required_mib } =>
                format!("Insufficient memory: {available_mib} MiB available, {required_mib} MiB required."),
            Self::NoEligibleDisk => "No eligible installation target was found.".into(),
            Self::DamagedMedium => "The installation medium appears damaged; payload digest check failed.".into(),
            Self::UnsupportedCpu => "The CPU does not meet the x86-64-v2 baseline required by Fedora.".into(),
        }
    }
}

/// A single-use confirmation token bound to a plan hash.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfirmationToken {
    pub plan_id: String,
    pub plan_hash: String,
    /// Monotonic seconds at which this token expires.
    pub expires_at: u64,
}

/// The installer session.  In production this wraps the D-Bus orchestrator proxy.
/// In tests and the front end it is a typed in-process model.
#[derive(Debug)]
pub struct InstallerSession {
    state: SessionState,
    plan_id: Option<String>,
    plan_hash: Option<String>,
    config: Option<SystemConfig>,
    block_reason: Option<BlockReason>,
    failure: Option<InstallFailure>,
    current_step: Option<ExecutionStep>,
    token: Option<ConfirmationToken>,
    /// Monotonic clock (seconds). Injected so tests are deterministic.
    clock_fn: fn() -> u64,
    /// Screens current state — used by the front end.
    current_screen: Option<Screen>,
}

impl Default for InstallerSession {
    fn default() -> Self {
        Self::new()
    }
}

impl InstallerSession {
    pub fn new() -> Self {
        Self {
            state: SessionState::Start,
            plan_id: None,
            plan_hash: None,
            config: None,
            block_reason: None,
            failure: None,
            current_step: None,
            token: None,
            clock_fn: default_clock,
            current_screen: None,
        }
    }

    /// For testing: inject a deterministic clock.
    pub fn with_clock(mut self, clock_fn: fn() -> u64) -> Self {
        self.clock_fn = clock_fn;
        self
    }

    // ── Accessors ────────────────────────────────────────────────────────────

    pub fn state(&self) -> &SessionState { &self.state }
    pub fn plan_id(&self) -> Option<&str> { self.plan_id.as_deref() }
    pub fn plan_hash(&self) -> Option<&str> { self.plan_hash.as_deref() }
    pub fn block_reason(&self) -> Option<&BlockReason> { self.block_reason.as_ref() }
    pub fn failure(&self) -> Option<&InstallFailure> { self.failure.as_ref() }
    pub fn current_step(&self) -> Option<ExecutionStep> { self.current_step }
    pub fn current_screen(&self) -> Option<&Screen> { self.current_screen.as_ref() }

    // ── Transitions ──────────────────────────────────────────────────────────

    /// Advance from Start to Preflight.
    pub fn begin_preflight(&mut self) -> InstallerResult<()> {
        self.require_state(&SessionState::Start)?;
        self.state = SessionState::Preflight;
        Ok(())
    }

    /// Complete preflight with no blockers → Scanned.
    pub fn complete_preflight(&mut self) -> InstallerResult<()> {
        self.require_state(&SessionState::Preflight)?;
        self.state = SessionState::Scanned;
        Ok(())
    }

    /// Block the installation with a reason → Blocked.
    pub fn block(&mut self, reason: BlockReason) -> InstallerResult<()> {
        match &self.state {
            SessionState::Preflight | SessionState::Scanned => {}
            other => return Err(InstallerError::WrongState {
                current: other.name().into(),
                required: "preflight or scanned".into(),
            }),
        }
        self.block_reason = Some(reason);
        self.state = SessionState::Blocked;
        Ok(())
    }

    /// Rescan storage.  Valid in Blocked, Scanned, Planned, Configured, Failed → Scanned.
    /// Voids any existing plan.
    pub fn rescan(&mut self) -> InstallerResult<()> {
        match &self.state {
            SessionState::Blocked
            | SessionState::Scanned
            | SessionState::Planned
            | SessionState::Configured
            | SessionState::Failed => {}
            other => return Err(InstallerError::WrongState {
                current: other.name().into(),
                required: "blocked, scanned, planned, configured, or failed".into(),
            }),
        }
        self.plan_id = None;
        self.plan_hash = None;
        self.config = None;
        self.block_reason = None;
        self.failure = None;
        self.token = None;
        self.state = SessionState::Scanned;
        Ok(())
    }

    /// Create a plan.  Valid in Scanned, Planned, Configured → Planned.
    pub fn create_plan(&mut self, plan_id: String, plan_hash: String) -> InstallerResult<()> {
        match &self.state {
            SessionState::Scanned | SessionState::Planned | SessionState::Configured => {}
            other => return Err(InstallerError::WrongState {
                current: other.name().into(),
                required: "scanned, planned, or configured".into(),
            }),
        }
        if plan_id.is_empty() {
            return Err(InstallerError::InvalidArgument("plan_id must not be empty".into()));
        }
        if plan_hash.is_empty() {
            return Err(InstallerError::InvalidArgument("plan_hash must not be empty".into()));
        }
        self.plan_id = Some(plan_id);
        self.plan_hash = Some(plan_hash);
        self.config = None;
        self.token = None;
        self.state = SessionState::Planned;
        Ok(())
    }

    /// Supply system configuration.  Valid in Planned, Configured → Configured.
    pub fn set_configuration(&mut self, plan_id: &str, config: SystemConfig) -> InstallerResult<()> {
        match &self.state {
            SessionState::Planned | SessionState::Configured => {}
            other => return Err(InstallerError::WrongState {
                current: other.name().into(),
                required: "planned or configured".into(),
            }),
        }
        self.check_plan_id(plan_id)?;
        config.validate().map_err(InstallerError::InvalidArgument)?;
        self.config = Some(config);
        self.state = SessionState::Configured;
        Ok(())
    }

    /// Confirm the plan.  Valid in Configured → Confirmed.
    /// `phrase` is required for destructive modes.
    pub fn confirm(
        &mut self,
        plan_id: &str,
        expected_hash: &str,
        phrase: Option<&str>,
        required_phrase: Option<&str>,
        bitlocker_ack: bool,
        bitlocker_required: bool,
    ) -> InstallerResult<ConfirmationToken> {
        self.require_state(&SessionState::Configured)?;
        self.check_plan_id(plan_id)?;

        if self.config.is_none() {
            return Err(InstallerError::MissingConfiguration);
        }
        if bitlocker_required && !bitlocker_ack {
            return Err(InstallerError::BitlockerAckRequired);
        }

        let hash = self.plan_hash.as_deref().unwrap_or("");
        if hash != expected_hash {
            return Err(InstallerError::ConfirmationMismatch);
        }

        if let Some(required) = required_phrase {
            match phrase {
                Some(p) if p == required => {}
                _ => return Err(InstallerError::ConfirmationMismatch),
            }
        }

        let token = ConfirmationToken {
            plan_id: plan_id.into(),
            plan_hash: expected_hash.into(),
            expires_at: (self.clock_fn)() + 300,
        };
        self.token = Some(token.clone());
        self.state = SessionState::Confirmed;
        Ok(token)
    }

    /// Begin execution.  Valid in Confirmed → Executing.
    pub fn execute(&mut self, plan_id: &str, token: &ConfirmationToken) -> InstallerResult<()> {
        self.require_state(&SessionState::Confirmed)?;
        self.check_plan_id(plan_id)?;

        let stored = self.token.as_ref().ok_or(InstallerError::TokenExpired)?;
        if stored != token {
            return Err(InstallerError::ConfirmationMismatch);
        }
        if (self.clock_fn)() > stored.expires_at {
            self.token = None;
            self.state = SessionState::Scanned;
            return Err(InstallerError::TokenExpired);
        }

        self.token = None;
        self.current_step = Some(ExecutionStep::X1Inhibit);
        self.state = SessionState::Executing;
        Ok(())
    }

    /// Advance to the next execution step.  Only valid in Executing.
    pub fn advance_step(&mut self, step: ExecutionStep) -> InstallerResult<()> {
        self.require_state(&SessionState::Executing)?;
        self.current_step = Some(step);
        Ok(())
    }

    /// Mark execution as successfully verified → Verified.
    pub fn mark_verified(&mut self) -> InstallerResult<()> {
        self.require_state(&SessionState::Executing)?;
        self.current_step = None;
        self.state = SessionState::Verified;
        Ok(())
    }

    /// Mark installation complete → Complete.
    pub fn mark_complete(&mut self) -> InstallerResult<()> {
        self.require_state(&SessionState::Verified)?;
        self.state = SessionState::Complete;
        Ok(())
    }

    /// Record a failure → Failed.  Valid only in Executing.
    pub fn fail(
        &mut self,
        class: FailureClass,
        step: Option<ExecutionStep>,
        disk_condition: DiskCondition,
        detail: &str,
        user_action: &str,
    ) -> InstallerResult<()> {
        self.require_state(&SessionState::Executing)?;
        self.failure = Some(InstallFailure {
            class,
            step,
            disk_condition,
            other_os_affected: None,
            detail: detail.into(),
            user_action: user_action.into(),
        });
        self.current_step = None;
        self.state = SessionState::Failed;
        Ok(())
    }

    /// Begin cleanup → CleaningUp.  Valid in Failed.
    pub fn begin_cleanup(&mut self) -> InstallerResult<()> {
        self.require_state(&SessionState::Failed)?;
        self.state = SessionState::CleaningUp;
        Ok(())
    }

    /// Complete cleanup.  Transitions to Scanned (if safe) or Failed.
    pub fn finish_cleanup(&mut self, success: bool) -> InstallerResult<()> {
        self.require_state(&SessionState::CleaningUp)?;
        if success {
            self.failure = None;
            self.plan_id = None;
            self.plan_hash = None;
            self.config = None;
            self.state = SessionState::Scanned;
        } else {
            self.state = SessionState::Failed;
        }
        Ok(())
    }

    /// Cancel the installation.  Returns to Scanned.
    /// Allowed in any state before Executing, and during Executing only before X3 begins.
    pub fn cancel(&mut self) -> InstallerResult<()> {
        match &self.state {
            SessionState::Executing => {
                // Allow cancel only while still in X1 or X2 (before point of no return).
                match self.current_step {
                    Some(ExecutionStep::X1Inhibit) | Some(ExecutionStep::X2Snapshot) | None => {}
                    _ => return Err(InstallerError::Busy),
                }
            }
            other if !other.cancel_allowed() => return Err(InstallerError::Busy),
            _ => {}
        }
        self.plan_id = None;
        self.plan_hash = None;
        self.config = None;
        self.token = None;
        self.current_step = None;
        self.state = SessionState::Scanned;
        Ok(())
    }

    /// Set the current screen (called by the front end).
    pub fn set_screen(&mut self, screen: Screen) {
        self.current_screen = Some(screen);
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    fn require_state(&self, required: &SessionState) -> InstallerResult<()> {
        if &self.state != required {
            Err(InstallerError::WrongState {
                current: self.state.name().into(),
                required: required.name().into(),
            })
        } else {
            Ok(())
        }
    }

    fn check_plan_id(&self, plan_id: &str) -> InstallerResult<()> {
        match &self.plan_id {
            Some(id) if id == plan_id => Ok(()),
            Some(_) => Err(InstallerError::PlanIdMismatch),
            None => Err(InstallerError::NoPlan),
        }
    }
}

fn default_clock() -> u64 {
    0
}
