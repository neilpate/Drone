# ADR 0008 — Repository folder layout

- **Status:** Accepted
- **Date:** 2026-05-23
- **Related:** [ADR 0005](0005-pc-software-language-rust.md), [ADR 0006](0006-mechanical-cad-fusion360.md), [ADR 0007](0007-testing-and-ci-strategy.md), [AGENTS.md](../../AGENTS.md)

## Context

The repository holds three distinct kinds of artefact:

- **Rust code** — firmware for the drone, firmware for the ground micro:bit, the PC-side ground-station application, a shared `proto` crate ([ADR 0005](0005-pc-software-language-rust.md)), and an `xtask` build runner ([ADR 0007](0007-testing-and-ci-strategy.md)). At least four crates from day one, more later.
- **Mechanical design files** — Fusion 360 sources, STEP exports, print-ready meshes ([ADR 0006](0006-mechanical-cad-fusion360.md)).
- **Electrical design files** — KiCad schematics and PCB layouts for the custom nRF5340 carrier board, from Phase 4 onwards.

Plus the existing `doc/` tree and top-level repo files (README, AGENTS, licences, `.gitignore`).

The choice of top-level layout determines what a stranger sees in the first second of `git clone && ls`. It is small enough to look like a non-decision, but it is **load-bearing for the "landable cold" criterion** in [00-vision.md](../00-vision.md) (Definition of done) and is awkward to change later because every cross-document link and every developer's muscle memory depend on it.

Combining Rust firmware with open hardware (CAD + PCB) in a single repo is unusual at this scale — most comparable projects (Bitcraze Crazyflie, Oxide Computer, System76, Pine64) split firmware and hardware into separate repos. There is therefore no dominant external convention to copy wholesale; we are picking among defensible options.

## Decision

Top-level layout:

```
Drone/
├── crates/                    Rust workspace members
├── doc/                       Markdown documentation, ADRs, research notes
├── hardware/
│   ├── mechanical/            Fusion 360 + STEP + print-ready meshes (ADR 0006)
│   └── electrical/            KiCad sources + fab outputs (Phase 4 onwards)
├── AGENTS.md
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
└── .gitignore
```

Rules:

1. **Three top-level domain folders:** `crates/`, `doc/`, `hardware/`. Each names what it *is*, not what it's *made of*.
2. **All folder names lowercase.** No exceptions. Matches the dominant Rust ecosystem convention (Embassy, tokio, hubris, Tock, OpenSK, bevy — all lowercase top-level dirs).
3. **Rust code lives under `crates/`,** one subdirectory per workspace member, not flat at the repo root. Top-of-repo stays as "project domains", not "Rust workspace with hangers-on".
4. **`hardware/` is subdivided by engineering discipline** — `mechanical/` and `electrical/`, the industry-standard split. "Electrical" covers both digital electronics and power / wiring; "Electronic" was considered and rejected as narrower.
5. **Each folder owns a `README.md`** explaining its purpose, expected file types, and naming conventions. A stranger landing in any subfolder knows what belongs.
6. **Crate names, the `core`/`task` split layout within each crate, and whether to introduce `crates/firmware/`-style further grouping are deferred** to Phase 1 design.

## Why this shape

### Why a workspace from day one (not single crate)

We know from ADR 0005 (shared `proto` crate) and ADR 0007 (`xtask` + `core`/`task` split per crate) that the repo will hold **at least four Rust crates** before Phase 2 ends. Single-crate-now-grow-later means duplicated `Cargo.lock`s, multiple `target/` dirs, and refactoring pain when the second crate lands. Workspace ceremony is essentially zero on a fresh repo and saves real friction within weeks.

### Why `crates/` (not flat-at-root)

Two idiomatic Rust patterns exist for workspaces:

- **Flat at repo root** — Embassy, tokio, serde. Crate directories sit alongside `README.md` and any non-code folders.
- **Under `crates/`** — wasmtime, deno, bevy. Top of repo collapses code into one entry; non-code domains stand out.

Both are idiomatic, neither surprises a Rust reader. The deciding factor here is the presence of `hardware/` as a sibling: top-of-repo reading as **"three project domains — code, docs, hardware"** is clearer than reading as **"a Rust workspace that also has some other folders"**. With ~5 crates the path-noise cost (`crates/proto/` vs `proto/`) is trivial; cargo commands use crate names (`-p proto`) regardless.

### Why nest mechanical and electrical under `hardware/`

They are the same noun (hardware) split by discipline (mechanical engineering vs electrical engineering). Top-level `mechanical/` and `electrical/` as peer entries would imply they are unrelated project domains; nesting them under `hardware/` matches how the product is actually built.

### Why "electrical" (not "electronic")

"Electrical Engineering" is the umbrella industry-standard discipline name, covering electronics, power, wiring, batteries, and (eventually) any non-PCBA wiring or harness work. "Electronic" is narrower and would feel constraining the first time we want to commit a wiring diagram or a battery-management note that isn't strictly a PCB.

### Why all-lowercase

Considered both **all-lowercase** (Rust ecosystem default) and **initial-cap-everywhere** (aesthetic preference).

The Rust ecosystem is overwhelmingly consistent on lowercase top-level dirs — every reference project we look at (Embassy, tokio, hubris, Tock, OpenSK, bevy) does it this way. Capitalised top-level dirs in Rust read as wrong to anyone fluent in the ecosystem. The AGENTS.md "prefer the idiomatic choice" rule applies: surprises are reserved for problem-specific decisions, not aesthetic ones. Lowercase wins.

We briefly held a mixed-casing position (`crates/` lowercase, `doc/` and `hardware/` capitalised) on the grounds that Rust convention only governs `crates/`. That logic was defensible but invents an internal rule that isn't a real external one. Consistency was preferred.

### Why not `src/` at the top level

Rejected on convention-collision grounds. Every Rust crate has its own `src/`. A top-level `src/` containing whole crates would make every reader of the repo do a double-take.

## Consequences

### What this commits us to

- All Rust code goes under `crates/<crate-name>/`. No firmware code at the repo root.
- Future hardware additions (e.g. a wiring diagram for the bench rig) go under `hardware/electrical/` (electrical-side) or `hardware/mechanical/` (mechanical-side). Adding a new top-level folder requires a new ADR.
- Folder names stay lowercase. Renaming case later is git-mv-friendly but disruptive enough to be avoided.
- Documentation links and tooling configuration (CI paths, `cargo` workspace globs) use the chosen paths from the first commit.

### What this rules out

- Per-target top-level folders (`firmware/`, `pc/`, `shared/`) — code stays unified under `crates/`. Differentiation between firmware and PC code is at the crate level, not the directory level.
- Capitalised folder names, anywhere in the tree, for files or folders we own. (External tooling that creates capitalised paths in `target/` etc. is exempt — that's downstream of our control.)
- A separate top-level `Mech/` or `PCB/` folder — both fold under `hardware/`. [ADR 0006](0006-mechanical-cad-fusion360.md) was amended in-place to match.

### What stays open

- **Crate names and granularity.** Whether the drone firmware lives in `crates/drone-firmware/` vs `crates/firmware-drone/` vs split further; whether the ground firmware earns a `core`/`task` split; whether `proto` is one crate or splits into wire-format + transport. Phase 1 design.
- **Sub-grouping inside `crates/`.** If the workspace grows past ~8 members, `crates/firmware/`, `crates/pc/`, `crates/shared/` style grouping may become worthwhile. Defer; revisit when it bites.
- **Per-crate internal layout.** The `core`/`task` split from [ADR 0007](0007-testing-and-ci-strategy.md) is mandatory; whether it is realised as sub-modules (`src/core/`, `src/task/`) or sub-crates (`firmware-drone-core/` lib + `firmware-drone/` bin) is a Phase 1 design decision driven by what host tests actually need.
- **Mechanical and electrical folder internals.** Per-part vs per-revision vs per-assembly grouping under `hardware/mechanical/` and `hardware/electrical/` is left to each respective folder's README, decided when the first real file lands.
