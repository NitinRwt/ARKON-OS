# ARKON OS — Installer Technology Review

Status: **DRAFT — design only.** Phase 2 is not open: the Phase 1 audit on record is "not approved" ([PHASE1_AUDIT.md](../../PHASE1_AUDIT.md)).
Research date: 2026-11-16. Nothing here was tested; every capability claim comes from upstream documentation or issue trackers and is marked where it is unconfirmed.

## Question

Which mechanism should install an ARKON bootc image onto a physical x86-64 UEFI machine, including machines that already hold Windows, without ever erasing a disk silently?

## What bootc itself provides

| Command | What it does | Fit |
|---|---|---|
| `bootc install to-disk` | Wipes one whole block device, creates ESP (at least 512 MiB) and root, deploys the image, installs the bootloader. Filesystems: xfs, ext4, btrfs. Can use systemd-repart definitions (`--run-repart`). Its TPM-LUKS mode is "not recommended for new deployments" | Whole-disk only. No dual boot, no reuse of anything |
| `bootc install to-filesystem` | The caller prepares and mounts the target root; bootc deploys the image and installs the bootloader. Options for root and `/boot` mount specifications, kernel arguments, target image reference and signature enforcement | **The upstream extension point.** bootc's documentation says this exists so that a distribution can ship its own installer for complex storage and reuse bootc for the rest. There is no library API; the command line is the interface |
| `bootc install to-existing-root` | Converts a running Linux system in place. Wipes `/boot` and reinitialises the ESP's `EFI/` and `loader/` directories | Not an installer for new machines |

Two facts from upstream that shape everything below:

1. A bootc maintainer states that **`bootc install` is always destructive with respect to `/boot` and the ESP**. A change merged in January 2026 empties `/boot` and the ESP during installation.
2. bootc delegates bootloader files and the firmware boot entry to bootupd, which was observed running `efibootmgr --create` during installation.

Consequence: pointing bootc at an ESP that Windows or another Linux uses would destroy that system's boot files. Any design that shares an existing ESP is unsafe today (decision D-035).

## Candidates

### A. Anaconda with its own interface

Fedora's installer. Since late 2025 it has a `bootc` kickstart command: Anaconda creates the storage, then runs the equivalent of `bootc install to-filesystem`.

| Aspect | Finding |
|---|---|
| Maturity of the bootc path | New. Demonstrated on Fedora Rawhide in December 2025; Technology Preview in RHEL 10.2 (June 2026) |
| Documented limits | No layouts spanning several disks; no arbitrary mount points; a user report of "does not yet support user-specified mount points" for `/boot/efi`; authenticated registries need a workaround |
| Dual boot | The web interface has "share disk" and "reclaim space" options for package-based Fedora. Open bug reports describe it reformatting an existing ESP and breaking other systems' boot entries. Combined with bootc's ESP behaviour this is the dangerous case |
| Atomic desktops | Fedora's own Atomic documentation says dual boot and manual partitioning are "not fully functional", and that the installer accepts incompatible custom layouts without an error. Web-interface installers for Atomic variants are still a roadmap item in Fedora 45 |
| Interactive use with a bootc payload | Shown in osbuild's documentation through a defaults kickstart. Not confirmed for the web interface |
| ARKON-specific checks | Hard to add. Validation between Anaconda's storage plan and its execution needs an Anaconda add-on, which the web interface does not support in the same way |
| Cost | Lowest for a whole-disk install |

### B. ARKON front end, Anaconda as a non-interactive engine

ARKON collects the choices, writes a complete kickstart, and Anaconda executes it.

- Uses the documented, kickstart-driven bootc path.
- ARKON controls what is shown and confirmed.
- Inherits every limit in A: single disk, no extra mount points, no control over which ESP bootc is given, so same-disk dual boot and reinstall-keeping-home remain out of reach.
- Carries the full Anaconda stack on the ISO for a job that is then mostly orchestration.

### C. ARKON front end and orchestrator over systemd-repart and `bootc install to-filesystem`

ARKON owns the flow. Partitioning, formatting and encryption are done by `systemd-repart` from declarative definition files; deployment and bootloader by bootc.

| Aspect | Finding |
|---|---|
| Upstream position | This is the use bootc documents `to-filesystem` for |
| Safety properties of systemd-repart | Declarative; by default it only adds partitions in free space and does not delete or shrink existing ones; erasing requires an explicit mode; it has a dry-run that reports the planned changes in machine-readable form, so the plan shown to the user and the plan executed can be the same object |
| Precedent | Universal Blue announced in 2025 that it was moving from Anaconda to an installer of this shape (Readymade) because Anaconda's new interface was not available for its images |
| What ARKON must own | The wizard, storage classification, plan validation, the first-boot handoff, error handling and recovery. It does **not** own code that edits partition tables or makes filesystems |
| Unknowns | The exact `to-filesystem` invocation for Btrfs subvolumes and LUKS; what bootupd writes when two ESPs exist; repart behaviour beside existing partitions (V-17…V-20) |
| Cost | Highest of the viable options. See the roadmap impact below |

### D. Readymade (Fyra Labs)

A Rust installer that is a front end to systemd-repart, used by Ultramarine Linux.

- Its README describes it as work in progress, shows no releases, and says a dedicated backend is "planned". No bootc backend, dual boot, encryption or Secure Boot handling is documented there.
- Forum posts from 2025 describe a bootc backend that wraps `bootc install`; this could not be confirmed from the project itself.
- GPL-3.0-or-later.
- Verdict: the right shape, not yet something to depend on. Worth watching and worth reading before building C.

### E. Calamares

Distribution-independent installer framework. No evidence of bootc or container-image deployment support was found. Its partitioning module performs its own destructive operations, which would have to be reconciled with bootc. Not pursued.

### F. `bootc install to-disk` alone

Fine for the project's own test installs in Phase 1 and for unattended whole-disk provisioning. It has no interface, no detection and no dual boot. It remains available as a documented expert path, not as the product installer.

## Building the installation medium

| Tool | Status | Use |
|---|---|---|
| `image-builder` (osbuild) | Current. Absorbed `bootc-image-builder`, which was archived on 2026-06-18 | Produces the ISO |
| `generic-iso` image type | Current. Turns a bootc container into a bootable live medium; `--bootc-installer-payload-ref` copies an image into the ISO's container storage for offline installation | **Selected.** The live environment is itself an ARKON image with the installer added |
| `bootc-installer` image type | Listed under "historical" on one osbuild page and without that label on another; the documentation says to prefer `generic-iso` | Not used |
| `anaconda-iso` image type | Historical; not available in `image-builder` | Not used |

## Comparison

| Requirement | A. Anaconda UI | B. Anaconda engine | C. repart + bootc | D. Readymade |
|---|---|---|---|---|
| Clean install, whole disk | Yes | Yes | Yes | Likely |
| Windows on another disk left alone | Yes if the user picks correctly | Yes | Yes | Unknown |
| Same-disk dual boot without touching the Windows ESP | No | No | Possible, unverified | Unknown |
| Manual partitioning that cannot produce an unbootable bootc system | No | Limited | Constrained by design | Unknown |
| Reinstall keeping home | No | No | Possible, unverified | Unknown |
| ARKON checks before anything is written | Weak | Yes | Yes | Would need changes |
| Plan shown equals plan executed | No | Approximately | Yes (repart dry-run) | Unknown |
| Maturity for bootc today | Preview | Preview | Building blocks are stable; integration is ARKON's | Work in progress |
| ARKON code to write and maintain | Small | Medium | Large | Medium, on an unstable base |

## Recommendation

**Option C**, delivered in stages, with **option B kept as a fallback for the whole-disk mode only** if the first spike shows C is not workable. This replaces D-012 (Anaconda) and is recorded as proposed decision D-034. It is the owner's call (U-19), because it is the single largest scope increase since Phase 0.

Why, in one paragraph: the stated priorities include existing Windows installations and the stated rules include no silent erasure and preserving existing boot entries. The only route that can give ARKON control over which ESP bootc touches, and that can show the user the exact plan before execution, is the one bootc's own documentation designates for distribution installers. Anaconda's bootc path is a preview with documented gaps exactly where those requirements sit.

What argues against it, stated plainly:

- It contradicts the charter principle of not reinventing existing infrastructure unless there is a measurable reason. The reason offered here is a documented capability gap, not a measurement.
- The project has one engineer, no Phase 1 implementation yet, and an installer is a classic place for distributions to lose months.
- Its hardest mode, same-disk dual boot, depends on behaviour (V-18, V-19) that may turn out to be unsafe, in which case C's advantage over B shrinks to plan visibility and reinstall-keeping-home.

Staging limits that risk: the first deliverable is whole-disk installation only, which is also all that option B could have delivered.

## Roadmap impact

The Phase 0 roadmap budgeted four to six weeks for Phase 2 on the assumption of Anaconda. With option C the estimate is roughly 14–17 engineer-weeks across all stages (11–13 without same-disk dual boot), of which the spikes take about two and the first stage (whole-disk install from a signed ISO) about four to five. The breakdown is in [PHASE2_TASKS.md](PHASE2_TASKS.md). These figures are unvalidated.

## Unverified items raised by this review

Listed with identifiers in [PHASE2_ARCHITECTURE.md](PHASE2_ARCHITECTURE.md#verification-items). The ones that decide the recommendation are V-17 (the `to-filesystem` invocation works on the intended layout), V-18 and V-19 (what bootc and bootupd do to ESPs and firmware entries), and V-20 (repart leaves existing partitions unchanged).
