# Learning notes

Short notes on things I did not know before starting this project. Rust, Embassy, ARM, embedded conventions — anything that came up during real work and was worth writing down so future-me does not have to re-derive it.

Different from `decisions/` (ADRs are project choices) and `research/` (external references). These are personal "I learned a thing" notes, scoped to topics that actually appeared while building.

## Conventions

- One topic per file. Short. Linkable.
- Lead with the question or the observation that prompted the note. Then the answer. Then optional detail.
- Compare to systems programming / C where it helps the intuition.
- Date significant additions: `(YYYY-MM-DD)`.
- File naming: `topic-name.md`, lowercase, hyphenated.

## Index

- [rust-value-semantics.md](rust-value-semantics.md) — returning values, RVO, sret, AAPCS registers, why "return by value" is free. (2026-05-30)
- [nested-result-with-timeout.md](nested-result-with-timeout.md) — `with_timeout` produces `Result<Result<T, E>, TimeoutError>`; the three-arm flat match, the peel-layer-by-layer alternative, and the `.ok()??` shortcut. (2026-05-30)
- [slices-and-arrays.md](slices-and-arrays.md) — `[T; N]` vs `[T]` vs `&[T]`; fat pointers; why slices are the answer to C's "pass an array and a length" pattern; relevance for `no_std`. (2026-05-30)
- [gpio-state-during-reset.md](gpio-state-during-reset.md) — why the motor spun during firmware flashing: GPIO is hi-Z at reset, an active-low driver with a floating input means "on"; pull-up resistors and proper enable pins as the two fixes. (2026-06-10)
- [embassy-time-and-tickless-timers.md](embassy-time-and-tickless-timers.md) — how `Timer::after_secs(1).await` actually works: compile-time `TICK_HZ`, the 32_768 Hz RTC, tickless scheduling with compare registers, multiple pending timers from one hardware register. (2026-06-16)
- [serial-uart-framing-and-gotchas.md](serial-uart-framing-and-gotchas.md) — telemetry over the serial link: packet-vs-stream framing, postcard + COBS, `CobsAccumulator`, embassy `UarteRx::read` fills the whole buffer, `serialport::try_clone` deadlocks on Windows, single-field structs are wire-identical to their field, and `Watch::get().await` stalling until the first publish (seed a safe neutral default at startup). (2026-06-19)
- [cpu-load-profiling-idle-spinner.md](cpu-load-profiling-idle-spinner.md) — measuring CPU load with no OS: a lone lowest-priority spinner of fixed work timed against the free-running RTC, `load = 1 - T0/T1`; why the spinner must be alone and never `.await`, `black_box`/`inline(never)`, defmt's missing float precision and the basis-point trick, and the cooperative-starvation cliff. (2026-06-26)
- [postcard-maxsize-sizing-buffers.md](postcard-maxsize-sizing-buffers.md) — a `SerializeBufferFull` hard-fault from a hand-picked `[0u8; 32]` buffer; deriving postcard's `MaxSize` (`experimental-derive`) to compute the worst-case serialized size at compile time, how postcard counts bytes (varint/f32/struct/enum), COBS framing overhead, and exporting one `FRAME_MAX_SIZE_BYTES` const so every buffer grows with the wire type. (2026-06-26)
- [micromath-atan2-nan-and-valid-seeds.md](micromath-atan2-nan-and-valid-seeds.md) — estimated roll came back NaN on hardware but not host: micromath's `atan2` returns NaN at the origin where std returns 0, the estimator seeded from an all-zero (physically impossible) IMU default, NaN is absorbing so it stuck forever; fix by making seed defaults physically valid (1 g down) and scrubbing NaN at the angle-newtype boundary. Host tests can't catch target-only numeric edge cases. (2026-07-21)
