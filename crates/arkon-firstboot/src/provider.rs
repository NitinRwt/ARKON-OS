use crate::error::{FirstBootError, FirstBootResult};
use crate::handoff::HandoffData;
use crate::health::{
    AudioStatus, BootDeploymentStatus, GraphicsDriverStatus, NvidiaStatus,
    UpdateConfigStatus, VulkanStatus,
};
use crate::state::SetupJournal;

/// Abstracts all host-system interactions for testability.
/// The real implementation calls systemd APIs, /etc files, and sysfs.
/// Never passes arbitrary commands from the caller.
pub trait FirstBootProvider {
    // ── Handoff ──────────────────────────────────────────────────────────────
    fn read_handoff(&self) -> FirstBootResult<HandoffData>;
    fn delete_handoff(&self) -> FirstBootResult<()>;
    fn handoff_exists(&self) -> bool;

    // ── Journal (restart-safe state) ─────────────────────────────────────────
    fn journal_exists(&self) -> bool;
    fn read_journal(&self) -> FirstBootResult<SetupJournal>;
    fn write_journal(&self, journal: &SetupJournal) -> FirstBootResult<()>;

    // ── Account setup ────────────────────────────────────────────────────────
    fn user_exists(&self, username: &str) -> FirstBootResult<bool>;
    /// Create the user. The implementation calls `useradd`/`usermod` via D-Bus
    /// or a fixed helper; it never accepts arbitrary shell commands.
    fn create_user(&self, username: &str, full_name: &str, password_hash: &str, administrator: bool) -> FirstBootResult<()>;

    // ── System configuration ─────────────────────────────────────────────────
    fn hostname_configured(&self) -> FirstBootResult<bool>;
    fn set_hostname(&self, hostname: &str) -> FirstBootResult<()>;
    fn timezone_configured(&self) -> FirstBootResult<bool>;
    fn set_timezone(&self, timezone: &str) -> FirstBootResult<()>;
    fn network_configured(&self) -> FirstBootResult<bool>;
    fn apply_network_config(&self, locale: &str, keymap: &str, language: &str) -> FirstBootResult<()>;

    // ── Health checks (read-only) ────────────────────────────────────────────
    fn check_graphics_driver(&self) -> FirstBootResult<GraphicsDriverStatus>;
    fn check_nvidia_gpu(&self) -> FirstBootResult<NvidiaStatus>;
    fn check_vulkan(&self) -> FirstBootResult<VulkanStatus>;
    fn check_audio(&self) -> FirstBootResult<AudioStatus>;
    fn check_update_config(&self) -> FirstBootResult<UpdateConfigStatus>;
    fn check_boot_deployment(&self, expected_image_ref: &str) -> FirstBootResult<BootDeploymentStatus>;
}

// ── Mock provider ─────────────────────────────────────────────────────────────

/// In-memory mock for integration testing.
#[derive(Debug, Clone)]
pub struct MockProvider {
    pub handoff: Option<HandoffData>,
    pub journal: Option<SetupJournal>,
    pub existing_users: Vec<String>,
    pub hostname_set: Option<String>,
    pub timezone_set: Option<String>,
    pub network_applied: bool,
    pub graphics_driver: GraphicsDriverStatus,
    pub nvidia: NvidiaStatus,
    pub vulkan: VulkanStatus,
    pub audio: AudioStatus,
    pub update_config: UpdateConfigStatus,
    pub boot_deployment: BootDeploymentStatus,
    /// Simulate a journal write failure.
    pub fail_journal_write: bool,
    /// Simulate user creation failure.
    pub fail_user_create: Option<String>,
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            handoff: None,
            journal: None,
            existing_users: vec![],
            hostname_set: None,
            timezone_set: None,
            network_applied: false,
            graphics_driver: GraphicsDriverStatus::Loaded { driver_name: "i915".into() },
            nvidia: NvidiaStatus::NotPresent,
            vulkan: VulkanStatus::Available { device_count: 1 },
            audio: AudioStatus::Running,
            update_config: UpdateConfigStatus::Valid { policy: "atomic".into() },
            boot_deployment: BootDeploymentStatus::Valid { image_ref: "registry.arkonos.com/arkon:latest".into(), deployment_index: 0 },
            fail_journal_write: false,
            fail_user_create: None,
        }
    }

    pub fn with_handoff(mut self, handoff: HandoffData) -> Self {
        self.handoff = Some(handoff);
        self
    }

    pub fn with_existing_user(mut self, username: &str) -> Self {
        self.existing_users.push(username.into());
        self
    }

    pub fn with_nvidia(mut self, status: NvidiaStatus) -> Self {
        self.nvidia = status;
        self
    }

    pub fn with_vulkan(mut self, status: VulkanStatus) -> Self {
        self.vulkan = status;
        self
    }

    pub fn with_graphics_driver(mut self, status: GraphicsDriverStatus) -> Self {
        self.graphics_driver = status;
        self
    }

    pub fn with_audio(mut self, status: AudioStatus) -> Self {
        self.audio = status;
        self
    }

    pub fn with_update_config(mut self, status: UpdateConfigStatus) -> Self {
        self.update_config = status;
        self
    }

    pub fn with_boot_deployment(mut self, status: BootDeploymentStatus) -> Self {
        self.boot_deployment = status;
        self
    }

    pub fn fail_user_create(mut self, reason: &str) -> Self {
        self.fail_user_create = Some(reason.into());
        self
    }
}

impl Default for MockProvider {
    fn default() -> Self { Self::new() }
}

impl FirstBootProvider for MockProvider {
    fn read_handoff(&self) -> FirstBootResult<HandoffData> {
        self.handoff.clone().ok_or(FirstBootError::HandoffAbsent)
    }
    fn delete_handoff(&self) -> FirstBootResult<()> { Ok(()) }
    fn handoff_exists(&self) -> bool { self.handoff.is_some() }

    fn journal_exists(&self) -> bool { self.journal.is_some() }
    fn read_journal(&self) -> FirstBootResult<SetupJournal> {
        self.journal.clone().ok_or_else(|| FirstBootError::JournalError("no journal".into()))
    }
    fn write_journal(&self, _journal: &SetupJournal) -> FirstBootResult<()> {
        if self.fail_journal_write {
            Err(FirstBootError::JournalError("simulated write failure".into()))
        } else {
            Ok(())
        }
    }

    fn user_exists(&self, username: &str) -> FirstBootResult<bool> {
        Ok(self.existing_users.iter().any(|u| u == username))
    }
    fn create_user(&self, username: &str, _full_name: &str, _password_hash: &str, _administrator: bool) -> FirstBootResult<()> {
        if let Some(ref reason) = self.fail_user_create {
            return Err(FirstBootError::AccountCreationFailed(format!("{username}: {reason}")));
        }
        Ok(())
    }

    fn hostname_configured(&self) -> FirstBootResult<bool> { Ok(self.hostname_set.is_some()) }
    fn set_hostname(&self, _hostname: &str) -> FirstBootResult<()> { Ok(()) }
    fn timezone_configured(&self) -> FirstBootResult<bool> { Ok(self.timezone_set.is_some()) }
    fn set_timezone(&self, _timezone: &str) -> FirstBootResult<()> { Ok(()) }
    fn network_configured(&self) -> FirstBootResult<bool> { Ok(self.network_applied) }
    fn apply_network_config(&self, _locale: &str, _keymap: &str, _language: &str) -> FirstBootResult<()> { Ok(()) }

    fn check_graphics_driver(&self) -> FirstBootResult<GraphicsDriverStatus> { Ok(self.graphics_driver.clone()) }
    fn check_nvidia_gpu(&self) -> FirstBootResult<NvidiaStatus> { Ok(self.nvidia.clone()) }
    fn check_vulkan(&self) -> FirstBootResult<VulkanStatus> { Ok(self.vulkan.clone()) }
    fn check_audio(&self) -> FirstBootResult<AudioStatus> { Ok(self.audio.clone()) }
    fn check_update_config(&self) -> FirstBootResult<UpdateConfigStatus> { Ok(self.update_config.clone()) }
    fn check_boot_deployment(&self, _expected: &str) -> FirstBootResult<BootDeploymentStatus> {
        Ok(self.boot_deployment.clone())
    }
}

/// Stub real provider — returns `Provider` error on all calls.
/// Actual implementation requires Linux sysfs, systemd D-Bus, and useradd.
pub struct RealProvider;

macro_rules! unimpl {
    () => {
        Err(FirstBootError::Provider("RealProvider not implemented on this platform".into()))
    };
}

impl FirstBootProvider for RealProvider {
    fn read_handoff(&self) -> FirstBootResult<HandoffData> { unimpl!() }
    fn delete_handoff(&self) -> FirstBootResult<()> { unimpl!() }
    fn handoff_exists(&self) -> bool { false }
    fn journal_exists(&self) -> bool { false }
    fn read_journal(&self) -> FirstBootResult<SetupJournal> { unimpl!() }
    fn write_journal(&self, _: &SetupJournal) -> FirstBootResult<()> { unimpl!() }
    fn user_exists(&self, _: &str) -> FirstBootResult<bool> { unimpl!() }
    fn create_user(&self, _: &str, _: &str, _: &str, _: bool) -> FirstBootResult<()> { unimpl!() }
    fn hostname_configured(&self) -> FirstBootResult<bool> { unimpl!() }
    fn set_hostname(&self, _: &str) -> FirstBootResult<()> { unimpl!() }
    fn timezone_configured(&self) -> FirstBootResult<bool> { unimpl!() }
    fn set_timezone(&self, _: &str) -> FirstBootResult<()> { unimpl!() }
    fn network_configured(&self) -> FirstBootResult<bool> { unimpl!() }
    fn apply_network_config(&self, _: &str, _: &str, _: &str) -> FirstBootResult<()> { unimpl!() }
    fn check_graphics_driver(&self) -> FirstBootResult<GraphicsDriverStatus> { unimpl!() }
    fn check_nvidia_gpu(&self) -> FirstBootResult<NvidiaStatus> { unimpl!() }
    fn check_vulkan(&self) -> FirstBootResult<VulkanStatus> { unimpl!() }
    fn check_audio(&self) -> FirstBootResult<AudioStatus> { unimpl!() }
    fn check_update_config(&self) -> FirstBootResult<UpdateConfigStatus> { unimpl!() }
    fn check_boot_deployment(&self, _: &str) -> FirstBootResult<BootDeploymentStatus> { unimpl!() }
}
