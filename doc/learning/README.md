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
