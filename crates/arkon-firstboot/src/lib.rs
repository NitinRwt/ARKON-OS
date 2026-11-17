pub mod error;
pub mod handoff;
pub mod health;
pub mod provider;
pub mod report;
pub mod runner;
pub mod state;

pub use error::{FirstBootError, FirstBootResult};
pub use handoff::{HandoffData, MokChoice};
pub use health::{
    AudioStatus, BootDeploymentStatus, GraphicsDriverStatus,
    NvidiaStatus, UpdateConfigStatus, VulkanStatus,
};
pub use provider::{FirstBootProvider, MockProvider, RealProvider};
pub use report::{DiagnosticEntry, Severity, SystemHealthReport};
pub use runner::{FirstBootOutcome, FirstBootRunner};
pub use state::{SetupJournal, SetupStep, StepStatus};
