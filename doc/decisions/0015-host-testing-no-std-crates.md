# ADR 0015 — Host-testable `no_std` crates: wiring pattern

- **Status:** Accepted
- **Date:** 2026-06-01
- **Related:** [ADR 0007](0007-testing-and-ci-strategy.md) (strategy), [ADR 0009](0009-workspace-bootstrap-and-crate-naming.md) (crate split), [ADR 0012](0012-lint-and-format-policy.md)

## Context

[ADR 0007](0007-testing-and-ci-strategy.md) committed us to unit-testing all non-trivial logic on the host, and made the `core` / `task` split a hard rule precisely so this is possible. It deliberately left tooling and concrete wiring open.

The first real test suite (`crates/firmware-types`, postcard round-trip and `Throttle` invariant tests) forced those open questions to be answered. The crate is `no_std` on the target but needs `std` in `cargo test`. Several papercuts surfaced:

1. **`#![no_std]` blocks `cargo test`.** The default test harness needs `std` (the `test` crate links it). A crate marked `#![no_std]` unconditionally won't build for tests on the host.
2. **Workspace-shared `postcard` pulls in `use-defmt`.** Our `[workspace.dependencies] postcard` enables `use-defmt` for the on-target use. As a `[dev-dependencies]` reused via `workspace = true`, the feature came along too, and the test binary failed to link with unresolved `_defmt_*` externals because no global logger is registered on the host.
3. **`cargo test` from the workspace root tries to test firmware crates.** `firmware-drone` / `firmware-remote` are `no_std` Cortex-M binaries with custom panic handlers. Compiling their test harness for the host fails with `can't find crate for test` and `#[panic_handler] function required`.
4. **VS Code Testing view's "Run All" invokes `cargo test --workspace`,** which trips problem 3. Per-test CodeLens arrows work because they pass `-p <crate>`.

None of these are decisions in the ADR-0007 sense — they're implementation details. But the right shape is non-obvious if you've not hit it before, and getting it wrong wastes a session. Worth recording the pattern once.

## Decision

### 1. Host-testable `no_std` crates use conditional `no_std`

For any crate that compiles `no_std` on target but needs `std` for its test harness:

```rust
#![cfg_attr(not(test), no_std)]
```

Effect: `no_std` for normal builds (target and host alike), `std` for `cargo test`. The crate's own code must stay `no_std`-compatible — `cargo build` from the repo root will still fail any `std`-only API use.

This applies to `firmware-types`, `firmware-drone-core`, `firmware-remote-core`, and any future `*-core` / shared crate.

It does **not** apply to firmware bin crates (`firmware-drone`, `firmware-remote`). Those stay `#![no_std]` unconditionally — they have no host tests and live behind the `default-members` filter (see point 4).

### 2. Tests live as inline `#[cfg(test)] mod tests`

Per-file `mod tests` blocks at the bottom of the source file, **not** a top-level `tests/` directory. Reasons:

- Inline tests can exercise private items via `use super::*;`. The wire-format invariant tests on `Throttle` need that — the inner `f32` is private.
- `tests/` integration tests compile as separate crates and can only see the public API. Use them only when the test deliberately wants to assert "this works the way a downstream consumer would use it".
- Inline tests are conventional in idiomatic Rust crates (per AGENTS.md "prefer the idiomatic choice").

### 3. `[dev-dependencies]` are pinned **directly**, not via `workspace = true`

When a host test needs a crate that the same project uses on target with different features, declare it under `[dev-dependencies]` with its own explicit version and feature set:

```toml
[dev-dependencies]
# Pinned directly (not via workspace) to avoid the workspace `use-defmt`
# feature, which would require a global defmt logger that doesn't exist
# on host.
postcard = { version = "1", default-features = false }
```

The version still matches the workspace pin (kept consistent by review, same as any duplicated literal). The comment is mandatory — it tells the next reader why the pattern was chosen over `workspace = true`.

This is the **only** sanctioned exception to ADR 0009's "use `workspace = true` for shared deps" rule. The cost (one duplicated version literal) buys us a clean host build without polluting target features.

### 4. Workspace tests run via `default-members`, never `--workspace`

`Cargo.toml` already lists host-buildable crates in `[workspace.default-members]`. `cargo test` from the repo root (no flags) honours that list and skips firmware bin crates. **This is the canonical command for running the full host test suite.**

- `cargo test --workspace` is **forbidden** in this project. It will try to build firmware bin crates for the host and fail.
- `cargo test -p <crate>` is the right thing for focused runs.
- CI invokes `cargo test` from the repo root, no `-p`, no `--workspace`.

### 5. VS Code Test Explorer is best-effort

rust-analyzer's Testing view ("beaker icon") is enabled (`"rust-analyzer.testExplorer": true`). It populates from discovered `#[test]` functions and lets you run individual tests or whole modules with their correct `-p <crate>` scope.

The **"Run All Tests"** button at the top of the Testing view invokes `cargo test --workspace` with no opt-out. **Do not use it.** Use one of:

- **CodeLens ▶ Run Test** arrows above each `#[test]` (per-test, correctly scoped). Primary workflow.
- **CodeLens ▶ Run Tests** above each `mod tests` (per-module).
- **Terminal:** `cargo test` from the repo root for the full suite (honours `default-members`).

A VS Code task bound to a hotkey is a reasonable upgrade if the terminal hop becomes annoying; not done preemptively.

## Why this shape

- **`cfg_attr(not(test), no_std)`** is the standard pattern in the embedded Rust ecosystem (used in `heapless`, `embedded-hal`, `serde-core` and many others). It costs one line and a one-time conceptual step; the alternative ("two crates, one `no_std`, one `std`-using wrapper for tests") is far heavier for the same outcome.
- **Inline `mod tests`** is what `cargo new --lib` generates and what 90% of the ecosystem uses. Privacy access is the killer feature; we use it on the `Throttle` invariant tests.
- **Pinned dev-dependencies for feature divergence** is uglier than `workspace = true`, but the alternative is either (a) splitting our `postcard` workspace dep in two (`postcard-target` / `postcard-host`) which is worse, or (b) silently broken host builds. Pin + comment wins.
- **`default-members` over `--workspace`** is a feature explicitly designed for mixed embedded/host workspaces. Using it is the idiomatic answer, not a workaround.
- **Accepting "Run All Tests" is broken in VS Code** is the honest position. rust-analyzer has no setting to scope it; chasing a workaround (e.g. `rust-analyzer.runnables.extraArgs = ["-p", "firmware-types"]`) would scope **every** runnable to one crate, breaking per-test runs in any future testable crate. Better to teach the (single) developer to use the working buttons.

## Consequences

### What this commits us to

- Every host-testable `no_std` crate uses `#![cfg_attr(not(test), no_std)]`.
- Tests live inline in `#[cfg(test)] mod tests`. `tests/` only for deliberate "integration test against public API".
- New `[dev-dependencies]` whose feature set differs from the on-target use are pinned directly with a comment explaining why.
- The canonical "run all host tests" command is `cargo test` (no flags) from the repo root.
- CI uses the same command.
- The pre-push hook (when it exists, per ADR 0007) will use the same command.

### What this rules out

- `#![no_std]` on host-testable crates (use `cfg_attr` instead).
- `cargo test --workspace` invocations anywhere — scripts, CI YAML, documentation, terminal aliases.
- Treating the Testing view's "Run All" button as a working primary workflow.
- Adding test-only test infrastructure to firmware bin crates without first splitting the testable logic into a `*-core` crate (which is already ADR 0007's hard rule).

### What stays open

- Whether to add a `[tasks]` or `xtask` entry that wraps `cargo test` + `cargo clippy` + `cargo fmt --check` as a single command. Deferred to when the pre-push hook lands (ADR 0007).
- Coverage tooling. Same parking-lot status as ADR 0007.
- HIL test wiring (`embedded-test`) — Tier 2 of ADR 0007, not in scope here.

## References

- ADR 0007 — testing strategy (the "what" and "why" this ADR realises).
- ADR 0009 — crate naming and the `workspace = true` rule that point 3 carves an exception to.
- Implementation: [`crates/firmware-types/`](../../crates/firmware-types/) — first crate to use this pattern end-to-end.
- [The Embedded Rust Book — `no_std` testing](https://docs.rust-embedded.org/book/intro/no-std.html) — background on the `cfg_attr(not(test), no_std)` idiom.
