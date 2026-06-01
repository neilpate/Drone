# Architecture Decision Records

Each non-trivial design decision gets its own file here.

## Format

- Filename: `NNNN-short-kebab-title.md` (zero-padded sequential number).
- Status field at top: `Proposed` / `Accepted` / `Superseded by NNNN` / `Deprecated`.
- Structure: **Context** (why we're deciding this now), **Decision** (what we chose), **Consequences** (what this commits us to and what it rules out).
- Don't edit an accepted ADR to reverse it — write a new one that supersedes it. ADRs are append-only history.

## Index

- [0001 — Platform, airframe, and firmware approach](0001-platform-airframe-stack.md) — Accepted, 2026-05-21
- [0002 — MCU and firmware language: micro:bit v2 + Rust](0002-mcu-and-language.md) — Accepted, 2026-05-21 (amended by 0004)
- [0003 — IMU: ICM-42688-P on SPI](0003-imu-icm42688-spi.md) — Accepted, 2026-05-21
- [0004 — Concurrency model: Embassy with channel-based actor pattern](0004-concurrency-embassy-channels.md) — Accepted, 2026-05-22
- [0005 — PC-side software in Rust](0005-pc-software-language-rust.md) — Accepted, 2026-05-23
- [0006 — Mechanical CAD: Fusion 360](0006-mechanical-cad-fusion360.md) — Accepted, 2026-05-23
- [0007 — Testing and CI strategy](0007-testing-and-ci-strategy.md) — Accepted, 2026-05-23
- [0008 — Repository folder layout](0008-repository-folder-layout.md) — Accepted, 2026-05-23
- [0009 — Workspace bootstrap and crate naming](0009-workspace-bootstrap-and-crate-naming.md) — Accepted, 2026-05-23
- [0010 — Board Support Package (BSP) layer](0010-board-support-package.md) — Accepted, 2026-05-23
- [0011 — Task tracking: GitHub Issues as canonical backlog, batched filing](0011-task-tracking-issues-and-batches.md) — Accepted, 2026-05-24
- [0012 — Lint and format policy: rustfmt + clippy clean before commit](0012-lint-and-format-policy.md) — Accepted, 2026-05-24
- [0013 — Async inter-task communication: when to use Channel, Watch, Signal, PubSubChannel](0013-async-communication-primitives.md) — Accepted, 2026-05-24
- [0014 — Radio link protocol: IEEE 802.15.4 (raw PHY/MAC)](0014-radio-protocol-ieee802154.md) — Accepted, 2026-05-27
- [0015 — Host-testable `no_std` crates: wiring pattern](0015-host-testing-no-std-crates.md) — Accepted, 2026-06-01
