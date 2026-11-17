use crate::error::{FirstBootError, FirstBootResult};
use crate::handoff::HandoffData;
use crate::health::{NvidiaStatus, VulkanStatus};
use crate::provider::FirstBootProvider;
use crate::report::SystemHealthReport;
use crate::state::{SetupJournal, SetupStep, StepStatus};

/// The outcome of a complete or partial first-boot run.
#[derive(Debug)]
pub struct FirstBootOutcome {
    pub journal: SetupJournal,
    pub report: Option<SystemHealthReport>,
    pub interactive_setup_required: bool,
}

impl FirstBootOutcome {
    pub fn is_complete(&self) -> bool {
        self.journal.finished
    }
}

/// Orchestrates the full first-boot setup sequence.
/// Restart-safe: on each call it loads or creates a journal, skips completed
/// steps, executes pending ones, and writes the journal after each step.
pub struct FirstBootRunner<P: FirstBootProvider> {
    provider: P,
}

impl<P: FirstBootProvider> FirstBootRunner<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    pub fn run(&self) -> FirstBootResult<FirstBootOutcome> {
        // Load or create the journal.
        let mut journal = if self.provider.journal_exists() {
            self.provider.read_journal()?
        } else {
            SetupJournal::new()
        };

        // If already finished, just return the existing state.
        if journal.finished {
            return Ok(FirstBootOutcome {
                journal,
                report: None,
                interactive_setup_required: false,
            });
        }

        // Read the handoff file.
        let handoff = if self.provider.handoff_exists() {
            match self.provider.read_handoff() {
                Ok(h) => {
                    if let Err(e) = h.validate() {
                        journal.set_status(SetupStep::CreateUser, StepStatus::Failed { error: format!("handoff invalid: {e}") });
                        self.provider.write_journal(&journal).ok();
                        return Err(FirstBootError::HandoffInvalid(e));
                    }
                    journal.handoff_present = true;
                    Some(h)
                }
                Err(e) => {
                    journal.set_status(SetupStep::CreateUser, StepStatus::Failed { error: e.to_string() });
                    self.provider.write_journal(&journal).ok();
                    return Err(e);
                }
            }
        } else {
            journal.handoff_present = false;
            None
        };

        // Run each step in order.
        for &step in SetupStep::all() {
            if journal.status(step).is_complete() {
                continue; // already done — skip
            }

            let result = self.execute_step(step, handoff.as_ref(), &mut journal);

            match result {
                Ok(status) => {
                    journal.set_status(step, status);
                }
                Err(ref e) if step.is_fatal_on_failure() => {
                    journal.set_status(step, StepStatus::Failed { error: e.to_string() });
                    journal.abort_reason = Some(e.to_string());
                    self.provider.write_journal(&journal).ok();
                    return Err(result.unwrap_err());
                }
                Err(e) => {
                    journal.set_status(step, StepStatus::Failed { error: e.to_string() });
                }
            }

            // Persist after every step (restart safety).
            self.provider.write_journal(&journal).ok();
        }

        journal.finished = true;
        self.provider.write_journal(&journal).ok();

        // Consume the handoff file — delete it so it isn't re-applied.
        if journal.handoff_present {
            self.provider.delete_handoff().ok();
        }

        // Build the health report from the last check results.
        let report = self.build_report(&journal)?;

        Ok(FirstBootOutcome {
            interactive_setup_required: !journal.handoff_present,
            journal,
            report: Some(report),
        })
    }

    fn execute_step(
        &self,
        step: SetupStep,
        handoff: Option<&HandoffData>,
        journal: &mut SetupJournal,
    ) -> FirstBootResult<StepStatus> {
        match step {
            SetupStep::CreateUser => {
                let h = handoff.ok_or_else(|| {
                    FirstBootError::HandoffAbsent
                })?;
                if self.provider.user_exists(&h.username)? {
                    return Ok(StepStatus::Skipped {
                        reason: format!("user '{}' already exists", h.username),
                    });
                }
                self.provider.create_user(&h.username, &h.full_name, &h.password_hash, h.administrator)?;
                Ok(StepStatus::Complete)
            }

            SetupStep::ConfigureHostname => {
                if self.provider.hostname_configured()? {
                    return Ok(StepStatus::Skipped { reason: "hostname already set".into() });
                }
                let h = handoff.ok_or(FirstBootError::HandoffAbsent)?;
                self.provider.set_hostname(&h.hostname)?;
                Ok(StepStatus::Complete)
            }

            SetupStep::ConfigureTimezone => {
                if self.provider.timezone_configured()? {
                    return Ok(StepStatus::Skipped { reason: "timezone already set".into() });
                }
                let h = handoff.ok_or(FirstBootError::HandoffAbsent)?;
                self.provider.set_timezone(&h.timezone)?;
                Ok(StepStatus::Complete)
            }

            SetupStep::ConfigureNetwork => {
                if self.provider.network_configured()? {
                    return Ok(StepStatus::Skipped { reason: "network already configured".into() });
                }
                let h = handoff.ok_or(FirstBootError::HandoffAbsent)?;
                self.provider.apply_network_config(&h.locale, &h.keymap, &h.language)?;
                Ok(StepStatus::Complete)
            }

            SetupStep::VerifyGraphicsDriver => {
                let status = self.provider.check_graphics_driver()?;
                if status.is_ok() {
                    Ok(StepStatus::Complete)
                } else {
                    Ok(StepStatus::Warning { message: status.summary().into() })
                }
            }

            SetupStep::VerifyNvidiaGpu => {
                let status = self.provider.check_nvidia_gpu()?;
                match &status {
                    NvidiaStatus::NotPresent => Ok(StepStatus::Skipped { reason: "no NVIDIA GPU present".into() }),
                    NvidiaStatus::ModuleLoaded { .. } => Ok(StepStatus::Complete),
                    NvidiaStatus::ModuleFailed { error, .. } => {
                        Ok(StepStatus::Warning { message: format!("NVIDIA module failed: {error}") })
                    }
                    NvidiaStatus::UnknownGeneration { pci_id } => {
                        Ok(StepStatus::Warning { message: format!("unknown NVIDIA generation: {pci_id}") })
                    }
                }
            }

            SetupStep::VerifyVulkan => {
                let status = self.provider.check_vulkan()?;
                if status.is_ok() {
                    Ok(StepStatus::Complete)
                } else {
                    Ok(StepStatus::Warning { message: status.summary().into() })
                }
            }

            SetupStep::VerifyAudio => {
                let status = self.provider.check_audio()?;
                if status.is_ok() {
                    Ok(StepStatus::Complete)
                } else {
                    Ok(StepStatus::Warning { message: status.summary().into() })
                }
            }

            SetupStep::VerifyUpdates => {
                let status = self.provider.check_update_config()?;
                if status.is_ok() {
                    Ok(StepStatus::Complete)
                } else {
                    Ok(StepStatus::Warning { message: status.summary().into() })
                }
            }

            SetupStep::VerifyBootDeployment => {
                let image_ref = handoff.map(|h| h.image_ref.as_str()).unwrap_or("");
                let status = self.provider.check_boot_deployment(image_ref)?;
                if status.is_ok() {
                    Ok(StepStatus::Complete)
                } else {
                    Ok(StepStatus::Warning { message: status.summary().into() })
                }
            }

            SetupStep::GenerateHealthReport => {
                // This step is a marker; the actual report is built after the loop.
                Ok(StepStatus::Complete)
            }
        }
    }

    fn build_report(&self, journal: &SetupJournal) -> FirstBootResult<SystemHealthReport> {
        // Re-query health status for the report (checks are read-only and idempotent).
        let graphics  = self.provider.check_graphics_driver().unwrap_or(crate::health::GraphicsDriverStatus::Unknown);
        let nvidia    = self.provider.check_nvidia_gpu().unwrap_or(NvidiaStatus::NotPresent);
        let vulkan    = self.provider.check_vulkan().unwrap_or(VulkanStatus::LoaderMissing);
        let audio     = self.provider.check_audio().unwrap_or(crate::health::AudioStatus::Missing);
        let updates   = self.provider.check_update_config().unwrap_or(crate::health::UpdateConfigStatus::Missing);
        let boot      = self.provider.check_boot_deployment("").unwrap_or(crate::health::BootDeploymentStatus::Missing);

        Ok(SystemHealthReport::build(journal, graphics, nvidia, vulkan, audio, updates, boot))
    }
}
