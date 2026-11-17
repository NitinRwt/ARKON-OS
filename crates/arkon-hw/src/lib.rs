/// arkon-hw — hardware discovery library for the ARKON OS installer.
///
/// Entry point: [`probe`].  All reads are from /proc and /sys; nothing is
/// written, nothing is mounted.
pub mod cpu;
pub mod error;
pub mod firmware;
pub mod gpu;
pub mod provider;
pub mod report;
pub mod storage;

pub use error::{ProbeError, ProbeResult};
pub use provider::{MockProvider, RealProvider, SysfsProvider};
pub use report::{HardwareReport, HardwareWarning, Variant, WarningCode};

/// Run the full hardware probe using the given provider and return a
/// `HardwareReport`.
///
/// Soft failures (e.g. a single unreadable sysfs file) are recorded as
/// warnings in the report.  Hard failures (e.g. /proc/cpuinfo entirely
/// absent) are returned as `Err(ProbeError)`.
pub fn probe(provider: &dyn SysfsProvider) -> ProbeResult<HardwareReport> {
    let cpu      = cpu::detect(provider)?;
    let gpus     = gpu::detect(provider)?;
    let storage  = storage::detect(provider)?;
    let firmware = firmware::detect(provider)?;

    Ok(report::build_report(cpu, gpus, storage, firmware))
}
