use serde::{Deserialize, Serialize};
use crate::esp::{EspFile, EspVerification, EspVerdict};
use crate::firmware::{FirmwareEntry, FirmwareSnapshot};
use crate::mok::MokEnrolmentState;

/// A diagnostic finding from repair-mode inspection (UEFI_BOOT_DESIGN.md §7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiagnosticFinding {
    /// The expected ESP partition UUID is not found on any disk.
    EspPartitionMissing { expected_uuid: String },
    /// The ESP partition exists but is missing bootloader files.
    EspFileMissing { esp_uuid: String, file: EspFile },
    /// The firmware has no entry pointing at the ARKON ESP.
    FirmwareEntryMissing { expected_label: String },
    /// A firmware entry with the ARKON label exists but points at the wrong partition.
    FirmwareEntryPointsElsewhere {
        label: String,
        entry_number: u16,
        found_uuid: String,
        expected_uuid: String,
    },
    /// The ARKON firmware entry exists but is not in the boot order.
    ArkonNotInBootOrder { entry_number: u16 },
    /// The ARKON module-signing certificate is not enrolled in MOK.
    MokNotEnrolled,
    /// An NVIDIA driver variant is installed but the module cannot load.
    NvidiaModuleCannotLoad { reason: String },
    /// A previous ARKON deployment is available for rollback.
    PreviousDeploymentAvailable { count: usize },
    /// A stale ARKON boot entry points to a partition that no longer exists.
    StaleArkonEntry { entry: FirmwareEntry },
}

/// An action the installer's repair mode can take.
///
/// **`DisableSecureBoot` is intentionally not a variant** — the project
/// charter and SECURE_BOOT_DESIGN.md §10 forbid ever suggesting this.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RepairAction {
    /// Add a firmware boot entry pointing at shim on the ARKON ESP.
    RecreateFirmwareEntry { label: String, esp_uuid: String, loader_path: String },
    /// Run bootupd's install step against the ARKON ESP only.
    ReinstallBootloaderFiles { esp_uuid: String },
    /// Queue ARKON certificate enrolment via `mokutil`.
    QueueMokEnrolment,
    /// Select a previous bootc deployment as the default for the next boot.
    SelectPreviousDeployment { deployment_index: usize },
    /// Remove a stale ARKON boot entry after user confirmation.
    RemoveStaleEntry { entry_number: u16, label: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub findings: Vec<DiagnosticFinding>,
    pub recommended_actions: Vec<RepairAction>,
    /// True when no findings were detected and the system is healthy.
    pub healthy: bool,
}

impl DiagnosticReport {
    fn new() -> Self {
        Self { findings: vec![], recommended_actions: vec![], healthy: false }
    }

    fn finalize(mut self) -> Self {
        self.healthy = self.findings.is_empty();
        self
    }
}

/// Run repair-mode diagnostics against the current system state.
///
/// This function is **read-only** — it only records findings and recommended
/// actions; the orchestrator decides which actions to execute with explicit
/// per-action confirmation (D-039, UEFI_BOOT_DESIGN.md §7).
pub fn diagnose(
    snapshot: &FirmwareSnapshot,
    esp_verification: &EspVerification,
    expected_esp_uuid: &str,
    mok_state: &MokEnrolmentState,
    previous_deployments: usize,
    stale_entries: Vec<&FirmwareEntry>,
) -> DiagnosticReport {
    let mut report = DiagnosticReport::new();

    // — ESP contents ———————————————————————————————————————————————————————————
    for missing_file in &esp_verification.missing {
        report.findings.push(DiagnosticFinding::EspFileMissing {
            esp_uuid: expected_esp_uuid.into(),
            file: missing_file.clone(),
        });
    }
    if matches!(esp_verification.verdict, EspVerdict::Incomplete { .. }) {
        report.recommended_actions.push(RepairAction::ReinstallBootloaderFiles {
            esp_uuid: expected_esp_uuid.into(),
        });
    }

    // — Firmware entry ————————————————————————————————————————————————————————
    let arkon_entry = snapshot.entries.iter().find(|e| {
        e.is_arkon_label() && e.part_uuid.as_deref() == Some(expected_esp_uuid)
    });

    if arkon_entry.is_none() {
        // Look for an entry with the right label but pointing elsewhere.
        let mispointed = snapshot.entries.iter().find(|e| e.is_arkon_label());
        if let Some(e) = mispointed {
            report.findings.push(DiagnosticFinding::FirmwareEntryPointsElsewhere {
                label: e.label.clone(),
                entry_number: e.number,
                found_uuid: e.part_uuid.clone().unwrap_or_default(),
                expected_uuid: expected_esp_uuid.into(),
            });
        } else {
            report.findings.push(DiagnosticFinding::FirmwareEntryMissing {
                expected_label: "ARKON OS".into(),
            });
        }
        report.recommended_actions.push(RepairAction::RecreateFirmwareEntry {
            label: "ARKON OS".into(),
            esp_uuid: expected_esp_uuid.into(),
            loader_path: r"\EFI\fedora\shimx64.efi".into(),
        });
    } else if let Some(e) = arkon_entry {
        // Entry exists — check boot order.
        if !snapshot.boot_order.contains(&e.number) {
            report.findings.push(DiagnosticFinding::ArkonNotInBootOrder {
                entry_number: e.number,
            });
        }
    }

    // — MOK —————————————————————————————————————————————————————————————————
    if mok_state.is_queued() {
        report.findings.push(DiagnosticFinding::MokNotEnrolled);
        report.recommended_actions.push(RepairAction::QueueMokEnrolment);
    }

    // — Previous deployments ——————————————————————————————————————————————————
    if previous_deployments > 1 {
        report.findings.push(DiagnosticFinding::PreviousDeploymentAvailable {
            count: previous_deployments - 1,
        });
        report.recommended_actions.push(RepairAction::SelectPreviousDeployment {
            deployment_index: previous_deployments - 1,
        });
    }

    // — Stale entries —————————————————————————————————————————————————————————
    for stale in stale_entries {
        report.findings.push(DiagnosticFinding::StaleArkonEntry { entry: stale.clone() });
        report.recommended_actions.push(RepairAction::RemoveStaleEntry {
            entry_number: stale.number,
            label: stale.label.clone(),
        });
    }

    report.finalize()
}
