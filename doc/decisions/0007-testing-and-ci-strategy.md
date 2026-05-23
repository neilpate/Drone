# ADR 0007 — Testing and CI strategy

- **Status:** Accepted
- **Date:** 2026-05-23
- **Related:** [ADR 0001](0001-platform-airframe-stack.md), [ADR 0004](0004-concurrency-embassy-channels.md), [ADR 0005](0005-pc-software-language-rust.md), [02-architecture.md](../02-architecture.md)

## Context

The project is a roll-your-own flight controller built primarily for learning ([ADR 0001](0001-platform-airframe-stack.md)). The blast radius of "code that compiles, flashes, doesn't work, kills props" is real even on the bench, and gets worse at every phase boundary. We also commit often (AGENTS.md commit discipline) and want to keep `main` runnable on real hardware at all times.

Two things make testing genuinely tractable here:

1. **A lot of flight-controller logic is pure functions of numbers.** Mahony filter, PID, motor mixer, attitude math, quaternion ops, protocol serialization — none of it needs a HAL to be exercised. If we structure the code so this logic doesn't accidentally depend on the HAL, we can test it on the host with regular `cargo test`.
2. **Embedded Rust HIL (hardware-in-the-loop) tooling has matured.** `embedded-test` lets `#[test]` functions compile into a firmware binary, run on real hardware via `probe-rs`, and report results back to the host as if they were normal `cargo test` output. Self-hosted GitHub Actions runners with tethered boards make this a viable CI tier, not just a manual workflow.

The user's stated preferences: **unit test everything possible; quick local feedback before anything reaches CI.**

## Decision

### 1. Tests are first-class, not optional

- **All non-trivially-correct logic gets unit tests.** The bar is "can I write a host-runnable test for this?" not "do I have time for tests today?". If the answer is yes, the test exists.
- **Algorithmic / numeric code is mandatory to test.** Filters, controllers, mixers, attitude math, framing, serialization — no excuses.
- **Glue / HAL-wiring code is exempt** from unit tests (until HIL exists), but should be kept thin enough that it can't go wrong in interesting ways.

### 2. `core` / `task` split (architectural rule)

To make rule 1 possible, every firmware actor is structured as **two pieces**:

- **`core`** — a pure module of types and functions with no HAL dependency, no timers, no DMA, no Embassy. Takes inputs, returns outputs. Host-testable with `cargo test` on the workstation.
- **`task`** — the thin Embassy task wrapper that owns peripherals, reads from the inbox channel, calls `core`, and dispatches the results. Not unit-tested; covered by HIL later.

This is a hard rule, not a guideline: if a new actor can't be split this way, that is itself a design smell worth fixing.

### 3. Local-first feedback loop

- **CI and "what I run locally" are the same recipe.** A single task-runner script (likely `cargo xtask`; tool not formally locked here) defines the full check suite. Both the local git hook and the GitHub Actions workflow invoke it. No drift.
- **A `pre-push` git hook is committed to the repo** under `tools/git-hooks/`, with a one-shot installer script that the developer runs after cloning. The hook runs the same recipe. Push is blocked if it fails.
- **`--no-verify` is the conscious escape hatch.** The hook stays strict; bypass it deliberately when needed.

### 4. CI tiers, adopted in order

- **Tier 0 — Static checks (from day one of code):** `cargo fmt --check`, `cargo clippy -D warnings`, `cargo check`, `cargo doc --no-deps`, across all crates and targets. GitHub Actions on every push and PR.
- **Tier 1 — Host-side unit tests (as soon as testable code exists):** `cargo test` on the workspace, exercising every `core` module and the `proto` crate. Run locally first (rule 3) and in CI.
- **Tier 2 — Hardware-in-the-loop (desired end state, deferred):** `embedded-test` running on real micro:bit / nRF5340 hardware via `probe-rs`, on a **self-hosted GitHub Actions runner** with the board permanently tethered. Adopted when project complexity (probably Phase 3+) makes regressions on hardware expensive enough to justify the setup.

Tier 0 and 1 are mandatory. Tier 2 is a planned future investment.

### 5. Tooling left deliberately unspecified

The following are *not* locked in by this ADR (deferred to a future `doc/05-ci-and-testing.md` written when the first crate lands):

- The specific task-runner (`cargo xtask` is the leading candidate; `just` is acceptable).
- The exact GitHub Actions YAML layout, action versions, cache strategy.
- The hardware platform for the self-hosted runner (Raspberry Pi, mini-PC, repurposed laptop).
- Whether to add a fast `pre-commit` hook alongside `pre-push`.
- Whether to add `cargo deny` and at what cadence.

These are tools, not decisions. The decisions above govern them.

## Why this shape

- **Quick local feedback is the highest-leverage developer ergonomic available.** CI catching a broken push is *useful*. The local hook catching it before push, in seconds, is *transformative* — it changes how often you're willing to refactor.
- **One recipe, two callers** is the only sane way to keep CI and local in sync over time. Anything else drifts.
- **The `core`/`task` split is a forcing function for testable architecture.** If you don't insist on it from day one, retrofitting it later is painful. The Embassy actor pattern already pushes us toward "task owns state, communicates via channels" — extracting the pure logic into a `core` module costs ~nothing on top.
- **HIL deferred but explicitly planned** is the honest position. Setup investment is non-trivial and the value scales with project complexity. Committing to it now without setting it up now is the right balance.

## Consequences

### What this commits us to

- Every firmware actor split into `core` + `task` from the first crate onwards.
- A committed `tools/git-hooks/pre-push` (or equivalent) and one-shot installer.
- A task-runner script that defines the full check suite, called by both hook and CI.
- A GitHub Actions workflow file from the first crate onwards. Tier 0 minimum; Tier 1 as soon as tests exist.
- Architectural discipline: if you can't write a host-runnable test for a piece of pure logic, that's a code-organisation bug to fix.

### What this rules out

- Untested numerical / algorithmic / protocol code merging to `main`.
- CI and local check suites diverging.
- Bypassing local checks silently (`--no-verify` is allowed when conscious, but the hook is not weakened to make bypass easier).
- "We'll add tests later" as a posture. Tests are part of the work, not a follow-up task.

### What stays open

- The task-runner choice (`cargo xtask` vs `just`) — picked when the first crate lands.
- The hook framework (raw git hook + installer script vs `cargo-husky` magic) — picked then too.
- The self-hosted runner hardware and setup process — Phase 2/3 problem.
- The decision to adopt `embedded-test` specifically (vs alternative HIL approaches) — Phase 2/3 problem.
- Coverage tooling — currently parked as "probably not worth it on `no_std`"; revisit if the project gets big enough.
