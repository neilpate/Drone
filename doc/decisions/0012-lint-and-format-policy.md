# ADR 0012 — Lint and format policy: rustfmt + clippy clean before commit

- **Status:** Accepted
- **Date:** 2026-05-24
- **Related:** [ADR 0007](0007-testing-and-ci-strategy.md)

## Context

`rustfmt` and `clippy` are the Rust ecosystem's canonical formatter and linter.
Both ship with the toolchain, both have well-curated defaults, and both are
universally expected to be clean on any serious Rust codebase. The
showcase-quality bar in [doc/00-vision.md](../00-vision.md) calls for
"current best practice for embedded Rust" — running both, and treating their
output as build errors rather than suggestions, *is* current best practice.

The local setup is already in place ([.vscode/settings.json](../../.vscode/settings.json)):
`rustfmt` runs on save, `rust-analyzer` runs `cargo clippy` as the on-save
check. What's been missing is the *rule* that commits and `main` stay clean
against both, with an explicit policy on the rare case where a lint is
genuinely wrong.

## Decision

### 1. `main` is `rustfmt`-clean and `clippy`-clean at all times

Every commit landing on `main` satisfies, against the active board feature:

```pwsh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

When CI lands (separate issue), it will run exactly this gate. Until then
it is a personal-discipline gate. Both commands run fast on this workspace;
there is no excuse for skipping them.

### 2. Suppressions require a justification

A clippy lint can be wrong for the local context (false positive, embedded
specific footgun the lint doesn't model, or a deliberate stylistic choice
that's been weighed and kept). When that happens:

- Suppress at the **narrowest scope** that fixes it: prefer `#[allow(...)]`
  on the item, not on a module, never crate-wide.
- **Always attach a comment** on the line above the `#[allow]` explaining
  *why* the lint is wrong here. A bare `#[allow(...)]` is not acceptable.
- If the same suppression appears more than two or three times across the
  codebase, that's a signal: the lint is wrong for this project, raise it
  to a crate-level `#![allow(...)]` in `lib.rs` / `main.rs` with the same
  justifying comment, and consider whether a `clippy.toml` entry would
  capture the intent better.

The same rule applies to `rustfmt` — `#[rustfmt::skip]` requires a comment.
In practice rustfmt is essentially never wrong; reach for skip only on
hand-aligned tables (register bit definitions, pin maps) where the
auto-layout actively hurts readability.

### 3. No `clippy.toml` until a real pattern demands one

Start with default clippy. Add a `clippy.toml` only when a project-wide
threshold needs tuning (e.g. `cognitive-complexity-threshold` for a
state-machine module, or `too-many-arguments-threshold` for an FFI shim).
A `clippy.toml` exists to encode considered project-wide decisions, not to
silence inconvenient lints case by case — that's what `#[allow]` plus a
comment is for.

### 4. Opt-in groups stay opt-in

Default clippy is the gate. `clippy::pedantic` and `clippy::nursery` are
opt-in groups that catch real issues but also bikeshed; they're worth
running occasionally with `cargo clippy -- -W clippy::pedantic` as a
"is there anything worth fixing here" pass, but not as a gate. Revisit if
the codebase grows enough that the noise/signal ratio improves.

## Consequences

**Positive.**

- Every commit on `main` reads consistently. A stranger landing on the
  repo never sees a stale lint warning or a hand-mangled format.
- Suppressions are self-documenting. The comment next to every `#[allow]`
  explains the trade-off to future-you and to any reader.
- The discipline catches real issues — embedded-relevant lints
  (`await_holding_lock`, `large_stack_arrays`, `unwrap_used` if opted in)
  fire on the actual mistakes they're designed to catch.

**Negative.**

- Slightly more friction per commit. Mitigated by editor integration:
  rustfmt-on-save and clippy-on-save mean most lints are fixed before
  they ever reach a `git status`.
- Occasionally a clippy lint is wrong and requires the suppression dance.
  Accepted on purpose: the dance is short, and the resulting comment is
  itself useful documentation.

**Operational notes.**

- Local commands:
  ```pwsh
  cargo fmt
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  ```
- Editor: `rustfmt` runs on save; `rust-analyzer.check.command = "clippy"`
  surfaces lints in the Problems panel as you type.
- Toolchain pins `rustfmt` and `clippy` via [rust-toolchain.toml](../../rust-toolchain.toml)
  so versions don't drift between local and (future) CI.

## Alternatives considered

**Clippy as a suggestion, not a gate.** Rejected. "Suggestion" is how
lint warnings turn into background noise that nobody fixes. The whole
value of clippy comes from *acting* on its output.

**`cargo fix` / `cargo clippy --fix` on save.** Rejected as a workflow.
Automated rewrites at save time are surprising and occasionally wrong;
the manual `Ctrl + .` quick-fix path is the idiomatic flow and keeps the
author in control of the diff.

**Enable `clippy::pedantic` as part of the gate.** Rejected for now.
Pedantic is genuinely noisy on small embedded code (lots of `must_use`,
`missing_errors_doc`, `cast_possible_truncation`, ...) and the
cost/benefit only flips on larger codebases. Run it manually when
curious; promote individual pedantic lints into the gate one at a time
if they earn it.

**Project-wide `clippy.toml` from day one.** Rejected as premature
configuration. Defaults are well-chosen; encode deviations only when a
real recurring pattern justifies the entry.

## References

- [rustfmt configuration reference](https://rust-lang.github.io/rustfmt/).
- [clippy lint list](https://rust-lang.github.io/rust-clippy/master/).
- [.vscode/settings.json](../../.vscode/settings.json) — editor integration.
