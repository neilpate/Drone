# crates/

Rust workspace member crates live here.

Empty at the moment — populated when Phase 1 design lands and the first `cargo new` runs. See [Doc/00-vision.md](../Doc/00-vision.md) for the phase plan and [Doc/dev-environment.md](../Doc/dev-environment.md) for toolchain setup.

Expected initial members (subject to Phase 1 design):

- `proto` — shared wire-format types ([ADR 0005](../Doc/decisions/0005-pc-software-language-rust.md)).
- `firmware-drone-core` / `firmware-drone` — drone firmware, `core`/`task` split per [ADR 0007](../Doc/decisions/0007-testing-and-ci-strategy.md).
- `firmware-ground` — ground micro:bit firmware (transparent USB ↔ radio bridge).
- `groundstation` — PC-side application ([ADR 0005](../Doc/decisions/0005-pc-software-language-rust.md)).
- `xtask` — build / flash / test / lint runner ([ADR 0007](../Doc/decisions/0007-testing-and-ci-strategy.md)).
