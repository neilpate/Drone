# ADR 0009 — Workspace bootstrap and crate naming

- **Status:** Accepted
- **Date:** 2026-05-23
- **Related:** [ADR 0005](0005-pc-software-language-rust.md), [ADR 0007](0007-testing-and-ci-strategy.md), [ADR 0008](0008-repository-folder-layout.md)

## Context

[ADR 0008](0008-repository-folder-layout.md) settled the top-level repo layout but deferred three sub-questions to Phase 1 design:

- What do we call the firmware crates?
- How is the mandatory `core` / `task` split from [ADR 0007](0007-testing-and-ci-strategy.md) physically realised — sub-modules in one crate, or sibling crates?
- Does the first Phase 1 commit set up a full Cargo workspace, or just a standalone crate?

Phase 1 is now starting (drone firmware first). These have to be answered before any `Cargo.toml` exists, because every later crate inherits the conventions chosen here.

## Decision

### 1. Crate naming convention: `firmware-<role>` for on-target binaries

- Drone firmware: **`firmware-drone`** (binary) + **`firmware-drone-core`** (library).
- Ground micro:bit firmware (later): **`firmware-ground`** + **`firmware-ground-core`** if it earns a split.
- Custom-PCBA firmware (Phase 4+): **`firmware-pcba`** (or similar role-suffixed name; locked when that crate lands).

PC-side and shared crates keep plain role names:

- **`groundstation`** — PC-side application.
- **`proto`** — shared wire-format crate.
- **`xtask`** — build runner.

Rationale: the `firmware-` prefix groups all on-target binaries together alphabetically in `crates/`, in `cargo tree`, and in CI matrix output. It also makes the role obvious at the call site — `cargo run -p groundstation` vs `cargo flash -p firmware-drone` reads as what it is.

### 2. `core` / `task` split realised as **sibling crates**

For every firmware actor-set that earns the split (mandatory per [ADR 0007](0007-testing-and-ci-strategy.md) for the drone firmware; deferred decision for `firmware-ground`):

- **`firmware-<role>-core`** — `lib` crate, `#![no_std]`, host-testable. **Forbidden** from depending on `embassy-*`, `defmt`, `panic-probe`, or any HAL.
- **`firmware-<role>`** — `bin` crate, `#![no_std]` `#![no_main]`. Depends on `firmware-<role>-core` plus the HAL / async runtime / logging stack.

The discipline is enforced **by the Cargo dependency graph**: if the `-core` crate's `Cargo.toml` does not list `embassy-nrf`, the compiler refuses any `use embassy_nrf::...` regardless of programmer intent or review carefulness.

Sub-modules-in-one-crate was considered (cheaper file count, no extra `Cargo.toml`) and rejected because the discipline relies on `[features]` gymnastics and human vigilance instead of the build graph. The failure mode — `defmt` macros expanding silently in host tests, or accidentally pulling HAL types into pure logic — is exactly the kind of thing the rule exists to prevent.

A secondary benefit: a `-core` crate is freely depended on by other crates (e.g. `firmware-ground` may want to call into shared framing logic in `firmware-drone-core`, or the future `groundstation` may want to replay sensor-fusion math). The internals of a `bin` crate cannot be shared this way.

### 3. Full Cargo workspace from the first commit

The first Phase-1 commit creates the top-level workspace skeleton even though it will contain only two members initially:

```
Cargo.toml              # [workspace] members = ["crates/*"]
crates/
├── firmware-drone-core/
│   └── Cargo.toml
└── firmware-drone/
    ├── Cargo.toml
    ├── memory.x
    └── .cargo/config.toml
```

Top-level `Cargo.toml` is a pure workspace manifest (no `[package]`). It also hosts `[workspace.dependencies]` and `[workspace.lints]` so version pinning and lint configuration are inherited by every crate.

A single shared `target/` and `Cargo.lock` at the repo root follows automatically. `cargo test`, `cargo check`, `cargo clippy` from the repo root operate on the whole workspace — directly supporting the "one recipe, two callers" rule from [ADR 0007](0007-testing-and-ci-strategy.md).

## Why this shape

- **Naming consistency now is free.** Renaming a published crate later is painful (every `use`, every `Cargo.toml`, every doc link). Picking the prefix scheme on day one avoids a future "rename party".
- **Sibling crates is the discipline-by-construction choice.** Matches "make the invariant impossible to violate" — the same instinct that put the `core` / `task` split into [ADR 0007](0007-testing-and-ci-strategy.md) in the first place.
- **Workspace-from-day-one removes a known-imminent refactor.** ADRs 0005 and 0007 already commit us to at least `proto`, `firmware-ground`, `groundstation`, and `xtask` within weeks. Standing the workspace up now costs roughly ten lines of TOML; converting a single crate into a workspace later means moving files, regenerating `Cargo.lock`, and rewiring `target/`.

## Consequences

### What this commits us to

- All on-target firmware binaries get the `firmware-<role>` name.
- Every firmware crate that earns the `core`/`task` split realises it as `<bin>` + `<bin>-core` sibling crates. The `-core` crate's `Cargo.toml` must not list any HAL, async-runtime, or device-logging dependency.
- A top-level `Cargo.toml` defining a workspace exists from the first Phase 1 commit. New crates are added by appending to `[workspace.members]` (or via the glob), not by creating parallel workspaces.
- `[workspace.dependencies]` is the canonical place to pin shared crate versions (Embassy stack, `defmt`, `serde`, etc.). Per-crate `Cargo.toml`s reference them with `workspace = true`.
- `cargo test` / `cargo check` from the repo root must work and exercise everything host-runnable.

### What this rules out

- Single-crate firmware ("`drone-firmware`" doing everything in one bin). The split is mandatory before the first non-trivial logic lands.
- Standalone-crate-then-workspace bootstrap path. We are not adopting workspaces later "if it gets complex enough" — we know from the existing ADRs that it will.
- HAL / `defmt` / `panic-probe` dependencies appearing in any `-core` crate's `Cargo.toml`. Reviewer reflex: scan the `-core` `[dependencies]` block on every PR that touches it.
- Crate names that imply role without the `firmware-` prefix for on-target binaries (e.g. plain `drone`, `fc`, `quad`). Reserved for non-firmware roles (`groundstation`, `proto`, `xtask`).

### What stays open

- **Whether `firmware-ground` earns a `core`/`task` split.** It may be thin enough (transparent USB ↔ radio bridge) that there is no pure logic worth extracting. Decided when that crate lands.
- **Whether `proto` splits.** Currently one crate; may grow into `proto-wire` (framing / serialization) + `proto-types` (message definitions) if either side needs only one half.
- **Sub-grouping inside `crates/`** (e.g. `crates/firmware/`, `crates/pc/`, `crates/shared/`) — deferred per [ADR 0008](0008-repository-folder-layout.md) until the workspace grows past ~8 members.
- **Toolchain channel pinning (`rust-toolchain.toml`), Rust edition, target triple defaults, and `[workspace.lints]` content.** Tactical setup, not ADR-worthy on its own; lands with the first commit and is recorded in `doc/dev-environment.md`.
