# ARKON OS — Repository Structure

One repository holds everything until a concrete reason to split appears (for example, the kmods pipeline needing a different release cadence). A single repository keeps an image, the services inside it and the tests for both at one commit.

Directories are created when their phase starts, not before. Today only the root documents and `docs/` exist.

## Layout

```
ARKON/
├── PROJECT_STATE.md        entry point, read first every session
├── ARCHITECTURE.md         index and dependency map
├── DECISIONS.md            decision log and open decisions
├── TASKS.md                task list per phase
├── CHANGELOG.md            what changed, by date
├── TEST_RESULTS.md         what was run, on what, with what result
├── RISKS.md                pointer and top risks
├── docs/                   Phase 0 design package (this directory)
│
├── image/                  Phase 1 — the OS image
│   ├── Containerfile         multi-stage, both variants
│   ├── packages/             one manifest per stage: base, desktop, gaming, nvidia, removed
│   ├── rootfs/               files copied into the image, mirroring their final paths
│   ├── build/                numbered scripts run inside the build
│   ├── repos/                allow-listed repository files and GPG keys
│   └── tests/                checks run against the built container
│
├── kmods/                  Phase 1 — NVIDIA module build and signing
│
├── installer/              Phase 2 — ISO definition
│   ├── Containerfile         live installer environment
│   ├── kickstart/            install logic, hardware probe
│   └── image-builder/        builder configuration
│
├── branding/               Phase 3 — release package, logos, boot splash, Plasma defaults
│
├── services/               Rust workspace (starts in Phase 2 with arkon-hw)
│   ├── Cargo.toml
│   ├── crates/
│   │   ├── arkon-hw/           Phase 2 — hardware detection and the variant rule (library)
│   │   ├── arkon-hwprobe/      Phase 2 — CLI over arkon-hw; also shipped in the installer
│   │   ├── arkon-dbus-api/     Phase 4 — generated interface types; the only shared contract
│   │   ├── arkon-common/       logging, error types, validation helpers
│   │   ├── arkon-updated/      sandboxed D-Bus front for updates
│   │   ├── arkon-secureboot/
│   │   ├── arkon-diag/
│   │   ├── arkon-session-agent/  user-session notifications, Flatpak runtime sync
│   │   └── arkonctl/
│   ├── dbus/                 interface XML (source of truth), bus policy
│   ├── polkit/               action definitions and rules
│   ├── systemd/              unit files, including the arkon-bootc@ worker template
│   ├── data/                 streams.json and the GPU device table shipped in the image
│   └── selinux/              policy module
│
├── center/                 Phase 4 — ARKON Center
│   ├── src-tauri/            Rust side: D-Bus client, command definitions
│   └── src/                  React + TypeScript front end
│
├── packaging/              RPM spec files for every arkon-* package
├── containers/             Phase 6 — development and AI container definitions, if needed
├── bench/                  Phase 8 — benchmark harness, scenarios, result schema
├── tests/                  cross-component tests (pytest)
│   ├── integration/          against a built image or running services
│   ├── hardware/             bare-metal suite, run on physical machines
│   └── smoke/                boot checks
├── tools/                  developer scripts
├── .github/workflows/      CI definitions
├── Justfile                the documented entry point for every build and test command
└── LICENSE                 pending U-07
```

## Ownership and dependency rules

| Rule | Reason |
|---|---|
| `services/crates/*` binaries depend on the library crates (`arkon-dbus-api`, `arkon-common`, `arkon-hw`) only, never on another binary's crate. At run time they talk over D-Bus or through a documented CLI | Services stay independently testable and replaceable |
| `center/` depends on `arkon-dbus-api` and `arkon-hw` only | The UI cannot grow a private back channel |
| The GPU-to-variant rule exists only in `arkon-hw` | The installer and the running system can never disagree (D-024) |
| No image build step installs, upgrades or removes a kernel package | The base digest is the kernel pin (D-023) |
| `services/dbus/*.xml` is edited by hand; generated code is not | One source of truth for the typed interface |
| `image/` consumes ARKON components only as RPMs built from `packaging/` | The image build never compiles product code, so no toolchain enters the image |
| `image/packages/*.txt` is the only place a package name is added | Reviewable diffs; no install commands scattered in scripts |
| `image/repos/` is the only place a repository is defined | Enforces the allow-list (D-007) |
| `kmods/` receives the kernel version as an input and emits a manifest | The image build can prove kernel and module match |
| `tests/` and `bench/` are never imported by product code | Test code cannot ship |
| Nothing under version control contains a private key, token or password | Checked in CI with a secret scanner |

## Conventions

- **Commands.** Every build, test and lint operation is a `just` recipe. CI calls the same recipes developers do. A command that exists only in a workflow file is a defect.
- **Rust.** Stable toolchain pinned in `rust-toolchain.toml`; `cargo fmt`, `cargo clippy` with warnings denied, `cargo deny` for licences and advisories; `unsafe` forbidden unless justified in a comment and a decision entry.
- **TypeScript.** Strict mode; types for service calls are generated, not written.
- **Shell.** Only inside `image/build/` and `tools/`; `set -euo pipefail`; checked with ShellCheck. Logic that needs more than straight-line steps is written in Rust or Python instead.
- **Python.** Tests and the benchmark harness only; pytest.
- **Decisions.** A change that contradicts DECISIONS.md is not merged until the log is updated first.
- **Documentation.** A change to behaviour updates the matching document in `docs/` in the same pull request. PROJECT_STATE.md, TASKS.md and CHANGELOG.md are updated at the end of every working session.

## Branching and release

- `main` is always buildable; all work arrives by pull request with passing CI.
- Release streams are image tags, not branches (see [UPDATE_AND_RECOVERY_DESIGN.md](UPDATE_AND_RECOVERY_DESIGN.md)).
- A branch per Fedora major (`f45`, `f46`) is created only when two majors must be maintained at once. The Fedora major is a single build parameter, so a rebase (D-031) is a one-line change plus whatever it breaks.

## Build and test commands

None exist yet, because there is nothing to build. The Phase 1 design step defines them in the `Justfile`; this section is then replaced with the exact recipes. Planned names, so that documents can refer to them consistently:

| Recipe | Purpose | Phase |
|---|---|---|
| `just lint` | All linters | 1 |
| `just build-image <variant>` | Build one image variant locally | 1 |
| `just test-image <variant>` | Container-level checks | 1 |
| `just build-kmods <flavour>` | Build and sign NVIDIA modules, open or closed | 1 |
| `just build-iso` | Build the installer ISO | 2 |
| `just test-services` | Rust unit and integration tests | 4 |
| `just test-hardware` | Bare-metal suite on the current machine | 1 onward |
| `just bench <scenario>` | Run a benchmark scenario | 8 |

Local image builds need a Linux host with Podman. On the current Windows development machine the practical routes are CI, or Podman running inside WSL2 as build tooling. Neither is a deployment target.
