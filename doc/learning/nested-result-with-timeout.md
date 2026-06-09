# Nested `Result` from `with_timeout`

_Captured 2026-05-30 while writing `receive()` in `firmware-remote/src/tasks/drone_link.rs` (then named `comm_link.rs`). Easily the most confusing Rust syntax encountered so far._

## The question

`embassy_time::with_timeout(duration, future)` wraps any future with a timeout. Its signature is roughly:

```rust
pub async fn with_timeout<F: Future>(duration: Duration, fut: F)
    -> Result<F::Output, TimeoutError>;
```

If you call it on a future that *itself* returns a `Result`, you get a `Result<Result<T, E>, TimeoutError>` — a `Result` inside a `Result`. Three things can have happened:

1. The future finished successfully.
2. The future finished with an error.
3. The timeout fired (the future never finished).

The Rust syntax for distinguishing all three reads alien at first. This note collects the patterns I tried, what they look like, and which one is idiomatic.

## The concrete case

```rust
match with_timeout(
    Duration::from_millis(100),
    radio.receive(&mut rx_packet),
).await {
    Ok(Ok(())) => { /* got a packet */ }
    Ok(Err(e)) => { /* radio error */ }
    Err(_)     => { /* timeout */ }
}
```

- Outer `Result` is from `with_timeout`: `Ok(inner_result)` if the future completed, `Err(TimeoutError)` if it didn't.
- Inner `Result` is from `radio.receive`: `Ok(())` on success, `Err(RadioError)` on failure.

The patterns `Ok(Ok(()))` and `Ok(Err(e))` look like nested function calls but they aren't — they're patterns describing the *shape* of the value. They read "outer is `Ok`, and the thing inside is `Ok(())`" or "outer is `Ok`, and the thing inside is `Err(e)`."

This is the same idea as destructuring a struct in a function argument: you're not building anything, you're matching a layout that already exists.

## Three styles, all equivalent

### Style 1: one flat match (recommended)

```rust
match with_timeout(Duration::from_millis(100), radio.receive(&mut rx_packet)).await {
    Ok(Ok(()))  => {}                                                       // success — fall through
    Ok(Err(e))  => { defmt::warn!("radio error: {:?}", e); return None; }
    Err(_)      => { defmt::warn!("timeout");              return None; }
}
```

Three arms, three outcomes, one match. This is the version a Rust reviewer would write. The nested `Ok(Ok(()))` pattern is the bit that looks weird but it's purely descriptive — read it as "outcome is Ok, and inside is Ok of unit."

### Style 2: peel one layer at a time

```rust
let outcome = with_timeout(Duration::from_millis(100), radio.receive(&mut rx_packet)).await;

let rx_result = match outcome {
    Ok(r) => r,                                                             // unwrap outer
    Err(_) => { defmt::warn!("timeout"); return None; }
};

if let Err(e) = rx_result {                                                 // check inner
    defmt::warn!("radio error: {:?}", e);
    return None;
}
```

More lines, but each step is one decision: "did the timeout fire? OK, now did the radio succeed?" Useful as a teaching aid; verbose for production. The intermediate `rx_result` is the only thing carrying meaning — without a name for the peeled value, you'd be back to the nested pattern from Style 1.

### Style 3: `.ok()??` chain (terse, loses logging)

```rust
with_timeout(Duration::from_millis(100), radio.receive(&mut rx_packet))
    .await
    .ok()?      // Result<_, TimeoutError> → Option, ? returns None on timeout
    .ok()?;     // Result<(), RadioError>  → Option, ? returns None on radio error
```

Four lines, no match at all. The trick:

- `.ok()` on a `Result<T, E>` discards the error and gives `Option<T>`.
- `?` on an `Option` inside a function returning `Option<_>` unwraps `Some` or returns `None`.

Together, `.ok()?` reads as "if this succeeded, give me the value; otherwise bail out of the function with `None`." Apply twice to peel both layers.

The cost: you can't tell whether the function returned `None` because of a timeout or because of a radio error. For initial bring-up that's fine. Once you have a failsafe (drone disarms after N missed commands), you'll want to distinguish them — at which point Style 1 returns.

## What I tried that did not work

### Mistake 1: empty `Ok` arm

```rust
let rx_result = match outcome {
    Ok(r) => {},                  // <-- returns () instead of r
    Err(_) => return None,
};
```

The `{}` block evaluates to unit `()`, so `rx_result: ()`. The next `if let Err(e) = rx_result` then fails to compile because you can't pattern-match `Err(e)` against `()`. Fix: `Ok(r) => r,` (no braces — or `Ok(r) => { r }` with the value as the last expression).

This is the single most common Rust beginner mistake when coming from C/JS: forgetting that **blocks are expressions** and an empty block has value `()`, not "no value."

### Mistake 2: mixing styles half-and-half

After peeling the outer layer into `rx_result: Result<(), RadioError>`, then trying to match it with arms from Style 1:

```rust
match rx_result {
    Ok(Ok(())) => {}             // <-- won't compile
    Ok(Err(e)) => { ... }        // <-- won't compile
    Err(_)     => { ... }
}
```

`rx_result` only has one layer of `Result` left, so patterns like `Ok(Ok(()))` don't match its type. The compiler error is clear once you see it, but easy to produce while refactoring between styles. Pick one and commit to it.

## Why this is awkward in Rust specifically

In C you'd write:

```c
if (timed_out) { ... return; }
int rc = radio_receive(&pkt);
if (rc != 0)   { ... return; }
// use pkt
```

Two flat checks, each on a single value. The error path and the timeout path are separate concepts at the source level.

Rust folds both into the type system: a `Result` is the value-or-error, and `with_timeout` adds another `Result` layer for the timeout-vs-completed distinction. This is more honest (everything is in the type) and forces you to handle every case (the compiler will refuse a non-exhaustive match), but it produces these nested types that look noisy at first.

The cure is to recognise the pattern. Once you've seen `Result<Result<T, E1>, E2>` a few times — and it shows up *everywhere* `with_timeout` is used — the three-arm match becomes muscle memory.

## The general lesson

When a future-returning combinator wraps another future-returning thing that has its own error, you get nested `Result`s. Don't fight it. Pattern-match it flat:

| Outer | Inner   | Meaning                          |
|-------|---------|----------------------------------|
| `Ok`  | `Ok(v)` | Both succeeded                   |
| `Ok`  | `Err(e)`| Inner future failed              |
| `Err` | _n/a_   | Outer combinator failed (timeout)|

For two-layer cases, one flat match is the right answer. For deeper nesting (rare), peel layer-by-layer with named intermediates so each decision is readable.

## Related Embassy combinators that produce nested results

- `with_timeout(d, fut)` → `Result<Output, TimeoutError>`.
- `select(a, b)` / `select3` / `select4` → `Either<A::Output, B::Output>` (not a `Result` but the same shape-matching applies).
- `join(a, b)` → `(A::Output, B::Output)` — a tuple. If both are `Result`s, you have `(Result, Result)` to inspect.

Same playbook for all of them: write out the three (or four) arms explicitly, let the compiler check exhaustiveness, ship.

## Sources

- Embassy `with_timeout`: <https://docs.embassy.dev/embassy-time/git/default/fn.with_timeout.html>
- Rust book, "Patterns and Matching": <https://doc.rust-lang.org/book/ch18-00-patterns.html>
- Rust by Example, "Combinators: and_then, etc.": <https://doc.rust-lang.org/rust-by-example/error/option_unwrap/and_then.html> (alternative style: chain combinators instead of matching)
