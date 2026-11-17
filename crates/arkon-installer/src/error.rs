use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum InstallerError {
    /// The called method is not valid in the current state.
    #[error("wrong state: method requires {required}, session is in {current}")]
    WrongState { current: String, required: String },

    /// A required argument was missing or failed validation.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// The plan validator rejected the request.
    #[error("plan rejected by rule {rule}: {detail}")]
    PlanRejected { rule: String, detail: String },

    /// The disk layout changed between plan-build and confirmation.
    #[error("plan is stale: the disk layout has changed since the plan was built")]
    StalePlan,

    /// The confirmation phrase or hash did not match.
    #[error("confirmation mismatch: phrase or plan hash is incorrect")]
    ConfirmationMismatch,

    /// The confirmation token has expired or been used.
    #[error("confirmation token expired or already used")]
    TokenExpired,

    /// System configuration has not been provided yet.
    #[error("missing system configuration: call configure() before confirm()")]
    MissingConfiguration,

    /// An encryption passphrase is required but was not provided.
    #[error("missing encryption passphrase")]
    MissingSecret,

    /// A BitLocker acknowledgement is required but was not provided.
    #[error("BitLocker acknowledgement required: confirm you have the recovery key")]
    BitlockerAckRequired,

    /// There is no current plan.
    #[error("no plan: call create_plan() first")]
    NoPlan,

    /// The plan_id supplied does not match the current plan.
    #[error("plan ID mismatch")]
    PlanIdMismatch,

    /// A storage scan produced no eligible disks.
    #[error("no eligible installation target found")]
    NoEligibleDisk,

    /// The session is locked during execution.
    #[error("busy: installation is in progress")]
    Busy,
}

pub type InstallerResult<T> = Result<T, InstallerError>;
