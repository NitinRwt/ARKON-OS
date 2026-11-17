use serde::{Deserialize, Serialize};
use crate::config::{Language, KeyboardLayout};
use crate::progress::{InstallProgress, InstallFailure};

/// Network connectivity status for screen 4.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NetworkStatus {
    Connected { ssid: Option<String> },
    Disconnected,
    Checking,
}

/// Hardware requirement check result for screen 5.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardwareCheck {
    pub label: String,
    pub passed: bool,
    pub detail: Option<String>,
}

/// One detected operating system shown on storage and partition screens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedOs {
    pub kind: String,
    pub name: String,
    pub disk_id: String,
}

/// A storage disk candidate shown on screen 7.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskSummary {
    pub id: String,
    pub model: String,
    pub serial_tail: String,
    pub size_bytes: u64,
    pub eligible_modes: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub detected_os: Vec<DetectedOs>,
}

/// Installation mode for screen 6 (profile) and screen 7 (storage).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InstallMode {
    EraseDisk,
    FreeSpace,
    DualBoot,
    ReinstallKeepHome,
}

impl InstallMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::EraseDisk        => "Clean install — erase entire disk",
            Self::FreeSpace        => "Install alongside existing OS",
            Self::DualBoot         => "Dual boot — keep Windows",
            Self::ReinstallKeepHome => "Reinstall, keep home partition",
        }
    }

    pub fn is_destructive(&self) -> bool {
        matches!(self, Self::EraseDisk | Self::ReinstallKeepHome)
    }
}

/// A partition row shown on the partition configuration screen (screen 8).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionRow {
    pub number: u32,
    pub label: String,
    pub size_bytes: u64,
    pub fs_type: String,
    pub role: String,
    pub is_new: bool,
}

/// Per-screen typed state.  Each variant carries exactly the data its screen needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Screen {
    /// Screen 1 — Welcome.
    Welcome {
        arkon_version: String,
        tagline: String,
    },

    /// Screen 2 — Language selection.
    Language {
        available: Vec<Language>,
        selected: Option<String>,
    },

    /// Screen 3 — Keyboard layout.
    Keyboard {
        available: Vec<KeyboardLayout>,
        selected: Option<String>,
        test_input: String,
    },

    /// Screen 4 — Network status.
    Network {
        status: NetworkStatus,
        /// Whether network is required (online install) or optional (offline).
        required: bool,
    },

    /// Screen 5 — Hardware compatibility.
    HardwareCompatibility {
        checks: Vec<HardwareCheck>,
        recommended_variant: String,
        all_passed: bool,
        warnings: Vec<String>,
    },

    /// Screen 6 — Installation profile.
    InstallationProfile {
        available_modes: Vec<InstallMode>,
        selected_mode: Option<InstallMode>,
        encryption_available: bool,
        encryption_enabled: bool,
    },

    /// Screen 7 — Storage selection.
    StorageSelection {
        disks: Vec<DiskSummary>,
        selected_disk_id: Option<String>,
        mode: Option<InstallMode>,
        /// True if any candidate disk has BitLocker.
        bitlocker_present: bool,
        bitlocker_ack: bool,
    },

    /// Screen 8 — Partition configuration.
    PartitionConfiguration {
        disk_model: String,
        disk_id: String,
        partitions: Vec<PartitionRow>,
        /// Whether this is a dual-boot layout.
        dual_boot: bool,
        /// Total bytes assigned to ARKON.
        arkon_bytes: u64,
        encryption_enabled: bool,
    },

    /// Screen 9 — Installation summary.
    Summary {
        disk_model: String,
        disk_id: String,
        mode: InstallMode,
        partitions: Vec<PartitionRow>,
        variant: String,
        language: String,
        keyboard: String,
        username: String,
        hostname: String,
        encryption_enabled: bool,
        /// True if any Windows OS will be preserved.
        windows_preserved: bool,
        /// Confirmation phrase for destructive operations.
        confirmation_phrase: Option<String>,
    },

    /// Screen 10 — Destructive operation confirmation.
    DestructiveConfirmation {
        /// The phrase the user must type, derived from disk model and serial.
        required_phrase: String,
        typed_phrase: String,
        bitlocker_ack_required: bool,
        bitlocker_acked: bool,
    },

    /// Screen 11 — Installation progress.
    Progress {
        progress: InstallProgress,
        /// True once X3 has started — cancel is no longer available.
        past_point_of_no_return: bool,
        elapsed_seconds: u64,
    },

    /// Screen 12 — Error and recovery.
    Error {
        failure: InstallFailure,
        /// Log excerpt (last N lines).
        log_tail: Vec<String>,
        can_cleanup: bool,
        can_rescan: bool,
        can_reboot: bool,
    },

    /// Screen 13 — Completion and reboot.
    Complete {
        variant: String,
        mok_enrolment_required: bool,
        mok_instructions: Option<String>,
    },
}

impl Screen {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Welcome { .. }                  => "welcome",
            Self::Language { .. }                 => "language",
            Self::Keyboard { .. }                 => "keyboard",
            Self::Network { .. }                  => "network",
            Self::HardwareCompatibility { .. }    => "hardware-compatibility",
            Self::InstallationProfile { .. }      => "installation-profile",
            Self::StorageSelection { .. }         => "storage-selection",
            Self::PartitionConfiguration { .. }   => "partition-configuration",
            Self::Summary { .. }                  => "summary",
            Self::DestructiveConfirmation { .. }  => "destructive-confirmation",
            Self::Progress { .. }                 => "progress",
            Self::Error { .. }                    => "error",
            Self::Complete { .. }                 => "complete",
        }
    }

    /// Whether the Next button should be enabled.
    pub fn can_proceed(&self) -> bool {
        match self {
            Self::Welcome { .. }   => true,
            Self::Language { selected, .. } => selected.is_some(),
            Self::Keyboard { selected, .. } => selected.is_some(),
            Self::Network { status, required } => {
                if *required { matches!(status, NetworkStatus::Connected { .. }) } else { true }
            }
            Self::HardwareCompatibility { all_passed, .. } => *all_passed,
            Self::InstallationProfile { selected_mode, .. } => selected_mode.is_some(),
            Self::StorageSelection { selected_disk_id, mode, bitlocker_present, bitlocker_ack, .. } => {
                selected_disk_id.is_some()
                    && mode.is_some()
                    && (!bitlocker_present || *bitlocker_ack)
            }
            Self::PartitionConfiguration { .. } => true,
            Self::Summary { .. } => true,
            Self::DestructiveConfirmation {
                required_phrase, typed_phrase,
                bitlocker_ack_required, bitlocker_acked
            } => {
                typed_phrase == required_phrase
                    && (!bitlocker_ack_required || *bitlocker_acked)
            }
            Self::Progress { .. } => false,
            Self::Error { .. }    => false,
            Self::Complete { .. } => true,
        }
    }
}
