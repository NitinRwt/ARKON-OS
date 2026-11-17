# ARKON OS — AI and Developer Architecture

## Principle

The host stays small and identical on every machine. Toolchains, SDKs, CUDA and frameworks live in containers that the user chooses, pins and throws away. Nothing a project needs is installed into the OS image.

This follows from the image-based design: `/usr` is read-only, and layering packages on the client would break the property that every machine on a given image is the same.

## What the host provides

| Component | Variant | Purpose |
|---|---|---|
| Podman 6 (rootless, pasta networking) | Both | Container engine |
| Development-container front end (Toolbx or Distrobox — chosen in Phase 6) | Both | Mutable, home-integrated development environments |
| git | Both | Needed before any container exists |
| NVIDIA driver (kernel modules and userspace) | `arkon-nvidia` | GPU access |
| nvidia-container-toolkit | `arkon-nvidia` | Generates the CDI specification that exposes the GPU to containers |
| Flatpak | Both | Editors and IDEs |

Not on the host: compilers, language runtimes beyond what the OS itself needs, CUDA toolkit, cuDNN, PyTorch, TensorFlow, Docker, Kubernetes tooling.

Pulling upstream images works because ARKON's signature requirement is scoped to its own registry path and leaves the system's default container policy alone (D-026).

Podman 6 removed slirp4netns and the BoltDB backend. ARKON starts on Podman 6, so there is no migration concern, but guides written for older Podman versions that reference slirp4netns will not apply.

## Development workflow

| Need | Mechanism |
|---|---|
| Day-to-day shell with a compiler and package manager | A development container with the home directory mounted; packages installed inside it with `dnf` or `apt` |
| Reproducible project environment | A `Containerfile` or devcontainer definition committed with the project |
| Services for local testing (databases, queues) | Rootless Podman, optionally as user-level systemd units (Quadlet) |
| Tools that expect Docker | Podman's Docker-compatible socket, enabled per user on request; not on by default |
| Editor or IDE | Flatpak. Editor integration with host containers from inside a Flatpak sandbox has known friction; Phase 6 must test the common editors and document what works instead of assuming it |

ARKON publishes a small number of thin development images only if Phase 6 shows upstream images are insufficient. Maintaining language images is a standing cost and duplicates work that upstream projects already do.

## GPU containers

```
boot ─► NVIDIA modules load ─► nvidia-cdi-refresh.service
                                 writes the CDI specification for the
                                 driver version that is actually loaded
user ─► podman run --device nvidia.com/gpu=all <image>
          └─ container receives the device nodes and matching
             driver libraries from the host; CUDA comes from the image
```

Design points:

- The CDI specification is runtime state. It is generated at boot and after driver changes, never baked into the image, because it must describe the driver that is loaded.
- The CUDA version is a property of the container image, not of the OS. Two projects can use different CUDA versions on the same machine. The constraint runs one way: the host driver must be new enough for the container's CUDA version. ARKON tracks NVIDIA's current driver branch, which keeps that satisfied for current CUDA releases.
- SELinux stays enforcing. Device access for containers may need a specific SELinux boolean or label; Phase 6 determines the narrowest setting that works and ships that, not a blanket relaxation.
- GPU access is rootless. No ARKON service is involved at run time.
- After a rollback to a deployment with an older driver, the CDI specification is regenerated on that boot; containers keep working as long as the older driver still satisfies their CUDA version.
- GPU generations age out of CUDA as well as out of the driver. CUDA 13 already excludes everything older than Turing. A machine can therefore have a working driver and still be unable to run current framework images; ARKON documents the pairing it tested and does not promise more.
- Container storage lives in `/var`, which a rollback does not revert. A container engine upgrade that changes its storage format would not be undone by rolling the OS back; this is tested at each Fedora major transition.

## Frameworks

| Framework | Delivery | Note |
|---|---|---|
| PyTorch | Upstream container images with CUDA, or `pip` inside a development container | Wheels bundle their own CUDA runtime libraries. Current builds use CUDA 13, whose minimum GPU generation is Turing, so R1 is at the floor |
| TensorFlow | Upstream GPU container images, or `pip` with the CUDA extra inside a container | TensorFlow pins specific CUDA and cuDNN versions per release; using the upstream image avoids mismatches. The current state of TensorFlow's Linux GPU support was **not confirmed** during Phase 0 research (V-16); the charter lists it, so Phase 6 tests it and reports what it finds |
| CUDA toolkit for native builds | NVIDIA's CUDA development container images | Never installed on the host |
| Local model runners | User's choice, in a container or Flatpak | Not preinstalled |

ARKON does not rebuild or redistribute these frameworks. It tests that documented upstream images work on a fresh install and records the image tags and digests it tested.

## Non-NVIDIA compute

| Path | 1.0 position |
|---|---|
| AMD ROCm | Not supported. No test hardware, and ROCm supports a narrow list of Radeon GPUs |
| Intel oneAPI / OpenVINO on Arc | Not supported. No test hardware |
| CPU-only | Works in any container; no ARKON involvement |
| Vulkan compute | Whatever Mesa provides |

## Limits of the reference hardware

R1's GPU has 4 GB of VRAM. That is enough to prove the plumbing (device visible, CUDA initialises, a small model trains and infers) and not enough to say anything about real AI workloads. Throughput numbers from R1 are baseline regression data, not marketing data. Meaningful AI benchmarks need an RTX GPU with substantially more memory (U-11).

## Measurement

| Metric | Method |
|---|---|
| Inference throughput | A fixed small vision model at fixed batch size and precision in a pinned PyTorch image; images per second after warm-up |
| Token throughput | A fixed quantised language model that fits in 4 GB, fixed prompt and settings; tokens per second |
| Container GPU overhead | The same workload in a container on ARKON and on Fedora KDE 45 with the same driver |
| Cold-start time | `podman run` to first CUDA call |

Model choices are fixed in Phase 6 and recorded with their checksums.

## Phase 6 test outline

1. Fresh `arkon-nvidia` install on R1: `podman run --device nvidia.com/gpu=all` with a CUDA base image prints the GPU from `nvidia-smi`, rootless, SELinux enforcing, with no manual setup.
2. The upstream PyTorch GPU image reports CUDA available and runs the reference inference.
3. The upstream TensorFlow GPU image lists the GPU and runs a short training step.
4. An OS update that changes the driver version: all three still pass after reboot without user action.
5. Rollback to the previous deployment: all three still pass.
6. A development container is created, a compiler is installed inside it, and the host's package set is unchanged afterwards (verified by comparing the booted image digest and `rpm -qa` before and after).
7. `arkon` variant: steps 1–5 are not applicable; step 6 passes.
