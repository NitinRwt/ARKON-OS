# ARKON OS — Risk Register

Likelihood (L) and impact (I): High, Medium, Low. Reviewed at every phase gate. New risks are appended; closed risks are marked, not deleted.

## Project and resourcing

| ID | Risk | L | I | Mitigation | Phase |
|---|---|---|---|---|---|
| R-01 | Only one test machine exists, and it has no RTX GPU. RTX, desktop NVIDIA, AMD and Intel paths ship unproven | High | High | Tiered matrix that separates tested from expected; no RTX claims until tested; acquire hardware (U-11); recruit testers at beta | All |
| R-02 | Scope is large for the available people. A nine-phase OS project with one engineer stalls | High | High | Narrow 1.0 scope; non-goals list; phases 5 and 6 are verification, not new build; estimates revisited after Phase 1 | All |
| R-03 | The test machine is also the developer's Windows workstation with a single disk. A mistaken install target destroys the working environment; a firmware, boot-order or Secure Boot change can make Windows demand its BitLocker recovery key | Medium | High | Separate test disk (U-01); installer never auto-selects a disk; the assistant performs no disk operations; pre-install checklist: confirm encryption status, save the recovery key, back up, note the boot order | 1, 2 |
| R-30 | Every stable promotion needs a manual bare-metal run by one person on a laptop that is also their workstation. Releases slow down or testing gets skipped | High | High | Cadence set from measured test time (U-17); suite scripted so the manual part is install and reboot only; dedicated test machine with remote power control is the real fix (U-11) | 1 onward |
| R-14 | A similar, established project (Bazzite) already serves much of the audience | High | Medium | Position on the three differentiators in the product vision; be honest if Phase 8 shows no performance advantage | 0, 8, 9 |
| R-15 | Bus factor of one: maintainer unavailable, keys lost, updates stop | Medium | High | Documented procedures; second key holder before beta; published end-of-support policy | 9 |

## Upstream and compatibility

| ID | Risk | L | I | Mitigation | Phase |
|---|---|---|---|---|---|
| R-04 | Starting on the ostree backend with GRUB means a later move to composefs with systemd-boot requires every user to reinstall | Medium | High | Spike in Phase 1; re-decide at the Phase 2 gate and before public beta, while there are still no users | 1, 2, 9 |
| R-05 | A Fedora kernel update breaks the NVIDIA module build. This has happened repeatedly upstream | High | Medium | Kernel pinned per image; kernel and driver move together; NVIDIA variant held back while Mesa variant ships; tested in `testing` stream first | 1 onward |
| R-06 | NVIDIA drops a GPU generation from the current branch and users are updated into a driver that cannot drive their GPU | Medium | High | Supported-device table checked before staging (criterion 7.7); GTX 16-series (R1) is the oldest generation on the current branch and is the most exposed | 7 |
| R-07 | Fedora's bootc tooling for desktops is still maturing; base image, installer path or bootc behaviour changes under the project | Medium | Medium | Pin by digest; track the Atomic SIG; fall back to the Kinoite container base if needed (D-001) | 1, 2 |
| R-08 | Anaconda's bootc installation path has rough edges (a known `systemd-remount-fs` failure; unclear support for hardware-dependent image choice and for reusing a home volume) | Medium | Medium | Verify V-05 early in Phase 2; fallback is one ISO per variant with the choice made at download time | 2 |
| R-09 | Tauri's WebKitGTK webview renders incorrectly on NVIDIA + Wayland, brings an unwelcome GTK stack into a KDE system, or loses its library when Fedora retires `webkit2gtk-4.1` (GTK 3) in favour of the GTK 4 API that Tauri does not use | Medium | Medium | Spike before any UI work (criterion 4.1); services are UI-independent; Qt fallback; watch Fedora change proposals | 4, ongoing |
| R-10 | Hybrid-graphics behaviour on R1 (GPU power-down, external outputs, offload) needs laptop-specific work | High | Medium | Treated as a first-class case; measured in Phase 1; documented as laptop-specific where it is | 1, 5 |
| R-11 | Gamescope is unreliable on NVIDIA | Medium | Low | Wrapper is optional in 1.0; result recorded honestly; no console session | 5 |
| R-12 | Fedora removes i686 packages in a later release and native Steam stops being installable | Low | High | Proposal was withdrawn for now; track it; Flatpak Steam is the documented fallback | Ongoing |
| R-13 | Fedora's six-month cadence and roughly thirteen-month support life outpace the project: the roadmap is longer than Fedora 45's remaining life, and each rebase can break the NVIDIA build, the installer or the desktop | High | High | Track the current Fedora (D-031); Fedora major is one build parameter; rebase budgeted as recurring work; a major transition with rollback is a beta criterion (9.9) | All |
| R-26 | After a driver-changing update, Flatpak applications lose GPU acceleration because the Flatpak NVIDIA runtime no longer matches the host driver | High | Medium | Runtime fetched before the reboot; promotion waits for Flathub to publish it; degraded state shown to the user (D-028, criterion 7.10) | 4, 7 |
| R-28 | A rollback restores the OS but not `/var` or home directories; data migrated by the newer version may not be readable by the older one | Medium | Medium | Documented limit; round-trip test at each Fedora major (criterion 7.11); release notes flag known cases | 7, 9 |
| R-29 | Open NVIDIA modules do not power down a Turing discrete GPU. The reference laptop, and every GTX 16 or RTX 20 laptop, runs hot and drains its battery; or a third variant is needed to avoid that | High | Medium | Measure on R1 in Phase 1; flavour is a build parameter (D-032); decide U-14 from data | 1 |
| R-33 | Hosted CI runners lack the disk space or privileges for desktop-image and ISO builds | Medium | Medium | Reclaim runner disk before builds; split jobs; self-hosted runner as fallback | 1, 2 |
| R-22 | Health checks misfire: a broken image passes, or a healthy image is rolled back in a loop, including when a user repeatedly powers off at the disk-passphrase prompt | Medium | High | Negative and false-positive tests are gate criteria (7.1–7.4, 7.9); MOK-not-enrolled is explicitly not a failure | 7 |

## Security

| ID | Risk | L | I | Mitigation | Phase |
|---|---|---|---|---|---|
| R-16 | Module-signing key leaks; attacker can load kernel code on every machine that enrolled the certificate. With an unrestricted certificate the attacker could also replace the boot chain | Low | High | Certificate limited to module signing (D-029, criterion 1.14); separate key; KMS custody (U-08); never in a layer or log; revocation procedure rehearsed before first public signature | 1 |
| R-17 | Image-signing key leaks or CI is compromised; malicious updates are accepted by clients | Low | High | Protected branches; pinned actions; minimal token scope; rotation built in from the first image | 1 |
| R-27 | shim and GRUB are updated outside the A/B mechanism and cannot be rolled back; conversely a stale boot chain stops booting after a firmware revocation update | Low | High | Keep Fedora's automatic bootloader updates; bare-metal boot before promoting any image that changes them (D-033); installer USB as recovery | 1 onward |
| R-31 | An old, validly signed image is replayed to a client to reintroduce fixed vulnerabilities | Low | Medium | Freshness check in the updater (D-027, criterion 4.13) | 4 |
| R-32 | A locally edited container policy file stops receiving key rotations, and updates later fail | Low | Medium | Updater compares the policy with the image default and reports drift before it matters | 4 |
| R-34 | A local unprivileged user reads other users' data from the system journal through the diagnostics service | Medium | Medium | Journal content behind administrator authentication (D-025, criterion 4.14) | 4 |
| R-18 | A privileged ARKON service is exploitable from the user session | Medium | High | No free-form inputs; closed-set validation; sandboxing; SELinux policy; fuzzing; interface review before implementation | 4 |
| R-19 | `/usr` is not verified at boot in 1.0; offline tampering is possible on unencrypted disks | Medium | Medium | Encryption offered at install; sealed boot is the post-1.0 target | 2, post-1.0 |
| R-20 | MOK list is cleared by a firmware update and the NVIDIA module silently stops loading | Medium | Low | Session agent detects it through the hardware probe; fallback desktop still works; health checks do not treat it as an update failure | 4, 7 |

## Legal

| ID | Risk | L | I | Mitigation | Phase |
|---|---|---|---|---|---|
| R-21 | Redistributing the NVIDIA driver, the Steam client or patent-encumbered codecs inside an image is not permitted on the terms assumed; or Fedora or "ARKON" trademark problems | Medium | High | Qualified legal review before any public image (U-13); fallback designs exist (driver and Steam fetched at install instead of shipped) but are worse for users; trademark search (U-07) | 9, ideally 1 |

## Performance

| ID | Risk | L | I | Mitigation | Phase |
|---|---|---|---|---|---|
| R-23 | ARKON shows no measurable performance advantage over Fedora, since it uses the same kernel, Mesa and driver | High | Medium | Expected and acceptable; the value is integration and recovery; publish the data as it is | 8 |
| R-24 | Image size and update download size are too large for typical connections (multilib, NVIDIA userspace, two retained deployments) | Medium | Medium | Measure in Phase 1; layer ordering for reuse; set storage minimums from data | 1 |
| R-25 | Benchmarks from a 4 GB entry-level laptop GPU are read as representative of the product | Medium | Medium | Every published result states the hardware; no extrapolation (criterion 8.6) | 8 |

## Installer (Phase 2 design draft)

Added 2026-11-16. Detail in [phase2/PHASE2_ARCHITECTURE.md](phase2/PHASE2_ARCHITECTURE.md).

| ID | Risk | L | I | Mitigation | Phase |
|---|---|---|---|---|---|
| R-35 | Installation destroys another operating system's boot files because bootc empties the ESP it is given | High if an ESP were shared | High | ARKON-owned ESP always (D-035); validator rule P5; post-execution comparison | 2 |
| R-36 | Building and maintaining an installer exceeds the project's capacity; Phase 2 grows from about 5 to about 15 weeks | High | High | Staged delivery; stage 1 equals an Anaconda-based result; fallback executor; owner decision U-19 | 2 |
| R-37 | bootupd or bootc writes to more than ARKON's ESP when a disk has two, at install or on later bootloader updates | Unknown | High | Same-disk dual boot gated on V-18 and V-19; disabled if either fails | 2 |
| R-38 | Firmware loses or reorders boot entries, or Windows moves itself first; the user cannot reach one of the systems | Medium | Medium | Fallback loader on ARKON's ESP; repair mode; boot-menu documentation; no automatic reordering war | 2 |
| R-39 | Windows demands its BitLocker recovery key after installation or enrolment and the user does not have it | Medium | High | Detection and mandatory acknowledgement before any write; ARKON changes nothing Windows measures; tested on a disposable install (V-22) | 2 |
| R-40 | The partitioner grows or otherwise alters an existing partition | Low | High | Dry-run validation; post-execution comparison; content hashes in tests (D-040) | 2 |
| R-41 | The first-boot handoff fails and no user exists, or the file exposes a password hash | Low | Medium | Interactive fallback; root-only file; deleted after use; hash only | 2 |
| R-42 | Same-disk dual boot and BitLocker behaviour cannot be tested: the only Windows system is the owner's workstation | High | High | Mode stays disabled until a disposable Windows install exists (U-25) | 2 |
| R-43 | An NVMe drive is hidden by Intel RST/VMD firmware mode and the installer finds no disk | Medium | Medium | Controller detection with specific guidance; cannot be bare-metal tested without Intel hardware | 2 |
| R-44 | The Phase 2 design was drawn before Phase 1 exists; boot chain, sizes, bootc flags and signature enforcement are assumed, not known | High | Medium | Mandatory revalidation task P2-00d before any Phase 2 implementation | 2 |
| R-45 | Microsoft's Secure Boot certificate transition leaves some firmware unable to verify the installation medium's shim | Unknown | High | Check on every test machine (V-29); firmware-update guidance; never advise disabling Secure Boot | 2 |
| R-46 | The graphical front end does not render in the live session on some GPUs | Medium | Medium | CLI through the same API; spike P2-S7 | 2 |

## Top risks to watch

Revised after the architecture review on 2026-11-16.

1. R-01 — untested hardware classes.
2. R-02 and R-30 — scope and manual test load against one person's capacity.
3. R-13 — Fedora's release cadence outpacing the roadmap.
4. R-05 — kernel and NVIDIA driver coupling.
5. R-21 — redistribution and trademark.
6. R-04 — boot-chain decision becoming irreversible after release.
7. R-29 — the reference laptop's GPU generation is the one the default driver flavour serves worst.
