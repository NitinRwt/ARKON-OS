use serde::{Deserialize, Serialize};

/// The bootloader in use for this installation.
///
/// **systemd-boot is not assumed compatible** with every Fedora bootc
/// deployment configuration (D-003, UEFI_BOOT_DESIGN.md §10).  The installer
/// validates the selection against the actual generated image before proceeding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Bootloader {
    /// GRUB2 with Fedora's shim — the supported boot chain for ARKON 1.0 (D-003).
    Grub2WithShim,
    /// systemd-boot — deferred; not verified for this ARKON release.
    SystemdBoot,
    /// An unrecognised or future bootloader.
    Unknown(String),
}

impl std::fmt::Display for Bootloader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Bootloader::Grub2WithShim => write!(f, "GRUB2+shim"),
            Bootloader::SystemdBoot => write!(f, "systemd-boot"),
            Bootloader::Unknown(s) => write!(f, "unknown ({})", s),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BootloaderVerdict {
    /// Bootloader is known to be compatible with the target image.
    Valid,
    /// Bootloader is not compatible; installation must not proceed.
    Invalid { reason: String },
    /// Compatibility has not been verified against this image.
    ///
    /// A warning is surfaced; the user must acknowledge before proceeding.
    Unverified { reason: String },
}

impl BootloaderVerdict {
    pub fn is_acceptable(&self) -> bool {
        !matches!(self, BootloaderVerdict::Invalid { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BootloaderWarning {
    /// systemd-boot support has not been verified for this release.
    SystemdBootNotVerified,
    /// The bootloader could not be matched against the image manifest.
    ImageManifestNotChecked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootloaderValidation {
    pub selected: Bootloader,
    /// OCI image reference being installed.
    pub image_ref: String,
    pub verdict: BootloaderVerdict,
    pub warnings: Vec<BootloaderWarning>,
}

/// Validate the bootloader selection against the target image.
///
/// For ARKON 1.0 the only supported bootloader is `Grub2WithShim` (D-003).
/// `SystemdBoot` is not refused — a future spike may validate it — but it is
/// flagged as unverified so the user is informed before proceeding.
pub fn validate_bootloader(
    selected: Bootloader,
    image_ref: impl Into<String>,
) -> BootloaderValidation {
    let image_ref = image_ref.into();
    let (verdict, warnings) = match &selected {
        Bootloader::Grub2WithShim => (BootloaderVerdict::Valid, vec![]),
        Bootloader::SystemdBoot => (
            BootloaderVerdict::Unverified {
                reason: "systemd-boot compatibility with Fedora bootc has not been \
                         verified for ARKON 1.0 (D-003, V-18). Use GRUB2+shim."
                    .into(),
            },
            vec![BootloaderWarning::SystemdBootNotVerified],
        ),
        Bootloader::Unknown(name) => (
            BootloaderVerdict::Invalid {
                reason: format!("bootloader '{}' is not recognised", name),
            },
            vec![],
        ),
    };

    BootloaderValidation { selected, image_ref, verdict, warnings }
}
