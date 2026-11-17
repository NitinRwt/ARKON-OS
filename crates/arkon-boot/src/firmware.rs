use serde::{Deserialize, Serialize};
use crate::error::BootResult;
use crate::provider::BootProvider;

/// A single EFI firmware boot entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirmwareEntry {
    /// Boot variable number (0x0000..=0xFFFF).
    pub number: u16,
    /// Human-readable label shown in the firmware boot menu.
    pub label: String,
    /// GPT PARTUUID of the target ESP, if parseable from the device path.
    pub part_uuid: Option<String>,
    /// Loader path relative to the ESP root (e.g. `\EFI\fedora\shimx64.efi`).
    pub loader_path: Option<String>,
    /// Whether the firmware has this entry enabled (not suppressed).
    pub active: bool,
}

impl FirmwareEntry {
    pub fn new(number: u16, label: impl Into<String>) -> Self {
        Self {
            number,
            label: label.into(),
            part_uuid: None,
            loader_path: None,
            active: true,
        }
    }

    pub fn with_esp(mut self, uuid: impl Into<String>, loader: impl Into<String>) -> Self {
        self.part_uuid = Some(uuid.into());
        self.loader_path = Some(loader.into());
        self
    }

    pub fn inactive(mut self) -> Self {
        self.active = false;
        self
    }

    /// True when this entry carries the ARKON OS boot label.
    pub fn is_arkon_label(&self) -> bool {
        self.label == "ARKON OS" || self.label.starts_with("ARKON")
    }
}

/// A complete snapshot of EFI firmware boot configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareSnapshot {
    pub entries: Vec<FirmwareEntry>,
    pub boot_order: Vec<u16>,
    pub boot_next: Option<u16>,
}

impl FirmwareSnapshot {
    pub fn capture(provider: &dyn BootProvider) -> BootResult<Self> {
        Ok(Self {
            entries: provider.list_entries()?,
            boot_order: provider.boot_order()?,
            boot_next: provider.boot_next()?,
        })
    }

    pub fn entry(&self, number: u16) -> Option<&FirmwareEntry> {
        self.entries.iter().find(|e| e.number == number)
    }
}

/// A change to a single boot entry between two snapshots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntryChange {
    pub number: u16,
    pub before: FirmwareEntry,
    pub after: FirmwareEntry,
}

/// A violation of the firmware-entry invariants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiffViolation {
    EntryChanged(EntryChange),
    EntryRemoved(FirmwareEntry),
    MultipleNewEntries { count: usize },
}

/// The raw diff between two firmware snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareDiff {
    pub removed: Vec<FirmwareEntry>,
    pub changed: Vec<EntryChange>,
    /// Entries in `after` that were not present in `before`.
    pub added: Vec<FirmwareEntry>,
}

/// Outcome of validating a firmware diff against the installation rules
/// (UEFI_BOOT_DESIGN.md §3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffOutcome {
    /// All pre-existing entries preserved, at most one new ARKON entry.
    Clean { arkon_entry: Option<FirmwareEntry> },
    /// One or more invariants violated; installation must stop.
    Violations(Vec<DiffViolation>),
}

/// Compare two firmware snapshots.
///
/// Rules:
/// - Every entry that existed before still exists with identical contents.
/// - At most one new entry is added (the ARKON OS entry created by bootupd).
pub fn compare_snapshots(before: &FirmwareSnapshot, after: &FirmwareSnapshot) -> FirmwareDiff {
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    for entry in &before.entries {
        match after.entry(entry.number) {
            None => removed.push(entry.clone()),
            Some(after_entry) if after_entry != entry => {
                changed.push(EntryChange {
                    number: entry.number,
                    before: entry.clone(),
                    after: after_entry.clone(),
                });
            }
            _ => {}
        }
    }

    let added = after
        .entries
        .iter()
        .filter(|e| before.entry(e.number).is_none())
        .cloned()
        .collect();

    FirmwareDiff { removed, changed, added }
}

/// Validate a diff against the installation invariants.
pub fn validate_diff(diff: &FirmwareDiff) -> DiffOutcome {
    let mut violations = Vec::new();

    for entry in &diff.removed {
        violations.push(DiffViolation::EntryRemoved(entry.clone()));
    }
    for change in &diff.changed {
        violations.push(DiffViolation::EntryChanged(change.clone()));
    }
    if diff.added.len() > 1 {
        violations.push(DiffViolation::MultipleNewEntries { count: diff.added.len() });
    }

    if violations.is_empty() {
        DiffOutcome::Clean { arkon_entry: diff.added.first().cloned() }
    } else {
        DiffOutcome::Violations(violations)
    }
}

/// Detect entries that look like stale ARKON remnants from a previous
/// installation (label matches but partition UUID is not in the current layout).
pub fn find_stale_arkon_entries<'a>(
    snapshot: &'a FirmwareSnapshot,
    known_esp_uuids: &[String],
) -> Vec<&'a FirmwareEntry> {
    snapshot
        .entries
        .iter()
        .filter(|e| {
            e.is_arkon_label()
                && e.part_uuid
                    .as_ref()
                    .map(|u| !known_esp_uuids.contains(u))
                    .unwrap_or(false)
        })
        .collect()
}
