# ARKON OS — System Architecture

Decision references (D-nnn) point to [DECISIONS.md](../DECISIONS.md). Verification items (V-nn) are listed in [PROJECT_STATE.md](../PROJECT_STATE.md).
Revised 2026-11-16 after the Phase 0 architecture review.

## 1. Upstream component status (checked 2026-11-16)

| Component | Status found | Consequence |
|---|---|---|
| Fedora 45 | Released 2026-11-03. Kernel 7.2. KDE edition ships Plasma 6.8.2 | Current development base (D-001, D-031) |
| Fedora Atomic Desktop bootc images | Atomic SIG calls its bootable container images unofficial; "ready for general consumption" is still in progress | Derive from `fedora-bootc`, not from Kinoite |
| bootc | composefs backend documented as stable for upgrades and core functions; ostree backend is the default | See D-003 |
| systemd-boot under bootc | Supported with the composefs backend only. Gaps: no boot counting, kernel arguments ignored on the BLS install path, no in-place migration from ostree, single ESP | Deferred (D-003) |
| Bootloader updates | Fedora applies shim and GRUB updates through `bootloader-update.service` (bootupd); on by default for Atomic Desktops since Fedora 41, with a Fedora 43 change extending it to bootc systems | D-033, V-15 |
| greenboot | Rust rewrite (0.16.x) is packaged in Fedora and supports bootc systems | D-010, V-03 |
| bootc-image-builder | Archived 2026-06-18, merged into `image-builder`. `generic-iso` is the current installer type | D-012 |
| KDE Plasma 6.8 | Wayland only; X11 apps through Xwayland | D-013 |
| NVIDIA driver | Current branch 615.x supports Turing and newer. The 580 branch is the last for Maxwell, Pascal and Volta | D-005 |
| NVIDIA open kernel modules | Runtime power-down (RTD3) is documented for Ampere and newer only; the closed modules support it on Turing too | D-032, U-14 |
| RPM Fusion on Fedora 45 | Carries current, 580xx, 470xx and 390xx NVIDIA branches, and Steam | D-007 |
| nvidia-container-toolkit | Packaged in Fedora. From 1.18 a `nvidia-cdi-refresh` service regenerates the CDI specification | D-018 |
| Flatpak NVIDIA runtime | Must equal the host driver version; mismatches after driver updates are a recurring upstream problem | D-028 |
| Podman | 6.x. slirp4netns and the BoltDB backend are removed | Use pasta networking only |
| i686 packages | Still in Fedora 45; the removal proposal was withdrawn | Native Steam remains possible |
| Tauri | 2.x line, built on `webkit2gtk-4.1`, which Fedora still packages. Exact current version not confirmed (V-08) | D-016 |

## 2. Image composition

One multi-stage `Containerfile` builds every variant. Each stage is a layer group with one responsibility, so a change to a late stage does not rebuild early ones and variants share every layer below the graphics stage.

```
quay.io/fedora/fedora-bootc:<N>           (pinned by digest; carries the kernel)
  └─ stage base      : arkon-release, repo allow-list, signature policy
      └─ stage desktop : KDE Plasma, Wayland, PipeWire, fonts, core apps, branding
          └─ stage gaming : Steam, Gamescope, MangoHud, controller udev rules
              ├─ target arkon         : Mesa only, ARKON services, lint
              └─ target arkon-nvidia  : + signed kmod RPMs, userspace driver,
                                          nvidia-container-toolkit, kargs
```

What is pinned and what is not (D-023):

- **Pinned:** the base image, by digest. The kernel is whatever that digest contains. No build step installs, upgrades or removes a kernel package; a test enforces this.
- **Not pinned:** packages installed on top come from the Fedora and RPM Fusion repositories as they are at build time. Every build records them in a lock file (name, epoch, version, release, architecture, source repository) and an SBOM. Builds are traceable, not bit-for-bit reproducible.
- A scheduled job proposes base-digest bumps as pull requests. A bump is a kernel change and triggers a module rebuild.

Other rules:

- Package lists live in plain-text manifests, one per stage. Build scripts install exactly those lists.
- `bootc container lint` runs as the last build step of each target.
- No compiler, SDK or language toolchain is installed in a final image. Anything needed only to build is used in a throwaway stage.
- Kernel arguments are delivered as files in `/usr/lib/bootc/kargs.d/` (V-10), never by editing bootloader configuration on the client.
- Each image carries labels with its ARKON version, build timestamp, variant, and (NVIDIA variant) driver version and supported-device table. The updater reads these before staging.

Filesystem layout on an installed system:

| Path | Nature |
|---|---|
| `/usr` | Read-only, from the image |
| `/etc` | Mutable; three-way merged across updates. ARKON ships as little as possible here, because a locally modified file stops receiving the image's changes |
| `/var` (includes `/var/home`) | Mutable; never touched by updates **and never reverted by a rollback** |
| `/opt`, `/usr/local` | Redirected into `/var` |

## 3. NVIDIA kernel module pipeline

```
kernel NEVRA of the pinned base ──► kmods/ build container
                          1. obtain kernel-devel for exactly that NEVRA,
                             Fedora-signed (repository, or Fedora's build
                             system if the repository has moved on — V-13)
                          2. build the kernel modules from the RPM Fusion source
                             package, in the flavour given as a parameter
                          3. sign each .ko with the ARKON module key (sign-file)
                          4. emit kmod RPMs + a manifest (kernel NEVRA, driver
                             version, flavour, supported PCI IDs)
                              │
image build (arkon-nvidia) ◄──┘  refuses to proceed if the manifest's kernel
                                 does not equal the image's kernel
```

**Flavour (D-032).** NVIDIA ships open and closed kernel modules; they cannot be installed together. Open is required for RTX 50 and is the default. Closed is the only flavour with runtime power-down on Turing laptops such as R1. The pipeline takes the flavour as an input so that a closed-module variant can be added without redesign. Whether one is added is U-14, decided by measurement on R1 in Phase 1.

**Kernel and driver move together.** If the driver does not build for the kernel in a new base digest:

- the `arkon` variant advances to the new base;
- `arkon-nvidia` stays on the last good base digest and receives userspace-only package updates;
- this divergence is an incident with a time limit set in Phase 1, not a steady state. While it lasts the variants do not share base layers.

## 4. Build and release pipeline

```
pull request ─► lint · unit tests · image build (unsigned) · container-level tests
                 · optional QEMU boot smoke test (U-03)
merge to main ─► build all variants ─► sign (cosign) ─► push :dev
                 ─► generate SBOM + lock file ─► attach as attestations
nightly ───────► rebuild on the pinned base with current repository
                 contents ─► push :testing
promotion ─────► bare-metal test record in TEST_RESULTS.md
                 ─► (NVIDIA) matching Flatpak runtime exists on Flathub
                 ─► retag :stable
release ───────► build installer ISO from the :stable digests ─► checksum + signature
```

Tags per variant: `<N>-dev`, `<N>-testing`, `<N>-stable`, and immutable `<N>.YYYYMMDD.n`, where N is the Fedora major.
Promotion to `stable` is a retag of an already-tested digest. Stable images are never rebuilt.

Practical limits of the build system of record (D-020): GitHub-hosted runners have limited disk, so desktop-image and ISO jobs need space reclaimed before building; ISO builds need rootful Podman. Both are known to work for comparable projects but are unproven for ARKON until Phase 1.

## 5. ARKON components

Design rules (D-015, D-024, D-025):

- One responsibility per component. No general "run this command as root" interface exists anywhere.
- Services are D-Bus activated and exit when idle. Only timers and the session agent persist.
- Every mutating method is authorised by a polkit action.
- The process that parses untrusted D-Bus input never holds the privileges needed to change the system.
- Interface definitions (D-Bus introspection XML in `services/dbus/`) are the contract. Rust and TypeScript types are generated from them.
- Interfaces are versioned in the name (`…Update1`). A breaking change creates `…Update2`.
- Components call each other only over D-Bus or by running a documented CLI. No crate depends on another service's crate.

Bus names use the placeholder prefix `org.arkonos` until U-07 is answered.

| Component | Kind | Runs as | Responsibility |
|---|---|---|---|
| `arkon-hw` | Rust library | Caller | Hardware detection and the GPU-to-variant rule. The only implementation of both |
| `arkon-hwprobe` | CLI over `arkon-hw` | Caller, unprivileged | Prints the capability report as JSON. Used by the installer, health checks and scripts |
| `arkon-updated` | D-Bus service | Unprivileged system user, fully sandboxed | Validates requests, checks polkit, holds update state, starts worker units, relays progress |
| `arkon-bootc@.service` | Fixed worker units | root | Run exactly one predefined `bootc` operation each (`upgrade`, `rollback`, `switch` to a validated target). No caller-supplied arguments reach the command line except values the front has already matched against the image-shipped list |
| `arkon-secureboot` | D-Bus service, one-shot | root, sandboxed | Queue the ARKON module certificate for MOK enrolment |
| `arkon-diag` | D-Bus service | Dedicated user | Build a diagnostics bundle and hand it back as a file descriptor |
| `arkon-session-agent` | User-session service | The logged-in user | Listens for update and driver-state signals; shows desktop notifications; fetches the matching Flatpak NVIDIA runtime (D-028) |
| `arkonctl` | CLI | The user | Same operations as ARKON Center; the test harness for the services |
| ARKON Center | Tauri application | The user | Graphical front end |

Why there is no hardware daemon: everything it would report is readable by an unprivileged process, and the installer needs the same logic in an environment where no ARKON service runs. A library with a CLI serves every caller with one implementation and removes a bus-exposed service.

How `arkon-updated` starts a worker without being root: a polkit rule lets its service user start only the `arkon-bootc@` instances, and the set of instance names is fixed in the unit template.

Deliberately not built, because upstream already does it: power profiles (tuned with its power-profiles interface, already surfaced in Plasma), display and audio settings (Plasma), firmware updates (fwupd through Discover), Flatpak management (Discover), boot health checking (greenboot), bootloader updates (bootupd).

### Interface sketch

Shapes only; the authoritative definitions are written in Phase 4.

```
arkon-hwprobe --json → Capabilities
    cpu        { vendor, family, model, cores, threads, cpufreq_driver }
    gpus[]     { vendor, pci_id, generation, bound_driver, is_boot_display }
    hybrid     bool
    secureboot { enabled, arkon_cert_enrolled }
    recommended_variant   one of the image-shipped variant names
    running_variant       string

org.arkonos.Update1
  GetStatus() → { booted, staged?, rollback?, stream, last_check, blocked_reason? }
  Check()                       polkit: update.check        (active user)
  Stage()                       polkit: update.stage        (active user)
  Rollback()                    polkit: update.rollback     (admin auth)
  SwitchVariant(variant)        polkit: update.switch       (admin auth)
  SwitchStream(stream)          polkit: update.switch       (admin auth)
  signal Progress(phase, fraction), StatusChanged()

org.arkonos.SecureBoot1
  QueueEnrollment() → one_time_password     polkit: secureboot.enroll (admin auth)

org.arkonos.Diagnostics1
  CreateBundle(level) → fd
      level "basic"   hardware report, update state, versions
                      polkit: diag.basic   (active user)
      level "full"    adds redacted system journal excerpts
                      polkit: diag.full    (admin auth)
```

Rules that follow from the review:

- `SwitchVariant` and `SwitchStream` accept only names listed in `/usr/share/arkon/streams.json`, which is part of the signed, read-only image. A new Fedora major becomes selectable when an ordinary update delivers a list that includes it. A caller can never supply an image reference.
- `Stage` refuses an image whose build timestamp is older than the booted image's (D-027), and an NVIDIA image whose device table does not include the present GPU.
- `QueueEnrollment` generates the one-time password itself and returns it for display. It is passed to `mokutil` on standard input, never on a command line.
- `CreateBundle` returns a descriptor to a file in the service's private directory. The service never writes into a home directory.

## 6. ARKON Center

- Tauri 2 shell, React + TypeScript front end, unprivileged, runs as the logged-in user.
- The Tauri Rust side is a thin D-Bus client and links `arkon-hw` for the hardware view. The front end can call only a fixed set of typed commands; it has no shell, filesystem or network capability, and loads no remote content. Tauri's self-update mechanism is not used; the application is updated with the image.
- Shipped as an RPM inside the image, built against the image's own `webkit2gtk-4.1`.
- Phase 4 opens with a rendering spike on NVIDIA + Wayland (D-016, R-09).

## 7. Data flows

**Update.** Timer → `arkon-updated` validates and starts `arkon-bootc@upgrade` → bootc verifies the signature (D-026) → front checks freshness and device support from the image labels → deployment staged → `StatusChanged` → session agent notifies; on the NVIDIA variant it first fetches the Flatpak runtime matching the staged driver → the deployment becomes active at the next boot, whenever the user restarts or shuts down → health checks pass and the boot is marked good, or fail and the previous deployment is selected ([UPDATE_AND_RECOVERY_DESIGN.md](UPDATE_AND_RECOVERY_DESIGN.md)).

Upstream's `bootc-fetch-apply-updates.timer` reboots the machine on its own after applying an update. ARKON masks it; a desktop must not restart without the user.

**Hardware change.** User swaps a GPU → at next login the session agent runs the probe and sees `recommended_variant ≠ running_variant` → notification → ARKON Center offers `SwitchVariant` → new image staged → reboot. Until then the system runs on the fallback path described in [HARDWARE_SUPPORT_MATRIX.md](HARDWARE_SUPPORT_MATRIX.md).

**Installation.** Installer environment runs `arkon-hwprobe --json` → kickstart selects the image reference for `recommended_variant` → Anaconda installs it ([DEVELOPMENT_ROADMAP.md](DEVELOPMENT_ROADMAP.md), Phase 2).

**GPU container.** Boot → `nvidia-cdi-refresh` writes the CDI specification for the loaded driver → `podman run --device nvidia.com/gpu=all …` → container sees the GPU with no host CUDA install ([AI_DEVELOPER_ARCHITECTURE.md](AI_DEVELOPER_ARCHITECTURE.md)).

## 8. Open architecture points

| Point | Where it is decided |
|---|---|
| Boot chain after the systemd-boot spike | D-003, Phase 1 |
| Closed-module variant for Turing laptops | U-14, Phase 1 measurement on R1 |
| Maximum duration of a kernel-hold divergence between variants | Phase 1 |
| SELinux policy for ARKON services (custom module versus a generic confined domain) | Phase 4 design step |
| Whether KDE Discover can show bootc update state, making part of ARKON Center redundant | Phase 3 discovery |
| Core application set shipped as RPMs | U-15, Phase 3 |
| Toolbx or Distrobox as the default development-container front end | Phase 6 design step |
| One ISO with every variant or one ISO per variant | V-06, Phase 2 |
| Partition layout: ESP and `/boot` sizes for the number of retained deployments | Phase 2 design step |
