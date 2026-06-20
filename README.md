# Drone

A learning project: build a quadcopter from scratch — own hardware, own firmware, no existing flight stack.

The drone is the artefact; understanding the whole stack end-to-end is the deliverable.

## Status

**Phase 1 in progress.** End-to-end throttle control working on hardware (2026-06-01): the remote sweeps a `Throttle` value, ships it inside a `PilotCommand` over the IEEE 802.15.4 link ([ADR 0014](doc/decisions/0014-radio-protocol-ieee802154.md)), the drone's `remote_link` task republishes via `Watch<PilotCommand>` ([ADR 0013](doc/decisions/0013-async-communication-primitives.md)), and the `motor_controller` task drives a brushed motor through the L9110 H-bridge via the nRF PWM peripheral. First shared wire types live in `firmware-types`, which is also the first crate to carry host unit tests — postcard round-trip plus a custom `Deserialize` that enforces the `Throttle` 0..=1 invariant at the wire boundary (the bench surfaced a real garbage-packet hard fault and the test pins the fix in place). Test-wiring pattern recorded in [ADR 0015](doc/decisions/0015-host-testing-no-std-crates.md).

**Supervisor failsafe wired in (2026-06-02):** the `supervisor` task now sits between `remote_link` and `motor_controller` and is the sole publisher of motor commands ([ADR 0017](doc/decisions/0017-supervisor-failsafe-state-machine.md)). Loss-of-link is detected when no `PilotCommand` arrives for 100 ms (10 ticks of a 10 ms ticker), at which point the supervisor publishes a zero-throttle `MotorCommand` and flags `SystemState::Degraded`. Verified on the bench: powering off the remote stops the motor within ~100 ms. New host-testable crate `firmware-drone-core` exists for the pure state-machine logic; the actual `Initialising`/`Armed`/`Degraded`/`Fault` machine and ramp-down behaviour are next.

**PC ground-station controlling motor speed (2026-06-09):** first end-to-end pilot input. A new host-only `groundstation` crate (binary `gs`) opens an [egui](https://github.com/emilk/egui) window with a single throttle slider; dragging it sends the value as ASCII over USB-CDC at 115 200 8N1 to the remote micro:bit. On the remote, `serial_link` parses the line and publishes a `Throttle` to a `Watch`; `drone_link` subscribes and replaces its self-incrementing test counter with the live slider value. Plain ASCII for the first cut — the postcard + COBS framing described in [ADR 0018](doc/decisions/0018-pc-link-uart-postcard-cobs.md) lands when telemetry needs it. egui chosen as a deliberately low-friction GUI: immediate-mode, built-in `Slider`, no Elm-architecture or web-toolchain to learn; we did not go deep on it on purpose.

**Telemetry pipeline drone → remote (2026-06-14):** first payload flowing the other way. A new `telemetry` task on the drone reads the nRF52833 internal TEMP sensor at 2 Hz, converts the `I30F2` fixed-point reading to `f32` °C, and broadcasts a `TelemetryState` (sequence number + temperature) on a `Watch`. `remote_link` subscribes and ships the latest sample back to the remote in the radio reply to each `PilotCommand`, so the previous hard-coded placeholder telemetry is gone. Half of the round-trip chain is real now; forwarding the value out the remote's UART to the groundstation, then plotting it, is next. Alongside the feature: per-role state enums (`DroneState`, `RemoteState`) promoted into `firmware-types`, and the per-firmware signals layout settled into one-file-per-signal under `crates/firmware-<role>/src/signals/<name>.rs` so signal modules never define wire types inline.

**Full round trip, live plot, 100 Hz, gamepad input (2026-06-20):** the loop is closed and visible. Telemetry now flows all the way back to the PC — the remote's `serial_link_tx` task forwards each `TelemetryState` out the UART using postcard + COBS framing ([ADR 0018](doc/decisions/0018-pc-link-uart-postcard-cobs.md), now in real use, not just throttle), and the `groundstation` decodes it and plots throttle, temperature and drone-state as live time series with [`egui_plot`](https://github.com/emilk/egui_plot) (per-signal show/hide, clear, latest-frame readout). A dedicated `telemetry_aggregator` task on the drone is the sole publisher of `TelemetryState`, tick-sampling its sources and stamping the sequence number ([ADR 0020](doc/decisions/0020-telemetry-aggregator-single-publisher.md)). Cadence pushed to **100 Hz** end-to-end — aggregator, radio round-trip and UART all run on a 10 ms tick (UART ~18 % utilised), and the plotted throttle is butter-smooth despite the value crossing UART → IEEE 802.15.4 → drone → aggregator → radio → remote → UART on every frame. Finally, the throttle source can now be a **Bluetooth gamepad**: the groundstation reads it via [`gilrs`](https://gitlab.com/gilrs-project/gilrs) and maps the right trigger (analog 0.0–1.0) onto the throttle the slider used to own. Next: ICM-42688 SPI bring-up once the breakout arrives.

**Earlier in Phase 1:** Cargo workspace bootstrapped, Embassy-based firmware booting and logging over RTT (`defmt`), board-support-package layer in place ([ADR 0010](doc/decisions/0010-board-support-package.md)), source-level debugging working from VS Code via the probe-rs DAP adapter. Twenty ADRs in place.

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
