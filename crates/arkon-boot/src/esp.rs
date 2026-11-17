use serde::{Deserialize, Serialize};
use crate::provider::BootProvider;

/// A required file on the EFI System Partition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EspFile {
    /// `EFI/fedora/shimx64.efi` — the signed shim; first loader in the UEFI chain.
    Shim,
    /// `EFI/fedora/grubx64.efi` — GRUB2 loader, loaded by shim.
    Grub,
    /// `EFI/BOOT/BOOTX64.EFI` — the fallback loader (shim copy); required for
    /// "boot from disk" firmware menu entries and for firmware that loses entries.
    FallbackLoader,
}

impl EspFile {
    /// Path relative to the ESP root, using forward slashes.
    pub fn path(&self) -> &'static str {
        match self {
            EspFile::Shim => "EFI/fedora/shimx64.efi",
            EspFile::Grub => "EFI/fedora/grubx64.efi",
            EspFile::FallbackLoader => "EFI/BOOT/BOOTX64.EFI",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            EspFile::Shim => "Fedora shim (UEFI Secure Boot entry point)",
            EspFile::Grub => "GRUB2 bootloader",
            EspFile::FallbackLoader => "Fallback bootloader (EFI/BOOT/BOOTX64.EFI)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EspVerdict {
    /// All required files are present.
    Ok,
    /// One or more required files are missing.
    Incomplete { missing_count: usize },
}

impl EspVerdict {
    pub fn is_ok(&self) -> bool {
        matches!(self, EspVerdict::Ok)
    }
}

/// Result of verifying the contents of an EFI System Partition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EspVerification {
    /// GPT PARTUUID of the ESP that was checked.
    pub esp_uuid: String,
    pub present: Vec<EspFile>,
    pub missing: Vec<EspFile>,
    pub verdict: EspVerdict,
}

/// Check whether the ESP (accessible via `provider`) contains the files
/// required for the GRUB2+shim boot chain (UEFI_BOOT_DESIGN.md §6).
pub fn verify_esp(provider: &dyn BootProvider, esp_uuid: impl Into<String>) -> EspVerification {
    let required = [EspFile::Shim, EspFile::Grub, EspFile::FallbackLoader];
    let mut present = Vec::new();
    let mut missing = Vec::new();

    for file in &required {
        if provider.esp_file_exists(file.path()) {
            present.push(file.clone());
        } else {
            missing.push(file.clone());
        }
    }

    let verdict = if missing.is_empty() {
        EspVerdict::Ok
    } else {
        EspVerdict::Incomplete { missing_count: missing.len() }
    };

    EspVerification { esp_uuid: esp_uuid.into(), present, missing, verdict }
}
