# CI and testing

How the project keeps `main` green. Tracks the strategy in [ADR 0007](decisions/0007-testing-and-ci-strategy.md); this file is the practical "how to run it" companion.

## What is tested where

Host-testable logic lives in crates with no HAL / async-runtime / `defmt` dependency, so `cargo test` runs them on the developer's workstation with no cross toolchain:

- `firmware-types` — wire types, newtype invariants (`Throttle` clamping / NaN-scrub), postcard round-trips.
- `firmware-drone-core` — the supervisor failsafe state machine.
- `firmware-remote-core` — remote-side pure logic (input mapping, parsing) as it lands.
- `groundstation` — host-only egui app; pure helpers (state mapping, framing) as they are extracted out of `main.rs`.

On-target firmware crates (`firmware-drone`, `firmware-remote`) need a cross target and per-crate linker flags, so they are **not** part of `cargo test`. Their logic is meant to live in the matching `-core` crate. Hardware-in-the-loop testing is deferred (ADR 0007).

## Running the tests

Tests are run with [cargo-nextest](https://nexte.st/), which gives a single aggregated summary (`Summary [..] N tests run: N passed`) instead of one block per test binary. Install it once:

```sh
cargo install cargo-nextest --locked
```

From the repo root:

```sh
cargo nextest run                                              # workspace default-members
cargo nextest run --manifest-path crates/groundstation/Cargo.toml   # host-only, excluded from the workspace
```

The root run honours `default-members` in the top-level `Cargo.toml`, which lists exactly the host-testable workspace crates (per ADR 0015). `groundstation` is intentionally excluded from the workspace, so it needs its own invocation.

Plain `cargo test` still works and is the fallback if nextest is not installed; it just prints per-binary results and does not aggregate. Note nextest does not run doctests (there are none currently); if any are added, run `cargo test --doc` alongside.

## Pre-push hook

A tracked `pre-push` hook runs both test scopes before any push is allowed, so a green `main` does not depend on remembering to run `cargo test`.

The hook lives at [`.githooks/pre-push`](../.githooks/pre-push) (tracked in the repo, unlike `.git/hooks/`). Git does not use a tracked hooks directory automatically — **enable it once per clone**:

```sh
git config core.hooksPath .githooks
```

To bypass in a genuine emergency: `git push --no-verify` (discouraged; the next push runs the tests anyway).

The hook runs `cargo nextest run` only (and fails with an install hint if nextest is missing). Format and lint enforcement (`cargo fmt --check`, `cargo clippy`) per [ADR 0012](decisions/0012-lint-and-format-policy.md) is run manually for now; folding it into the hook or a future CI runner is a candidate when the test habit is established.
