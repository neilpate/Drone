# nRF5340 inter-core IPC — "dumb pipe" transport for the radio

Background research for [ADR 0029 §10](../decisions/0029-multi-board-firmware-shared-library.md). Captured so Phase-4/5 bring-up does not re-derive this cold. Records *what was found* and *how to de-risk it*; the ADR records the decision.

## The problem, and why it is unavoidable

On the nRF5340 the two cores are not peers over one bus — peripherals are partitioned. The **`RADIO` is a network-core peripheral**; the application core cannot access it. Confirmed in `embassy-nrf`: the `radio` module appears under the `nrf5340-net` chip build and **not** under `nrf5340-app-s`.

Consequence: our 802.15.4 link ([ADR 0014](../decisions/0014-radio-protocol-ieee802154.md)) *must* run on the net core, and the app core (where all the flight logic lives) *must* reach it across an inter-core transport. This is forced by silicon, not a design preference. The only design freedom is how much protocol runs on each side — ADR 0029 §6 chooses to keep the net core a dumb byte pipe and do all decode/`firmware-types` work on the app core.

## What it is, in systems terms

Two lock-free single-producer/single-consumer (SPSC) ring buffers in shared RAM — one per direction — with an inter-core interrupt as the doorbell:

```
app core                         net core (802.15.4 PHY)
  remote_link_body                 radio bridge
      |  write frame -> TX ring         ^  drain TX ring -> radio.transmit()
      |  DMB; bump head; ring doorbell  |
      +--------- IPC event ------------>+  (IPC ISR wakes drain task)
      ^  drain RX ring <- frame         |  radio.receive() -> write RX ring
      |  (IPC ISR wakes drain task)     |  DMB; bump head; ring doorbell
      +<-------- IPC event -------------+
```

Direct analogues a systems programmer already knows:

- **virtio**: the ring is the vring, the doorbell is the eventfd/notify. Same shape.
- **A NIC**: descriptor ring in shared memory + an MSI/interrupt to say "ring advanced".

### Cache

The nRF5340 application and network cores are both Cortex-M33 with **no data cache** (the app core has an instruction cache for XIP flash only). So writes to shared RAM are coherent between the cores with no flush/invalidate. You still need a memory **barrier** (`cortex_m::asm::dmb()`) for *ordering* — write the frame bytes, barrier, *then* publish the head index — but not cache maintenance. This is simpler than the same trick on a cached M7 or an application processor.

## What `embassy-nrf` gives you (verified, git docs)

| Piece | API | Notes |
|---|---|---|
| Doorbell | `ipc::Ipc::new(peri, irqs)` | 16 `Event`s; `EventTrigger` fires one, `Event` awaits one. **Signal only — carries no data.** `Send + Sync`. |
| Boot the net core | `reset::release_network_core()` | plus `hold_network_core()`, `network_core_held()`. |
| Net-core 802.15.4 | `radio` module (`nrf5340-net`) | the *same* `ieee802154::Radio` API `firmware-remote`/`remote_link` already use. |

## What you hand-roll (not provided)

- **The shared-RAM ring transport.** No embassy type for this. A `#[unsafe(link_section = "...")]` struct (head/tail indices + byte buffer) placed at a fixed address both cores agree on. Two of them (TX, RX).
- **The SPU carve-out.** The app core must make the shared-RAM region accessible to the net core. Open question whether `embassy_nrf::init` sets enough up or it needs a `pac`-level SPU register write — establish empirically (first thing the spike answers).
- **A worked example.** There is **none** in embassy: `examples/nrf5340/src/bin` is only `blinky`, `uart`, `cryptocell_rng`, `gpiote_channel`, `nrf5340dk_internal_caps`. No inter-core, no radio. So first bring-up has no reference to copy — this is the main cost.

The takeaway: risk is **integration** (assemble known parts, no reference), not **feasibility** (the capability is all present).

## The fiddly bits to plan for

1. **Memory contract.** Both `memory.x` must agree byte-for-byte on the shared region's address and size. No compiler checks this across two binaries; a mismatch corrupts the pipe silently.
2. **SPU permissions.** See above — the first empirical unknown.
3. **Startup ordering.** Initialise the ring headers *before* `release_network_core()`, or use a magic-word handshake so the net core spins until the app core marks the region live. Classic shared-memory init race.
4. **Two-image flashing.** probe-rs flashes both cores, but the net-core `.hex` is a separate build + flash step — new versus today's single micro:bit image.
5. **Frame format.** Length-prefixed raw PHY bytes. The net core never parses them (dumb pipe); all postcard/COBS + `firmware-types` decode stays on the app core.

## De-risking spike (do this before any `RadioLink` code)

Two throwaway binaries, no radio yet:

1. App core: init the shared region (headers, magic word) → `reset::release_network_core()`.
2. Net core: wait for magic word → loop: read TX ring, echo the value into the RX ring, ring the doorbell.
3. App core: write an incrementing `u32` into the TX ring, doorbell, await the RX doorbell, check it came back; log latency.

Success = the counter round-trips on hardware. Then the `RadioLink`-over-IPC design (ADR 0029 §6) sits on known-good plumbing. If SPU/memory setup fights, it is learned on ~30 lines, not wired through the radio path.

**Fallback** if the bare ring proves painful: a thin **ICMsg-style** framed transport (still our own Rust, a little more structure than a raw ring) — not the heavyweight RPMsg/OpenAMP stack Nordic's SDK uses.

## References

- embassy-nrf HAL docs (per-core): <https://docs.embassy.dev/embassy-nrf/git/nrf5340-app-s/index.html> and `.../nrf5340-net/index.html` — confirm `radio` is net-core only; `ipc` and `reset` modules on the app core.
- embassy-nrf `ipc` module: <https://docs.embassy.dev/embassy-nrf/git/nrf5340-app-s/ipc/index.html>
- embassy-nrf `reset` module (net-core release): <https://docs.embassy.dev/embassy-nrf/git/nrf5340-app-s/reset/index.html>
- embassy nrf5340 examples (note: no inter-core example): <https://github.com/embassy-rs/embassy/tree/main/examples/nrf5340/src/bin>
- Nordic nRF5340 Product Specification — IPC peripheral, SPU, inter-core memory (for the register-level detail the spike will need). Link from the vendor page when consulted.
