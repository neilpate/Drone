# crates/

Rust crates live here. Firmware crates build for their own target from inside
their directory, which picks up that crate's `.cargo/config.toml`; host crates
are the workspace's default members.

- `firmware-types` — shared wire types (postcard), used by the drone, remote and
  ground station ([ADR 0005](../doc/decisions/0005-pc-software-language-rust.md)).
- `firmware-drone-core` — pure, host-tested drone logic: supervisor, attitude
  estimation, control law ([ADR 0009](../doc/decisions/0009-workspace-bootstrap-and-crate-naming.md),
  [ADR 0015](../doc/decisions/0015-host-testing-no-std-crates.md)).
- `firmware-drone-shared` — board-agnostic Embassy tasks and signals used by
  every drone binary ([ADR 0029](../doc/decisions/0029-multi-board-firmware-shared-library.md)).
- `firmware-drone-microbit` — drone firmware for the BBC micro:bit v2 (nRF52833).
- `firmware-drone-nrf5340-app` — drone firmware for the `drone_fc` v2 nRF5340
  application core.
- `firmware-drone-nrf5340-radio` — `drone_fc` v2 nRF5340 network core: a raw
  802.15.4 payload bridge to the application core.
- `nrf5340-ipc` — nRF5340-only shared frame queues, IPC doorbell and startup
  handshake. The app and radio binaries select its `app` and `net` features in
  separate builds. Without those features its queue tests run on the host as a
  default workspace member. See the
  [transport contract](../hardware/electrical/README.md#inter-core-radio-transport).
- `firmware-remote` / `firmware-remote-core` — radio remote firmware for the
  micro:bit and its host-tested logic.
- `groundstation` — PC ground-station GUI and telemetry tools. Host-only and
  excluded from the workspace; build from its own directory.
- `swd-test` — host-only SWD probe bench test
  ([bring-up notes](../hardware/electrical/README.md#swd-probe-bench-test)).
  Excluded from the workspace.
