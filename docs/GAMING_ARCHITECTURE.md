# ARKON OS — Gaming Architecture

## Goal

After installation on supported hardware, a user signs in to Steam and plays, with the correct GPU driver already active and no terminal use. ARKON integrates existing components; it does not fork or rebuild them.

## Stack

| Layer | Component | Source | In image? |
|---|---|---|---|
| Client | Steam | RPM Fusion nonfree (D-017) | Yes |
| Windows compatibility | Proton, including DXVK and VKD3D-Proton | Downloaded and updated by Steam | No — managed by Steam in the user's library |
| Native runtime | Steam Linux Runtime | Downloaded by Steam | No |
| Vulkan / OpenGL | Mesa, or the NVIDIA userspace driver, in 64-bit and 32-bit builds | Fedora / RPM Fusion | Yes |
| Micro-compositor | Gamescope | Fedora | Yes |
| Measurement overlay | MangoHud | Fedora | Yes |
| Input | Steam's controller udev rules | Fedora | Yes |
| Audio | PipeWire | Fedora | Yes |
| Other launchers (Heroic, Lutris, Bottles) | — | Flathub | No — user installs |
| Community Proton builds | — | User tools | No — unsupported |

DXVK and VKD3D-Proton are not packaged by ARKON. They ship inside Proton, are versioned with it, and a second system-wide copy would create exactly the duplicated-functionality problem the charter forbids.

## Why native Steam (pending U-06)

| | Native RPM | Flatpak |
|---|---|---|
| NVIDIA driver coupling | Uses the host driver directly | Needs a Flatpak NVIDIA runtime whose version equals the host driver exactly; after a driver update it can lag |
| Gamescope, MangoHud | Host packages work directly | Need matching Flatpak extensions |
| Extra library drives, controllers | Work | Need permission overrides |
| Image impact | About 195 i686 packages | None |
| Isolation | None beyond the user account | Sandboxed |

Because ARKON updates the driver through the image, the Flatpak driver-match problem would appear on every driver bump. Native Steam keeps games working through such a bump even when the machine is offline, at the price of multilib packages in the image and a weaker isolation story, recorded in the security document.

The same coupling still applies to every other GPU-using Flatpak, including Flatpak launchers such as Heroic or Lutris. ARKON handles it by fetching the matching Flatpak NVIDIA runtime before the reboot that changes the driver (D-028). Until that fetch succeeds, Flatpak applications run without GPU acceleration.

## Graphics paths

| Hardware | Path |
|---|---|
| AMD or Intel GPU | Mesa (RADV, ANV). Nothing to configure |
| NVIDIA desktop | NVIDIA driver; Wayland session; games run through Xwayland or native Wayland as Proton and the game choose |
| NVIDIA hybrid laptop (R1) | Desktop on the integrated GPU; games need to render on the NVIDIA GPU. ARKON sets the system so that Steam-launched games use the discrete GPU by default. The candidate mechanism is the desktop's existing "run on discrete GPU" launcher support (`switcheroo-control` plus the flag in Steam's launcher entry), so that no ARKON-specific wrapper is needed; it is confirmed and tested in Phase 5, and verified per game by reading the active GPU from MangoHud |

## Gamescope

In 1.0 Gamescope is available as a per-game launch wrapper inside the Plasma session, for resolution scaling, frame limiting and isolating badly behaved fullscreen games.

A console-style session that boots directly into Steam's Big Picture interface is **not** in 1.0. Fedora ships the Gamescope compositor but no such session, so ARKON would have to own session scripts and their interaction with updates and display managers. Gamescope's behaviour on NVIDIA has also been less reliable than on Mesa historically; on R1 it must be tested before any claim is made.

## What ARKON configures

Kept deliberately short. Each item is a default that can be measured.

| Item | Setting | Status |
|---|---|---|
| 32-bit graphics libraries | Present for both Mesa and NVIDIA | Required for Steam and 32-bit games |
| Memory map limit (`vm.max_map_count`) | Fedora's default, already raised for games | Verify value in Phase 5, do not override blindly |
| Controller access | udev rules installed | — |
| Shader pre-caching | Steam's default | — |
| Discrete GPU selection on hybrid laptops | Default to the NVIDIA GPU for games | Mechanism decided in Phase 5 |
| MangoHud | Installed, off by default | Used by the benchmark harness |

Not configured in 1.0, because there is no measured case for them yet (D-021): custom CPU schedulers, kernel command-line tuning, `gamemode` governor switching, transparent hugepage changes, I/O scheduler changes, memory compaction tweaks. Each is a candidate experiment for Phase 8 and ships only if it shows a reproducible gain without a stability cost.

## Compatibility statement

ARKON documents these limits wherever it talks about gaming:

- Not every Windows game runs under Proton. Compatibility is per title and changes over time.
- Games with kernel-level anti-cheat generally do not run. Games using Easy Anti-Cheat or BattlEye run only if the publisher enabled Proton support.
- Ray tracing, DLSS, frame generation, HDR and VRR depend on GPU, driver, Proton and compositor together. None of them can be tested on the current reference hardware, so ARKON makes no claim about them until hardware is available (U-11).
- Launchers other than Steam are the user's responsibility in 1.0.
- Video playback outside games (browser video, media players, hardware decoding) depends on the codec policy, which is undecided (U-16). Proton handles the media formats used inside games itself.
- Games on a Turing laptop may run with the discrete GPU permanently powered, depending on U-14.

## Measurement

Gaming performance is reported, never asserted. The harness lives in `bench/` (Phase 8); Phase 5 only needs enough of it to confirm the stack works.

| Metric | Tool |
|---|---|
| Average FPS, 1% low, frame-time percentiles | MangoHud frame logs |
| CPU and GPU utilisation, temperatures | MangoHud log columns; `nvidia-smi` for NVIDIA |
| Power | RAPL counters for the CPU package; GPU-reported draw; a wall meter if available |
| Memory | MangoHud, `/proc` sampling |

Method: fixed game version, fixed settings file, built-in benchmark scenes where they exist, at least five runs after a warm-up run, AC power, thermal soak, median and spread reported, raw logs published. The comparison target is Fedora KDE 45 with the same driver version on the same machine.

Reference titles are not chosen yet. The set needs one native Vulkan title, one DirectX 11 title and one DirectX 12 title through Proton, each with a repeatable benchmark, and they must be titles the project owner can legally run (open point for Phase 5).

R1 has 4 GB of VRAM. Results from it describe an entry-level laptop and must not be presented as representative of RTX desktops.

## Phase 5 test outline

1. Fresh install on R1; Steam starts and signs in without added packages.
2. One native Linux title and one Proton title launch and render on the NVIDIA GPU.
3. A controller is recognised in Steam without configuration.
4. Gamescope wraps a game on R1; result recorded whether it passes or fails.
5. MangoHud produces a frame log for the benchmark harness.
6. An OS update that changes the NVIDIA driver version leaves Steam and an installed game working, and a GPU-using Flatpak application still has acceleration after the reboot.
7. The same checks on the `arkon` variant on AMD or Intel hardware when available; until then that variant's gaming path is recorded as untested.
