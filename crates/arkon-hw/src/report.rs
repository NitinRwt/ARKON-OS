/// Hardware report assembly and image variant recommendation.
///
/// The variant rule follows the design decision in PHASE2_ARCHITECTURE.md §1:
///   - Any NVIDIA Ampere or newer GPU → arkon-nvidia (open modules)
///   - Any NVIDIA Turing GPU          → arkon-nvidia-turing (closed modules)
///   - Any older/unknown NVIDIA GPU   → arkon-nvidia-turing as a conservative choice
///   - AMD or Intel GPU only, or no GPU → arkon (Mesa)
use crate::cpu::CpuInfo;
use crate::firmware::FirmwareInfo;
use crate::gpu::{GpuClass, GpuInfo, GpuVendor, NvidiaGeneration};
use crate::storage::StorageDevice;
use serde::{Deserialize, Serialize};

/// The bootc image variant the hardware requires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    /// Mesa drivers; no proprietary NVIDIA components.
    Arkon,
    /// NVIDIA open kernel modules (Ampere / Ada Lovelace; RTX 30+).
    ArkonNvidia,
    /// NVIDIA closed proprietary kernel module (Turing / RTX 20 / GTX 16xx
    /// and all pre-Turing NVIDIA hardware).
    ArkonNvidiaTuring,
}

impl std::fmt::Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Arkon            => f.write_str("arkon"),
            Self::ArkonNvidia      => f.write_str("arkon-nvidia"),
            Self::ArkonNvidiaTuring => f.write_str("arkon-nvidia-turing"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareReport {
    pub cpu: CpuInfo,
    pub gpus: Vec<GpuInfo>,
    pub storage: Vec<StorageDevice>,
    pub firmware: FirmwareInfo,
    pub recommended_variant: Variant,
    /// Non-fatal conditions the installer UI should surface.
    pub warnings: Vec<HardwareWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareWarning {
    pub code: WarningCode,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WarningCode {
    /// x86-64-v2 baseline not advertised; Fedora 40+ may not boot.
    CpuBelowBaseline,
    /// Legacy BIOS mode — the installer cannot proceed.
    LegacyBios,
    /// No display controller found (headless server?).
    NoGpuDetected,
    /// NVIDIA Turing GPU found; closed driver required; open module flag is off.
    NvidiaTuringClosedDriver,
    /// An NVIDIA GPU with an unrecognised device ID was found.
    NvidiaUnknownGeneration,
    /// No NVMe or SATA disk found (VM? unusual hardware path).
    NoStorageDetected,
    /// A LUKS-encrypted disk is present; installer will warn before writing.
    EncryptedDiskPresent,
}

pub fn build_report(
    cpu: CpuInfo,
    gpus: Vec<GpuInfo>,
    storage: Vec<StorageDevice>,
    firmware: FirmwareInfo,
) -> HardwareReport {
    let mut warnings = Vec::new();

    // CPU baseline
    if !cpu.meets_baseline {
        warnings.push(HardwareWarning {
            code: WarningCode::CpuBelowBaseline,
            detail: format!(
                "CPU '{}' does not advertise all x86-64-v2 flags required by Fedora 40+",
                cpu.model_name
            ),
        });
    }

    // Firmware mode
    if firmware.mode == crate::firmware::FirmwareMode::LegacyBios {
        warnings.push(HardwareWarning {
            code: WarningCode::LegacyBios,
            detail: "Machine booted in legacy BIOS mode; UEFI is required".to_owned(),
        });
    }

    // GPU
    if gpus.is_empty() {
        warnings.push(HardwareWarning {
            code: WarningCode::NoGpuDetected,
            detail: "No PCI display controller found".to_owned(),
        });
    }

    for gpu in &gpus {
        match &gpu.class {
            GpuClass::NvidiaDiscrete { generation, open_modules_supported: false } => {
                match generation {
                    NvidiaGeneration::Turing => warnings.push(HardwareWarning {
                        code: WarningCode::NvidiaTuringClosedDriver,
                        detail: format!(
                            "GPU {} ({:04x}:{:04x}) is NVIDIA Turing; \
                             the closed proprietary module is required for power management",
                            gpu.model_name.as_deref().unwrap_or("unknown"),
                            gpu.vendor_id,
                            gpu.device_id,
                        ),
                    }),
                    NvidiaGeneration::Other => warnings.push(HardwareWarning {
                        code: WarningCode::NvidiaUnknownGeneration,
                        detail: format!(
                            "NVIDIA GPU {:04x}:{:04x} has an unrecognised device ID; \
                             defaulting to the Turing variant (closed modules)",
                            gpu.vendor_id, gpu.device_id,
                        ),
                    }),
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // Storage
    if storage.is_empty() {
        warnings.push(HardwareWarning {
            code: WarningCode::NoStorageDetected,
            detail: "No NVMe or SATA storage device detected".to_owned(),
        });
    }
    let has_luks = storage.iter().flat_map(|d| &d.partitions).any(|p| {
        p.class == crate::storage::PartitionClass::Luks
    });
    if has_luks {
        warnings.push(HardwareWarning {
            code: WarningCode::EncryptedDiskPresent,
            detail: "One or more LUKS-encrypted partitions are present; \
                     the installer will require acknowledgement before writing"
                .to_owned(),
        });
    }

    let recommended_variant = recommend_variant(&gpus);

    HardwareReport {
        cpu,
        gpus,
        storage,
        firmware,
        recommended_variant,
        warnings,
    }
}

fn recommend_variant(gpus: &[GpuInfo]) -> Variant {
    for gpu in gpus {
        if gpu.vendor != GpuVendor::Nvidia {
            continue;
        }
        return match &gpu.class {
            GpuClass::NvidiaDiscrete { generation, .. } => match generation {
                NvidiaGeneration::Ampere | NvidiaGeneration::AdaLovelace => Variant::ArkonNvidia,
                // Turing and anything unrecognised → closed module variant
                _ => Variant::ArkonNvidiaTuring,
            },
            _ => Variant::ArkonNvidiaTuring,
        };
    }
    // No NVIDIA GPU found
    Variant::Arkon
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::{Architecture, CpuVendor};
    use crate::firmware::{FirmwareMode, SecureBootState};
    use crate::gpu::{GpuClass, NvidiaGeneration};

    fn dummy_cpu(meets_baseline: bool) -> CpuInfo {
        CpuInfo {
            vendor: CpuVendor::Amd,
            model_name: "AMD Ryzen 5 4600H".to_owned(),
            architecture: Architecture::X86_64,
            physical_cores: Some(6),
            logical_threads: 12,
            meets_baseline,
        }
    }

    fn uefi_firmware() -> FirmwareInfo {
        FirmwareInfo {
            mode: FirmwareMode::Uefi,
            bits: Some(64),
            secure_boot: Some(SecureBootState {
                enabled: true,
                setup_mode: false,
                arkon_cert_enrolled: None,
            }),
        }
    }

    fn nvidia_gpu(generation: NvidiaGeneration, open: bool) -> GpuInfo {
        GpuInfo {
            pci_address: "0000:01:00.0".to_owned(),
            vendor: GpuVendor::Nvidia,
            vendor_id: 0x10de,
            device_id: 0x1f91,
            class: GpuClass::NvidiaDiscrete {
                generation,
                open_modules_supported: open,
            },
            model_name: Some("GeForce GTX 1650".to_owned()),
            is_boot_gpu: true,
        }
    }

    #[test]
    fn turing_recommends_arkon_nvidia_turing() {
        let r = build_report(
            dummy_cpu(true),
            vec![nvidia_gpu(NvidiaGeneration::Turing, false)],
            vec![],
            uefi_firmware(),
        );
        assert_eq!(r.recommended_variant, Variant::ArkonNvidiaTuring);
        assert!(r.warnings.iter().any(|w| w.code == WarningCode::NvidiaTuringClosedDriver));
    }

    #[test]
    fn ampere_recommends_arkon_nvidia_open() {
        let r = build_report(
            dummy_cpu(true),
            vec![nvidia_gpu(NvidiaGeneration::Ampere, true)],
            vec![],
            uefi_firmware(),
        );
        assert_eq!(r.recommended_variant, Variant::ArkonNvidia);
        assert!(!r.warnings.iter().any(|w| w.code == WarningCode::NvidiaTuringClosedDriver));
    }

    #[test]
    fn no_gpu_recommends_arkon_mesa() {
        let r = build_report(dummy_cpu(true), vec![], vec![], uefi_firmware());
        assert_eq!(r.recommended_variant, Variant::Arkon);
        assert!(r.warnings.iter().any(|w| w.code == WarningCode::NoGpuDetected));
    }

    #[test]
    fn cpu_below_baseline_produces_warning() {
        let r = build_report(dummy_cpu(false), vec![], vec![], uefi_firmware());
        assert!(r.warnings.iter().any(|w| w.code == WarningCode::CpuBelowBaseline));
    }

    #[test]
    fn legacy_bios_produces_warning() {
        let fw = FirmwareInfo {
            mode: FirmwareMode::LegacyBios,
            bits: None,
            secure_boot: None,
        };
        let r = build_report(dummy_cpu(true), vec![], vec![], fw);
        assert!(r.warnings.iter().any(|w| w.code == WarningCode::LegacyBios));
    }
}
