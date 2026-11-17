use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum FirstBootError {
    /// The handoff file was not found (absent installer run).
    #[error("handoff absent: installer did not write a handoff file")]
    HandoffAbsent,

    /// The handoff file exists but could not be parsed.
    #[error("handoff invalid: {0}")]
    HandoffInvalid(String),

    /// The handoff file has already been consumed (deleted after first use).
    #[error("handoff already consumed")]
    HandoffConsumed,

    /// User account creation failed.
    #[error("account creation failed: {0}")]
    AccountCreationFailed(String),

    /// The proposed username fails validation rules.
    #[error("invalid username '{0}': {1}")]
    InvalidUsername(String, String),

    /// A required configuration value is missing.
    #[error("missing configuration field: {0}")]
    MissingConfig(String),

    /// A system configuration step failed.
    #[error("configuration failed for {step}: {detail}")]
    ConfigFailed { step: String, detail: String },

    /// A health check produced a diagnostic failure (non-fatal).
    #[error("health check failed: {check}: {detail}")]
    HealthCheckFailed { check: String, detail: String },

    /// The setup journal could not be read or written.
    #[error("journal error: {0}")]
    JournalError(String),

    /// A step was attempted in a state that does not allow it.
    #[error("step '{step}' cannot run: journal is in state '{state}'")]
    WrongJournalState { step: String, state: String },

    /// A setup step that must not be repeated was already completed.
    #[error("step '{0}' already completed — will not repeat")]
    AlreadyComplete(String),

    /// Provider returned an unexpected error.
    #[error("provider error: {0}")]
    Provider(String),
}

pub type FirstBootResult<T> = Result<T, FirstBootError>;
