# ARKON OS — Hardware Support Matrix

## Support tiers

| Tier | Meaning | Release rule |
|---|---|---|
| 1 — Tested | The project owns the hardware and runs the bare-metal suite on it for every stable promotion | A failure blocks the release |
| 2 — Expected | Uses the same driver stack as a Tier 1 device; not tested by the project | Bugs accepted and fixed when reproducible |
| 3 — Best effort | Boots to a desktop with open drivers; gaming or compute performance not promised | Bugs accepted, no commitment |
| Unsupported | Known not to work, or outside scope | Installer warns or refuses |

No hardware may be described publicly as "supported" unless it is Tier 1 or Tier 2.

## Tier 1 today

| ID | Machine | CPU | GPU | Notes |
|---|---|---|---|---|
| R1 | ASUS TUF Gaming A15 FA506IH | Ryzen 5 4600H (Zen 2) | GTX 1650 Mobile (Turing, 4 GB) + Radeon integrated | Hybrid graphics laptop, 16 GB RAM, NVMe |

Gaps that Tier 1 does not cover, and which therefore remain unproven until hardware is added (U-11, R-01):

- any RTX GPU, including ray tracing, DLSS and more than 4 GB of VRAM;
- a desktop where the NVIDIA GPU drives the display directly;
- an AMD discrete GPU;
- an Intel GPU or Intel CPU;
- high-refresh, VRR or HDR displays;
- a current-generation Ryzen (Zen 4/5).

## Platform requirements

| Item | Minimum | Recommended |
|---|---|---|
| Architecture | x86_64 | — |
| Firmware | UEFI | UEFI with Secure Boot enabled and TPM 2.0 |
| RAM | 8 GB | 16 GB or more |
| Storage | 64 GB SSD | 256 GB NVMe or larger (two OS deployments plus games) |
| Install target | A whole disk | A disk that holds no other operating system |

Storage minimums are provisional until Phase 1 measures real image sizes.

## CPU

| CPU family | Tier | Notes |
|---|---|---|
| AMD Ryzen, Zen 2 and newer | 2 (R1 is Tier 1) | Charter priority |
| AMD Ryzen, Zen and Zen+ | 3 | Works with the Fedora kernel; untested |
| Intel Core, 8th generation and newer | 3 until tested | Hybrid-core scheduling behaviour unmeasured |
| Anything without UEFI or below x86-64 baseline | Unsupported | |

No CPU-specific tuning ships in 1.0 (D-021). AMD support in 1.0 means the following, all inherited from Fedora:

- **Microcode:** AMD microcode from `linux-firmware`, loaded early at boot.
- **Frequency scaling:** whichever driver the Fedora kernel selects. `amd-pstate` is used where firmware exposes the required CPPC interface; older platforms, possibly including R1's Zen 2 mobile chip, fall back to `acpi-cpufreq`. The driver in use is recorded for every Tier 1 device instead of being assumed.
- **Sensors and power counters:** in-tree `k10temp` for temperatures; package energy counters through the kernel's RAPL interface, which is readable by root only, so the benchmark harness reads it through an explicit privileged step.
- **Newer features** such as preferred-core scheduling and the 3D V-Cache mode driver exist in the kernel for Zen 4/5 parts. They are untested by the project and no claim is made.
- **Sleep:** suspend behaviour differs between S3 and s2idle platforms. Tested on R1 only.

## GPU to image variant

The NVIDIA driver is not uniform across generations. This table is the source of truth for the variant rule, which is implemented once in the `arkon-hw` library and used by the installer and the running system alike (D-024).

| GPU family | Variant | Driver | Tier |
|---|---|---|---|
| NVIDIA RTX 50 series | `arkon-nvidia` | Current branch, open kernel modules (the only modules that support this series) | 2 — untested by the project |
| NVIDIA RTX 30/40 series (Ampere, Ada) | `arkon-nvidia` | Current branch, open kernel modules | 2 — untested by the project |
| NVIDIA GTX 16 and RTX 20 series (Turing) | `arkon-nvidia`, or a closed-module variant if U-14 adds one | Current branch. Open modules work but NVIDIA documents runtime power-down for them on Ampere and newer only; the closed modules support it on Turing | 2 (R1 is Tier 1) |
| NVIDIA GTX 900/10 series, Titan V (Maxwell, Pascal, Volta) | `arkon` | nouveau/NVK from Mesa. The proprietary 580 branch is not shipped (U-09) | 3 — desktop use only, not a gaming target |
| NVIDIA GTX 700 series and older | `arkon` | nouveau | Unsupported for gaming |
| AMD Radeon RX 5000 and newer, and Ryzen integrated graphics | `arkon` | Mesa RADV / radeonsi | 2 |
| AMD Radeon RX 400/500, Vega | `arkon` | Mesa | 3 |
| Intel Xe integrated, Arc | `arkon` | Mesa ANV / iris | 3 until tested |

Selection is by PCI vendor and device ID against a table generated from the driver's own supported-device list at build time, not by product-name matching.

## Hybrid graphics laptops

R1 is this class, so it is designed for, not treated as an edge case.

- The integrated GPU drives the panel. The NVIDIA GPU renders on demand through PRIME render offload.
- The `arkon-nvidia` variant is selected whenever a supported NVIDIA GPU is present, even if it is not the boot display.
- The NVIDIA GPU should power down when idle. On R1's Turing GPU this is in doubt with the default open modules, which NVIDIA documents as supporting runtime power-down on Ampere and newer only. If it does not power down, the laptop runs hotter and drains its battery faster on ARKON than it could. Phase 1 measures idle power and suspend behaviour on R1 under both module flavours, and that measurement decides U-14.
- External display outputs wired to the NVIDIA GPU are a known source of laptop-specific behaviour. Tested on R1 only.

## Failure and fallback behaviour

| Situation | Behaviour |
|---|---|
| NVIDIA variant, ARKON certificate not enrolled, Secure Boot on | The kernel refuses the module. Hybrid laptop: desktop runs on the integrated GPU. Desktop PC: desktop runs on the firmware framebuffer with software rendering. ARKON Center shows how to enrol. Secure Boot is not disabled. |
| NVIDIA GPU removed or replaced with AMD/Intel | Mesa drivers in the image drive the new GPU. The session agent recommends switching to `arkon`. |
| AMD/Intel machine gains a supported NVIDIA GPU | Desktop runs on nouveau or the firmware framebuffer. The session agent recommends switching to `arkon-nvidia`. |
| Unsupported NVIDIA generation | Installer chooses `arkon` and states that gaming performance is not supported on this GPU. |
| Legacy BIOS boot | Installer refuses. |

The fallback paths require that the NVIDIA variant does not make the system unbootable when the module fails to load. That is an explicit Phase 1 test (V-11).

Only the hybrid-laptop row of that fallback can be tested on R1. The desktop-PC row, where the Plasma session must run on the firmware framebuffer with software rendering, is a design intention that stays unverified until a desktop with an NVIDIA GPU is available.

## Peripherals and other hardware

| Class | Position for 1.0 |
|---|---|
| Wi-Fi, Bluetooth, audio | Whatever the Fedora 45 kernel and `linux-firmware` support. No out-of-tree network drivers |
| Game controllers | Steam's udev rules shipped in the image. Xbox, PlayStation and Switch Pro controllers expected; tested only with what the project owns |
| Displays | VRR and HDR depend on GPU, driver and Plasma support together. Untested on R1's panel; no claim made |
| Fingerprint readers, RGB control, fan control | Not in scope. Vendor tools may be installed by the user |
| Firmware updates | fwupd, as in Fedora |

## How a device gains a tier

1. Run the bare-metal suite (`tests/hardware/`) on the device.
2. Commit the result to TEST_RESULTS.md with the exact hardware IDs and image digest.
3. Tier 1 requires the project to keep access to the device for every stable promotion. Otherwise it is Tier 2 with a dated test record.
