# Drone

A learning project: build a quadcopter from scratch — own hardware, own firmware, no existing flight stack.

The drone is the artefact; understanding the whole stack end-to-end is the deliverable.

## Status

Early days. Decisions made, no code yet.

- **Platform:** BBC micro:bit v2 (nRF52833) for Tiers 0–3, expected MCU migration later.
- **Language:** Rust (`no_std`, `embassy-nrf`).
- **IMU:** ICM-42688-P on SPI (external; micro:bit's onboard sensor has no gyro).
- **Airframe:** quadcopter.
- **Flight stack:** rolling our own — no PX4 / ArduPilot.

See [Doc/00-vision.md](Doc/00-vision.md) for the full vision and the tiered milestone plan.

## Repository layout

- [`AGENTS.md`](AGENTS.md) — shared context file for AI coding assistants (Copilot, Claude, etc.). Read first.
- [`Doc/`](Doc/README.md) — design notes, vision, architecture, hardware/software/control docs.
- [`Doc/02-architecture.md`](Doc/02-architecture.md) — system architecture overview (two micro:bits, RF link, ground-station evolution).
- [`Doc/decisions/`](Doc/decisions/README.md) — Architecture Decision Records (ADRs).

## Decisions so far

- [ADR 0001](Doc/decisions/0001-platform-airframe-stack.md) — Real-hardware quadcopter, roll our own firmware, learning-first scope.
- [ADR 0002](Doc/decisions/0002-mcu-and-language.md) — BBC micro:bit v2 + Rust for Tiers 0–3.
- [ADR 0003](Doc/decisions/0003-imu-icm42688-spi.md) — External IMU: ICM-42688-P on SPI.
- [ADR 0004](Doc/decisions/0004-concurrency-embassy-channels.md) — Concurrency model: Embassy + channel-based actor pattern, no BSP.
