# ARKON OS — Update and Recovery Design

Principle: every system-level change has a way back that does not depend on the changed system working.
Revised 2026-11-16 after the Phase 0 architecture review.

## Update model

The OS is one image. An update downloads a new image, writes it beside the running one, and switches at the next boot. The running system is never modified in place, so an update that fails halfway changes nothing.

| State | Meaning |
|---|---|
| Booted | The deployment currently running |
| Staged | Downloaded and verified; becomes active at the next boot |
| Rollback | The previously booted deployment, kept on disk and in the boot menu |

At least two deployments exist at all times after the first update. A user can pin a deployment so that later updates do not remove it. Each retained deployment keeps a kernel and initramfs in `/boot`, so the number that can be kept is bounded by the `/boot` size chosen in the installer.

## What an update and a rollback cover

| Area | Update | Rollback |
|---|---|---|
| `/usr` (the OS) | Replaced | Restored |
| `/etc` | Merged: local changes win; unchanged files take the new default | Returns to the previous deployment's copy; edits made after the update are left behind |
| `/var`, home directories, Flatpaks, containers, games | Untouched | **Not reverted** |
| shim and GRUB on the EFI partition | Updated separately (below) | **Not reverted** |

The third row is a real limit. If newer software migrates data in `/var` or in a home directory to a format the older version cannot read, a rollback restores the OS but not that data. ARKON cannot prevent this in general; release notes call out any update known to do it, and the Fedora major transition is tested for it.

## Streams and versioning

| Stream | Source | Audience |
|---|---|---|
| `dev` | Every merge to main | Developers |
| `testing` | Nightly rebuild on the pinned base with current repository contents | Testers; soak for at least 7 days before promotion |
| `stable` | A `testing` digest that passed the bare-metal suite, retagged | Everyone |

- Version format: `<fedora-major>.<YYYYMMDD>.<n>`, for example `45.20270115.0`.
- **Stable cadence is undecided (U-17).** Every promotion needs a bare-metal run, which today means one person at one laptop. The cadence is set after Phase 1 shows how long that run takes; security fixes are promoted out of band either way.
- A machine follows one stream of one variant. Changing either is an explicit switch through `arkon-updated`, limited to the names in the image-shipped list.

## Update flow

1. A daily timer (randomised, skipped on metered connections and on battery below a threshold) asks `arkon-updated` to check.
2. A worker unit runs `bootc upgrade`. The signature is verified before anything is written as a deployment (D-026).
3. `arkon-updated` reads the new image's labels and discards the staged deployment if it is older than the booted image (D-027) or if its NVIDIA driver no longer supports the GPU that is present.
4. The session agent notifies the user. On the NVIDIA variant, if the staged driver version differs from the running one, the agent first downloads the Flatpak NVIDIA runtime for the staged version, keeping the current one installed (D-028).
5. The staged deployment becomes active at the next boot. That is any restart or shutdown, not only one started from ARKON Center. ARKON never restarts the machine on its own.
6. Health checks run on the new deployment (below).

Failure handling:

| Failure | Result |
|---|---|
| Network loss, power loss or disk full during download | Nothing staged. The next check resumes. The running system is unaffected |
| Signature invalid, image older than booted, or GPU unsupported by the new driver | Rejected with a stated reason in ARKON Center; nothing staged |
| Power loss between staging and reboot | Either the old or the new deployment boots; both are complete |
| Flatpak NVIDIA runtime could not be fetched (offline, or not yet published) | Update still applies. Flatpak applications run without GPU acceleration until the agent can fetch it; ARKON Center shows this state. Native Steam and games are unaffected |
| New deployment fails to boot or fails health checks | Automatic rollback (below) |

Why step 4 exists: Flatpak applications use an NVIDIA runtime that must equal the host driver version exactly. On package-based systems this mismatch after driver updates is a well-known source of broken applications. Because ARKON knows the next driver version before the reboot, it can fetch the runtime in advance. Stable promotion of an NVIDIA image additionally waits until Flathub has published the matching runtime.

## Bootloader updates (D-033)

shim and GRUB are not part of the A/B scheme. Fedora's `bootloader-update.service` applies newer versions from the image on the following boot. ARKON keeps that mechanism and adds one rule: an image that changes shim or GRUB is promoted only after it has booted on R1 with Secure Boot on. There is no automatic way back from a bad bootloader update; recovery is the installer USB.

## Health checks and automatic rollback

Mechanism: greenboot with the boot counter of the GRUB chain (D-003, V-03). A new deployment gets a limited number of boot attempts. If the required checks do not pass within that count, the boot loader returns to the previous deployment and the failure is recorded.

Required checks (failure counts against the deployment):

| Check | Passes when |
|---|---|
| System reached the graphical target | `graphical.target` active within a time limit |
| Display manager is running | Its unit is active |
| GPU driver matches the variant | On `arkon-nvidia`: the NVIDIA module is loaded, **or** the ARKON certificate is not enrolled (a known, non-update condition that must not cause a rollback loop) |
| Update service is startable | `arkon-updated` answers on the bus, so a rollback can always be requested later |

Advisory checks (logged, shown in ARKON Center, never trigger rollback): network reachable, registry reachable, free disk space above a threshold, Flatpak NVIDIA runtime matches the driver.

After an automatic rollback the failed image digest is remembered, and `arkon-updated` will not stage the same digest again.

Known limits of this mechanism, each of which is a test case before Phase 7 can pass:

- **It cannot see the screen.** A deployment whose display manager runs but shows nothing passes the checks. Manual rollback from the boot menu covers this.
- **Aborted boots count as failures.** The counter decreases on every attempt that does not reach the health check. A user who powers off at the disk-passphrase prompt several times in a row after an update would trigger a rollback of a healthy system. The count and the point at which it is decremented must be chosen so that ordinary behaviour does not do this.
- **A check that fails for reasons unrelated to the update causes a rollback loop**; a check that passes on a broken system is worse than none. Each required check needs a negative test (a deliberately broken image) and a false-positive test.

## Manual recovery ladder

Ordered from least to most invasive. Each step works without the one above it.

| Step | When | How |
|---|---|---|
| 1. Roll back from the desktop | Update works but something regressed | ARKON Center → Roll back, then reboot |
| 2. Choose the previous deployment at boot | New deployment does not reach the desktop | Boot menu lists both deployments. The key that reveals the menu when it is hidden is documented and shown on the boot splash |
| 3. Boot to a text console | Graphics broken on both deployments | Boot menu edit: boot to `multi-user.target`; log in; `bootc rollback` or `arkonctl variant switch` |
| 4. Switch variant | Wrong GPU driver for the hardware | `arkonctl variant switch arkon` from a console |
| 5. Reinstall keeping user data | System state in `/etc` or `/var` is damaged | Boot the installer USB; reinstall to the same disk reusing `/var/home`. **Design not finished** — depends on Anaconda's bootc support for reusing a home subvolume; Phase 2 must confirm or document that this step is a backup-and-restore instead |
| 6. Full reinstall | Disk replaced, bootloader update failed, or unrecoverable | Installer USB |

The installer USB is the rescue medium. ARKON does not build a separate rescue image in 1.0.

## Version transitions

**Fedora major upgrade (N → N+1).** ARKON tracks the current Fedora (D-031). A new stream is published after it passes the bare-metal suite, expected several weeks after the Fedora release because the NVIDIA driver and kernel must settle first. An ordinary update on the old stream delivers a stream list that includes the new one; users then move with one `SwitchStream` action. The old deployment remains as the rollback. Because a major transition is where `/var` and home-directory formats are most likely to change, a round trip (upgrade, use, roll back) is part of its test. The old stream gets security updates for a stated overlap period, then ARKON Center prompts to move.

**Driver branch change.** If NVIDIA drops a GPU generation from the current branch, affected machines must not be updated into a driver that no longer supports their GPU. Step 3 of the update flow enforces this. It has already happened upstream once: Fedora 44 users with GTX 10-series cards were upgraded into a driver that had dropped them. The oldest generation on the current branch, Turing, is the one R1 uses.

**Kernel hold.** If the NVIDIA modules do not build for a new kernel, the NVIDIA variant stays on the previous base and receives userspace updates only, for a bounded time (D-023).

**Boot chain change (D-003).** Moving from the ostree backend to composefs with systemd-boot has no in-place path upstream. If ARKON makes that move after public release, it is a reinstall for every user. The decision is therefore re-examined at the Phase 2 gate and before Phase 9.

## User data

Updates and rollbacks do not protect user data from user error or disk failure. For 1.0 ARKON documents backup options and does not ship its own backup tool. Home-directory snapshots on Btrfs are a candidate for a later release, not a 1.0 promise.

## Open points

| Point | Resolved in |
|---|---|
| greenboot behaviour with bootc on the GRUB chain: counter size and when it is decremented (V-03) | Phase 1 |
| Presence of automatic bootloader updates in the derived image (V-15) | Phase 1 |
| How many deployments to retain by default, given image and `/boot` size | Phase 1 measurements, Phase 2 partition layout |
| Stable cadence (U-17) | After Phase 1 |
| Reinstall-keeping-home path | Phase 2 |
| Overlap period for the previous Fedora major | Phase 9 release policy |
| Download size of a typical update | Phase 1 measurements |
