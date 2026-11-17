# ARKON OS — Development Roadmap

Each phase follows the mandatory workflow (Discovery → … → Phase gate) and ends at the gate defined in [ACCEPTANCE_CRITERIA.md](ACCEPTANCE_CRITERIA.md). A phase does not start until the previous gate passes.

Effort figures assume one full-time engineer and are unvalidated estimates (U-10). They exclude waiting on hardware purchases and on upstream fixes. No calendar dates are given until Phase 1 produces real velocity data.

**Fedora version.** Development starts on Fedora 45 and follows the current Fedora release (D-031). At the estimated pace the public beta falls after Fedora 46 and possibly after Fedora 47, and Fedora 45 is reported to reach end of life in December 2027. A phase gate is passed on the Fedora release current at that time; one or two rebases happen before 1.0 and are budgeted under cross-cutting work, not inside the phase estimates.

## Overview

| Phase | Outcome | Estimate | Depends on |
|---|---|---|---|
| 0 | Approved architecture and plan | — (this package) | Owner decisions U-01…U-07 |
| 1 | Both image variants build in CI, are signed, and boot on R1 | 4–6 weeks | Phase 0 gate, test disk (U-01), GitHub organisation (U-02) |
| 2 | Installer ISO installs ARKON on R1 from USB; hardware detection library exists | 4–6 weeks | Phase 1 |
| 3 | Branded Plasma desktop with defined defaults | 2–3 weeks | Phase 2 |
| 4 | Services and ARKON Center | 8–11 weeks | Phase 3, identity (U-07) |
| 5 | Steam and Proton verified | 2–4 weeks | Phase 1 (can overlap Phase 4) |
| 6 | Development and GPU container workflow verified | 2–3 weeks | Phase 1 (can overlap Phase 4) |
| 7 | Automatic rollback, diagnostics, recovery paths proven | 4–6 weeks | Phase 4 |
| 8 | Published baseline and evaluated optimisations | 3–5 weeks | Phases 5, 6 |
| 9 | Public beta | 4–6 weeks, then ongoing | All, plus U-11 and U-13 |

Total: roughly 33–50 engineer-weeks before a public beta, plus Fedora rebases. Phases 5 and 6 build little new code and mostly verify the image; they are separate phases because each has its own gate.

## Phase 1 — Fedora bootc OS foundation

Scope: repository and CI; base, desktop (unbranded Plasma), gaming-package and NVIDIA stages; kmods pipeline with both module flavours and a module-only signing certificate; image signing with scoped client enforcement; lock file and SBOM; first boot on R1 by installing to the test disk with `bootc install`; idle-power and suspend measurement under each module flavour (decides U-14); confirmation of boot counting and automatic bootloader updates; systemd-boot spike.

Out of scope: installer ISO, branding, ARKON services, tuning. MOK enrolment in this phase is done by hand with `mokutil`.

Largest unknowns: V-01, V-02, V-03, V-04, V-10 to V-15; NVIDIA on a hybrid laptop under Secure Boot; disk space on hosted CI runners.

## Phase 2 — Bare-metal installer

Scope: `arkon-hw` library and `arkon-hwprobe` CLI, the project's first Rust code, because the installer is their first user (D-024); ISO built with image-builder (`generic-iso`); Anaconda flow; variant chosen from the probe's output; offline payload; partition layout sized for the retained deployments; optional passphrase encryption; MOK enrolment step; signature enforcement set on the installed system; ISO signing and checksums; documented USB writing procedure including a pre-install checklist for machines that also hold Windows.

Out of scope: automated dual boot, custom installer UI, TPM unlock.

Largest unknowns: V-05, V-06; the known `systemd-remount-fs` failure on Anaconda-installed bootc systems; reinstall-keeping-home.

**Pending change.** A Phase 2 design draft in [phase2/](phase2/) proposes replacing Anaconda with an ARKON orchestrator over systemd-repart and bootc (D-034), in order to support existing Windows installations safely. If the owner approves it (U-19), this phase's scope and its estimate (about 14–17 weeks instead of 4–6) are rewritten from that draft. Until then the text above is the plan of record.

Before the gate: re-decide D-003 using the Phase 1 spike result.

## Phase 3 — ARKON desktop experience

Scope: `arkon-release`, `arkon-logos`, boot splash, login screen, wallpaper and Plasma look-and-feel package; core application set as RPMs and the rest as Flatpaks (U-15); codec policy (U-16); first-login experience; removal of all Fedora marks (D-022).

Out of scope: custom Plasma widgets, forked KDE components.

Check during discovery: whether KDE Discover already presents bootc update state.

## Phase 4 — ARKON Center and system services

Order inside the phase:

1. Tauri rendering spike on R1 (D-016). Stop and switch to the Qt fallback if it fails.
2. D-Bus interface definitions and polkit actions; review against the security checklist.
3. `arkon-updated` front and the `arkon-bootc@` worker units, with `arkonctl` as the first client.
4. Freshness, device-support and policy-drift checks in the updater.
5. `arkon-secureboot`, `arkon-diag`.
6. `arkon-session-agent`: notifications and Flatpak NVIDIA runtime sync.
7. ARKON Center screens over the finished services.
8. SELinux policy, RPM packaging, inclusion in the image.

Out of scope: any service not listed in SYSTEM_ARCHITECTURE.md.

## Phase 5 — Gaming infrastructure

Scope: verification of the gaming stack per [GAMING_ARCHITECTURE.md](GAMING_ARCHITECTURE.md); discrete-GPU default on hybrid laptops; Gamescope wrapper test; choice of reference titles; frame-log capture.

Out of scope: console session, launchers other than Steam, tuning.

## Phase 6 — Developer and AI/ML environments

Scope: development-container front end choice; CDI on boot under SELinux enforcing; PyTorch and TensorFlow upstream image verification; editor workflow documentation; reference AI workloads.

Out of scope: ROCm, oneAPI, ARKON-built framework images.

## Phase 7 — System intelligence and recovery

"Intelligence" here means rule-based, local, hardware-aware behaviour. No machine learning, no cloud service, no data collection.

Scope: health checks with negative and false-positive tests; automatic rollback; refusal to restage a failed digest; driver-support check before staging; variant recommendation after hardware change; diagnostics bundle; every step of the recovery ladder exercised on R1 and written up.

## Phase 8 — Benchmarking and optimisation

Order is fixed:

1. Build the harness and validate its repeatability (run-to-run spread must be known before any comparison means anything).
2. Baseline: Fedora KDE 45 on R1, same driver version.
3. ARKON on R1, unmodified.
4. Publish both with raw data.
5. Only then propose optimisation experiments, one variable at a time. An experiment ships if it shows a gain larger than the measured run-to-run spread, on every Tier 1 device, with no stability or security regression.

Metrics: the charter's full list (FPS, 1% lows, frame-time percentiles, CPU/GPU utilisation and temperature, power, memory, boot time, storage throughput, AI inference throughput).

## Phase 9 — Public beta and distribution

Scope: download site and mirrors; release notes; signature verification guide; support channels; issue triage process; security contact and disclosure policy; key-holder redundancy; legal review complete (U-13); final D-003 decision; at least one RTX desktop and one AMD-GPU machine in Tier 1 or dated Tier 2 (U-11).

A beta is not announced with claims about hardware or features that have no test record.

## Cross-cutting work

| Stream | Starts | Content |
|---|---|---|
| Upstream tracking | Phase 1 | Weekly check of Fedora kernel updates, NVIDIA driver releases, bootc releases; base digest bumps by pull request |
| Security | Phase 1 | Key custody, CI hardening, advisory monitoring for image contents |
| Fedora rebase | Each Fedora release (next expected spring 2027) | Parallel `-dev` stream on the new major; move development to it once the bare-metal suite passes. A recurring cost of roughly one to two weeks per release, unvalidated |
| Maintenance load | Phase 1 | Every stable promotion needs a manual bare-metal run. Track the time it takes; it sets the release cadence (U-17) and is the first thing to automate if a dedicated test machine appears |
| Documentation | Phase 1 | User-facing documentation grows with each phase; state documents updated every session |

## What would change this roadmap

- Phase 1 shows the fedora-bootc base cannot carry a KDE desktop cleanly → fall back to deriving from Fedora's Kinoite container image (D-001).
- The systemd-boot spike succeeds without the listed gaps → adopt it before the installer is built, which changes Phase 2 and the Secure Boot key design.
- Tauri fails on NVIDIA + Wayland, or Fedora retires the `webkit2gtk-4.1` library it needs → Phase 4 front end switches to Qt; services are unaffected.
- Open NVIDIA modules prove unacceptable on R1 → a third, closed-module variant is added (U-14), increasing build and test work in every later phase.
- No additional hardware becomes available → 1.0 is released with an explicitly narrow Tier 1 and without RTX claims.
