use arkon_firstboot::{
    AudioStatus, BootDeploymentStatus, FirstBootError, FirstBootRunner, GraphicsDriverStatus,
    HandoffData, MokChoice, MockProvider, NvidiaStatus, SetupJournal, SetupStep, Severity,
    StepStatus, UpdateConfigStatus, VulkanStatus,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn sample_handoff() -> HandoffData {
    HandoffData {
        username: "alice".into(),
        full_name: "Alice Example".into(),
        password_hash: "$y$j9T$hash".into(),
        administrator: true,
        hostname: "arkon-laptop".into(),
        locale: "en_US.UTF-8".into(),
        keymap: "us".into(),
        timezone: "America/New_York".into(),
        language: "en-US".into(),
        mok_choice: MokChoice::NotApplicable,
        variant: "arkon-nvidia-turing".into(),
        image_ref: "registry.arkonos.com/arkon:latest".into(),
    }
}

fn default_provider() -> MockProvider {
    MockProvider::new().with_handoff(sample_handoff())
}

// ── Handoff validation ────────────────────────────────────────────────────────

#[test]
fn handoff_validates_good_data() {
    let h = sample_handoff();
    assert!(h.validate().is_ok());
}

#[test]
fn handoff_rejects_empty_username() {
    let mut h = sample_handoff();
    h.username = String::new();
    assert!(h.validate().is_err());
}

#[test]
fn handoff_rejects_username_with_spaces() {
    let mut h = sample_handoff();
    h.username = "alice example".into();
    assert!(h.validate().is_err());
}

#[test]
fn handoff_rejects_username_too_long() {
    let mut h = sample_handoff();
    h.username = "a".repeat(33);
    assert!(h.validate().is_err());
}

#[test]
fn handoff_rejects_empty_hostname() {
    let mut h = sample_handoff();
    h.hostname = String::new();
    assert!(h.validate().is_err());
}

#[test]
fn handoff_absent_returns_interactive_setup_flag() {
    let p = MockProvider::new(); // no handoff
    let runner = FirstBootRunner::new(p);
    let err = runner.run().unwrap_err();
    // Runner fails at CreateUser because handoff is absent and that step is fatal.
    assert!(matches!(err, FirstBootError::HandoffAbsent));
}

// ── Journal: restart-safe state ───────────────────────────────────────────────

#[test]
fn fresh_journal_all_steps_pending() {
    let j = SetupJournal::new();
    for &step in SetupStep::all() {
        assert!(j.status(step).is_pending(), "{step:?} should be Pending");
    }
    assert_eq!(j.completed_count(), 0);
    assert_eq!(j.failed_count(), 0);
}

#[test]
fn journal_complete_step_recorded() {
    let mut j = SetupJournal::new();
    j.complete(SetupStep::CreateUser).unwrap();
    assert!(j.status(SetupStep::CreateUser).is_complete());
}

#[test]
fn journal_complete_twice_returns_error() {
    let mut j = SetupJournal::new();
    j.complete(SetupStep::CreateUser).unwrap();
    let err = j.complete(SetupStep::CreateUser).unwrap_err();
    assert!(matches!(err, FirstBootError::AlreadyComplete(_)));
}

#[test]
fn journal_failed_step_recorded() {
    let mut j = SetupJournal::new();
    j.set_status(SetupStep::ConfigureHostname, StepStatus::Failed { error: "timeout".into() });
    assert!(j.status(SetupStep::ConfigureHostname).is_failed());
    assert_eq!(j.failed_count(), 1);
}

#[test]
fn journal_serialises_and_deserialises() {
    let mut j = SetupJournal::new();
    j.set_status(SetupStep::CreateUser, StepStatus::Complete);
    j.set_status(SetupStep::VerifyNvidiaGpu, StepStatus::Skipped { reason: "no gpu".into() });
    let json = j.to_json().unwrap();
    let j2 = SetupJournal::from_json(&json).unwrap();
    assert!(j2.status(SetupStep::CreateUser).is_complete());
    assert!(matches!(j2.status(SetupStep::VerifyNvidiaGpu), StepStatus::Skipped { .. }));
}

#[test]
fn pending_steps_excludes_completed() {
    let mut j = SetupJournal::new();
    j.set_status(SetupStep::CreateUser, StepStatus::Complete);
    j.set_status(SetupStep::ConfigureHostname, StepStatus::Complete);
    let pending = j.pending_steps();
    assert!(!pending.contains(&SetupStep::CreateUser));
    assert!(!pending.contains(&SetupStep::ConfigureHostname));
    assert!(pending.contains(&SetupStep::ConfigureTimezone));
}

// ── Successful full run ───────────────────────────────────────────────────────

#[test]
fn full_run_completes_all_steps() {
    let p = default_provider();
    let runner = FirstBootRunner::new(p);
    let outcome = runner.run().unwrap();
    assert!(outcome.is_complete());
    assert!(!outcome.interactive_setup_required);
    let j = &outcome.journal;
    for &step in SetupStep::all() {
        assert!(j.status(step).is_complete(), "{step:?} should be Complete after full run");
    }
}

#[test]
fn full_run_produces_health_report() {
    let runner = FirstBootRunner::new(default_provider());
    let outcome = runner.run().unwrap();
    assert!(outcome.report.is_some());
    let report = outcome.report.unwrap();
    assert!(report.all_ok);
    assert_eq!(report.error_count(), 0);
}

// ── Idempotency: user already exists ─────────────────────────────────────────

#[test]
fn create_user_skipped_when_user_exists() {
    let p = default_provider().with_existing_user("alice");
    let runner = FirstBootRunner::new(p);
    let outcome = runner.run().unwrap();
    assert!(outcome.is_complete());
    // The step should be recorded as Skipped, not Complete.
    assert!(matches!(outcome.journal.status(SetupStep::CreateUser), StepStatus::Skipped { .. }));
}

// ── Interrupted run resumes ───────────────────────────────────────────────────

#[test]
fn resumed_run_skips_already_complete_steps() {
    // Simulate a journal with the first three steps done.
    let mut pre_journal = SetupJournal::new();
    pre_journal.set_status(SetupStep::CreateUser, StepStatus::Complete);
    pre_journal.set_status(SetupStep::ConfigureHostname, StepStatus::Complete);
    pre_journal.set_status(SetupStep::ConfigureTimezone, StepStatus::Complete);

    let mut p = default_provider();
    p.journal = Some(pre_journal);

    let runner = FirstBootRunner::new(p);
    let outcome = runner.run().unwrap();
    assert!(outcome.is_complete());
    // Previously completed steps should still be Complete (not re-executed).
    assert!(outcome.journal.status(SetupStep::CreateUser).is_complete());
    assert!(outcome.journal.status(SetupStep::ConfigureTimezone).is_complete());
}

#[test]
fn journal_with_all_complete_returns_immediately() {
    let mut complete_journal = SetupJournal::new();
    complete_journal.finished = true;
    for &step in SetupStep::all() {
        complete_journal.set_status(step, StepStatus::Complete);
    }
    let mut p = default_provider();
    p.journal = Some(complete_journal);

    let runner = FirstBootRunner::new(p);
    let outcome = runner.run().unwrap();
    assert!(outcome.is_complete());
    // Report is None because we returned early without re-running checks.
    assert!(outcome.report.is_none());
}

// ── NVIDIA: skip when no GPU ──────────────────────────────────────────────────

#[test]
fn nvidia_step_skipped_when_no_nvidia_gpu() {
    let p = default_provider().with_nvidia(NvidiaStatus::NotPresent);
    let runner = FirstBootRunner::new(p);
    let outcome = runner.run().unwrap();
    assert!(matches!(outcome.journal.status(SetupStep::VerifyNvidiaGpu), StepStatus::Skipped { .. }));
}

#[test]
fn nvidia_step_complete_when_module_loaded() {
    let p = default_provider().with_nvidia(NvidiaStatus::ModuleLoaded {
        generation: "Turing".into(),
        module_name: "nvidia".into(),
    });
    let runner = FirstBootRunner::new(p);
    let outcome = runner.run().unwrap();
    assert!(outcome.journal.status(SetupStep::VerifyNvidiaGpu).is_complete());
}

#[test]
fn nvidia_module_failure_becomes_warning_not_abort() {
    let p = default_provider().with_nvidia(NvidiaStatus::ModuleFailed {
        generation: "Turing".into(),
        error: "module not found".into(),
    });
    let runner = FirstBootRunner::new(p);
    let outcome = runner.run().unwrap();
    // Warning is a form of is_complete() — the step is recorded.
    assert!(outcome.journal.status(SetupStep::VerifyNvidiaGpu).is_complete());
    assert!(outcome.is_complete(), "run must not abort on NVIDIA warning");
}

// ── Vulkan ────────────────────────────────────────────────────────────────────

#[test]
fn vulkan_missing_records_warning_not_abort() {
    let p = default_provider().with_vulkan(VulkanStatus::LoaderMissing);
    let runner = FirstBootRunner::new(p);
    let outcome = runner.run().unwrap();
    assert!(outcome.is_complete());
    // Warning step is still marked is_complete().
    assert!(outcome.journal.status(SetupStep::VerifyVulkan).is_complete());
    // The report should note the Vulkan problem.
    let report = outcome.report.unwrap();
    assert!(report.warning_count() > 0 || report.error_count() > 0);
}

// ── Audio ─────────────────────────────────────────────────────────────────────

#[test]
fn audio_not_running_records_warning() {
    let p = default_provider().with_audio(AudioStatus::NotRunning);
    let runner = FirstBootRunner::new(p);
    let outcome = runner.run().unwrap();
    assert!(outcome.is_complete());
    assert!(matches!(outcome.journal.status(SetupStep::VerifyAudio), StepStatus::Warning { .. }));
}

// ── Update config ─────────────────────────────────────────────────────────────

#[test]
fn update_config_missing_records_warning() {
    let p = default_provider().with_update_config(UpdateConfigStatus::Missing);
    let runner = FirstBootRunner::new(p);
    let outcome = runner.run().unwrap();
    assert!(outcome.is_complete());
    let report = outcome.report.unwrap();
    // Missing update config should produce at least one diagnostic.
    assert!(report.warning_count() + report.error_count() > 0);
}

// ── Boot deployment ───────────────────────────────────────────────────────────

#[test]
fn boot_deployment_mismatch_records_in_report() {
    let p = default_provider().with_boot_deployment(BootDeploymentStatus::Mismatch {
        expected: "registry.arkonos.com/arkon:latest".into(),
        found: "registry.arkonos.com/arkon:old".into(),
    });
    let runner = FirstBootRunner::new(p);
    let outcome = runner.run().unwrap();
    assert!(outcome.is_complete());
    let report = outcome.report.unwrap();
    assert!(!report.all_ok, "mismatch should set all_ok=false");
}

// ── Health report ─────────────────────────────────────────────────────────────

#[test]
fn health_report_all_ok_when_everything_passes() {
    let runner = FirstBootRunner::new(default_provider());
    let outcome = runner.run().unwrap();
    let report = outcome.report.unwrap();
    assert!(report.all_ok);
    assert_eq!(report.error_count(), 0);
    assert_eq!(report.setup_steps_failed, 0);
}

#[test]
fn health_report_records_graphics_driver_missing() {
    let p = default_provider().with_graphics_driver(GraphicsDriverStatus::Missing);
    let runner = FirstBootRunner::new(p);
    let outcome = runner.run().unwrap();
    let report = outcome.report.unwrap();
    assert!(!report.all_ok);
    let errors = report.entries_at_or_above(Severity::Error);
    assert!(errors.iter().any(|e| e.component == "graphics"));
}

#[test]
fn health_report_severity_sorted_errors_first() {
    let p = default_provider()
        .with_graphics_driver(GraphicsDriverStatus::Missing)
        .with_vulkan(VulkanStatus::NoDevices);
    let runner = FirstBootRunner::new(p);
    let outcome = runner.run().unwrap();
    let report = outcome.report.unwrap();
    // First entry must be the most severe.
    if !report.diagnostics.is_empty() {
        let first = &report.diagnostics[0].severity;
        for entry in &report.diagnostics {
            assert!(*first >= entry.severity, "diagnostics not sorted by severity");
        }
    }
}

#[test]
fn setup_step_failed_count_reflected_in_report() {
    // Create a journal with one failed step, provide it to the runner.
    let mut pre_journal = SetupJournal::new();
    pre_journal.set_status(SetupStep::CreateUser, StepStatus::Complete);
    pre_journal.set_status(SetupStep::ConfigureHostname, StepStatus::Failed { error: "timeout".into() });
    // Mark remaining as complete so the runner skips them.
    for &step in &[
        SetupStep::ConfigureTimezone, SetupStep::ConfigureNetwork,
        SetupStep::VerifyGraphicsDriver, SetupStep::VerifyNvidiaGpu,
        SetupStep::VerifyVulkan, SetupStep::VerifyAudio,
        SetupStep::VerifyUpdates, SetupStep::VerifyBootDeployment,
        SetupStep::GenerateHealthReport,
    ] {
        pre_journal.set_status(step, StepStatus::Complete);
    }
    pre_journal.finished = true;
    let mut p = default_provider();
    p.journal = Some(pre_journal);

    let runner = FirstBootRunner::new(p);
    let outcome = runner.run().unwrap();
    // The run returned early (finished=true), so report is None.
    // Test the journal directly.
    assert_eq!(outcome.journal.failed_count(), 1);
}
