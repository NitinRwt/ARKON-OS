use serde::{Deserialize, Serialize};
use crate::secureboot::SecureBootState;

/// Whether the ARKON module-signing certificate should be enrolled via MOK.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MokEnrolmentState {
    /// No NVIDIA variant — MOK enrolment is not needed.
    NotRequired,
    /// ARKON certificate is already enrolled from a previous installation.
    AlreadyEnrolled,
    /// Secure Boot is off — MOK has no effect; enrolment is skipped.
    SecureBootOff,
    /// User declined enrolment at the final installation screen.
    Declined,
    /// Enrolment has been queued via `mokutil`.
    ///
    /// `password_hint` is shown to the user so they can complete the physical
    /// console step at the next boot (SECURE_BOOT_DESIGN.md §5).
    Queued { password_hint: String },
}

impl MokEnrolmentState {
    pub fn is_queued(&self) -> bool {
        matches!(self, MokEnrolmentState::Queued { .. })
    }
}

/// A plan for MOK certificate enrolment.
///
/// Planning only — this struct is never executed by `arkon-boot`.
/// The orchestrator passes `cert_path` to `mokutil` based on this plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MokEnrolmentPlan {
    pub state: MokEnrolmentState,
    /// Path to the certificate inside the payload image.
    pub cert_path: Option<String>,
    /// Instructions to show on the final installation screen.
    pub user_instructions: Option<String>,
}

impl MokEnrolmentPlan {
    fn not_required() -> Self {
        Self { state: MokEnrolmentState::NotRequired, cert_path: None, user_instructions: None }
    }

    fn sb_off() -> Self {
        Self {
            state: MokEnrolmentState::SecureBootOff,
            cert_path: None,
            user_instructions: Some(
                "Secure Boot is off. If you enable it later, re-queue MOK enrolment \
                 from the ARKON desktop to load the NVIDIA driver."
                    .into(),
            ),
        }
    }

    fn already_enrolled() -> Self {
        Self {
            state: MokEnrolmentState::AlreadyEnrolled,
            cert_path: None,
            user_instructions: Some("ARKON certificate is already enrolled; no action needed.".into()),
        }
    }

    fn queued(cert_path: impl Into<String>) -> Self {
        Self {
            state: MokEnrolmentState::Queued {
                password_hint: "You will be prompted to enter a one-time password at \
                                the next boot to complete MOK enrolment."
                    .into(),
            },
            cert_path: Some(cert_path.into()),
            user_instructions: Some(
                "On the next boot, the firmware will show the MOK management screen. \
                 Select \"Enroll MOK\" → \"Continue\", enter the password shown above, \
                 and confirm. This enrolls the ARKON certificate so the NVIDIA driver \
                 can load. Skipping leaves the system without NVIDIA acceleration until \
                 enrolment is completed from the ARKON desktop."
                    .into(),
            ),
        }
    }
}

/// Determine the MOK enrolment plan for the current installation.
///
/// - `nvidia_variant`: whether an NVIDIA variant was selected.
/// - `secure_boot`: the current Secure Boot state.
/// - `already_enrolled`: whether the ARKON cert is already in MokList.
pub fn plan_mok_enrolment(
    nvidia_variant: bool,
    secure_boot: &SecureBootState,
    already_enrolled: bool,
) -> MokEnrolmentPlan {
    if !nvidia_variant {
        return MokEnrolmentPlan::not_required();
    }
    if !secure_boot.is_on() {
        return MokEnrolmentPlan::sb_off();
    }
    if already_enrolled {
        return MokEnrolmentPlan::already_enrolled();
    }
    MokEnrolmentPlan::queued("/usr/share/arkon/arkon-module-signing.cer")
}
