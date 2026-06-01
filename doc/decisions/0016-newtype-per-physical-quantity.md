# ADR 0016 — Newtype per physical quantity for shared types

- **Status:** Accepted
- **Date:** 2026-06-01
- **Related:** [ADR 0009](0009-workspace-bootstrap-and-crate-naming.md) (where shared types live), [ADR 0015](0015-host-testing-no-std-crates.md) (how invariants are tested)

## Context

`firmware-types` now hosts the first real shared wire type, `PilotCommand`, with a `Throttle` field. `Throttle` is implemented as a newtype around `f32` with a private inner value, a validating constructor (`from_normalised`) that clamps to `0.0..=1.0` and scrubs NaN, and a custom `Deserialize` that routes through the constructor so the invariant holds on values that came off the radio.

The next handful of types follow the same shape but with different constraints:

- **Pitch / roll / yaw rate commands** — `f32`, signed, typically `-1.0..=1.0` (full deflection either side of centre).
- **Arm state** — boolean-ish but really an enum (`Disarmed` / `Armed` / future `ArmedTakeoff`).
- **Battery voltage** — `f32`, positive, plausibly `0.0..=30.0` V (depends on cell count, but bounded).
- **Motor mix output** — `f32`, `0.0..=1.0` per channel (same as throttle, conceptually).

It's tempting to factor the common bits into a shared base — e.g. a `PercentageValue(f32)` or `NormalisedSigned(f32)` parameterised by min/max constants — and have `Throttle`, `Pitch`, `Roll`, etc. all be type aliases or thin wrappers. Less code; one set of tests; one custom `Deserialize`. Standard DRY play.

This ADR records the decision to **not** do that, and to give every physical quantity its own newtype.

## Decision

Every distinct physical quantity that travels on the wire or between tasks gets **its own newtype** in `firmware-types`. No shared `PercentageValue` / `NormalisedFloat` / generic-over-bounds base type.

Concrete shape, illustrated against `Throttle`:

```rust
// crates/firmware-types/src/throttle.rs
pub struct Throttle(f32);          // private inner

impl Throttle {
    pub const ZERO: Self = Self(0.0);
    pub const MAX:  Self = Self(1.0);

    pub fn from_normalised(n: f32) -> Self { /* clamp + NaN scrub */ }
    pub fn as_normalised(self) -> f32      { self.0 }
}

// Custom Deserialize routes through `from_normalised` so wire bytes
// can't construct an out-of-range value. Per ADR 0015 §1.
impl<'de> Deserialize<'de> for Throttle { /* ... */ }
```

Each future quantity (`Pitch`, `Roll`, `Yaw`, `BatteryVoltage`, ...) follows the same template with its own constants, its own clamp range, its own units in the constructor name (`from_radians_per_second`, `from_volts`, ...), its own tests.

### When a macro becomes acceptable

If and only if **three or more** newtypes end up structurally identical (same inner type, same kind of bound, same constructor / accessor signature shape modulo names), a declarative macro `define_clamped_f32!` may be introduced to share the boilerplate. Until then, copy-paste. The macro is an optimisation against repeated typing, not a design statement.

The macro, when it lands, must still produce **distinct nominal types** — not type aliases. The whole point survives only if `fn set_throttle(t: Throttle)` will refuse a `Pitch` at compile time.

## Why distinct newtypes rather than a shared base

- **Different quantities have different invariants.** `Throttle` is `0..=1` (unsigned, "amount of go"). `Pitch` is `-1..=1` (signed, "which way to lean"). `BatteryVoltage` is `0..=30` (unsigned but a real physical unit, not normalised). A shared `PercentageValue` either forces them all into the smallest common shape (loses information) or grows feature flags (`signed`, `bounds`, `units`) until it's a small parameterised library no easier to read than the per-type version.
- **The type system catches argument-order bugs.** `mixer(throttle, pitch, roll, yaw)` with four `f32`s lets you silently swap two arguments and produce a flying-but-wrong drone. With four distinct newtypes, the compiler refuses the call. This is the headline reason — for a flight controller written for learning, where the cost of a swap is "the drone goes the wrong way on a bench test", typed-arguments-by-construction is exactly the lesson worth internalising.
- **Units belong in the type.** `BatteryVoltage(3.7)` reads as "3.7 volts". `PercentageValue(3.7)` requires a comment to know what 3.7 means. The newtype is documentation that can't drift from the code.
- **Defmt output is self-describing.** `Throttle(0.42)` in a log line is unambiguous. `PercentageValue { v: 0.42, kind: Throttle }` is a sad workaround.
- **Tests live next to the type.** A clamp test for `Pitch` belongs in `pitch.rs`, not in a shared test module for the generic base where you have to thread through const-generic bounds to assert anything. Per ADR 0015 §2, inline `mod tests` is the rule; a per-type file matches that cleanly.
- **The boilerplate is genuinely small.** Counting the `Throttle` source file: ~20 lines of impl + Deserialize. Five quantities = ~100 lines, well-localised, and 90% of it is the `clamp` constant and the type name. The cost of repetition is low; the cost of a generic base is high (every reader has to learn it).

## Why not a shared `PercentageValue` or generic base

Briefly enumerated, each rejected:

- **Type alias (`type Throttle = PercentageValue;`)** — defeats the whole purpose. `Throttle` and `Pitch` would be the same type and freely interchangeable. Compile-time argument-order checking is gone.
- **Generic-over-bounds (`Clamped<const LO: i32, const HI: i32>`)** — distinct types, so the safety holds. But const-generic `f32` isn't stable, so bounds would have to be `i32` constants reinterpreted, with `f32::from_bits` ceremony. Massive readability hit for a five-type universe.
- **Trait + per-type marker (`Bounded<Kind = ThrottleKind>`)** — works but introduces a layer of indirection (the `Kind` marker type carries no data, just discriminates). All the cost of generics, none of the readability saved.
- **`derive_more` / `nutype` crate** — pulls in a proc-macro dependency to save a dozen lines per type. Wrong trade for a small `no_std` crate that ADR 0009 wants kept lean.

## Consequences

### What this commits us to

- Each new physical quantity that travels on the wire or between tasks gets its own file in `firmware-types`: `pitch.rs`, `roll.rs`, `yaw.rs`, `battery_voltage.rs`, and so on. Each implements its own validating constructor, accessor, and custom `Deserialize` per ADR 0015 §1.
- Function signatures across the codebase use the typed values, not raw `f32`s. `fn mix(throttle: Throttle, pitch: Pitch, roll: Roll, yaw: Yaw) -> MotorMix` is mandatory; `fn mix(t: f32, p: f32, r: f32, y: f32)` is forbidden in any code path that crosses a task or module boundary.
- Tests for each newtype live inline in its own file (clamp behaviour, NaN scrubbing, postcard round-trip, deserialize-clamps-garbage). Per the `Throttle` template.

### What this rules out

- Sharing a `PercentageValue` / `NormalisedFloat` / `Bounded<...>` base type across distinct quantities.
- Type aliases for physical quantities (`type Throttle = f32;` or `type Pitch = Throttle;`).
- Passing bare `f32`s for physical quantities across task or module boundaries. Local computations inside one function can use `f32` freely; the moment a value crosses a boundary, it's wrapped.

### What stays open

- Whether to introduce a `define_clamped_f32!` declarative macro. Trigger: three or more structurally identical newtypes. Not before.
- Whether to expose a typed `Angle`, `AngularRate`, `Voltage` etc. with proper unit checking (e.g. `uom` crate). Currently rejected as over-engineering for a hobby project, but the door isn't closed.
- The exact field layout of composite types like `PilotCommand` — adding pitch/roll/yaw fields is a future commit, not part of this ADR.

## References

- ADR 0009 — `firmware-types` exists; this ADR governs what lives inside it.
- ADR 0015 — testing pattern and wire-boundary invariant enforcement; this ADR is the type-design counterpart.
- Implementation: [`crates/firmware-types/src/throttle.rs`](../../crates/firmware-types/src/throttle.rs) — canonical template for new quantities.
- [The Rust API Guidelines — "newtype for type safety"](https://rust-lang.github.io/api-guidelines/type-safety.html#newtypes-provide-static-distinctions-c-newtype) — ecosystem precedent.
