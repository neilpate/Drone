# ADR 0025 — Persist control parameters to flash

- **Status:** Proposed
- **Date:** 2026-08-14
- **Related:** [ADR 0024](0024-control-law-angle-mode-pd.md) (the `ControlSystemParameters` this persists), [ADR 0018](0018-pc-link-uart-postcard-cobs.md) (the ground-station link the save command rides on), [ADR 0013](0013-async-communication-primitives.md) (`Watch` the loaded params are published into), [ADR 0010](0010-board-support-package.md) (flash driver is board-specific, lives in the BSP), [ADR 0016](0016-newtype-per-physical-quantity.md) (the newtypes inside the params), [ADR 0017](0017-supervisor-failsafe-state-machine.md) (disarmed gating), [ADR 0007](0007-testing-and-ci-strategy.md) / [ADR 0009](0009-workspace-bootstrap-and-crate-naming.md) (crate split, host-testable core)

## Context

Control gains ([ADR 0024](0024-control-law-angle-mode-pd.md)) are tuned live: the ground station sends `ControlSystemParameters` updates over the link ([ADR 0018](0018-pc-link-uart-postcard-cobs.md)) and the drone applies them at runtime through a `Watch` ([ADR 0013](0013-async-communication-primitives.md)). But nothing on the drone remembers them. On any power cycle the values revert to the compiled-in `ControlSystemParameters::default()`, so a tuning session is only made permanent by editing the defaults in source and rebuilding + reflashing. That round-trip is the slow, tedious part of the tuning loop.

Persisting the parameters to the drone's own non-volatile flash removes that round-trip: tune live, commit once, and the values survive a power cycle without a rebuild. It is also a deliberately small first step onto the internal-flash stack (the `embedded-storage` traits, the nRF flash driver, carving a region out of the linker) that a later over-the-air-update effort will need — but without the hard part, a bootloader, because configuration is a separate data region and never overwrites running code.

This ADR decides how a small, mutable configuration blob is stored on the nRF52833's internal flash. It does not decide the gain values, and it does not decide over-the-air firmware update (a separate, later ADR).

## Decision

### 1. What is persisted

The single `ControlSystemParameters` struct ([ADR 0024](0024-control-law-angle-mode-pd.md)) — the roll/pitch/yaw gains plus the setpoint-scaling maxima. It is already `serde` + `postcard` serialisable in `firmware-types`, so it is stored as its postcard-encoded bytes. No new wire or storage type is introduced; the persisted form is the same encoding used on the link.

Nothing else is persisted for now. This is a configuration store for one small blob, not a general filesystem.

### 2. Storage mechanism: a log-structured store over the internal flash

nRF52833 internal flash is NOR: it erases a whole page at a time (4 KB), an erase is slow and CPU-blocking (~85 ms), a write can only clear bits, and each page has limited endurance (order 10 000 erase cycles). Erasing and rewriting a fixed page on every change would both stall the system and consume endurance quickly.

The store is therefore **log-structured**: new records are appended after the last valid one within a small reserved region, and a page is only erased once it fills (then compacted). The most recent valid record wins on read. Wear-levelling falls out of the append-only structure, and integrity is protected by a per-record CRC.

This is provided by the **`sequential-storage`** crate (a key/value map over the `embedded-storage` flash traits, in the Embassy ecosystem), layered on **embassy-nrf's `Nvmc`** flash driver. Hand-rolling the journal is rejected — it is exactly the wheel `sequential-storage` exists to provide, and the idiomatic-choice rule ([AGENTS.md](../../AGENTS.md)) applies.

### 3. A dedicated flash region, reserved in the linker

One to two 4 KB pages at the top of flash are reserved for the configuration store and carved out in `memory.x` so the application image never occupies them. The region is board-specific and its definition lives with the BSP ([ADR 0010](0010-board-support-package.md)); tasks receive an already-constructed store handle and never see flash addresses, mirroring the pin-erasure rule.

### 4. Write policy: explicit commit, disarmed only

Writes are **not** triggered per parameter update (that would erase-thrash the flash as a slider is dragged). Instead a parameter set is persisted only on an **explicit commit** — a "Save gains to flash" action from the ground station (or, as a later refinement, automatically on disarm). Persistence is gated to the **Disarmed** state ([ADR 0017](0017-supervisor-failsafe-state-machine.md)) and runs off the control path, so a blocking page erase can never stall an armed control loop or coincide with flight.

### 5. Boot flow and fallback

On boot the drone reads the most recent valid record and publishes it into the `control_system_parameter_update` `Watch` ([ADR 0013](0013-async-communication-primitives.md)), exactly as a live update would. If the region is empty (first boot, or just erased) or the record fails its CRC, the drone falls back to `ControlSystemParameters::default()`. Stored configuration is thus an override of the compiled-in defaults, never a hard dependency.

### 6. Schema evolution is fail-safe, not migrated

postcard is not self-describing: if the `ControlSystemParameters` layout changes in a future firmware, previously stored bytes may not decode. Rather than build a migration framework, a **version/magic tag** is stored alongside the blob; on any version mismatch or decode failure the stored record is ignored and the firmware falls back to its own defaults (§5). A firmware update that changes the parameter schema therefore silently discards stale config and re-defaults — safe, and the user simply re-tunes and re-saves. Migration can be added later if it ever earns its keep.

## Consequences

- **The tuning loop loses its rebuild step.** Tune live, click save, power-cycle — the gains persist. This directly addresses the slow part of bench tuning.
- **A flash region is committed.** One to two pages (4–8 KB of 512 KB) leave the application image; negligible for the current `no_std` firmware. The `memory.x` change is a permanent layout commitment that any future flash user (including OTA) must respect.
- **New dependencies:** `sequential-storage` and the use of embassy-nrf `Nvmc`. First real use of the `embedded-storage` traits in the project.
- **Endurance is bounded but ample.** With explicit-commit-only writes (not per-tick) and journalled wear-levelling, realistic tuning use is far inside the page endurance budget.
- **A stepping stone to OTA.** This exercises the flash driver, the `embedded-storage` traits, and linker partitioning that an over-the-air update will reuse — de-risking that work while delivering standalone value now, and without pulling in a bootloader.
- **Config never bricks or blocks flight.** CRC + version tag + default fallback means a bad or stale record degrades to compiled-in defaults; disarmed-only, off-control-path writes keep the blocking erase away from flight.
- **Host-testable core.** The serialise/deserialise/version-check logic is pure and lives testably alongside `firmware-types` / the core crates ([ADR 0007](0007-testing-and-ci-strategy.md)); only the `Nvmc`-backed store is on-target.

## Open questions

- Exact region size and address in `memory.x` (one page vs two), settled at implementation.
- Whether to also auto-save on disarm, or keep save strictly explicit.
- Whether the ground station should read back and display the currently persisted values (a "load from flash" / status affordance) versus the live values.
