# ADR 0014 — Radio link protocol: IEEE 802.15.4 (raw PHY/MAC)

- **Status:** Accepted
- **Date:** 2026-05-27
- **Related:** [ADR 0001](0001-platform-airframe-stack.md), [ADR 0002](0002-mcu-and-language.md), [00-vision.md](../00-vision.md) (open question: "Radio link")

## Context

[ADR 0001](0001-platform-airframe-stack.md) committed to two micro:bit v2 boards (drone + remote) for Phases 1–3 and to rolling our own firmware. The radio link between them was deliberately left open
([00-vision.md](../00-vision.md): *"Radio link — second micro:bit + ESB covers Phases 1–2; longer-term choice still open"*), with Enhanced ShockBurst (ESB) as the leading candidate by virtue of being Nordic's "obvious" proprietary 2.4 GHz protocol.

The nRF52833 has a single 2.4 GHz RADIO peripheral that can be configured for several PHY modes:

- **BLE 1/2 Mbit, BLE Long Range** — GFSK, programmable access address, programmable CRC, 1-byte preamble, data whitening.
- **Enhanced ShockBurst (ESB)** — Nordic-proprietary, built on top of BLE-style framing, adds auto-ACK and retransmission.
- **IEEE 802.15.4 250 kbit/s** — O-QPSK with DSSS (32-chip spreading per 4-bit symbol), 4-byte preamble, fixed SFD, fixed CRC.

The first telemetry experiment (drone TX → remote RX, incrementing u32 counter; issue [#5](https://github.com/neilpate/Drone/issues/5)) was attempted in BLE 1 Mbit mode via embassy-nrf's `radio::ble::Radio` driver. It surfaced two compounding problems:

1. **Multi-address acceptance in the driver.** The peripheral's `RXADDRESSES` register defaults to enabling logical addresses 0..4. The driver programs only address 0; addresses 1..4 are left at reset (zeros). In RF noise, any sequence resembling four zero bytes followed by a parseable length byte matches one of those phantom addresses, so RX delivers a constant stream of garbage frames.

2. **`receive()` completes on `END`, not `CRCOK`.** The caller has to check `CRCSTATUS` itself. We weren't, so the garbage frames were treated as valid.

Both are fixable by configuration / wrapper code, but together they made the BLE path feel like fighting the driver rather than using it. We pivoted to embassy-nrf's `radio::ieee802154::Radio` driver, confirmed end-to-end reception, and want to record that choice as a real decision rather than an artefact of debugging.

## Decision

The drone ↔ remote telemetry link uses the **raw IEEE 802.15.4 PHY and MAC framing**, via embassy-nrf's `radio::ieee802154::Radio` driver. No higher-layer 802.15.4 stack (Zigbee, Thread, 6LoWPAN) is involved — we use the PHY/MAC as a clean point-to-point datagram pipe.

- **Channel:** 11–26 (2.4 GHz band, 5 MHz spacing). Initial setting: channel 20 (2450 MHz), tweak per-environment.
- **Framing:** standard 802.15.4 PHR + PSDU + hardware CRC-16. CRC is checked by the driver; `receive()` returns `Err` on CRC fail and never delivers garbage frames.
- **Addressing:** not used at the PHY layer. The driver accepts every CRC-valid frame on the channel; application-layer addressing (if needed) is layered on top in our own framing.
- **HFCLK:** must be sourced from the external 32 MHz crystal (`HfclkSource::ExternalXtal`). The internal HFINT oscillator's ±1% drift is too large for the RADIO PLL to lock reliably; with HFINT, no packets get through in either BLE or 802.15.4 mode. This applies to **any** 2.4 GHz use of the nRF52, not just 802.15.4.

The choice is intentionally local to the telemetry link — it does not commit us to 802.15.4 for the post-Phase-5 analog FPV video downlink (which is a separate orthogonal payload, see [00-vision.md](../00-vision.md)) or for any future ground-station-to-internet path.

## Why 802.15.4 over the alternatives

- **Hardware CRC gating.** The driver wraps `CRCOK` checking and exposes a clean `Result<(), Error>` on `receive()`. No application-layer filtering needed to weed out noise.
- **No address-filter footguns.** The 802.15.4 driver does not enable phantom logical addresses; the only filter is CRC.
- **DSSS processing gain.** Each 4-bit symbol is spread across 32 chips. The despreader gives ~9 dB of effective SNR over an unspread modulation at the same chip-energy, which matters at the range we will eventually want (drone in a garden, ground station indoors). Trade-off: 250 kbit/s on-air rate vs. 1 Mbit/s for BLE — but telemetry is small and infrequent, so the rate is irrelevant and the link margin is not.
- **Cleaner driver API.** `set_channel(11..=26)` instead of raw frequency; `Packet` type with built-in length handling; `lqi()` exposed per-frame; no whitening / access-address / CRC-init configuration to get wrong.
- **Channel layout interacts well with WiFi.** 802.15.4 channels 15, 20, 25, 26 fall in the gaps between common 2.4 GHz WiFi channels. Channel 20 (chosen as default) sits between WiFi channels 5 and 6.

## Why not the alternatives

**BLE 1/2 Mbit raw.** Faster on-air, but the embassy driver's defaults (multi-address acceptance, END-not-CRCOK completion) cost us a full debugging session and would keep costing every time a new contributor or future-me touches the radio. Fixable, but the fix is "wrap the driver to make it safe", which is exactly the kind of yak-shaving we are trying to avoid for a hobby learning project. Reconsider only if we discover a concrete need for the higher bitrate.

**Enhanced ShockBurst (ESB).** The "obvious" Nordic proprietary choice. Has nice features (auto-ACK, hardware retransmission, pipe addressing). Rejected because:
- It's Nordic-proprietary. Locks us into nRF silicon at the radio layer. We are *currently* nRF-only (ADR 0002), but ADR 0001 explicitly keeps the door open to other MCUs in later phases; 802.15.4 is an open standard supported by many radios.
- It inherits the same BLE-derived framing pitfalls (programmable access addresses, no built-in DSSS).
- The auto-ACK / retransmission story is genuinely useful for control packets, but for this project's telemetry we want to *see* packet loss in our own counters as a learning signal, not have it papered over by the radio.
- No first-party `embassy-nrf` ESB driver. A third-party `esb` crate exists but isn't `embassy-nrf`-native.

**BLE proper (with SoftDevice or `nrf-softdevice`).** Rejected: drags in a closed-source binary blob, fragments the executor (SoftDevice owns several IRQs), and we don't need any of the BLE GAP / GATT machinery. We are not interoperating with phones.

**A higher-level 802.15.4 stack (Thread, Zigbee, OpenThread).** Rejected for the link itself: massive complexity (border routers, commissioning, mesh) for what is at heart a fixed two-node point-to-point pipe. Reconsider only if a multi-node use case emerges, which is well outside scope.

**WiFi / ESP-NOW / LoRa / SX12xx.** Rejected: all require additional hardware. We already have the nRF52 radio paid-for in both micro:bits.

## Consequences

### What this commits us to

- All telemetry framing decisions are made **above** the 802.15.4 MAC, in our own code. Our payload starts at the PSDU and is whatever bytes we choose to put there. We will eventually want a small header (message type, sequence number, maybe a CRC-32 over our own payload as belt-and-braces, since 802.15.4's CRC-16 is per-frame and not application-aware).
- The link is **unreliable by design**. No retransmission, no ACKs at the PHY/MAC level. The protocol layer above (future ADR when we design it) decides whether to add sequence numbers, ARQ, or fire-and-forget for each message class.
- **HfclkSource::ExternalXtal** is required in the board init for any radio use. This is encoded in each `board::microbit_v2::Board::new()`. Future board variants must do the same.
- The `radio_link` module currently exposes only `pub const CHANNEL: u8 = 20;`. Both crates have a copy — duplication is deliberate per [issue #5](https://github.com/neilpate/Drone/issues/5) discussion (no shared crate until the surface grows enough to warrant one).

### What this rules out (for now)

- **No use of nRF-specific radio features** that don't translate (ESB pipes, BLE long-range coding, on-air whitening tuning). If we ever port the radio task to a non-nRF MCU, 802.15.4 is the most portable choice we could have made.
- **No interoperability with off-the-shelf consumer devices.** A phone won't see our drone. This is a non-goal.

### What stays open

- **Application-layer framing.** Message types, sequence numbers, optional CRC-32, time-stamping, on-wire endianness. Will get its own ADR when designed (likely Phase 1 mid-to-late as the message set grows beyond a single u32 counter).
- **Channel agility / frequency hopping.** Currently fixed on channel 20. If WiFi interference proves problematic in practice, revisit.
- **TX power.** Default driver setting for now. Tune when range testing forces the issue.
- **Whether to use 802.15.4's address fields.** The standard supports 16-bit short and 64-bit extended addresses; the embassy driver doesn't currently expose hardware filtering on them, so for now any addressing we do is in software in our own payload.
- **The post-Phase-5 analog FPV video link is orthogonal.** It uses its own 5.8 GHz transmitter and does not touch the FC firmware. ADR 0014 is silent on it.

## References

- [embassy-nrf `radio::ieee802154` source](https://github.com/embassy-rs/embassy/blob/main/embassy-nrf/src/radio/ieee802154.rs) — driver API surface.
- [nRF52833 Product Specification, RADIO chapter](https://infocenter.nordicsemi.com/topic/ps_nrf52833/radio.html) — `RXADDRESSES`, `CRCSTATUS`, `RSSISAMPLE`, IEEE 802.15.4 mode details.
- [IEEE 802.15.4-2020 standard](https://standards.ieee.org/ieee/802.15.4/7029/) — PHY/MAC reference (the standard itself is paywalled; summaries widely available).
- Implementation: [`crates/firmware-drone/src/tasks/telemetry_tx.rs`](../../crates/firmware-drone/src/tasks/telemetry_tx.rs), [`crates/firmware-remote/src/tasks/telemetry_rx.rs`](../../crates/firmware-remote/src/tasks/telemetry_rx.rs), [`crates/firmware-drone/src/board/microbit_v2.rs`](../../crates/firmware-drone/src/board/microbit_v2.rs).
- Commit `95c7684` — first end-to-end working link.
