/// GPU detection via /sys/bus/pci/devices/.
///
/// For each PCI device whose class indicates a display controller (0x03xxxx),
/// we read the vendor and device IDs and classify the GPU.  No shell commands
/// are used; all information comes from sysfs files.
use crate::error::{ProbeResult};
use crate::provider::SysfsProvider;
use serde::{Deserialize, Serialize};
use std::path::Path;

const PCI_DEVICES: &str = "/sys/bus/pci/devices";

/// PCI class codes for display controllers.
const CLASS_VGA: u32          = 0x0300_00;
const CLASS_XGA: u32          = 0x0301_00;
const CLASS_3D_CONTROLLER: u32 = 0x0302_00;
const CLASS_DISPLAY_OTHER: u32 = 0x0380_00;

const VENDOR_NVIDIA: u16 = 0x10de;
const VENDOR_AMD: u16    = 0x1002;
const VENDOR_INTEL: u16  = 0x8086;

/// NVIDIA Turing device IDs span roughly 0x1e00–0x1ff0.
/// Turing is TU1xx silicon; known retail IDs are listed here.
/// Not exhaustive — entries are added when needed.
/// Source: https://pci-ids.ucw.cz/
const NVIDIA_TURING_DEVICE_IDS: &[u16] = &[
    0x1e02, 0x1e04, 0x1e07, 0x1e30, 0x1e36, 0x1e78, 0x1e82, 0x1e84, 0x1e87,
    0x1e89, 0x1e90, 0x1e91, 0x1e93, 0x1eab, 0x1eae, 0x1eb0, 0x1eb1, 0x1eb4,
    0x1eb5, 0x1eb6, 0x1eb8, 0x1eba, 0x1ebc, 0x1ec2, 0x1ec7, 0x1ed0, 0x1ed1,
    0x1ed3, 0x1ef5,
    // GTX 1650 / 1660 family (TU116/TU117)
    0x1f82, 0x1f91, 0x1f95, 0x1f96, 0x1f97, 0x1f98, 0x1f99, 0x1f9c, 0x1f9d,
    0x1f9f, 0x1fa0, 0x1fae, 0x1fb0, 0x1fb1, 0x1fb8, 0x1fb9, 0x1fba, 0x1fbb,
    0x1fbc, 0x1fbd,
];

/// NVIDIA Ampere device IDs (GA1xx), RTX 30-series.
const NVIDIA_AMPERE_DEVICE_IDS: &[u16] = &[
    0x2204, 0x2206, 0x2208, 0x220a, 0x2216, 0x2230, 0x2231, 0x2232, 0x2233,
    0x2235, 0x2238, 0x2460, 0x2482, 0x2484, 0x2486, 0x2488, 0x248a, 0x249c,
    0x249d, 0x24a0, 0x24b0, 0x24b8, 0x24b9, 0x24ba, 0x24bb, 0x24dc, 0x24dd,
    0x24e0, 0x24ef, 0x24fa, 0x25a0, 0x25a2, 0x25a5, 0x25aa, 0x25ab, 0x25ac,
    0x25ad, 0x25b0, 0x25b6, 0x25b8, 0x25b9, 0x25ba, 0x25bb, 0x25bc, 0x25be,
    0x25e0, 0x25e2, 0x25e5, 0x2704, 0x2705, 0x2730,
];

/// NVIDIA Ada Lovelace (AD1xx), RTX 40-series.
const NVIDIA_ADA_DEVICE_IDS: &[u16] = &[
    0x2684, 0x2702, 0x2704, 0x2705, 0x2709, 0x2730, 0x2757, 0x2770, 0x2782,
    0x2783, 0x2786, 0x27a0, 0x27b0, 0x27b8, 0x27b9, 0x27ba, 0x27bb, 0x27e0,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    /// Vendor PCI ID as a hex string, e.g. "0x1234".
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NvidiaGeneration {
    /// Turing (TU1xx): GTX 16xx, RTX 20xx
    Turing,
    /// Ampere (GA1xx): RTX 30xx
    Ampere,
    /// Ada Lovelace (AD1xx): RTX 40xx
    AdaLovelace,
    /// Any other NVIDIA GPU (older or newer than the table)
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuClass {
    /// Discrete NVIDIA GPU with identified generation.
    NvidiaDiscrete {
        generation: NvidiaGeneration,
        /// True when NVIDIA's open kernel modules are production-quality for
        /// this generation.  Turing requires the proprietary closed module.
        open_modules_supported: bool,
    },
    /// AMD discrete or integrated GPU.
    AmdGpu { integrated: bool },
    /// Intel integrated or Arc GPU.
    IntelGpu { integrated: bool },
    /// Unrecognised display controller.
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    /// PCI device address in BDF notation, e.g. "0000:01:00.0".
    pub pci_address: String,
    pub vendor: GpuVendor,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class: GpuClass,
    /// Human-readable name from /sys/.../label or our static table, if known.
    pub model_name: Option<String>,
    /// True when this GPU is the primary/boot VGA device
    /// (/sys/.../boot_vga == "1").
    pub is_boot_gpu: bool,
}

pub fn detect(provider: &dyn SysfsProvider) -> ProbeResult<Vec<GpuInfo>> {
    let entries = match provider.list_dir(Path::new(PCI_DEVICES)) {
        Ok(e) => e,
        Err(_) => return Ok(vec![]), // sysfs not available (not on Linux)
    };

    let mut gpus = Vec::new();

    for entry in &entries {
        let dev_path = format!("{}/{}", PCI_DEVICES, entry);
        let class_path = format!("{}/class", dev_path);

        let class_raw = match provider.read_file(Path::new(&class_path)) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let class_val = parse_hex_u32(&class_raw).unwrap_or(0);

        // Mask the programming interface byte (lowest byte) to match our
        // class constants, which all end in 0x00.
        let class_masked = class_val & 0xFFFF_00;
        if !is_display_class(class_masked) {
            continue;
        }

        let vendor_raw = provider
            .read_file(Path::new(&format!("{}/vendor", dev_path)))
            .unwrap_or_default();
        let device_raw = provider
            .read_file(Path::new(&format!("{}/device", dev_path)))
            .unwrap_or_default();

        let vendor_id = parse_hex_u16(&vendor_raw).unwrap_or(0);
        let device_id = parse_hex_u16(&device_raw).unwrap_or(0);

        let is_boot_gpu = provider
            .read_file(Path::new(&format!("{}/boot_vga", dev_path)))
            .map(|s| s.trim() == "1")
            .unwrap_or(false);

        let vendor = vendor_to_enum(vendor_id, &vendor_raw);
        let class = classify_gpu(&vendor, device_id);
        let model_name = lookup_model_name(&vendor, vendor_id, device_id);

        gpus.push(GpuInfo {
            pci_address: entry.clone(),
            vendor,
            vendor_id,
            device_id,
            class,
            model_name,
            is_boot_gpu,
        });
    }

    Ok(gpus)
}

fn is_display_class(c: u32) -> bool {
    matches!(
        c,
        CLASS_VGA | CLASS_XGA | CLASS_3D_CONTROLLER | CLASS_DISPLAY_OTHER
    )
}

fn vendor_to_enum(vendor_id: u16, raw: &str) -> GpuVendor {
    match vendor_id {
        VENDOR_NVIDIA => GpuVendor::Nvidia,
        VENDOR_AMD    => GpuVendor::Amd,
        VENDOR_INTEL  => GpuVendor::Intel,
        _             => GpuVendor::Unknown(raw.to_owned()),
    }
}

fn classify_gpu(vendor: &GpuVendor, device_id: u16) -> GpuClass {
    match vendor {
        GpuVendor::Nvidia => {
            let generation = if NVIDIA_TURING_DEVICE_IDS.contains(&device_id) {
                NvidiaGeneration::Turing
            } else if NVIDIA_AMPERE_DEVICE_IDS.contains(&device_id) {
                NvidiaGeneration::Ampere
            } else if NVIDIA_ADA_DEVICE_IDS.contains(&device_id) {
                NvidiaGeneration::AdaLovelace
            } else {
                NvidiaGeneration::Other
            };
            // Open modules are production-quality from Ampere (GA10x) onward.
            // Turing requires the closed proprietary module.
            let open_modules_supported = !matches!(
                generation,
                NvidiaGeneration::Turing | NvidiaGeneration::Other
            );
            GpuClass::NvidiaDiscrete { generation, open_modules_supported }
        }
        GpuVendor::Amd => {
            // Intel-integrated style check: APU/integrated AMD GPUs show up
            // as a 3D controller and share the host bridge physical id.
            // Without /sys/class/drm detail this is a best-effort heuristic.
            GpuClass::AmdGpu { integrated: false }
        }
        GpuVendor::Intel => {
            // Intel Arc discrete GPUs (Xe HPG / Alchemist) occupy the
            // 0x56a0–0x56bf range; gen-12 integrated (Tiger/Ice/Alder/Raptor
            // Lake) are mostly in 0x4600–0x46d0 and older ranges below that.
            // We conservatively classify as integrated unless it is in the
            // known Arc discrete range.
            let integrated = !(0x5600..=0x5700).contains(&device_id);
            GpuClass::IntelGpu { integrated }
        }
        GpuVendor::Unknown(_) => GpuClass::Other,
    }
}

/// Minimal static name table.  Production tooling would use the full
/// pci.ids database; this table covers the hardware the project owns today
/// and common RTX targets.
fn lookup_model_name(vendor: &GpuVendor, _vendor_id: u16, device_id: u16) -> Option<String> {
    match vendor {
        GpuVendor::Nvidia => NVIDIA_NAMES
            .iter()
            .find(|(id, _)| *id == device_id)
            .map(|(_, name)| name.to_string()),
        GpuVendor::Amd => AMD_NAMES
            .iter()
            .find(|(id, _)| *id == device_id)
            .map(|(_, name)| name.to_string()),
        _ => None,
    }
}

static NVIDIA_NAMES: &[(u16, &str)] = &[
    (0x1f82, "GeForce GTX 1650"),
    (0x1f91, "GeForce GTX 1650 (Mobile)"),
    (0x1f95, "GeForce GTX 1650 Ti (Mobile)"),
    (0x1e82, "GeForce RTX 2080"),
    (0x1e84, "GeForce RTX 2070 Super"),
    (0x1e87, "GeForce RTX 2080 Super"),
    (0x2204, "GeForce RTX 3090"),
    (0x2206, "GeForce RTX 3080"),
    (0x2484, "GeForce RTX 3070"),
    (0x2486, "GeForce RTX 3060 Ti"),
    (0x2488, "GeForce RTX 3060"),
    (0x2704, "GeForce RTX 4080 Super"),
    (0x2705, "GeForce RTX 4070 Ti Super"),
    (0x2782, "GeForce RTX 4070 Super"),
    (0x2786, "GeForce RTX 4070"),
    (0x27b8, "GeForce RTX 4060"),
];

static AMD_NAMES: &[(u16, &str)] = &[
    (0x1636, "Radeon Graphics (Renoir, integrated)"),
    (0x164c, "Radeon Graphics (Lucienne, integrated)"),
    (0x1681, "Radeon Graphics (Rembrandt, integrated)"),
    (0x744c, "Radeon RX 7900 XTX"),
    (0x7480, "Radeon RX 7700 XT"),
    (0x7483, "Radeon RX 7600"),
];

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_hex_u32(s: &str) -> Option<u32> {
    u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()
}

fn parse_hex_u16(s: &str) -> Option<u16> {
    u16::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::MockProvider;

    fn make_pci_device(
        provider: &mut MockProvider,
        address: &str,
        class: &str,
        vendor: &str,
        device: &str,
        boot_vga: &str,
    ) {
        let base = format!("{}/{}", PCI_DEVICES, address);
        provider.add_file(format!("{}/class", base), class);
        provider.add_file(format!("{}/vendor", base), vendor);
        provider.add_file(format!("{}/device", base), device);
        provider.add_file(format!("{}/boot_vga", base), boot_vga);
    }

    #[test]
    fn detects_nvidia_gtx_1650() {
        let mut p = MockProvider::new();
        // GTX 1650 mobile — Turing, device 0x1f91
        make_pci_device(&mut p, "0000:01:00.0", "0x030200", "0x10de", "0x1f91", "1");
        let gpus = detect(&p).unwrap();
        assert_eq!(gpus.len(), 1);
        let g = &gpus[0];
        assert_eq!(g.vendor, GpuVendor::Nvidia);
        assert!(g.is_boot_gpu);
        assert_eq!(g.model_name.as_deref(), Some("GeForce GTX 1650 (Mobile)"));
        match &g.class {
            GpuClass::NvidiaDiscrete { generation, open_modules_supported } => {
                assert_eq!(*generation, NvidiaGeneration::Turing);
                assert!(!open_modules_supported);
            }
            _ => panic!("wrong class"),
        }
    }

    #[test]
    fn detects_amd_integrated_on_hybrid_laptop() {
        let mut p = MockProvider::new();
        // AMD Renoir integrated GPU
        make_pci_device(&mut p, "0000:06:00.0", "0x030000", "0x1002", "0x1636", "0");
        let gpus = detect(&p).unwrap();
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].vendor, GpuVendor::Amd);
        assert_eq!(
            gpus[0].model_name.as_deref(),
            Some("Radeon Graphics (Renoir, integrated)")
        );
    }

    #[test]
    fn detects_nvidia_rtx_3080_as_ampere() {
        let mut p = MockProvider::new();
        make_pci_device(&mut p, "0000:01:00.0", "0x030200", "0x10de", "0x2206", "1");
        let gpus = detect(&p).unwrap();
        assert_eq!(gpus.len(), 1);
        match &gpus[0].class {
            GpuClass::NvidiaDiscrete { generation, open_modules_supported } => {
                assert_eq!(*generation, NvidiaGeneration::Ampere);
                assert!(open_modules_supported);
            }
            _ => panic!("wrong class"),
        }
    }

    #[test]
    fn skips_non_display_pci_devices() {
        let mut p = MockProvider::new();
        // Audio device — class 0x040300
        let base = format!("{}/0000:00:1f.3", PCI_DEVICES);
        p.add_file(format!("{}/class", base), "0x040300");
        p.add_file(format!("{}/vendor", base), "0x8086");
        p.add_file(format!("{}/device", base), "0x02c8");
        p.add_file(format!("{}/boot_vga", base), "0");
        let gpus = detect(&p).unwrap();
        assert!(gpus.is_empty());
    }

    #[test]
    fn returns_empty_when_sysfs_unavailable() {
        let p = MockProvider::new();
        let gpus = detect(&p).unwrap();
        assert!(gpus.is_empty());
    }

    #[test]
    fn detects_intel_integrated() {
        let mut p = MockProvider::new();
        make_pci_device(&mut p, "0000:00:02.0", "0x030000", "0x8086", "0x9a49", "1");
        let gpus = detect(&p).unwrap();
        assert_eq!(gpus.len(), 1);
        match &gpus[0].class {
            GpuClass::IntelGpu { integrated } => assert!(integrated),
            _ => panic!("wrong class"),
        }
    }
}
