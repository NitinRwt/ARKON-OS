use serde::{Deserialize, Serialize};
use crate::bootloader::{Bootloader, BootloaderValidation, BootloaderVerdict};
use crate::firmware::FirmwareSnapshot;
use crate::mok::MokEnrolmentPlan;
use crate::secureboot::{SecureBootReport, SecureBootState};
use crate::error::BootResult;

/// The user's choice for the ARKON firmware entry position in the boot order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BootOrderChoice {
    /// Place the ARKON entry first in the boot order.
    ArkonFirst,
    /// Append the ARKON entry after all existing entries.
    AppendArkon,
}

/// Input to `build_boot_plan`.
#[derive(Debug, Clone)]
pub struct BootPlanRequest {
    /// PARTUUID of the ESP that this installation created.
    pub esp_uuid: String,
    /// OCI image reference being installed (used for bootloader validation).
    pub image_ref: String,
    /// The bootloader to use.
    pub bootloader: Bootloader,
    /// The user's boot order preference.
    pub boot_order: BootOrderChoice,
    /// Current firmware snapshot (taken before deployment starts).
    pub firmware_snapshot: FirmwareSnapshot,
    /// Whether an NVIDIA variant is being installed.
    pub nvidia_variant: bool,
    /// Whether the ARKON module-signing certificate is already enrolled.
    pub cert_already_enrolled: bool,
    /// Current Secure Boot state.
    pub secure_boot: SecureBootState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BootPlanWarning {
    /// The selected bootloader has not been verified for this release.
    UnverifiedBootloader { detail: String },
    /// Secure Boot is off; the installed system supports Secure Boot on.
    SecureBootOff,
    /// A stale ARKON entry from a previous installation was detected.
    StaleArkonEntryDetected { entry_number: u16 },
}

/// Validation summary included in the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootPlanValidation {
    pub bootloader_ok: bool,
    pub secure_boot_ok: bool,
    pub findings: Vec<String>,
}

impl BootPlanValidation {
    pub fn is_valid(&self) -> bool {
        self.bootloader_ok
    }
}

/// A complete boot configuration plan.
///
/// This is a planning artefact only — no firmware writes or ESP modifications
/// are performed by `arkon-boot`.  The orchestrator executes the plan with
/// per-step confirmation (D-039).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootPlan {
    pub plan_id: String,
    /// PARTUUID of the ARKON ESP.
    pub esp_uuid: String,
    /// Label for the firmware boot entry.
    pub arkon_entry_label: String,
    /// Loader path on the ESP (`\EFI\fedora\shimx64.efi` for GRUB2+shim).
    pub arkon_loader_path: String,
    pub boot_order_choice: BootOrderChoice,
    /// Firmware state at plan-build time (before deployment).
    pub firmware_snapshot_before: FirmwareSnapshot,
    pub bootloader_validation: BootloaderValidation,
    pub secure_boot_report: SecureBootReport,
    pub mok_plan: MokEnrolmentPlan,
    pub validation: BootPlanValidation,
    pub warnings: Vec<BootPlanWarning>,
}

/// Build a boot configuration plan from a request.
///
/// Returns `Err` only when the plan cannot be constructed at all (e.g. not
/// UEFI).  Warnings — unverified bootloader, Secure Boot off, stale entries —
/// are carried in the plan itself, not as errors.
pub fn build_boot_plan(req: BootPlanRequest) -> BootResult<BootPlan> {
    use crate::mok::plan_mok_enrolment;
    use crate::secureboot::validate_secure_boot;
    use crate::bootloader::validate_bootloader;

    let bootloader_validation = validate_bootloader(req.bootloader.clone(), &req.image_ref);
    let secure_boot_report = validate_secure_boot(req.secure_boot.clone(), req.nvidia_variant);
    let mok_plan = plan_mok_enrolment(
        req.nvidia_variant,
        &req.secure_boot,
        req.cert_already_enrolled,
    );

    let mut findings = Vec::new();
    let mut warnings = Vec::new();

    let bootloader_ok = bootloader_validation.verdict.is_acceptable();
    match &bootloader_validation.verdict {
        BootloaderVerdict::Invalid { reason } => {
            findings.push(format!("bootloader invalid: {}", reason));
        }
        BootloaderVerdict::Unverified { reason } => {
            findings.push(format!("bootloader unverified: {}", reason));
            warnings.push(BootPlanWarning::UnverifiedBootloader { detail: reason.clone() });
        }
        BootloaderVerdict::Valid => {}
    }

    // Secure Boot state is shown, never gated.
    let secure_boot_ok = true;
    if !req.secure_boot.is_on() {
        warnings.push(BootPlanWarning::SecureBootOff);
    }

    // Detect stale ARKON entries (different UUID from the ESP we created).
    for entry in req.firmware_snapshot.entries.iter().filter(|e| {
        e.is_arkon_label() && e.part_uuid.as_deref() != Some(req.esp_uuid.as_str())
    }) {
        warnings.push(BootPlanWarning::StaleArkonEntryDetected { entry_number: entry.number });
    }

    let plan_id = format!(
        "boot-{}-{}",
        req.esp_uuid.replace('-', ""),
        req.image_ref.len()
    );

    Ok(BootPlan {
        plan_id,
        esp_uuid: req.esp_uuid,
        arkon_entry_label: "ARKON OS".into(),
        arkon_loader_path: r"\EFI\fedora\shimx64.efi".into(),
        boot_order_choice: req.boot_order,
        firmware_snapshot_before: req.firmware_snapshot,
        bootloader_validation,
        secure_boot_report,
        mok_plan,
        validation: BootPlanValidation { bootloader_ok, secure_boot_ok, findings },
        warnings,
    })
}
