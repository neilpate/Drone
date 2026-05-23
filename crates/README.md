# crates/

Rust workspace member crates live here.

Empty at the moment — populated when Phase 1 design lands and the first `cargo new` runs. See [doc/00-vision.md](../doc/00-vision.md) for the phase plan and [doc/dev-environment.md](../doc/dev-environment.md) for toolchain setup.

Expected initial members (subject to Phase 1 design):

- `firmware-drone-core` / `firmware-drone` — drone firmware, `core`/`task` split realised as sibling crates ([ADR 0007](../doc/decisions/0007-testing-and-ci-strategy.md), [ADR 0009](../doc/decisions/0009-workspace-bootstrap-and-crate-naming.md)). **First to land.**
- `proto` — shared wire-format types ([ADR 0005](../doc/decisions/0005-pc-software-language-rust.md)).
- `firmware-ground` — ground micro:bit firmware (transparent USB ↔ radio bridge).
- `groundstation` — PC-side application ([ADR 0005](../doc/decisions/0005-pc-software-language-rust.md)).
- `xtask` — build / flash / test / lint runner ([ADR 0007](../doc/decisions/0007-testing-and-ci-strategy.md)).
