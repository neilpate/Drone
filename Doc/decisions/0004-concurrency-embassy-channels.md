# ADR 0004 — Concurrency model: Embassy with channel-based actor pattern

- **Status:** Accepted
- **Date:** 2026-05-22
- **Related:** [ADR 0002](0002-mcu-and-language.md), [ADR 0003](0003-imu-icm42688-spi.md)
- **Amends:** [ADR 0002](0002-mcu-and-language.md) (removes the `microbit-v2` BSP from the baseline crate list)

## Context

[ADR 0002](0002-mcu-and-language.md) committed us to Rust on the micro:bit v2 and listed both the `microbit-v2` BSP crate (which sits on top of the blocking `nrf-hal`) and `embassy-nrf` (async HAL) as the baseline. Those two stacks are not naturally coherent — the BSP wraps `nrf-hal`, not `embassy-nrf`, and mixing them is friction without much payoff. The concurrency model and the HAL choice are entangled, so they need to be decided together before any code is written.

The user is experienced in systems programming and is comfortable with — and partial to — **actor-style concurrency** (typed messages, per-actor state, mailbox inputs). That preference shaped the candidate list.

Candidates considered:

- **Blocking `nrf-hal` + main loop + ISRs.** Lowest concept-load, what the [Discovery book](https://docs.rust-embedded.org/discovery-mb2/) teaches. Hits a wall once IMU sampling (kHz, interrupt-driven), control loop (deterministic), radio, and telemetry all want to run concurrently.
- **RTIC.** Priority-scheduled preemptive tasks, shared state mediated by the type system + priority ceilings. Excellent fit for hard real-time flight control. Different paradigm from actors — feels like "ISRs with safe shared state", not like "processes with mailboxes".
- **Ector** (actor framework on top of Embassy). The closest thing to a real embedded actor framework. Came out of the now-archived Drogue Device project. Small community, quiet development — usable but trailing edge.
- **Embassy + `embassy_sync::Channel`** (DIY actor pattern). Not a framework, a pattern: each async task owns its state, exposes a typed bounded channel as its inbox, loops `select!`-ing on inbox + timers + interrupts. Mainstream Embassy code, no extra dependency, statically wired.

## Decision

- **Concurrency model:** **Embassy with the channel-based actor pattern.** Each subsystem (IMU sampler, attitude estimator, control loop, radio TX, radio RX, telemetry logger) is an `async` task owning its state, with a typed `embassy_sync::Channel<Message>` as its inbox. Tasks are wired together explicitly at `main` time.
- **HAL:** **`embassy-nrf` directly. No BSP.** Pins are named by nRF peripheral IDs (`P0_14`, etc.). The `microbit-v2` BSP is removed from the baseline.
- **Async runtime:** `embassy-executor` (thumbv7em, single-core).
- **Synchronisation primitives:** `embassy-sync` (`Channel`, `Signal`, `Mutex`).
- **Timing:** `embassy-time`.
- **Scope:** Tiers 0 through ~3. Same revisit point as ADR 0002 (Tier 4+ flight-controller migration is a future ADR and may revisit this too).

## Why Embassy-with-channels (over the alternatives)

- **Matches the user's preferred mental model.** Typed mailboxes, per-actor state, explicit wiring. Closest embedded Rust gets to an actor framework without leaving the mainstream.
- **No framework debt.** Ector would give us a thinner actor abstraction but on a quiet codebase. The channel pattern is ~30 lines of boilerplate per actor and uses only the Embassy crates we'd be on anyway.
- **Static, allocation-free, bounded.** Channels have fixed capacity known at compile time. No heap, no dynamic dispatch, no surprises — appropriate for a flight controller.
- **Async maps cleanly to interrupt-driven peripherals.** IMU INT1 (see [ADR 0003](0003-imu-icm42688-spi.md)), SPI completion, timer ticks, UART RX — all become `.await` points. No hand-rolled state machines around ISRs.
- **Not RTIC** because the actor mental model is what the user wants. RTIC's task+priority model is excellent but conceptually different; if it turns out to be a better fit for the hard-real-time control loop later, that's a focused revisit, not a stack rewrite.
- **Not blocking nrf-hal** because we'd throw it away the moment concurrency bites (probably mid-Tier-2). Better to absorb the async learning curve up front than to rewrite.

## Why no BSP

- The `microbit-v2` BSP wraps `nrf-hal`, not `embassy-nrf`. Using it would mean mixing two HALs.
- Its main value is friendly accessors (`board.display_pins`, `board.buttons.button_a`) and the alignment with the Discovery book's examples. Once we've committed to Embassy + async, the Discovery book stops being a useful tutorial path anyway.
- Cost of skipping it: ~20 lines of pin setup in `main`. Acceptable.

## Consequences

### What this commits us to

- **Crate baseline (replaces the list in ADR 0002):**
  - `embassy-executor`, `embassy-nrf`, `embassy-sync`, `embassy-time`
  - `embassy-futures` (for `select!` / `join!`)
  - `cortex-m`, `cortex-m-rt`
  - `defmt`, `defmt-rtt`, `panic-probe`
  - `embedded-hal` 1.0+ and `embedded-hal-async` where drivers support it
  - No `microbit-v2`, no `nrf-hal` directly.
- **Architectural convention:** every subsystem is an `#[embassy_executor::task]` that owns its peripherals/state and communicates only through typed channels and `Signal`s. Shared mutable state behind `Mutex` is a last resort, not a default.
- **Async drivers preferred.** When picking or writing a driver (e.g. for the ICM-42688), prefer the `embedded-hal-async` flavour so SPI transactions yield rather than block.
- **Absorbing the async learning curve up front.** Futures, pinning, `Send`/`!Send` task bounds, `select!` semantics — all on the early-learning critical path alongside Rust itself and embedded itself.
- **Discovery book is not our tutorial.** We will lean on Embassy's own examples (`embassy/examples/nrf52833/`), `embassy-book`, and reading the HAL source instead.

### What this rules out (for now)

- **The `microbit-v2` BSP** — removed from ADR 0002's baseline.
- **Blocking `nrf-hal`** as the primary HAL. (We may still read it for reference; we don't depend on it.)
- **RTIC.** Revisitable if the hard-real-time control loop turns out to need priority preemption that Embassy can't deliver. Not expected, but flagged.
- **Ector** and other actor frameworks. The DIY channel pattern is the chosen idiom.
- **The Discovery book learning path.** Replaced by Embassy's own materials.

### What stays open

- **Successor flight-controller board for Tier 4+** (already open in ADR 0002) — may revisit the concurrency model at the same time.
- **Hard-real-time control loop strategy.** Likely a high-priority Embassy task driven by a timer interrupt; if jitter becomes a problem we revisit (RTIC, or a hand-rolled high-priority ISR alongside Embassy).
- **Telemetry transport on the wire** — separate ADR (radio link).
