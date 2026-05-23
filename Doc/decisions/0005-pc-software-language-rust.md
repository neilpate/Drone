# ADR 0005 — PC-side software in Rust

- **Status:** Accepted
- **Date:** 2026-05-23
- **Related:** [ADR 0002](0002-mcu-and-language.md), [ADR 0004](0004-concurrency-embassy-channels.md), [02-architecture.md](../02-architecture.md)

## Context

[02-architecture.md](../02-architecture.md) describes a v1 setup where a PC sits behind the ground micro:bit and owns the "App" layer: telemetry plotting, controller input, command shaping, logging. v2 eventually removes the PC, but v1 is where we'll spend Phases 1–3.

The PC-side application is non-trivial. It needs:

- A USB-serial reader that decodes a wire protocol (still to be specified — separate ADR).
- A real-time-ish plotter for streaming IMU / attitude / control telemetry.
- Gamepad / controller input.
- Command encoding back down the serial link.
- Logging to disk.

[ADR 0002](0002-mcu-and-language.md) chose Rust for firmware but explicitly scoped itself to the MCU side. Host-side tooling was listed as a future ADR (originally numbered 0006 there; this is that decision, pulled forward and renumbered to 0005 because it's blocking sooner).

Candidates considered:

- **Rust.** Shared language with the firmware. Shared wire-protocol types via a `no_std`-compatible crate used on both sides.
- **Python.** Fast to prototype, excellent plotting (`matplotlib`, `pyqtgraph`), trivial serial handling. Different language, different types, separate skill investment.
- **C / C++.** No upside over Rust for this role; significantly more friction.

## Decision

- **PC-side software is written in Rust.**
- **Wire-protocol types live in a shared crate** (`no_std` + `serde`-compatible) used by both the firmware and the PC application. The drone and the ground side compile the same struct definitions.
- **Workspace layout:** the repo will eventually be a Cargo workspace with at least three members — `firmware-drone`, `firmware-ground` (the ground micro:bit), and `pc-app` (the host application) — plus the shared `proto` crate. Exact layout deferred until we start writing code.

## Why Rust on the PC side

- **One language across the whole stack.** No context-switching between Rust's ownership model and Python's runtime semantics while debugging a link-layer issue at 2am.
- **Shared types end-to-end.** The wire protocol can be `#[derive(Serialize, Deserialize)]` once and used verbatim on both ends. No hand-translated struct definitions, no drift between firmware and PC.
- **Reinforces the learning goal.** The project is a vehicle for getting deep with Rust. Two Rust codebases that talk to each other is more Rust per unit of project, not less.
- **The ecosystem is sufficient.** `serialport` for USB-serial, `egui` / `eframe` for a plotting GUI, `gilrs` for controller input, `tokio` for async if needed. None are best-in-class against Python's plotting story, but all are good enough for the telemetry volumes involved (kHz IMU, ~Hz UI).
- **Async maps the same way on both sides.** The drone uses `embassy-executor`; the PC will use `tokio` or `smol`. Different runtimes, same mental model.

## Why not Python

- The plotting story is better in Python today, by a clear margin. That's the genuine cost of this decision.
- But the cost of *maintaining two ecosystems*, two build systems, two test stories, and a hand-translated wire protocol is larger than the cost of writing the plotter in Rust.
- Python remains fine for one-off analysis scripts on captured logs. This ADR is about the live ground-station application, not about ad-hoc data wrangling.

## Consequences

### What this commits us to

- A Cargo workspace with firmware crates and a PC crate side by side.
- A shared `proto` crate, `no_std`, that defines the wire protocol once.
- Picking a Rust GUI / plotting stack when we get there (likely `egui` + `egui_plot`; not locked in by this ADR).
- Picking a Rust async runtime for the PC side (`tokio` is the default; not locked in by this ADR).
- Some upfront pain on the plotting side relative to a Python prototype. Accepted.

### What this rules out (for now)

- Python as the language of the ground-station application.
- Mixed-language wire protocols (e.g. firmware emits JSON, PC parses with `json`). The wire format will be a Rust-defined binary or compact text protocol — exact format is a separate ADR.

### What stays open

- **Wire framing / encoding** (postcard, COBS-framed bincode, line-delimited JSON for early bring-up, …) — separate ADR.
- **GUI framework choice** (`egui`, `iced`, raw `wgpu`, terminal UI with `ratatui`) — separate ADR, deferred until Phase 1 starts needing plots.
- **Async runtime on PC side** — `tokio` is the obvious default; not formally locked in.
- **Logging format on disk** — separate decision, deferred.
