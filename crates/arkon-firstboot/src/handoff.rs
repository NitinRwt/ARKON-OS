use serde::{Deserialize, Serialize};

/// MOK enrolment choice recorded by the installer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MokChoice {
    Accept,
    Decline,
    NotApplicable,
}

/// Data written by `arkon-installd` at step X8 and consumed on first boot.
/// Once consumed the file is deleted; absence means interactive setup.
/// See INSTALLATION_STATE_MACHINE.md §8.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffData {
    /// Unix username (a-z, 0-9, underscore, hyphen; ≤ 32 chars).
    pub username: String,
    /// Full display name.
    pub full_name: String,
    /// Password hash (the installer hashed the password; never plain text).
    pub password_hash: String,
    /// Whether the account has sudo rights.
    pub administrator: bool,
    /// RFC 5321-style hostname (a-z, 0-9, hyphen; ≤ 63 chars).
    pub hostname: String,
    /// POSIX locale string, e.g. `en_US.UTF-8`.
    pub locale: String,
    /// XKB keymap identifier, e.g. `us`.
    pub keymap: String,
    /// IANA timezone, e.g. `America/New_York`.
    pub timezone: String,
    /// BCP-47 language tag, e.g. `en-US`.
    pub language: String,
    /// MOK enrolment choice.
    pub mok_choice: MokChoice,
    /// Installed variant name, e.g. `arkon-nvidia-turing`.
    pub variant: String,
    /// Image reference deployed by bootc.
    pub image_ref: String,
}

impl HandoffData {
    /// Validate that all required fields satisfy their constraints.
    pub fn validate(&self) -> Result<(), String> {
        if self.username.is_empty() {
            return Err("username is required".into());
        }
        if self.username.len() > 32 {
            return Err(format!("username '{}' exceeds 32 characters", self.username));
        }
        if !self.username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            return Err(format!("username '{}' contains invalid characters", self.username));
        }
        if self.password_hash.is_empty() {
            return Err("password_hash is required".into());
        }
        if self.hostname.is_empty() {
            return Err("hostname is required".into());
        }
        if self.hostname.len() > 63 {
            return Err(format!("hostname '{}' exceeds 63 characters", self.hostname));
        }
        if !self.hostname.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(format!("hostname '{}' contains invalid characters", self.hostname));
        }
        if self.locale.is_empty() {
            return Err("locale is required".into());
        }
        if self.timezone.is_empty() {
            return Err("timezone is required".into());
        }
        if self.variant.is_empty() {
            return Err("variant is required".into());
        }
        Ok(())
    }
}
