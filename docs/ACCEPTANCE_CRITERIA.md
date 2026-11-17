# ARKON OS — Acceptance Criteria

Rules for every gate:

- A criterion passes only with evidence recorded in TEST_RESULTS.md: what was run, the image digest, the hardware, the date and the result.
- "On R1" means on the physical reference laptop, not in a virtual machine.
- A criterion that cannot be tested is recorded as **not verified**, never as passed. A phase with an unverified "must" criterion does not pass.
- A failed mandatory test blocks the gate. Waivers require a decision entry signed off by the project owner.

## Phase 0 — Architecture

| # | Criterion | State on 2026-11-16 |
|---|---|---|
| 0.1 | Project environment and repository inspected; findings recorded | Done |
| 0.2 | The thirteen Phase 0 documents exist | Done |
| 0.3 | Upstream status of each stack component researched, with unverified points listed | Done; sixteen points open (V-01…V-16) |
| 0.4 | Documents are mutually consistent (same variants, services, decisions and identifiers everywhere) | Done by author review; no independent review |
| 0.5 | Every deviation from the charter's stack is documented with a reason | Done (D-003) |
| 0.6 | Unresolved decisions are listed, not silently answered | Done (U-01…U-13) |
| 0.7 | Project owner has answered the blocking decisions U-01…U-07 | **Open** |
| 0.8 | No implementation code written | Done |
| 0.9 | Architecture review completed across feasibility, bootc, NVIDIA/CUDA, AMD, gaming, Secure Boot, update/recovery, service boundaries, security and maintenance; contradictions corrected in the documents | Done 2026-11-16 by the author; eleven contradictions and nine design gaps corrected (D-023…D-033). No independent reviewer |
| 0.10 | Project owner has approved or rejected the proposed decisions D-001…D-033 | **Open** |

Gate: **not passed** until 0.7 and 0.10 are closed. Phase 1 does not start before then.

Items the review moved into later phases on purpose, because they need measurement or belong to that phase's design step, do not block this gate: U-08 to U-18 and V-01 to V-16.

## Phase 1 — OS foundation

Must:

| # | Criterion |
|---|---|
| 1.1 | Both variants build in CI from a clean checkout with one documented command each |
| 1.2 | Base image is pinned by digest; a build produces a lock file and an SBOM |
| 1.3 | A build fails if any package comes from a repository outside the allow-list |
| 1.4 | `bootc container lint` passes for both variants |
| 1.5 | Images are signed; an installed system refuses an unsigned ARKON image and one signed with a different key on `bootc upgrade` and `bootc switch` (all tested), while an unrelated upstream image can still be pulled with Podman |
| 1.6 | NVIDIA modules are built for exactly the image's kernel and are signed; the build fails on a mismatch (tested by forcing one) |
| 1.7 | `arkon-nvidia` boots to a Plasma Wayland session on R1 with Secure Boot on and the ARKON certificate enrolled; the NVIDIA module is loaded and a Vulkan application renders on the NVIDIA GPU |
| 1.8 | `arkon-nvidia` still boots to a usable desktop on R1 with Secure Boot on and the certificate **not** enrolled |
| 1.9 | `bootc upgrade` to a newer image, then `bootc rollback`, both succeed on R1 |
| 1.10 | No compiler or SDK is present in either final image (checked by a test, not by inspection) |
| 1.11 | Key rotation and compromise procedures are written, and rotation has been rehearsed with test keys |
| 1.12 | The systemd-boot spike has a written result and D-003 is confirmed or revised |
| 1.13 | V-01, V-02, V-03, V-04 and V-10 to V-15 are resolved |
| 1.14 | The module-signing certificate carries the module-only key usage, and shim on R1 refuses an EFI binary signed with that key |
| 1.15 | A test fails the build if any step changed a kernel package relative to the base image |
| 1.16 | Idle power draw and suspend/resume on R1 are measured with open and with closed NVIDIA modules; U-14 and U-18 are decided from the result |
| 1.17 | An image carrying a newer shim or GRUB updates the EFI partition on R1 and still boots with Secure Boot on, or the absence of automatic bootloader updates is recorded and D-033 revised |
| 1.18 | The cpufreq driver, microcode revision and sleep mode in use on R1 are recorded |

Should: `arkon` variant booted on AMD-GPU or Intel-GPU hardware (R1's integrated GPU with the NVIDIA GPU ignored is an acceptable partial substitute, recorded as such).

## Phase 2 — Installer

A fuller draft, written for the proposed installer engine (D-034), is in [phase2/PHASE2_ACCEPTANCE_CRITERIA.md](phase2/PHASE2_ACCEPTANCE_CRITERIA.md). It replaces the table below if the owner approves D-034 (U-19). Until then the table below stands.

Must:

| # | Criterion |
|---|---|
| 2.1 | The ISO builds in CI and is published with a checksum and signature |
| 2.2 | Installing from USB to an empty disk on R1 completes and boots into ARKON with Secure Boot on |
| 2.3 | The installer selects the NVIDIA variant on R1 without the user choosing. The choice comes from `arkon-hwprobe`; the `arkon-hw` library has unit tests covering every row of the GPU table, and no second implementation of the rule exists in the installer |
| 2.4 | Installation completes with no network connection and yields a desktop with a working file manager, terminal, text editor and browser |
| 2.5 | Installation with passphrase encryption completes and the system unlocks and boots; the installer offers no TPM unlock |
| 2.6 | The installer never selects a disk on its own; the target is always an explicit user choice, and a disk containing another OS triggers a clear warning |
| 2.7 | The installed system receives updates from the `stable` stream without reconfiguration |
| 2.8 | MOK enrolment is offered during or immediately after installation and works on R1 |
| 2.9 | A legacy-BIOS boot of the ISO is refused with an explanation |
| 2.10 | V-05 and V-06 resolved; D-003 re-decided |
| 2.11 | The installed system has signature enforcement active for its OS image source (checked after install, not assumed from the kickstart) |
| 2.12 | The partition layout holds the default number of retained deployments with a stated margin in `/boot` and the EFI partition |

## Phase 3 — Desktop

Must:

| # | Criterion |
|---|---|
| 3.1 | No Fedora logo or name appears in boot, login, desktop, system information or `os-release` identity fields (`ID_LIKE=fedora` is allowed) |
| 3.2 | Session is Wayland; an X11 application runs through Xwayland |
| 3.3 | Suspend and resume, external display hot-plug, audio output and input, Wi-Fi and Bluetooth work on R1 |
| 3.4 | The core application set (U-15) is present offline. Flatpak defaults install without a manual step once a network is available, and installing them does not modify `/usr` |
| 3.5 | Boot time to login screen on R1 is within 10% of stock Fedora KDE on the same machine and the same disk (median of five). If the test disk is USB-attached, the result is recorded as not representative of internal storage |
| 3.6 | U-15 and U-16 are decided |

## Phase 4 — Services and ARKON Center

Must:

| # | Criterion |
|---|---|
| 4.1 | Rendering spike passed on R1, or the fallback decision is recorded |
| 4.2 | Every service method has unit tests; each service has integration tests driven through `arkonctl` |
| 4.3 | An unprivileged user cannot invoke any admin-level method without authentication (tested per method) |
| 4.4 | No method accepts a free-form command, path or image reference (verified by interface review and fuzzing of every string parameter) |
| 4.5 | `systemd-analyze security` exposure score for each bus-facing unit is at or below a threshold fixed in the Phase 4 design step. Worker units are exempt from the threshold but each has a written justification per retained privilege and accepts no input beyond its instance name |
| 4.6 | SELinux is enforcing and produces no denials during the full test suite |
| 4.7 | ARKON Center shows update state, performs check, stage and rollback, shows hardware and driver state, and exports a diagnostics bundle, on R1 |
| 4.8 | ARKON Center makes no network request of its own (verified by capture) |
| 4.9 | Rust: no warnings under clippy; no `unsafe`; dependency audit clean |
| 4.10 | Interface types in Rust and TypeScript are generated from the D-Bus XML; a CI check fails on drift |
| 4.11 | The process that receives D-Bus calls for updates runs without root and cannot start any unit other than the `arkon-bootc@` instances (tested) |
| 4.12 | A stream or variant name absent from the image-shipped list is rejected; a name added by an update becomes selectable without changing the service binary |
| 4.13 | An older, validly signed image offered under the current tag is not staged |
| 4.14 | A non-administrator can obtain a basic diagnostics bundle but not system journal content |
| 4.15 | A desktop notification appears when an update is staged while ARKON Center is closed |

## Phase 5 — Gaming

Must: items 1, 2, 3, 5 and 6 of the Phase 5 test outline in [GAMING_ARCHITECTURE.md](GAMING_ARCHITECTURE.md) pass on R1. Item 4 (Gamescope) must be tested and its result recorded; a failure on NVIDIA is documented, not hidden. Compatibility limits are published in user documentation.

Should: item 7 on AMD or Intel hardware.

## Phase 6 — Developer and AI/ML

Must: items 1–6 of the Phase 6 test outline in [AI_DEVELOPER_ARCHITECTURE.md](AI_DEVELOPER_ARCHITECTURE.md) pass on R1 with SELinux enforcing and rootless Podman. Tested container image digests are recorded.

## Phase 7 — Intelligence and recovery

Must:

| # | Criterion |
|---|---|
| 7.1 | An image with a deliberately broken display manager is rolled back automatically on R1, and is not staged again |
| 7.2 | An image with a deliberately unloadable NVIDIA module is rolled back automatically on R1 |
| 7.3 | A missing MOK enrolment does **not** cause a rollback |
| 7.4 | Ten consecutive healthy updates produce zero false rollbacks |
| 7.5 | Power removed during download, and during staging, each leave a bootable system (three trials each on R1) |
| 7.6 | Recovery ladder steps 1–4 and 6 are each performed on R1 from written instructions; step 5 is either proven or documented as backup-and-restore |
| 7.7 | A staged image whose driver no longer supports the present GPU is refused with an explanation (tested with a fabricated device table) |
| 7.8 | Diagnostics bundle contains none of: hostname, username, serial numbers, MAC or IP addresses in its structured fields (automated check); free-text log redaction is tested against a seeded sample and its miss rate is reported |
| 7.9 | Powering off at the disk-passphrase prompt, and at the login screen, on consecutive boots after an update does not roll back a healthy deployment within the number of attempts a user could plausibly make (number fixed in the Phase 7 design step) |
| 7.10 | After an update that changes the NVIDIA driver version, a GPU-using Flatpak application has acceleration on first launch when the machine was online before the reboot; when it was offline, ARKON Center reports the degraded state |
| 7.11 | A rollback after real use of the newer deployment leaves the desktop, Flatpaks and containers working; anything in `/var` or home that does not survive is documented |

## Phase 8 — Benchmarking

Must:

| # | Criterion |
|---|---|
| 8.1 | Harness repeatability is measured; run-to-run spread is published per metric |
| 8.2 | Baseline (stock Fedora KDE of the same release, on the same disk) and ARKON results for every charter metric on R1, with raw data and exact versions |
| 8.3 | Anyone with the same hardware can rerun the suite from documented commands |
| 8.4 | Every shipped optimisation has a before/after measurement exceeding the spread and a recorded stability check |
| 8.5 | ARKON is not slower than the baseline beyond the measured spread on any metric, or each regression is explained and accepted by decision |
| 8.6 | Published text makes no claim that the data does not show, and none about hardware classes that were not measured |

## Phase 9 — Public beta

Must:

| # | Criterion |
|---|---|
| 9.1 | All earlier gates passed with no open waiver |
| 9.2 | Legal review complete (U-13); licence and trademark settled (U-07) |
| 9.3 | Signing keys have a second holder and a tested recovery path |
| 9.4 | Security contact and disclosure policy published |
| 9.5 | Hardware matrix states tier and test date for every listed class; RTX is named only if tested |
| 9.6 | A new user can download, verify, install and update using only public documentation (tested by someone other than the author) |
| 9.7 | Thirty days of `stable` stream updates on R1 with no manual recovery needed |
| 9.8 | D-003 final decision recorded |
| 9.9 | The beta is built on a Fedora release with at least six months of upstream support remaining, and one Fedora major transition has been performed and rolled back on R1 |
| 9.10 | Stable cadence (U-17) and end-of-support policy are published |
