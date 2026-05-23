# ADR 0002 — MCU and firmware language: BBC micro:bit v2 + Rust

- **Status:** Accepted (amended by [ADR 0004](0004-concurrency-embassy-channels.md), 2026-05-22)
- **Date:** 2026-05-21
- **Supersedes:** —
- **Related:** [ADR 0001](0001-platform-airframe-stack.md), [ADR 0004](0004-concurrency-embassy-channels.md)

> **Update (2026-05-22):** The crate baseline below originally listed both the `microbit-v2` BSP (which wraps `nrf-hal`) and `embassy-nrf` — those two stacks aren't naturally coherent. [ADR 0004](0004-concurrency-embassy-channels.md) resolves this: we drop the BSP and go straight to `embassy-nrf` with a channel-based actor pattern. The rest of this ADR (board choice, language choice, debug tooling) stands.

## Context

We need to pick the microcontroller and the firmware language before any code is written. The two decisions are coupled (toolchain, HAL crates, debugger story all hang off the combination).

Constraints / inputs:

- The user already owns **two BBC micro:bit v2 boards**. Zero extra cost to start.
- The user has strong systems-programming experience (C, low-level I/O) and wants to *learn*, including learning a new language if it earns its keep.
- Project is explicitly learning-focused (see [ADR 0001](0001-platform-airframe-stack.md)) — fast progress is not the priority; depth of understanding is.

## Decision

- **MCU:** BBC micro:bit v2 (Nordic **nRF52833**, ARM Cortex-M4F @ 64 MHz, 128 KB RAM, 512 KB flash).
- **Language:** **Rust** (embedded, `no_std`), using the `microbit-v2` BSP crate and the `embassy-nrf` HAL / async runtime.
- **Debug / flash:** `probe-rs` over the on-board DAPLink (CMSIS-DAP) — single USB cable does flash, debug, and `defmt` log streaming.
- **Scope of this choice:** micro:bit v2 is the platform for **Phases 1–3** of the vision (see [00-vision.md](../00-vision.md)). For Phase 4 onwards we expect to migrate to a purpose-built flight-controller board, almost certainly another Cortex-M part so the Rust codebase mostly carries over. That migration will get its own ADR when the time comes.

## Why micro:bit v2

- **Free.** Already owned.
- **Solid MCU.** nRF52833 is in the same architectural ballpark (Cortex-M4F, 64 MHz, single-precision FPU) as MCUs used in actual hobby flight controllers. Sensor fusion and PID loops will fit comfortably.
- **Best-in-class Rust support.** Dedicated BSP crate (`microbit-v2`), mature HAL (`nrf-hal` / `embassy-nrf`), well-trodden tutorials (the [Discovery book](https://docs.rust-embedded.org/discovery-mb2/) targets this exact board).
- **Built-in debugger.** DAPLink on board — no separate ST-Link / J-Link to buy and wire up. Flashing, semihosting / RTT logging, and breakpoints all work over the single USB cable.
- **Two boards is genuinely useful.** The nRF52833 has a 2.4 GHz radio. The second micro:bit can act as a **ground station / remote control / telemetry sink** using Nordic's proprietary radio protocol or BLE. This defers the radio-link ADR and the PC-software ADR (now [ADR 0005](0005-pc-software-language-rust.md)) and lets us reach Phase 2/3 with no extra hardware purchases.

## Why Rust (not C / C++)

For an *embedded learning project*, Rust trades a steep learning curve for:

- **Memory safety with no runtime cost.** Borrow checker catches the classes of bugs that destroy embedded systems silently (aliasing pointers, use-after-free, races between ISR and main loop). On a flight controller these bugs become crashes — literally.
- **Strong typing for hardware peripherals.** The `embedded-hal` ecosystem encodes pin modes, peripheral ownership, and bus contention in the type system. You cannot accidentally configure the same pin twice.
- **A modern async story (`embassy`)** that maps cleanly onto interrupt-driven peripherals — concurrency without an RTOS.
- **`cargo` + `probe-rs` + `defmt`** is, in 2026, the most pleasant embedded toolchain to use. No Makefiles, no linker-script gymnastics for the basics, structured logging that costs almost nothing on the wire.
- The user already knows C-level concepts; Rust adds new mental models (ownership, lifetimes) that are *themselves* worth learning. C would teach less.

The cost is a real one — compile times, occasionally fighting the borrow checker around HAL APIs, smaller community than C for *some* peripheral chips. Accepted.

## Consequences

### What this commits us to

- **Toolchain:** stable Rust, `thumbv7em-none-eabihf` target, `probe-rs`, `defmt`, `cargo-binutils`. Setup gets its own how-to doc when we write code.
- **Crate baseline (likely):** `microbit-v2`, `embassy-executor`, `embassy-nrf`, `embassy-time`, `defmt`, `defmt-rtt`, `panic-probe`, `embedded-hal` (1.0+).
- **An external IMU is mandatory.** The micro:bit v2's onboard sensor is an LSM303AGR — accelerometer + magnetometer, **no gyroscope**. Accel-only attitude is useless under vibration. This forces [ADR 0003 — IMU selection] to be the next decision, and pulls forward needing an I²C or SPI driver crate (or writing one) early.
- **A "ground station" micro:bit role.** The second board becomes a fixed part of the development setup — receiver of telemetry, sender of commands. Worth treating as a deliverable in its own right, with its own firmware tree.
- **Planned MCU migration around Phase 4.** Don't build anything that locks us to micro:bit hardware specifically; keep board-specific code behind the BSP boundary.

### What this rules out (for now)

- C / C++ firmware, including any temptation to copy-paste from Betaflight / Cleanflight sources. (We can read them for inspiration; we don't link to them.)
- ESP32 / RP2040 / Teensy ecosystems. Not because they're bad — they're great — but committing to one Rust+nRF toolchain keeps cognitive load low.
- Mounting the flight controller on a real drone frame *yet*. micro:bit v2's form factor and edge connector are bench-friendly, not flight-friendly. Phase 3 work happens on a rig, not in the air.

### Known limitations to revisit later

- **PWM / DShot generation for 4 motors** is awkward on micro:bit v2 — limited timer/PPI routing, and the edge connector doesn't expose enough convenient pins. Workable for 1–2 motors on a bench rig (Phase 3); a likely forcing function for the MCU migration at Phase 4.
- **Edge connector mechanical fragility.** Fine for desk work; not for anything vibrating.
- **Board weight & shape.** Not flyable on a small frame even if we wanted to.

### What stays open

- IMU part choice → ADR 0003.
- Frame / motor / ESC / battery → ADR 0004.
- Radio link choice (Nordic proprietary vs BLE vs eventually ELRS) → ADR 0005.
- Host-side tooling (what plots the telemetry stream?) — language settled by [ADR 0005](0005-pc-software-language-rust.md); framework choice still open.
- Successor flight-controller board for Phase 4+ → future ADR.
