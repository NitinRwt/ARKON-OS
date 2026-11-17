use arkon_boot::{
    BootPlanRequest, MockProvider,
    bootloader::Bootloader,
    diagnostics::{DiagnosticFinding, RepairAction, diagnose},
    esp::{EspFile, EspVerdict, verify_esp},
    firmware::{
        FirmwareEntry, FirmwareSnapshot, DiffOutcome, DiffViolation,
        compare_snapshots, find_stale_arkon_entries, validate_diff,
    },
    mok::{MokEnrolmentState, plan_mok_enrolment},
    plan::{BootOrderChoice, BootPlanWarning, build_boot_plan},
    secureboot::{SecureBootRecommendation, SecureBootState, validate_secure_boot},
};

// ── helpers ───────────────────────────────────────────────────────────────────

fn windows_entry() -> FirmwareEntry {
    FirmwareEntry::new(0x0000, "Windows Boot Manager")
        .with_esp(
            "aabbccdd-1234-5678-abcd-000000000001",
            r"\EFI\Microsoft\Boot\bootmgfw.efi",
        )
}

fn arkon_entry() -> FirmwareEntry {
    FirmwareEntry::new(0x0001, "ARKON OS")
        .with_esp(
            "aabbccdd-1234-5678-abcd-000000000002",
            r"\EFI\fedora\shimx64.efi",
        )
}

fn linux_entry() -> FirmwareEntry {
    FirmwareEntry::new(0x0002, "Fedora")
        .with_esp(
            "aabbccdd-1234-5678-abcd-000000000003",
            r"\EFI\fedora\shimx64.efi",
        )
}

fn snapshot_with(entries: Vec<FirmwareEntry>) -> FirmwareSnapshot {
    let order: Vec<u16> = entries.iter().map(|e| e.number).collect();
    FirmwareSnapshot { entries, boot_order: order, boot_next: None }
}

fn full_esp_provider() -> MockProvider {
    MockProvider::new()
        .with_esp_file("EFI/fedora/shimx64.efi")
        .with_esp_file("EFI/fedora/grubx64.efi")
        .with_esp_file("EFI/BOOT/BOOTX64.EFI")
}

const ARKON_ESP: &str = "aabbccdd-1234-5678-abcd-000000000002";

fn plan_req(secure_boot: SecureBootState, nvidia: bool) -> BootPlanRequest {
    BootPlanRequest {
        esp_uuid: ARKON_ESP.into(),
        image_ref: "ghcr.io/arkon-os/arkon:latest".into(),
        bootloader: Bootloader::Grub2WithShim,
        boot_order: BootOrderChoice::ArkonFirst,
        firmware_snapshot: snapshot_with(vec![windows_entry()]),
        nvidia_variant: nvidia,
        cert_already_enrolled: false,
        secure_boot,
    }
}

// ── firmware snapshot tests ───────────────────────────────────────────────────

#[test]
fn snapshot_records_all_entries() {
    let provider = MockProvider::new()
        .with_entry(windows_entry())
        .with_entry(linux_entry());
    let snap = FirmwareSnapshot::capture(&provider).unwrap();
    assert_eq!(snap.entries.len(), 2);
    assert_eq!(snap.boot_order.len(), 2);
    assert!(snap.boot_next.is_none());
}

#[test]
fn diff_clean_arkon_added() {
    let before = snapshot_with(vec![windows_entry()]);
    let after = snapshot_with(vec![windows_entry(), arkon_entry()]);
    let diff = compare_snapshots(&before, &after);
    assert!(diff.removed.is_empty());
    assert!(diff.changed.is_empty());
    assert_eq!(diff.added.len(), 1);
    assert!(matches!(validate_diff(&diff), DiffOutcome::Clean { .. }));
}

#[test]
fn diff_violation_entry_changed() {
    let before = snapshot_with(vec![windows_entry()]);
    let mut modified = windows_entry();
    modified.label = "Windows 11 Pro".into();
    let after = snapshot_with(vec![modified, arkon_entry()]);
    let diff = compare_snapshots(&before, &after);
    assert_eq!(diff.changed.len(), 1);
    let outcome = validate_diff(&diff);
    assert!(matches!(outcome, DiffOutcome::Violations(_)));
    if let DiffOutcome::Violations(v) = outcome {
        assert!(v.iter().any(|x| matches!(x, DiffViolation::EntryChanged(_))));
    }
}

#[test]
fn diff_violation_entry_removed() {
    let before = snapshot_with(vec![windows_entry(), linux_entry()]);
    // linux_entry removed; arkon added
    let after = snapshot_with(vec![windows_entry(), arkon_entry()]);
    let diff = compare_snapshots(&before, &after);
    assert_eq!(diff.removed.len(), 1);
    let outcome = validate_diff(&diff);
    assert!(matches!(outcome, DiffOutcome::Violations(_)));
    if let DiffOutcome::Violations(v) = outcome {
        assert!(v.iter().any(|x| matches!(x, DiffViolation::EntryRemoved(_))));
    }
}

#[test]
fn diff_violation_multiple_new_entries() {
    let before = snapshot_with(vec![windows_entry()]);
    let mystery = FirmwareEntry::new(0x0099, "Mystery Entry");
    let after = snapshot_with(vec![windows_entry(), arkon_entry(), mystery]);
    let diff = compare_snapshots(&before, &after);
    assert_eq!(diff.added.len(), 2);
    let outcome = validate_diff(&diff);
    assert!(matches!(outcome, DiffOutcome::Violations(_)));
    if let DiffOutcome::Violations(v) = outcome {
        assert!(v.iter().any(|x| matches!(x, DiffViolation::MultipleNewEntries { .. })));
    }
}

#[test]
fn windows_entry_preserved_in_diff() {
    let before = snapshot_with(vec![windows_entry()]);
    let after = snapshot_with(vec![windows_entry(), arkon_entry()]);
    let diff = compare_snapshots(&before, &after);
    assert!(!diff.removed.iter().any(|e| e.label.contains("Windows")));
    assert!(!diff.changed.iter().any(|c| c.before.label.contains("Windows")));
}

#[test]
fn stale_arkon_entry_detected() {
    let stale = FirmwareEntry::new(0x0005, "ARKON OS")
        .with_esp("deadbeef-0000-0000-0000-000000000000", r"\EFI\fedora\shimx64.efi");
    let snap = snapshot_with(vec![windows_entry(), stale]);
    let known = vec![ARKON_ESP.to_string()];
    let found = find_stale_arkon_entries(&snap, &known);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].number, 0x0005);
}

// ── ESP verification tests ────────────────────────────────────────────────────

#[test]
fn esp_all_files_present() {
    let result = verify_esp(&full_esp_provider(), "uuid-001");
    assert_eq!(result.verdict, EspVerdict::Ok);
    assert_eq!(result.present.len(), 3);
    assert!(result.missing.is_empty());
}

#[test]
fn esp_shim_missing() {
    let provider = MockProvider::new()
        .with_esp_file("EFI/fedora/grubx64.efi")
        .with_esp_file("EFI/BOOT/BOOTX64.EFI");
    let result = verify_esp(&provider, "uuid-002");
    assert!(matches!(result.verdict, EspVerdict::Incomplete { missing_count: 1 }));
    assert!(result.missing.contains(&EspFile::Shim));
}

#[test]
fn esp_grub_missing() {
    let provider = MockProvider::new()
        .with_esp_file("EFI/fedora/shimx64.efi")
        .with_esp_file("EFI/BOOT/BOOTX64.EFI");
    let result = verify_esp(&provider, "uuid-003");
    assert!(matches!(result.verdict, EspVerdict::Incomplete { missing_count: 1 }));
    assert!(result.missing.contains(&EspFile::Grub));
}

#[test]
fn esp_fallback_loader_missing() {
    let provider = MockProvider::new()
        .with_esp_file("EFI/fedora/shimx64.efi")
        .with_esp_file("EFI/fedora/grubx64.efi");
    let result = verify_esp(&provider, "uuid-004");
    assert!(matches!(result.verdict, EspVerdict::Incomplete { missing_count: 1 }));
    assert!(result.missing.contains(&EspFile::FallbackLoader));
}

// ── Secure Boot tests ─────────────────────────────────────────────────────────

#[test]
fn secure_boot_on_no_nvidia_no_action() {
    let report = validate_secure_boot(SecureBootState::On, false);
    assert!(report.state.is_on());
    assert_eq!(report.recommendation, SecureBootRecommendation::NoAction);
}

#[test]
fn secure_boot_off_shown_not_refused() {
    // Secure Boot off must not cause a failure — it is shown as information.
    let report = validate_secure_boot(SecureBootState::Off, false);
    assert_eq!(report.state, SecureBootState::Off);
    // No NVIDIA variant + SB off → NoAction (the system does not gate on SB state)
    assert_eq!(report.recommendation, SecureBootRecommendation::NoAction);
}

#[test]
fn secure_boot_on_nvidia_requires_mok() {
    let report = validate_secure_boot(SecureBootState::On, true);
    assert_eq!(report.recommendation, SecureBootRecommendation::EnrolArkonCert);
}

#[test]
fn setup_mode_state_is_recorded() {
    let report = validate_secure_boot(SecureBootState::SetupMode, false);
    assert_eq!(report.state, SecureBootState::SetupMode);
    assert_eq!(report.recommendation, SecureBootRecommendation::NoAction);
}

// ── MOK enrolment tests ───────────────────────────────────────────────────────

#[test]
fn mok_required_nvidia_secure_boot_on() {
    let plan = plan_mok_enrolment(true, &SecureBootState::On, false);
    assert!(matches!(plan.state, MokEnrolmentState::Queued { .. }));
    assert!(plan.cert_path.is_some());
    assert!(plan.user_instructions.is_some());
}

#[test]
fn mok_skip_secure_boot_off() {
    let plan = plan_mok_enrolment(true, &SecureBootState::Off, false);
    assert_eq!(plan.state, MokEnrolmentState::SecureBootOff);
}

#[test]
fn mok_skip_already_enrolled() {
    let plan = plan_mok_enrolment(true, &SecureBootState::On, true);
    assert_eq!(plan.state, MokEnrolmentState::AlreadyEnrolled);
}

#[test]
fn mok_not_required_no_nvidia() {
    let plan = plan_mok_enrolment(false, &SecureBootState::On, false);
    assert_eq!(plan.state, MokEnrolmentState::NotRequired);
}

// ── Bootloader validation tests ───────────────────────────────────────────────

#[test]
fn grub2_shim_is_valid() {
    use arkon_boot::bootloader::{BootloaderVerdict, validate_bootloader};
    let result = validate_bootloader(Bootloader::Grub2WithShim, "ghcr.io/arkon-os/arkon:1.0");
    assert_eq!(result.verdict, BootloaderVerdict::Valid);
    assert!(result.warnings.is_empty());
}

#[test]
fn systemd_boot_is_unverified() {
    use arkon_boot::bootloader::{BootloaderVerdict, BootloaderWarning, validate_bootloader};
    let result = validate_bootloader(Bootloader::SystemdBoot, "ghcr.io/arkon-os/arkon:1.0");
    assert!(matches!(result.verdict, BootloaderVerdict::Unverified { .. }));
    assert!(result.warnings.contains(&BootloaderWarning::SystemdBootNotVerified));
}

#[test]
fn unknown_bootloader_is_invalid() {
    use arkon_boot::bootloader::{BootloaderVerdict, validate_bootloader};
    let result =
        validate_bootloader(Bootloader::Unknown("rEFInd".into()), "ghcr.io/arkon-os/arkon:1.0");
    assert!(matches!(result.verdict, BootloaderVerdict::Invalid { .. }));
}

// ── Diagnostics tests ─────────────────────────────────────────────────────────

#[test]
fn diagnose_missing_firmware_entry() {
    let esp_ver = verify_esp(&full_esp_provider(), ARKON_ESP);
    // Snapshot has Windows but no ARKON entry.
    let snap = snapshot_with(vec![windows_entry()]);
    let report =
        diagnose(&snap, &esp_ver, ARKON_ESP, &MokEnrolmentState::NotRequired, 1, vec![]);
    assert!(!report.healthy);
    assert!(report.findings.iter().any(|f| matches!(f, DiagnosticFinding::FirmwareEntryMissing { .. })));
    assert!(report
        .recommended_actions
        .iter()
        .any(|a| matches!(a, RepairAction::RecreateFirmwareEntry { .. })));
}

#[test]
fn diagnose_mok_not_enrolled() {
    let esp_ver = verify_esp(&full_esp_provider(), ARKON_ESP);
    let snap = snapshot_with(vec![windows_entry(), arkon_entry()]);
    let mok = MokEnrolmentState::Queued { password_hint: "test-pw".into() };
    let report = diagnose(&snap, &esp_ver, ARKON_ESP, &mok, 1, vec![]);
    assert!(!report.healthy);
    assert!(report.findings.iter().any(|f| matches!(f, DiagnosticFinding::MokNotEnrolled)));
    assert!(report
        .recommended_actions
        .iter()
        .any(|a| matches!(a, RepairAction::QueueMokEnrolment)));
}

#[test]
fn diagnose_previous_deployment_available() {
    let esp_ver = verify_esp(&full_esp_provider(), ARKON_ESP);
    let snap = snapshot_with(vec![windows_entry(), arkon_entry()]);
    let report =
        diagnose(&snap, &esp_ver, ARKON_ESP, &MokEnrolmentState::NotRequired, 3, vec![]);
    assert!(report.findings.iter().any(|f| {
        matches!(f, DiagnosticFinding::PreviousDeploymentAvailable { count: 2 })
    }));
}

#[test]
fn diagnose_stale_arkon_entry() {
    let esp_ver = verify_esp(&full_esp_provider(), ARKON_ESP);
    let stale = FirmwareEntry::new(0x0007, "ARKON OS")
        .with_esp("deadbeef-dead-dead-dead-000000000000", r"\EFI\fedora\shimx64.efi");
    // Snapshot has the correct ARKON entry and a stale one.
    let snap = snapshot_with(vec![windows_entry(), arkon_entry(), stale.clone()]);
    let report = diagnose(
        &snap,
        &esp_ver,
        ARKON_ESP,
        &MokEnrolmentState::NotRequired,
        1,
        vec![&stale],
    );
    assert!(report
        .findings
        .iter()
        .any(|f| matches!(f, DiagnosticFinding::StaleArkonEntry { .. })));
    assert!(report
        .recommended_actions
        .iter()
        .any(|a| matches!(a, RepairAction::RemoveStaleEntry { entry_number: 7, .. })));
}

#[test]
fn diagnose_healthy_system() {
    let esp_ver = verify_esp(&full_esp_provider(), ARKON_ESP);
    let snap = snapshot_with(vec![windows_entry(), arkon_entry()]);
    let report =
        diagnose(&snap, &esp_ver, ARKON_ESP, &MokEnrolmentState::NotRequired, 1, vec![]);
    assert!(report.healthy);
    assert!(report.findings.is_empty());
}

// ── Boot plan tests ───────────────────────────────────────────────────────────

#[test]
fn plan_clean_install_grub2() {
    let plan = build_boot_plan(plan_req(SecureBootState::On, false)).unwrap();
    assert!(plan.validation.is_valid());
    assert_eq!(plan.arkon_entry_label, "ARKON OS");
    assert_eq!(plan.arkon_loader_path, r"\EFI\fedora\shimx64.efi");
    assert!(!plan.warnings.iter().any(|w| matches!(w, BootPlanWarning::UnverifiedBootloader { .. })));
}

#[test]
fn plan_windows_entry_preserved_in_snapshot() {
    let plan = build_boot_plan(plan_req(SecureBootState::On, false)).unwrap();
    // The before-snapshot must include the Windows entry that was present at
    // plan-build time; the plan must not include any action against it.
    assert!(plan.firmware_snapshot_before.entries.iter().any(|e| e.label.contains("Windows")));
}

#[test]
fn plan_nvidia_sb_on_queues_mok() {
    let plan = build_boot_plan(plan_req(SecureBootState::On, true)).unwrap();
    assert!(plan.mok_plan.state.is_queued());
    assert!(plan.mok_plan.user_instructions.is_some());
}

#[test]
fn plan_nvidia_sb_off_skips_mok() {
    let plan = build_boot_plan(plan_req(SecureBootState::Off, true)).unwrap();
    assert_eq!(plan.mok_plan.state, MokEnrolmentState::SecureBootOff);
}

#[test]
fn plan_systemd_boot_produces_warning() {
    let mut req = plan_req(SecureBootState::On, false);
    req.bootloader = Bootloader::SystemdBoot;
    let plan = build_boot_plan(req).unwrap();
    // Plan still builds (systemd-boot is unverified, not invalid).
    assert!(plan.warnings.iter().any(|w| matches!(w, BootPlanWarning::UnverifiedBootloader { .. })));
}

#[test]
fn plan_secure_boot_off_carries_warning() {
    let plan = build_boot_plan(plan_req(SecureBootState::Off, false)).unwrap();
    assert!(plan.warnings.iter().any(|w| matches!(w, BootPlanWarning::SecureBootOff)));
}

#[test]
fn plan_stale_arkon_entry_produces_warning() {
    let stale =
        FirmwareEntry::new(0x0007, "ARKON OS").with_esp("deadbeef-0000-0000-0000-dead00000000", r"\EFI\fedora\shimx64.efi");
    let mut req = plan_req(SecureBootState::On, false);
    // Inject a stale ARKON entry in the firmware snapshot.
    req.firmware_snapshot = snapshot_with(vec![windows_entry(), stale]);
    let plan = build_boot_plan(req).unwrap();
    assert!(plan
        .warnings
        .iter()
        .any(|w| matches!(w, BootPlanWarning::StaleArkonEntryDetected { entry_number: 7 })));
}
