# ADR 0018 — PC ground-station link: USB-CDC UART, postcard + COBS

- **Status:** Proposed
- **Date:** 2026-06-05
- **Related:** [ADR 0005](0005-pc-software-language-rust.md) (PC-side software in Rust), [ADR 0010](0010-board-support-package.md) (BSP), [ADR 0014](0014-radio-protocol-ieee802154.md) (radio wire format), [ADR 0016](0016-newtype-per-physical-quantity.md) (`firmware-types` shared between host and target)

## Context

[ADR 0005](0005-pc-software-language-rust.md) committed to "PC-side software written in Rust" with `firmware-types` as the shared wire-protocol crate, but no PC-side binary or transport had been built. Until now the `drone_link` task on the remote has been generating its own throttle value — a slowly-incrementing `f32` counter — because there was no other input source. That is enough to prove the radio link works end-to-end but useless for actually controlling the drone or for scripted bench testing.

The supervisor failsafe state machine landed in [ADR 0017](0017-supervisor-failsafe-state-machine.md) and is now ready for hardware verification. The three bench tests that matter (basic arm/disarm, ramp-down on link loss, refuse-recovery-until-zero) all involve precise, repeatable control over the throttle value the remote transmits. In particular, the refuse-recovery test requires the remote to *stop* transmitting for >100 ms and then *resume* with a non-zero throttle — a scenario that is awkward to produce with physical inputs (buttons, accelerometer) because the human in the loop is also the one that has to remember to hold the stick.

A scripted PC-side controller solves this and unlocks scripted bench testing in general. We need a transport that:

- works with the hardware we already have (no extra dongles, no schematic changes),
- can run alongside `cargo run` / `defmt` without contending for the debug probe,
- carries the same `firmware-types` payloads the radio uses (no second wire format),
- ports forward to non-micro:bit hardware (Phase 4 custom PCBA, future ground stations).

The micro:bit v2 exposes three host-visible interfaces over its single USB connector: a USB Mass Storage drive (drag-and-drop flashing, unused by us), a CMSIS-DAP debug interface (used by `probe-rs` for flashing and `defmt-rtt` log streaming), and a USB-CDC virtual COM port. The COM port is bridged by the on-board interface MCU to a UART on the nRF52833. We have not used that UART yet.

## Decision

The PC-to-firmware link is **USB-CDC virtual COM port → UART on the nRF52833**, using `postcard` for serialisation and COBS framing for stream delimitation. A new workspace crate `crates/groundstation/` (binary name `gs`) is the host-side counterpart.

### Transport: USB-CDC over UART, not RTT

The interface MCU on micro:bit v2 mounts a USB-CDC class device on every host that plugs the board in, exposing a virtual COM port (`COM3` on Windows, `/dev/ttyACMx` on Linux). It bridges that to a UART on the nRF52833. We use this path.

- **No probe contention.** Any host process can open the COM port at any time, with or without `probe-rs` attached. Flashing and ground-station traffic are independent.
- **No probe-specific dependencies on the host side.** The PC binary needs only the OS USB-CDC driver (built in everywhere) and a serial-port library. No `probe-rs` library, no CMSIS-DAP DLLs, no chip target descriptions.
- **Travels to the next hardware generation.** When the Phase-4 custom PCBA replaces the micro:bit, exposing UART (or USB-CDC directly from the nRF5340's USB peripheral) is straightforward. RTT would have to be re-bridged at every step.

RTT-down is rejected as the primary link for these reasons. It remains useful as a developer back-channel for *interactive debugging* and may be added later for that purpose; this ADR is silent on it.

### Pin assignment (micro:bit v2)

Verified against the BBC micro:bit v2 schematic (2026-06-05). Schematic labels are written from the **interface MCU's** perspective:

- nRF52833 **TXD: `P0_06`** → interface MCU RX (schematic: `UART_INT_RX`).
- nRF52833 **RXD: `P1_08`** ← interface MCU TX (schematic: `UART_INT_TX`).

These are fixed by the v2 board layout. Any other UART configuration will not appear at the USB-CDC port.

### UART line settings

- **Baud rate:** 115 200, 8N1, no flow control.
- **Rationale:** standard speed, supported by every serial terminal and library, vastly more headroom than we need. `PilotCommand` after postcard + COBS is ~10–15 bytes; at 100 Hz that is ~1.5 kB/s, well under the ~11 kB/s budget at 115 200 baud.
- Higher baud rates (230 400, 921 600, 1 Mbit) are available on the nRF UARTE peripheral. Reconsider only when telemetry payload size or rate forces the issue.

### Wire format: postcard + COBS framing

- **Serialisation:** [`postcard`](https://docs.rs/postcard) with `serde` derives — same crate, same encoding as the radio link ([ADR 0014](0014-radio-protocol-ieee802154.md)).
- **Framing:** COBS (Consistent Overhead Byte Stuffing) via `postcard::accumulator::CobsAccumulator` on the firmware side and `postcard::to_stdvec_cobs` / `from_bytes_cobs` on the host side. UART is a byte stream; postcard alone has no message boundary, COBS adds one zero-byte sentinel per message with predictable, bounded overhead (~1 byte per 254 bytes of payload).
- **Same types, same crate.** `PilotCommand`, `Throttle`, future `TelemetryState` are imported from `firmware-types` on both sides. There is no separate "PC-side wire format". The crate is `no_std`-on-target / `std`-on-host already (per [ADR 0015](0015-host-testing-no-std-crates.md) / [ADR 0016](0016-newtype-per-physical-quantity.md)) so it works unchanged.

### Initial direction: PC → remote only

The first cut carries `PilotCommand` from PC to remote. The remote's `drone_link` task subscribes to a UART-driven `Watch<Throttle>` instead of generating throttle internally, then forwards the command over the radio as before.

The reverse direction (remote → PC) for telemetry is deferred. The transport supports it symmetrically; no architectural change is needed when we wire it up later.

### `sequence_count` ownership

`PilotCommand.sequence_count: u32` is currently set by the remote's `drone_link`. With PC-driven commands there is a question of whether the PC or the remote should own it. Initial choice: **the remote keeps owning it**. The PC may set any value (including zero); the remote re-stamps with its own counter before transmitting over the radio. This sidesteps the cross-link sequence-tracking question for now. Revisit if/when end-to-end (PC → drone) loss visibility becomes useful.

### Repository layout

A new workspace member:

```
crates/groundstation/
  Cargo.toml          # bin crate, std, depends on firmware-types
  src/main.rs         # CLI (clap), serial I/O (tokio-serial or serialport)
```

- Binary name `gs` (`[[bin]] name = "gs"` in `Cargo.toml`) for typing convenience: `cargo run --bin gs -- ...`.
- Listed in `[workspace.members]`. **Not** in `[workspace.default-members]` if that would force it to build during `cargo test` on a target-only check; needs verification once the crate exists. It must run on the host without target-specific toolchains.
- Folder name `groundstation` matches the lowercase, hyphen-free convention used by `firmware-drone`, `firmware-remote`, `firmware-types`, `firmware-drone-core`. (Hyphen vs no hyphen across the existing crates is inconsistent; "groundstation" stays one word to avoid debating this in scope.)

## Why this shape

- **Standard COM port over USB-CDC** is the most boring, most portable embedded telemetry/control transport that exists. Every operating system has the driver. Every language has a library. PuTTY can sanity-check the link before any code is written. There is no novelty value in choosing something else.
- **postcard + COBS** is the canonical pairing in the embedded-Rust ecosystem. `postcard::accumulator` is purpose-built for exactly this case; using it costs ~5 lines on the firmware side. Any alternative (length-prefix, SLIP, custom framing) is more code for less robustness.
- **Same `firmware-types` crate** is the entire point of [ADR 0005](0005-pc-software-language-rust.md). The ground station doesn't get its own protocol crate; it depends on the same one the firmware does. Schemas can never drift.
- **Dedicated UART task** mirrors the actor-per-task pattern already in use ([ADR 0004](0004-concurrency-embassy-channels.md)). The task owns the UART peripheral, deframes incoming bytes, and publishes parsed `PilotCommand`s on a `Watch`. `drone_link` subscribes. The throttle source is now swappable — UART, buttons, accelerometer, future RC receiver — without touching `drone_link`.
- **115 200 baud** is the floor that "just works" on every system. Faster is available; slower is wasteful; this is the value embedded developers reach for by reflex.

## Consequences

### What this commits us to

- A new `groundstation` crate in the workspace, host-only.
- A new `serial_link` task in `firmware-remote`, owning the UARTE peripheral.
- BSP changes in `firmware-remote/src/board/microbit_v2.rs` to construct and expose the UARTE on `P0_06` (TXD) / `P1_08` (RXD). Future board variants must provide an equivalent typed UART handle from `Board::new()`.
- `firmware-types` becomes the shared dependency of `firmware-drone`, `firmware-remote`, `firmware-drone-core`, and `groundstation`. Any wire-format change must compile across all four.
- The remote's `drone_link` task switches from self-generated throttle to a `Watch<Throttle>` subscription. The throttle source becomes a configuration choice, not a hard-coded counter.
- COBS is the framing for any stream-oriented transport we use going forward (UART now, future TCP / serial-over-Bluetooth, etc.). Datagram transports (radio) keep their natural framing and stay COBS-free.

### What this rules out

- **No second wire format for PC traffic.** Whatever flows over the radio also flows over UART (and vice versa where applicable). Adding "ground-station-only" message types means adding them to `firmware-types`, where they remain available to any transport.
- **No RTT-based ground-station control.** RTT-down may be added for *developer* commands ("force fault now", "dump state") but is not the path for `PilotCommand` traffic. Probe contention and probe-stack dependency rule it out as the primary link.
- **No USB device peripheral on the nRF52833 directly.** The micro:bit v2 does not route USB D+/D- from the nRF52833 to the connector — the interface MCU owns the USB. This option re-opens on the Phase-4 custom PCBA and may be revisited there.

### What stays open

- **Telemetry direction (remote → PC).** Not yet wired; the transport supports it. Will fold in when the first telemetry message beyond the existing radio-side counter is defined.
- **CLI surface of `gs`.** Initial subcommands (`send-throttle`, `send-throttle-loop`, `monitor`) will be sketched in implementation; final shape evolves as the bench-test scripts demand. Not pinned by this ADR.
- **GUI / plotting.** Out of scope. The CLI is enough for the bench tests this ADR enables. A GUI is a separate decision when the data volume justifies it.
- **End-to-end sequence numbering (PC → drone).** Currently the remote re-stamps `sequence_count`. If we ever want PC-visible drop counts across both hops, a separate field (e.g. `pc_sequence_count`) gets added.
- **Drone-side UART.** `firmware-drone` is currently flashed via probe-rs only; the same USB-CDC bridge exists on the drone's micro:bit but is not wired up. Add it when there's a use case (live tuning, log capture from the drone without radio).
- **Authentication / framing integrity.** No CRC at the application layer (postcard is fully validated by `serde` deserialisation; bad bytes fail to decode). No authentication. Bench-only is fine; flying outdoors over an unrelated radio is fine; multi-user contention or hostile environments would force the issue.

## References

- [postcard documentation](https://docs.rs/postcard) — wire format and the `accumulator` module.
- [COBS — Consistent Overhead Byte Stuffing](https://en.wikipedia.org/wiki/Consistent_Overhead_Byte_Stuffing) — the framing scheme postcard uses.
- [BBC micro:bit v2 schematic](https://github.com/microbit-foundation/microbit-v2-hardware) — UART pin assignments verified against this.
- [embassy-nrf `uarte` module](https://docs.embassy.dev/embassy-nrf/git/nrf52833/uarte/index.html) — async UART driver used on the firmware side.
- ADR 0005 — the parent decision this ADR finally implements.
- ADR 0014 — sister ADR for the radio link; same wire format, different transport.
- Implementation: [`crates/groundstation/`](../../crates/groundstation/) (created by this ADR), [`crates/firmware-remote/src/tasks/serial_link.rs`](../../crates/firmware-remote/src/tasks/serial_link.rs) (created by this ADR), [`crates/firmware-remote/src/board/microbit_v2.rs`](../../crates/firmware-remote/src/board/microbit_v2.rs) (extended).
