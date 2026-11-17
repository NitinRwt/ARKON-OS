/// Firmware / UEFI detection.
///
/// Checks /sys/firmware/efi/ to confirm UEFI mode; reads /sys/firmware/efi/vars/
/// for Secure Boot state without requiring mokutil or efibootmgr.
use crate::error::ProbeResult;
use crate::provider::SysfsProvider;
use serde::{Deserialize, Serialize};
use std::path::Path;

const SYS_EFI: &str             = "/sys/firmware/efi";
const EFI_VARS: &str            = "/sys/firmware/efi/efivars";
const EFIBITS: &str             = "/sys/firmware/efi/fw_platform_size";

/// A UUID-based EFI variable name matching pattern for SecureBoot:
/// SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c
const SECUREBOOT_VAR: &str =
    "SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c";
const SETUPMODE_VAR: &str =
    "SetupMode-8be4df61-93ca-11d2-aa0d-00e098032b8c";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirmwareMode {
    Uefi,
    /// Legacy BIOS — the installer blocks here (Preflight step).
    LegacyBios,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureBootState {
    /// True when Secure Boot enforcement is currently active.
    pub enabled: bool,
    /// True when the firmware is in Setup Mode (key database writable).
    pub setup_mode: bool,
    /// Whether the ARKON MOK certificate is enrolled.  This is determined
    /// later by mokutil; here we always return None (requires root to read the
    /// MOK list).
    pub arkon_cert_enrolled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareInfo {
    pub mode: FirmwareMode,
    /// EFI bitness (32 or 64).  Present only in UEFI mode.
    pub bits: Option<u32>,
    pub secure_boot: Option<SecureBootState>,
}

pub fn detect(provider: &dyn SysfsProvider) -> ProbeResult<FirmwareInfo> {
    if !provider.exists(Path::new(SYS_EFI)) {
        return Ok(FirmwareInfo {
            mode: FirmwareMode::LegacyBios,
            bits: None,
            secure_boot: None,
        });
    }

    let bits = provider
        .read_file(Path::new(EFIBITS))
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok());

    let secure_boot = read_secure_boot(provider);

    Ok(FirmwareInfo {
        mode: FirmwareMode::Uefi,
        bits,
        secure_boot: Some(secure_boot),
    })
}

fn read_secure_boot(provider: &dyn SysfsProvider) -> SecureBootState {
    let sb_path = format!("{}/{}", EFI_VARS, SECUREBOOT_VAR);
    let sm_path = format!("{}/{}", EFI_VARS, SETUPMODE_VAR);

    // EFI variable files are binary: 4-byte attribute header + data bytes.
    // For SecureBoot and SetupMode the data is a single UINT8.
    // We read them as a file and check the last byte (index 4).
    let enabled = read_efivar_bool(provider, &sb_path);
    let setup_mode = read_efivar_bool(provider, &sm_path);

    SecureBootState {
        enabled,
        setup_mode,
        arkon_cert_enrolled: None,
    }
}

/// Read the first data byte of an efivars file (byte index 4, after the 4-byte
/// attribute prefix) and return true when it is 0x01.
///
/// For testing, the MockProvider stores plain text "1" or "0" because
/// raw binary blobs are inconvenient in test fixtures; the real provider reads
/// raw bytes.  We distinguish by file length: if it looks like a binary
/// efivars entry (> 1 byte) we index byte 4; if it is "0" or "1" we parse
/// directly.
fn read_efivar_bool(provider: &dyn SysfsProvider, path: &str) -> bool {
    provider
        .read_file(Path::new(path))
        .map(|content| {
            let trimmed = content.trim();
            // Test/mock path: plain "0" or "1"
            if trimmed == "1" {
                return true;
            }
            if trimmed == "0" {
                return false;
            }
            // Binary path: look for byte value 1 anywhere in the string
            // (after the first four attribute bytes).  Since SysfsProvider
            // returns a String, we check if any character has value 1.
            content.chars().skip(4).next().map(|c| c as u8 == 1).unwrap_or(false)
        })
        .unwrap_or(false)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::MockProvider;

    #[test]
    fn legacy_bios_when_efi_dir_absent() {
        let p = MockProvider::new();
        let info = detect(&p).unwrap();
        assert_eq!(info.mode, FirmwareMode::LegacyBios);
        assert!(info.secure_boot.is_none());
    }

    #[test]
    fn uefi_mode_detected() {
        let mut p = MockProvider::new();
        // Add a file inside /sys/firmware/efi so the directory appears to exist
        p.add_file(format!("{}/fw_platform_size", SYS_EFI), "64");
        let info = detect(&p).unwrap();
        assert_eq!(info.mode, FirmwareMode::Uefi);
        assert_eq!(info.bits, Some(64));
    }

    #[test]
    fn secure_boot_enabled_reported() {
        let mut p = MockProvider::new();
        p.add_file(format!("{}/fw_platform_size", SYS_EFI), "64");
        p.add_file(
            format!("{}/{}", EFI_VARS, SECUREBOOT_VAR),
            "1",
        );
        p.add_file(
            format!("{}/{}", EFI_VARS, SETUPMODE_VAR),
            "0",
        );
        let info = detect(&p).unwrap();
        let sb = info.secure_boot.unwrap();
        assert!(sb.enabled);
        assert!(!sb.setup_mode);
    }

    #[test]
    fn secure_boot_disabled_reported() {
        let mut p = MockProvider::new();
        p.add_file(format!("{}/fw_platform_size", SYS_EFI), "64");
        p.add_file(
            format!("{}/{}", EFI_VARS, SECUREBOOT_VAR),
            "0",
        );
        p.add_file(
            format!("{}/{}", EFI_VARS, SETUPMODE_VAR),
            "0",
        );
        let info = detect(&p).unwrap();
        let sb = info.secure_boot.unwrap();
        assert!(!sb.enabled);
    }

    #[test]
    fn setup_mode_detected() {
        let mut p = MockProvider::new();
        p.add_file(format!("{}/fw_platform_size", SYS_EFI), "64");
        p.add_file(format!("{}/{}", EFI_VARS, SECUREBOOT_VAR), "0");
        p.add_file(format!("{}/{}", EFI_VARS, SETUPMODE_VAR), "1");
        let info = detect(&p).unwrap();
        assert!(info.secure_boot.unwrap().setup_mode);
    }

    #[test]
    fn arkon_cert_enrolled_always_none_here() {
        let mut p = MockProvider::new();
        p.add_file(format!("{}/fw_platform_size", SYS_EFI), "64");
        let info = detect(&p).unwrap();
        assert!(info.secure_boot.unwrap().arkon_cert_enrolled.is_none());
    }
}
