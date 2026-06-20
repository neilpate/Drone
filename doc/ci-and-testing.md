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

The hook runs `cargo nextest run` only (and fails with an install hint if nextest is missing). Format and lint enforcement (`cargo fmt --check`, `cargo clippy`) per [ADR 0012](decisions/0012-lint-and-format-policy.md) runs in CI rather than the hook, to keep the local push fast.

## GitHub Actions

[`.github/workflows/ci.yml`](../.github/workflows/ci.yml) runs on every push to `main` and on every pull request. It is the server-side backstop the local hook cannot guarantee (a contributor may not have enabled the hook, or may have used `--no-verify`).

Two `ubuntu-latest` jobs run in parallel, each doing `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` ([ADR 0012](decisions/0012-lint-and-format-policy.md)) and `cargo nextest run`:

1. **host-checks** — the workspace host crates (`default-members`: `firmware-types` + the `*-core` crates). No system dependencies.
2. **groundstation** — the out-of-workspace GUI crate. It first installs the Linux libraries `eframe`, `gilrs` and `serialport` link against (`libudev-dev`, `libxkbcommon-dev`, `libwayland-dev`, the X11/GL dev libs); these are needed only to compile and link — the tests open no window, so no headless display is required.

On-target firmware crates need a cross target and hardware, so they are not built in CI (HIL deferred, ADR 0007). Rust is the stable toolchain; nextest is installed via [`taiki-e/install-action`](https://github.com/taiki-e/install-action) and the cargo build is cached with [`Swatinem/rust-cache`](https://github.com/Swatinem/rust-cache).
