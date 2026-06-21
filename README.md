# Drone

[![CI](https://github.com/neilpate/Drone/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/neilpate/Drone/actions/workflows/ci.yml)

A learning project: build a quadcopter from scratch — own hardware, own firmware, no existing flight stack.

The drone is the artefact; understanding the whole stack end-to-end is the deliverable.

## What we're building

- **Platform:** BBC micro:bit v2 (nRF52833) for Phases 1–3; custom nRF5340 PCBA for Phases 4–5.
- **Language:** Rust (`no_std`, `embassy-nrf`) on the firmware; Rust on the PC-side ground-station application too.
- **IMU:** ICM-42688-P on SPI (external; micro:bit's onboard sensor has no gyro).
- **Airframe:** quadcopter.
- **Flight stack:** rolling our own — no PX4 / ArduPilot.

See [`doc/00-vision.md`](doc/00-vision.md) for the full vision and the phased milestone plan.

## Status

**Phase 1 in progress.** A full pilot-command and telemetry round trip is working on hardware at 100 Hz. A PC ground station (the `groundstation` crate, binary `gs`) sends a four-axis pilot command — throttle plus roll, pitch and yaw — from on-screen sliders or a PlayStation gamepad (Mode 2 sticks, throttle on the right trigger) over USB-CDC to a remote micro:bit, which relays it to the drone micro:bit over an IEEE 802.15.4 link. The drone runs an Embassy task graph (`remote_link` → `supervisor` failsafe → `motor_controller`) that drives a brushed motor and detects loss-of-link within ~100 ms. Telemetry flows back the same path, framed with postcard + COBS, and the ground station plots the commanded axes as live time series. Each axis is a typed, normalised newtype clamped at the wire boundary, with frames and sign conventions fixed in [ADR 0021](doc/decisions/0021-coordinate-frames-and-command-semantics.md).

![Ground station live plot: throttle, roll, pitch and yaw traces driven by a PlayStation gamepad, alongside the matching on-screen sliders.](doc/images/groundstation%202.png)

The four traces above are the visible end of a busy round trip: every sample left the PC as a gamepad/slider value, crossed USB-CDC → UART → IEEE 802.15.4 to the drone, was sampled by the telemetry aggregator, then flew back drone → remote → UART → PC before being plotted — all at 100 Hz.

**Next:** ICM-42688 SPI bring-up once the breakout arrives.

See [`doc/progress.md`](doc/progress.md) for the dated milestone history, [`doc/dev-environment.md`](doc/dev-environment.md) for the toolchain, and [`doc/decisions/`](doc/decisions/README.md) for the full decision history.

## Repository layout

- [`AGENTS.md`](AGENTS.md) — shared context file for AI coding assistants (Copilot, Claude, etc.). Read first.
- [`crates/`](crates/README.md) — Cargo workspace. `firmware-drone` (on-target binary) + `firmware-drone-core` (host-testable logic).
- [`doc/`](doc/README.md) — design notes, vision, architecture, hardware/software/control docs.
- [`doc/02-architecture.md`](doc/02-architecture.md) — system architecture overview (two micro:bits, RF link, ground-station evolution).
- [`doc/decisions/`](doc/decisions/README.md) — Architecture Decision Records (ADRs).
- [`hardware/`](hardware/README.md) — mechanical (Fusion 360) and electrical (KiCad, from Phase 4).

## Testing

Host-testable logic (wire types, the supervisor state machine, the ground-station helpers) is unit-tested and run with [cargo-nextest](https://nexte.st/):

```sh
cargo nextest run                                                 # workspace host crates
cargo nextest run --manifest-path crates/groundstation/Cargo.toml # the GUI crate
```

A tracked `pre-push` git hook runs the suite before every push, and [GitHub Actions](.github/workflows/ci.yml) runs `fmt` + `clippy` + tests on every push and pull request (the badge above). On-target firmware is exercised on hardware, not in CI. See [`doc/ci-and-testing.md`](doc/ci-and-testing.md) for the details and the one-time hook setup.

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
- [ADR 0017](doc/decisions/0017-supervisor-failsafe-state-machine.md) — Supervisor task as failsafe state machine: 4-state enum, tick-driven, pure logic in `firmware-drone-core`, supervisor is the sole publisher of motor commands.
- [ADR 0018](doc/decisions/0018-pc-link-uart-postcard-cobs.md) — PC ground-station link: USB-CDC virtual COM port to nRF52833 UART, 115 200 8N1, postcard + COBS framing (Proposed; first cut shipped with plain ASCII).
- [ADR 0019](doc/decisions/0019-airframe-class-3in-4s-printed.md) — Airframe and propulsion class: 3" ducted cinewhoop, 4S LiPo, 1507-class motors, DShot 4-in-1 ESC, fully 3D-printed PETG frame (Proposed).
- [ADR 0020](doc/decisions/0020-telemetry-aggregator-single-publisher.md) — Telemetry aggregator: a dedicated task is the sole publisher of `TelemetryState`, tick-sampling per-source `Watch`es at 100 Hz and owning frame-level fields.

## Licence

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work, as defined in the Apache-2.0 license, shall be dual-licensed as above, without any additional terms or conditions.
