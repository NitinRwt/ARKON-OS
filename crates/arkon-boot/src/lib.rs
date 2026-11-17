//! arkon-boot — UEFI and Secure Boot planning for the ARKON OS installer.
//!
//! All operations in this crate are **planning only**.  No firmware variables
//! are written, no ESP files are created or modified, and no privileged
//! commands are executed.  The orchestrator uses the plans produced here to
//! carry out the actual boot configuration steps with per-action confirmation.
//!
//! **Gate caveat:** Phase 1 is not approved and Phase 2 is not open.  This
//! crate was written at the owner's explicit request (2026-11-17).  It must be
//! revalidated against Phase 1 facts once Phase 1 exists (R-44).

pub mod bootloader;
pub mod diagnostics;
pub mod error;
pub mod esp;
pub mod firmware;
pub mod mok;
pub mod plan;
pub mod provider;
pub mod secureboot;

pub use error::{BootError, BootResult};
pub use firmware::{FirmwareEntry, FirmwareSnapshot};
pub use plan::{BootPlan, BootPlanRequest, build_boot_plan};
pub use provider::{BootProvider, MockProvider, RealProvider};
