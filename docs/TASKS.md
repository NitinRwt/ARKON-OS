# ARKON OS — Tasks

Status: `done`, `open`, `blocked`, `not started`. Only the current phase is detailed; later phases are planned in their own design step.

## Phase 0 — Architecture (current)

| ID | Task | Status |
|---|---|---|
| P0-01 | Inspect repository and development host | done |
| P0-02 | Research upstream status of the proposed stack | done |
| P0-03 | Write the thirteen Phase 0 documents | done |
| P0-04 | Owner answers blocking decisions U-01…U-07 | **open — owner** |
| P0-05 | Record answers in DECISIONS.md; revise documents affected by any answer that differs from the recommendation | blocked on P0-04 |
| P0-06 | Re-run the Phase 0 gate | blocked on P0-05, P0-08 |
| P0-07 | Architecture review across the ten requested areas; correct contradictions and gaps in the documents | done 2026-11-16 |
| P0-08 | Owner approves or rejects proposed decisions D-001…D-033 | **open — owner** |
| P0-09 | Owner checks the laptop: Windows disk-encryption status and recovery key, Secure Boot state, free internal drive slot | **open — owner** (needed for U-01, not for the gate itself) |

## Audits

| ID | Task | Status |
|---|---|---|
| AUD-01 | Phase 1 completion audit | done 2026-11-16 — **not approved**; no Phase 1 implementation found. See PHASE1_AUDIT.md |
| AUD-02 | Phase 1 re-audit | not started — request when the re-audit criteria in PHASE1_REMEDIATION.md are met |
| AUD-03 | Phase 2 final engineering audit | done 2026-11-17 — **not approved**; 13 blocking issues. See PHASE2_FINAL_AUDIT.md |
| AUD-04 | Phase 2 re-audit | not started — request when all 8 re-audit criteria in PHASE2_FINAL_AUDIT.md §6 are met (Phase 1 must pass first) |

## Phase 1 — OS foundation (not started; blocked on the Phase 0 gate)

Draft breakdown. It is confirmed or changed in the Phase 1 requirements and design steps. Each task is independently testable.

| ID | Task | Resolves |
|---|---|---|
| P1-01 | Initialise git repository, licence, `.gitignore`, secret scanning, branch protection | U-02, U-07 |
| P1-02 | `Justfile` skeleton and CI workflow that runs lint only; confirm runner disk space is sufficient | R-33 |
| P1-03 | Base stage: pin `fedora-bootc` by digest, `arkon-release` placeholder, repository allow-list with keys, source-repository check, kernel-unchanged check | V-01, criteria 1.3, 1.15 |
| P1-04 | Lock file and SBOM generation; image labels (version, build timestamp, variant, driver, device table) | 1.2 |
| P1-05 | Desktop stage: KDE Plasma package manifest; container-level tests | V-01, V-07 |
| P1-06 | Confirm bootloader, boot counting and automatic bootloader updates on the ostree backend | V-02, V-03, V-15 |
| P1-07 | kmods pipeline with test keys: obtain matching `kernel-devel`, build open and closed modules, sign, emit manifest, mismatch check | V-04, V-13, 1.6 |
| P1-08 | Module-signing certificate with module-only key usage; prove shim rejects an EFI binary signed with it | V-12, 1.14 |
| P1-09 | NVIDIA stage: userspace driver, 32-bit libraries, kernel arguments, container toolkit | V-10 |
| P1-10 | Gaming-package stage (install only; verification is Phase 5) | — |
| P1-11 | Image signing with test keys; scoped client policy; bootc enforcement; negative tests; upstream image pull still works | V-14, 1.5 |
| P1-12 | Key custody decision, production key generation, rotation and compromise procedures, rotation rehearsal | U-08, 1.11 |
| P1-13 | systemd-boot + composefs spike, time-boxed; written result | D-003, 1.12 |
| P1-14 | "No toolchain in image" test | 1.10 |
| P1-15 | First boot on R1: owner installs to the test disk; record Secure Boot, module load, offload rendering, cpufreq driver, microcode, sleep mode | 1.7, 1.18 |
| P1-16 | Boot on R1 with the certificate not enrolled | V-11, 1.8 |
| P1-17 | Idle power and suspend/resume on R1 under open and closed modules; decide flavour and driver adoption policy | U-14, U-18, 1.16 |
| P1-18 | Upgrade and rollback on R1; bootloader-changing image on R1 | 1.9, 1.17 |
| P1-19 | Measure image size, update download size, retained-deployment disk and `/boot` use, duration of one full bare-metal run | R-24, U-17 |
| P1-20 | Phase 1 review, documentation update, gate | — |

Tasks P1-15 to P1-18 need the owner at the machine. All installation and disk operations are performed by the owner.

## Phase 2 — Installer (design drafted; phase not open)

| ID | Task | Status |
|---|---|---|
| P2-D1 | Installer technology review and Phase 2 architecture draft | done 2026-11-16, as a draft ahead of the Phase 1 gate |
| P2-D2 | Owner decides U-19 and reviews D-034…D-040 | **open — owner** |
| P2-D3 | Revalidate the draft against Phase 1's results | blocked on Phase 1 |
| P2-10 | `arkon-hw` library and `arkon-hwprobe` CLI — hardware discovery module | **done 2026-11-17** (gate violation; written at owner request; must be revalidated against Phase 1 facts, see R-44) |
| P2-11 | `arkon-storage` scanner and planner — partition planning engine | **done 2026-11-17** (gate violation; written at owner request) |
| P2-12 | `arkon-boot` — UEFI and Secure Boot planning (Step 7) | **done 2026-11-17** (gate violation; written at owner request) |
| P2-13 | `arkon-installer` Rust library — 12-state session, 13 screen models, 34 tests (Step 8, partial) | **done 2026-11-17** (gate violation; written at owner request) |
| P2-14 | `apps/arkon-installer` — HTML SPA + Tauri shell (Step 8, frontend) | **done 2026-11-17** (gate violation; written at owner request) |
| P2-15 | `arkon-firstboot` — restart-safe first-boot setup and health checks (Step 9) | **done 2026-11-17** (gate violation; written at owner request) |

The stage-by-stage task list is [docs/phase2/PHASE2_TASKS.md](docs/phase2/PHASE2_TASKS.md). None of it may start before Phase 1 passes a re-audit.

## Later phases

Planned at their own start. Entry points are listed in [docs/DEVELOPMENT_ROADMAP.md](docs/DEVELOPMENT_ROADMAP.md). Note that the hardware detection library (`arkon-hw`) is built in Phase 2, ahead of the other Rust components, because the installer is its first user.
