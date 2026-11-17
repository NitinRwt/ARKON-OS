use serde::{Deserialize, Serialize};

/// A locale / natural language available in the installer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Language {
    /// BCP-47 tag, e.g. `en-US`, `de-DE`.
    pub tag: String,
    /// Native name, e.g. "English (United States)".
    pub name: String,
    /// English name, used for search when the native script is different.
    pub name_en: String,
}

impl Language {
    pub fn new(tag: &str, name: &str, name_en: &str) -> Self {
        Self { tag: tag.into(), name: name.into(), name_en: name_en.into() }
    }
}

/// A keyboard layout available in the installer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyboardLayout {
    /// XKB layout identifier, e.g. `us`, `de`, `gb`.
    pub id: String,
    /// Human-readable name, e.g. "English (US)".
    pub name: String,
    /// Optional variant identifier, e.g. `dvorak`.
    pub variant: Option<String>,
}

impl KeyboardLayout {
    pub fn new(id: &str, name: &str) -> Self {
        Self { id: id.into(), name: name.into(), variant: None }
    }

    pub fn with_variant(mut self, variant: &str) -> Self {
        self.variant = Some(variant.into());
        self
    }
}

/// User account to be created on the installed system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// Unix username (a-z, 0-9, underscore; lowercase; not a reserved name).
    pub username: String,
    /// Full display name.
    pub full_name: String,
    /// Password hash in the scheme accepted by the target system (never plain text).
    /// The front end hashes the password before sending.
    pub password_hash: String,
    /// Whether this account has administrator (sudo) rights.
    pub administrator: bool,
}

/// MOK enrolment choice made on the final screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MokChoice {
    Accept,
    Decline,
    NotApplicable,
}

/// All user-supplied settings required before execution may begin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub user: UserConfig,
    pub hostname: String,
    pub locale: String,
    pub keymap: String,
    pub timezone: String,
    pub language: String,
    pub mok_choice: MokChoice,
}

impl SystemConfig {
    /// Validate that required fields are populated and plausible.
    /// Returns the first validation error found, or Ok.
    pub fn validate(&self) -> Result<(), String> {
        if self.user.username.is_empty() {
            return Err("username is required".into());
        }
        if self.user.username.len() > 32 {
            return Err("username too long (max 32 characters)".into());
        }
        if !self.user.username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            return Err("username may only contain a-z, 0-9, underscore, hyphen".into());
        }
        if self.user.password_hash.is_empty() {
            return Err("password hash is required".into());
        }
        if self.hostname.is_empty() {
            return Err("hostname is required".into());
        }
        if self.hostname.len() > 63 {
            return Err("hostname too long (max 63 characters)".into());
        }
        if !self.hostname.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err("hostname may only contain a-z, 0-9, hyphen".into());
        }
        if self.locale.is_empty() {
            return Err("locale is required".into());
        }
        if self.keymap.is_empty() {
            return Err("keyboard layout is required".into());
        }
        if self.timezone.is_empty() {
            return Err("timezone is required".into());
        }
        Ok(())
    }
}

/// Built-in language list (English first; expand for 1.0 per U-26).
pub fn available_languages() -> Vec<Language> {
    vec![
        Language::new("en-US", "English (United States)", "English (United States)"),
        Language::new("en-GB", "English (United Kingdom)", "English (United Kingdom)"),
        Language::new("de-DE", "Deutsch (Deutschland)", "German (Germany)"),
        Language::new("fr-FR", "Français (France)", "French (France)"),
        Language::new("es-ES", "Español (España)", "Spanish (Spain)"),
        Language::new("pt-BR", "Português (Brasil)", "Portuguese (Brazil)"),
        Language::new("ja-JP", "日本語", "Japanese"),
        Language::new("ko-KR", "한국어", "Korean"),
        Language::new("zh-CN", "中文（简体）", "Chinese (Simplified)"),
        Language::new("zh-TW", "中文（繁體）", "Chinese (Traditional)"),
        Language::new("ru-RU", "Русский", "Russian"),
        Language::new("ar-SA", "العربية", "Arabic"),
        Language::new("hi-IN", "हिन्दी", "Hindi"),
        Language::new("it-IT", "Italiano (Italia)", "Italian (Italy)"),
        Language::new("nl-NL", "Nederlands (Nederland)", "Dutch (Netherlands)"),
        Language::new("pl-PL", "Polski (Polska)", "Polish (Poland)"),
        Language::new("sv-SE", "Svenska (Sverige)", "Swedish (Sweden)"),
        Language::new("tr-TR", "Türkçe (Türkiye)", "Turkish (Turkey)"),
    ]
}

/// Built-in keyboard layout list.
pub fn available_keyboard_layouts() -> Vec<KeyboardLayout> {
    vec![
        KeyboardLayout::new("us", "English (US)"),
        KeyboardLayout::new("gb", "English (UK)"),
        KeyboardLayout::new("de", "German"),
        KeyboardLayout::new("fr", "French"),
        KeyboardLayout::new("es", "Spanish"),
        KeyboardLayout::new("pt", "Portuguese"),
        KeyboardLayout::new("br", "Portuguese (Brazil)"),
        KeyboardLayout::new("it", "Italian"),
        KeyboardLayout::new("nl", "Dutch"),
        KeyboardLayout::new("pl", "Polish"),
        KeyboardLayout::new("ru", "Russian"),
        KeyboardLayout::new("jp", "Japanese"),
        KeyboardLayout::new("kr", "Korean"),
        KeyboardLayout::new("cn", "Chinese"),
        KeyboardLayout::new("se", "Swedish"),
        KeyboardLayout::new("no", "Norwegian"),
        KeyboardLayout::new("dk", "Danish"),
        KeyboardLayout::new("fi", "Finnish"),
        KeyboardLayout::new("tr", "Turkish"),
        KeyboardLayout::new("us", "English (US, Dvorak)").with_variant("dvorak"),
        KeyboardLayout::new("us", "English (US, Colemak)").with_variant("colemak"),
    ]
}
