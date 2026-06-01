# Drone

A learning project: build a quadcopter from scratch — own hardware, own firmware, no existing flight stack.

The drone is the artefact; understanding the whole stack end-to-end is the deliverable.

## Status

**Phase 1 in progress.** End-to-end throttle control working on hardware (2026-06-01): the remote sweeps a `Throttle` value, ships it inside a `PilotCommand` over the IEEE 802.15.4 link ([ADR 0014](doc/decisions/0014-radio-protocol-ieee802154.md)), the drone's `comm_link` task republishes via `Watch<PilotCommand>` ([ADR 0013](doc/decisions/0013-async-communication-primitives.md)), and the `motor_controller` task drives a brushed motor through the L9110 H-bridge via the nRF PWM peripheral. First shared wire types live in `firmware-types`, which is also the first crate to carry host unit tests — postcard round-trip plus a custom `Deserialize` that enforces the `Throttle` 0..=1 invariant at the wire boundary (the bench surfaced a real garbage-packet hard fault and the test pins the fix in place). Test-wiring pattern recorded in [ADR 0015](doc/decisions/0015-host-testing-no-std-crates.md).

**Earlier in Phase 1:** Cargo workspace bootstrapped, Embassy-based firmware booting and logging over RTT (`defmt`), board-support-package layer in place ([ADR 0010](doc/decisions/0010-board-support-package.md)), source-level debugging working from VS Code via the probe-rs DAP adapter. `SystemState`-driven status LED (`Booting` / `Idle` / `Fault`) — first use of the embassy-sync pub/sub primitives. Fifteen ADRs in place. Next: real pilot inputs (PC-side joystick / ground station app), framing envelope with sequence-monotonicity rejection, then ICM-42688 SPI bring-up once the breakout arrives.

See [`doc/00-vision.md`](doc/00-vision.md) for the phase plan, [`doc/dev-environment.md`](doc/dev-environment.md) for the toolchain, and [`doc/decisions/`](doc/decisions/README.md) for the full decision history.

Headline choices (see ADRs below for the rest):

- **Platform:** BBC micro:bit v2 (nRF52833) for Phases 1–3; custom nRF5340 PCBA for Phases 4–5.
- **Language:** Rust (`no_std`, `embassy-nrf`) on the firmware; Rust on the PC-side ground-station application too.
- **IMU:** ICM-42688-P on SPI (external; micro:bit's onboard sensor has no gyro).
- **Airframe:** quadcopter.
- **Flight stack:** rolling our own — no PX4 / ArduPilot.

See [doc/00-vision.md](doc/00-vision.md) for the full vision and the phased milestone plan.

## Repository layout

- [`AGENTS.md`](AGENTS.md) — shared context file for AI coding assistants (Copilot, Claude, etc.). Read first.
- [`crates/`](crates/README.md) — Cargo workspace. `firmware-drone` (on-target binary) + `firmware-drone-core` (host-testable logic).
- [`doc/`](doc/README.md) — design notes, vision, architecture, hardware/software/control docs.
- [`doc/02-architecture.md`](doc/02-architecture.md) — system architecture overview (two micro:bits, RF link, ground-station evolution).
- [`doc/decisions/`](doc/decisions/README.md) — Architecture Decision Records (ADRs).
- [`hardware/`](hardware/README.md) — mechanical (Fusion 360) and electrical (KiCad, from Phase 4).

## Decisions so far

- [ADR 0001](doc/decisions/0001-platform-airframe-stack.md) — Real-hardware quadcopter, roll our own firmware, learning-first scope.
- [ADR 0002](doc/decisions/0002-mcu-and-language.md) — BBC micro:bit v2 + Rust for Phases 1–3.
- [ADR 0003](doc/decisions/0003-imu-icm42688-spi.md) — External IMU: ICM-42688-P on SPI.
- [ADR 0004](doc/decisions/0004-concurrency-embassy-channels.md) — Concurrency model: Embassy + channel-based actor pattern, no BSP.
- [ADR 0005](doc/decisions/0005-pc-software-language-rust.md) — PC-side software in Rust; shared `proto` crate for the wire protocol.
- [ADR 0006](doc/decisions/0006-mechanical-cad-fusion360.md) — Mechanical CAD: Fusion 360; commit `.f3d` + `.step` for portability.
- [ADR 0007](doc/decisions/0007-testing-and-ci-strategy.md) — Testing and CI: unit-test everything possible, local-first feedback, `core`/`task` split, HIL deferred.
- [ADR 0008](doc/decisions/0008-repository-folder-layout.md) — Repository folder layout: `crates/`, `doc/`, `hardware/{mechanical,electrical}/`, all lowercase.
- [ADR 0009](doc/decisions/0009-workspace-bootstrap-and-crate-naming.md) — Workspace bootstrap from day one; `firmware-<role>` naming; `core`/`task` split realised as sibling crates.
- [ADR 0010](doc/decisions/0010-board-support-package.md) — Board Support Package layer: `board` module inside `firmware-drone`, Cargo-feature-selected, tasks take erased types.
- [ADR 0011](doc/decisions/0011-task-tracking-issues-and-batches.md) — Task tracking: GitHub Issues as canonical backlog, Projects board as view, labels as taxonomy, batched filing.
- [ADR 0012](doc/decisions/0012-lint-and-format-policy.md) — Lint and format policy: `main` stays `rustfmt`-clean and `clippy`-clean; suppressions require justification.
- [ADR 0013](doc/decisions/0013-async-communication-primitives.md) — Async inter-task communication: 2×2 rule over `Channel` / `Watch` / `Signal` / `PubSubChannel`.
- [ADR 0014](doc/decisions/0014-radio-protocol-ieee802154.md) — Radio link: IEEE 802.15.4 (raw PHY/MAC), channel 20.
- [ADR 0015](doc/decisions/0015-host-testing-no-std-crates.md) — Host-testable `no_std` crates: `cfg_attr(not(test), no_std)`, inline `mod tests`, `cargo test` honours `default-members`.
- [ADR 0016](doc/decisions/0016-newtype-per-physical-quantity.md) — Newtype per physical quantity for shared types: distinct newtypes per quantity, no shared `PercentageValue` base.

## Licence

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work, as defined in the Apache-2.0 license, shall be dual-licensed as above, without any additional terms or conditions.
