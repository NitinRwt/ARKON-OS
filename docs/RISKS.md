# ARKON OS — Risks

The full register, with likelihood, impact and mitigations, is [docs/RISK_REGISTER.md](docs/RISK_REGISTER.md). It is the only place risks are edited. This file exists so the register is reachable from the repository root.

Top risks as of 2026-11-16, after the architecture review:

| ID | Risk |
|---|---|
| R-01 | One test machine, no RTX GPU: most hardware classes ship unproven |
| R-02, R-30 | Scope and manual bare-metal testing are large for one engineer |
| R-13 | Fedora's six-month cadence outpaces the roadmap |
| R-05 | Kernel updates break the NVIDIA module build |
| R-21 | Redistribution of the NVIDIA driver, Steam and codecs, and trademark questions, are not legally reviewed |
| R-04 | The boot-chain choice becomes a forced reinstall if changed after public release |
| R-29 | Open NVIDIA modules do not power down the reference laptop's GPU generation |
