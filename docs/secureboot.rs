use serde::{Deserialize, Serialize};

/// Current Secure Boot configuration as reported by firmware variables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecureBootState {
    /// Secure Boot is enabled and enforcing.
    On,
    /// Secure Boot is present but disabled.
    Off,
    /// Setup Mode: Secure Boot is in an unenrolled configuration state.
    SetupMode,
    /// State could not be determined (non-UEFI or variables not readable).
    Unknown,
}

impl SecureBootState {
    pub fn is_on(&self) -> bool {
        matches!(self, SecureBootState::On)
    }
}

/// A recommendation the installer may show to the user.
///
/// **`DisableSecureBoot` is intentionally not a variant.**
///
/// The installer never asks the user to disable Secure Boot and never
/// presents that as a fix (SECURE_BOOT_DESIGN.md §1 and §4; project charter:
/// "no default Secure Boot disabling").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecureBootRecommendation {
    /// No action needed; installation proceeds normally.
    NoAction,
    /// Enrol the ARKON module-signing certificate via MOK.
    /// Required to load the NVIDIA driver on the first boot.
    EnrolArkonCert,
    /// A firmware update is required to support the current Fedora shim.
    FirmwareUpdateRequired { reason: String },
    /// The firmware's third-party certificate trust must be enabled.
    /// This is a firmware configuration change, **not** disabling Secure Boot.
    EnableThirdPartyCertTrust,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureBootReport {
    pub state: SecureBootState,
    /// Whether the firmware trusts Microsoft's third-party UEFI CA.
    /// `None` when not determinable without booting.
    pub microsoft_third_party_trusted: Option<bool>,
    /// Whether the ARKON module-signing certificate is already enrolled in MOK.
    /// `None` when not checked (requires root access via `mokutil`).
    pub arkon_cert_enrolled: Option<bool>,
    pub recommendation: SecureBootRecommendation,
}

impl SecureBootReport {
    fn from_state(state: SecureBootState) -> Self {
        Self {
            state,
            microsoft_third_party_trusted: None,
            arkon_cert_enrolled: None,
            recommendation: SecureBootRecommendation::NoAction,
        }
    }

    fn with_nvidia_check(mut self, needs_mok: bool) -> Self {
        if needs_mok && self.state.is_on() {
            self.recommendation = SecureBootRecommendation::EnrolArkonCert;
        }
        self
    }
}

/// Validate a Secure Boot state for installation.
///
/// Returns a report in all cases.  Installation never fails because of
/// Secure Boot state; the user is informed, not coerced.
pub fn validate_secure_boot(state: SecureBootState, nvidia_variant: bool) -> SecureBootReport {
    SecureBootReport::from_state(state).with_nvidia_check(nvidia_variant)
}
