use serde::{Deserialize, Serialize};
use crate::health::{
    AudioStatus, BootDeploymentStatus, GraphicsDriverStatus, NvidiaStatus,
    UpdateConfigStatus, VulkanStatus,
};
use crate::state::SetupJournal;

/// Severity of a diagnostic finding in the health report.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// One diagnostic entry in the system health report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticEntry {
    pub severity: Severity,
    pub component: String,
    pub summary: String,
    pub detail: Option<String>,
    pub user_action: Option<String>,
}

impl DiagnosticEntry {
    pub fn info(component: &str, summary: &str) -> Self {
        Self { severity: Severity::Info, component: component.into(), summary: summary.into(), detail: None, user_action: None }
    }
    pub fn warning(component: &str, summary: &str, detail: &str) -> Self {
        Self { severity: Severity::Warning, component: component.into(), summary: summary.into(), detail: Some(detail.into()), user_action: None }
    }
    pub fn error(component: &str, summary: &str, detail: &str, action: &str) -> Self {
        Self { severity: Severity::Error, component: component.into(), summary: summary.into(), detail: Some(detail.into()), user_action: Some(action.into()) }
    }
    pub fn with_action(mut self, action: &str) -> Self {
        self.user_action = Some(action.into());
        self
    }
}

/// The initial system health report presented at the end of first boot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthReport {
    pub graphics_driver: GraphicsDriverStatus,
    pub nvidia: NvidiaStatus,
    pub vulkan: VulkanStatus,
    pub audio: AudioStatus,
    pub update_config: UpdateConfigStatus,
    pub boot_deployment: BootDeploymentStatus,
    pub diagnostics: Vec<DiagnosticEntry>,
    pub setup_steps_completed: usize,
    pub setup_steps_failed: usize,
    /// True when every required step passed and no errors are in the diagnostic list.
    pub all_ok: bool,
}

impl SystemHealthReport {
    pub fn build(
        journal: &SetupJournal,
        graphics_driver: GraphicsDriverStatus,
        nvidia: NvidiaStatus,
        vulkan: VulkanStatus,
        audio: AudioStatus,
        update_config: UpdateConfigStatus,
        boot_deployment: BootDeploymentStatus,
    ) -> Self {
        let mut diagnostics: Vec<DiagnosticEntry> = Vec::new();

        if !graphics_driver.is_ok() {
            diagnostics.push(DiagnosticEntry::error(
                "graphics",
                graphics_driver.summary(),
                "No DRM driver was loaded for the display adapter.",
                "Check ARKON Center → Hardware for driver status.",
            ));
        } else {
            diagnostics.push(DiagnosticEntry::info("graphics", graphics_driver.summary()));
        }

        match &nvidia {
            NvidiaStatus::ModuleFailed { generation, error } => {
                diagnostics.push(DiagnosticEntry::error(
                    "nvidia",
                    nvidia.summary(),
                    &format!("{generation} GPU: {error}"),
                    "Open ARKON Center → Hardware and re-install the NVIDIA driver.",
                ));
            }
            NvidiaStatus::UnknownGeneration { pci_id } => {
                diagnostics.push(DiagnosticEntry::warning(
                    "nvidia",
                    nvidia.summary(),
                    &format!("PCI ID {pci_id} is not in the ARKON supported-GPU table."),
                ).with_action("Check arkonos.com/hardware for the latest compatibility list."));
            }
            _ => {
                diagnostics.push(DiagnosticEntry::info("nvidia", nvidia.summary()));
            }
        }

        if !vulkan.is_ok() {
            diagnostics.push(DiagnosticEntry::warning(
                "vulkan",
                vulkan.summary(),
                "Vulkan is required for most games and GPU-accelerated applications.",
            ).with_action("Check ARKON Center → Hardware for Vulkan status."));
        } else {
            diagnostics.push(DiagnosticEntry::info("vulkan", vulkan.summary()));
        }

        if !audio.is_ok() {
            diagnostics.push(DiagnosticEntry::warning(
                "audio",
                audio.summary(),
                "Audio output may not be available. PipeWire may not have started.",
            ).with_action("Log in and check sound settings; PipeWire starts per-session."));
        } else {
            diagnostics.push(DiagnosticEntry::info("audio", audio.summary()));
        }

        if !update_config.is_ok() {
            diagnostics.push(DiagnosticEntry::error(
                "updates",
                update_config.summary(),
                "The system update configuration could not be found or parsed.",
                "Re-install ARKON OS or restore from a backup image.",
            ));
        } else {
            diagnostics.push(DiagnosticEntry::info("updates", update_config.summary()));
        }

        if !boot_deployment.is_ok() {
            diagnostics.push(DiagnosticEntry::error(
                "boot",
                boot_deployment.summary(),
                "The deployed image could not be verified. The installation may be incomplete.",
                "Re-install ARKON OS.",
            ));
        } else {
            diagnostics.push(DiagnosticEntry::info("boot", boot_deployment.summary()));
        }

        // Setup step failures become diagnostic errors.
        for (step, status) in &journal.steps {
            if status.is_failed() {
                if let crate::state::StepStatus::Failed { error } = status {
                    diagnostics.push(DiagnosticEntry::error(
                        "setup",
                        &format!("Setup step '{}' failed", step.label()),
                        error,
                        "Review the first-boot log at /var/log/arkon-firstboot.log.",
                    ));
                }
            }
        }

        // Sort: errors first, then warnings, then info.
        diagnostics.sort_by(|a, b| b.severity.cmp(&a.severity));

        let all_ok = diagnostics.iter().all(|d| d.severity != Severity::Error)
            && journal.failed_count() == 0;

        Self {
            graphics_driver,
            nvidia,
            vulkan,
            audio,
            update_config,
            boot_deployment,
            diagnostics,
            setup_steps_completed: journal.completed_count(),
            setup_steps_failed: journal.failed_count(),
            all_ok,
        }
    }

    /// All entries at or above the given severity.
    pub fn entries_at_or_above(&self, minimum: Severity) -> Vec<&DiagnosticEntry> {
        self.diagnostics.iter().filter(|e| e.severity >= minimum).collect()
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|e| e.severity == Severity::Error).count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics.iter().filter(|e| e.severity == Severity::Warning).count()
    }
}
