/// CPU vendor and model detection via /proc/cpuinfo.
use crate::error::{ProbeError, ProbeResult};
use crate::provider::SysfsProvider;
use serde::{Deserialize, Serialize};
use std::path::Path;

const CPUINFO: &str = "/proc/cpuinfo";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CpuVendor {
    Amd,
    Intel,
    #[serde(other)]
    Unknown,
}

impl CpuVendor {
    fn from_id(id: &str) -> Self {
        match id {
            "AuthenticAMD" => Self::Amd,
            "GenuineIntel" => Self::Intel,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    X86_64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub vendor: CpuVendor,
    /// Raw model name string from /proc/cpuinfo, e.g. "AMD Ryzen 5 4600H …"
    pub model_name: String,
    pub architecture: Architecture,
    /// Physical core count (deduplicated by core_id + physical id).
    pub physical_cores: Option<u32>,
    /// Logical thread count.
    pub logical_threads: u32,
    /// True when the x86-64-v2 baseline (SSE4.2 + POPCNT + CMPXCHG16B + …) is
    /// advertised by the firmware.  Fedora requires v2 since F40.
    pub meets_baseline: bool,
}

pub fn detect(provider: &dyn SysfsProvider) -> ProbeResult<CpuInfo> {
    let raw = provider.read_file(Path::new(CPUINFO))?;
    parse_cpuinfo(&raw)
}

fn parse_cpuinfo(raw: &str) -> ProbeResult<CpuInfo> {
    let mut vendor_id: Option<String> = None;
    let mut model_name: Option<String> = None;
    let mut flags: Option<String> = None;
    let mut logical_count: u32 = 0;
    // Track unique (physical id, core id) pairs to count physical cores.
    let mut physical_cores: std::collections::HashSet<(String, String)> = Default::default();
    let mut current_physical_id = String::new();
    let mut current_core_id = String::new();

    for line in raw.lines() {
        let Some((key, value)) = split_cpuinfo_line(line) else {
            // Blank line marks end of a processor block; commit the core pair.
            if !current_physical_id.is_empty() || !current_core_id.is_empty() {
                physical_cores.insert((
                    current_physical_id.clone(),
                    current_core_id.clone(),
                ));
                current_physical_id.clear();
                current_core_id.clear();
            }
            continue;
        };
        match key {
            "vendor_id" => {
                if vendor_id.is_none() {
                    vendor_id = Some(value.to_owned());
                }
            }
            "model name" => {
                if model_name.is_none() {
                    model_name = Some(value.to_owned());
                }
            }
            "flags" | "Features" => {
                if flags.is_none() {
                    flags = Some(value.to_owned());
                }
            }
            "processor" => {
                logical_count += 1;
            }
            "physical id" => {
                current_physical_id = value.to_owned();
            }
            "core id" => {
                current_core_id = value.to_owned();
            }
            _ => {}
        }
    }
    // Commit the last block if the file doesn't end with a blank line.
    if !current_physical_id.is_empty() || !current_core_id.is_empty() {
        physical_cores.insert((current_physical_id, current_core_id));
    }

    let vendor_str = vendor_id.as_deref().unwrap_or("");
    let vendor = CpuVendor::from_id(vendor_str);

    let model_name = model_name.ok_or_else(|| {
        ProbeError::parse(CPUINFO, "missing 'model name' field")
    })?;

    if model_name.trim().is_empty() {
        return Err(ProbeError::validation("CPU model name is empty"));
    }

    if logical_count == 0 {
        return Err(ProbeError::parse(CPUINFO, "no 'processor' entries found"));
    }

    let meets_baseline = flags.as_deref().map(check_x86_64_v2).unwrap_or(false);

    Ok(CpuInfo {
        vendor,
        model_name,
        architecture: Architecture::X86_64,
        physical_cores: if physical_cores.is_empty() {
            None
        } else {
            Some(physical_cores.len() as u32)
        },
        logical_threads: logical_count,
        meets_baseline,
    })
}

fn split_cpuinfo_line(line: &str) -> Option<(&str, &str)> {
    let pos = line.find(':')?;
    let key = line[..pos].trim();
    let value = line[pos + 1..].trim();
    if key.is_empty() {
        None
    } else {
        Some((key, value))
    }
}

/// Check that the flags string contains the x86-64-v2 required features.
/// Reference: https://fedoraproject.org/wiki/Changes/x86-64-v2
fn check_x86_64_v2(flags: &str) -> bool {
    let required = [
        "cx16", "lahf_lm", "popcnt", "sse4_1", "sse4_2", "ssse3",
    ];
    required.iter().all(|f| {
        flags.split_whitespace().any(|flag| flag == *f)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const AMD_CPUINFO: &str = "\
processor\t: 0
vendor_id\t: AuthenticAMD
cpu family\t: 23
model\t\t: 96
model name\t: AMD Ryzen 5 4600H with Radeon Graphics
stepping\t: 1
physical id\t: 0
siblings\t: 12
core id\t\t: 0
cpu cores\t: 6
flags\t\t: fpu vme de pse tsc cx16 lahf_lm popcnt sse4_1 sse4_2 ssse3

processor\t: 1
vendor_id\t: AuthenticAMD
cpu family\t: 23
model\t\t: 96
model name\t: AMD Ryzen 5 4600H with Radeon Graphics
stepping\t: 1
physical id\t: 0
siblings\t: 12
core id\t\t: 1
cpu cores\t: 6
flags\t\t: fpu vme de pse tsc cx16 lahf_lm popcnt sse4_1 sse4_2 ssse3

";

    const INTEL_CPUINFO: &str = "\
processor\t: 0
vendor_id\t: GenuineIntel
model name\t: Intel(R) Core(TM) i7-10750H CPU @ 2.60GHz
physical id\t: 0
core id\t\t: 0
flags\t\t: fpu vme cx16 lahf_lm popcnt sse4_1 sse4_2 ssse3

";

    #[test]
    fn parses_amd_vendor() {
        let info = parse_cpuinfo(AMD_CPUINFO).unwrap();
        assert_eq!(info.vendor, CpuVendor::Amd);
        assert!(info.model_name.contains("Ryzen 5 4600H"));
        assert_eq!(info.logical_threads, 2);
        assert_eq!(info.physical_cores, Some(2));
        assert!(info.meets_baseline);
    }

    #[test]
    fn parses_intel_vendor() {
        let info = parse_cpuinfo(INTEL_CPUINFO).unwrap();
        assert_eq!(info.vendor, CpuVendor::Intel);
        assert!(info.model_name.contains("i7-10750H"));
    }

    #[test]
    fn missing_model_name_is_error() {
        let raw = "processor\t: 0\nvendor_id\t: AuthenticAMD\n";
        let err = parse_cpuinfo(raw).unwrap_err();
        assert!(err.to_string().contains("model name"));
    }

    #[test]
    fn empty_model_name_is_error() {
        let raw = "processor\t: 0\nvendor_id\t: AuthenticAMD\nmodel name\t:\n";
        let err = parse_cpuinfo(raw).unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn no_processor_entries_is_error() {
        let raw = "vendor_id\t: AuthenticAMD\nmodel name\t: Foo\n";
        let err = parse_cpuinfo(raw).unwrap_err();
        assert!(err.to_string().contains("processor"));
    }

    #[test]
    fn baseline_check_requires_all_flags() {
        assert!(check_x86_64_v2("cx16 lahf_lm popcnt sse4_1 sse4_2 ssse3"));
        assert!(!check_x86_64_v2("cx16 lahf_lm popcnt sse4_1 ssse3")); // missing sse4_2
    }
}
