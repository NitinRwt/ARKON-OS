use serde::{Deserialize, Serialize};

// ── Graphics driver ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GraphicsDriverStatus {
    /// A kernel DRM driver is loaded.
    Loaded { driver_name: String },
    /// No DRM driver could be found (vga text mode only).
    Missing,
    /// sysfs is unreadable on this platform.
    Unknown,
}

impl GraphicsDriverStatus {
    pub fn is_ok(&self) -> bool { matches!(self, Self::Loaded { .. }) }
    pub fn summary(&self) -> &'static str {
        match self {
            Self::Loaded { .. } => "Graphics driver loaded",
            Self::Missing       => "No DRM graphics driver found",
            Self::Unknown       => "Graphics driver status unknown",
        }
    }
}

// ── NVIDIA GPU ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NvidiaStatus {
    /// No NVIDIA PCI device present — check not applicable.
    NotPresent,
    /// NVIDIA device found and the kernel module is loaded.
    ModuleLoaded { generation: String, module_name: String },
    /// NVIDIA device found but the kernel module failed to load.
    ModuleFailed { generation: String, error: String },
    /// NVIDIA device present but its generation is unrecognised.
    UnknownGeneration { pci_id: String },
}

impl NvidiaStatus {
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::NotPresent | Self::ModuleLoaded { .. })
    }
    pub fn summary(&self) -> &'static str {
        match self {
            Self::NotPresent              => "No NVIDIA GPU present",
            Self::ModuleLoaded { .. }     => "NVIDIA kernel module loaded",
            Self::ModuleFailed { .. }     => "NVIDIA kernel module failed to load",
            Self::UnknownGeneration { .. } => "NVIDIA GPU generation unrecognised",
        }
    }
}

// ── Vulkan ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VulkanStatus {
    /// One or more Vulkan-capable devices enumerated.
    Available { device_count: usize },
    /// Vulkan loader is present but no devices were found.
    NoDevices,
    /// Vulkan loader is not installed.
    LoaderMissing,
}

impl VulkanStatus {
    pub fn is_ok(&self) -> bool { matches!(self, Self::Available { device_count } if *device_count > 0) }
    pub fn summary(&self) -> &'static str {
        match self {
            Self::Available { .. }  => "Vulkan available",
            Self::NoDevices         => "Vulkan loader present but no devices found",
            Self::LoaderMissing     => "Vulkan loader not installed",
        }
    }
}

// ── Audio ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AudioStatus {
    /// PipeWire (or PulseAudio) is running and at least one sink is present.
    Running,
    /// The audio service is installed but not running.
    NotRunning,
    /// No audio sink detected.
    NoSink,
    /// Audio service not installed.
    Missing,
}

impl AudioStatus {
    pub fn is_ok(&self) -> bool { matches!(self, Self::Running) }
    pub fn summary(&self) -> &'static str {
        match self {
            Self::Running    => "Audio service running",
            Self::NotRunning => "Audio service not running",
            Self::NoSink     => "No audio output device found",
            Self::Missing    => "Audio service not installed",
        }
    }
}

// ── System update configuration ───────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UpdateConfigStatus {
    /// An update policy file is present and readable.
    Valid { policy: String },
    /// The update policy file is absent.
    Missing,
    /// The update policy file is present but cannot be parsed.
    Malformed { detail: String },
}

impl UpdateConfigStatus {
    pub fn is_ok(&self) -> bool { matches!(self, Self::Valid { .. }) }
    pub fn summary(&self) -> &'static str {
        match self {
            Self::Valid { .. }     => "Update configuration valid",
            Self::Missing          => "Update configuration missing",
            Self::Malformed { .. } => "Update configuration malformed",
        }
    }
}

// ── Boot deployment ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BootDeploymentStatus {
    /// The expected bootc deployment is present.
    Valid { image_ref: String, deployment_index: u32 },
    /// No deployment could be found.
    Missing,
    /// The deployed image reference does not match the expected value.
    Mismatch { expected: String, found: String },
    /// bootc status could not be queried.
    QueryFailed { detail: String },
}

impl BootDeploymentStatus {
    pub fn is_ok(&self) -> bool { matches!(self, Self::Valid { .. }) }
    pub fn summary(&self) -> &'static str {
        match self {
            Self::Valid { .. }       => "Boot deployment verified",
            Self::Missing            => "No boot deployment found",
            Self::Mismatch { .. }    => "Boot deployment image reference mismatch",
            Self::QueryFailed { .. } => "Failed to query boot deployment",
        }
    }
}
