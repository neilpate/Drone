# Serial links: framing, embassy UART, and host `serialport` gotchas

_Captured 2026-06-19 while wiring telemetry back from the remote to the groundstation over the USB-CDC serial link. Several non-obvious things bit in a row — all worth keeping._

## The observation

The plan was simple: the remote already receives `TelemetryState` from the drone over the radio, so "just send it to the PC over the serial port." It turned into a tour of four separate gotchas — packet-vs-stream framing, the embassy `read` API, host-side `try_clone` deadlocks, and buffer sizing. None of them are hard once understood; all of them cost time first.

## 1. A radio packet is framed; a UART byte stream is not

The IEEE 802.15.4 radio is **packet-based**. `radio.receive(&mut packet)` hands you one complete, length-delimited message — the PHY layer carries the boundary for free. That is why over the radio we send bare `postcard::to_slice` / `from_bytes` with **no framing**: the packet *is* the frame.

A UART (and the PC serial port on the other end) is a **raw byte stream**. No boundaries, no lengths — just bytes. A single read can land you half a message, exactly one, or three-and-a-bit. So the byte stream needs framing re-added on top. That is what COBS is for.

The consequence: you cannot simply forward the raw postcard bytes from the radio packet onto the UART. The radio's framing does not survive on a byte stream. You decode the packet into a typed `TelemetryState`, then re-encode it with framing for the UART. The re-encode is microseconds for an ~8-byte struct, and it keeps the two transports decoupled — the radio wire format and the UART wire format can evolve independently.

```
drone --radio packet (postcard, self-framed)--> remote
remote: decode -> TelemetryState -> Watch
remote: re-encode (postcard + COBS) --UART byte stream--> groundstation
```

The general rule: **decode off one transport into a transport-independent type, re-encode onto the next.** The type in the middle (`TelemetryState`, shared via `firmware-types`) is the single source of truth both ends compile against.

## 2. postcard + COBS, and why two layers

Two orthogonal jobs:

- **postcard** — serialization. Turns a `serde`-derived struct into compact, varint-packed bytes and back. Says *what the bytes mean*. Both ends agree because they share the struct definition in `firmware-types`.
- **COBS** (Consistent Overhead Byte Stuffing) — framing. Reserves the byte `0x00` as an end-of-frame marker and encodes the payload so a real `0x00` can never appear inside it. Says *where the message ends*. Bounded overhead: ~1 byte per 254 bytes of payload, so the worst-case encoded size is known up front.

COBS is **not** compression — it makes data marginally *larger* on purpose, to buy unambiguous message boundaries and the ability to resync after a dropped byte (discard until the next `0x00`).

postcard does both in one call on the sending side:

```rust
let framed = postcard::to_slice_cobs(&value, &mut buf)?; // serialize + frame + 0x00 terminator
uart_tx.write(framed).await?;                            // write exactly the returned sub-slice
```

On the receiving side the `CobsAccumulator` buffers bytes across reads and yields a fully-typed value only when a `0x00` completes a frame. `feed::<T>` does the COBS-decode *and* the postcard-deserialize:

```rust
match cobs.feed::<TelemetryState>(window) {
    FeedResult::Consumed              => {} // buffered, need more bytes
    FeedResult::OverFull(rest)        => {} // frame > buffer size N; discarded, resync at rest
    FeedResult::DeserError(rest)      => {} // hit 0x00 but bytes != valid T; resync at rest
    FeedResult::Success { data, remaining } => { /* use data; continue on remaining */ }
}
```

`OverFull` and `DeserError` are the self-healing paths: start mid-stream or drop a byte, and the accumulator recovers at the next terminator instead of getting stuck forever. `CobsAccumulator<N>` must be sized `>=` the largest framed message; keep it in step with the sender's buffer.

postcard's COBS is built on the standalone `cobs` crate. You only reach for `cobs::encode` directly if you are framing bytes that are not a postcard `T` (e.g. forwarding already-serialized bytes).

## 3. embassy-nrf `UarteRx::read` fills the *whole* buffer

This is the one that cost the most time, and it bit in both directions.

`embassy_nrf::uarte::UarteRx::read(&mut buf)` returns `Result<(), Error>` and **blocks until it has filled the entire buffer** (`buf.len()` bytes). It is not `std::io::Read::read` — there is no count returned, and it does not return "whatever is available." If you call it with `[0u8; 256]`, the task sits there until 256 bytes have arrived.

Symptom: the RX task appears dead. We were sending ~6-byte COBS frames per throttle update, so a 256-byte read would not return until ~40 frames had piled up — looked like "the remote never receives anything."

Two ways to read a stream of unknown-length frames:

**Option A — read one byte at a time (no extra hardware).** Make the buffer one byte; `read` returns after each byte; feed each into the accumulator. Because you feed exactly one byte, `remaining` is always empty, so no inner drain loop is needed.

```rust
let mut byte = [0u8; 1];
loop {
    uart_rx.read(&mut byte).await?;
    if let FeedResult::Success { data, .. } = cobs.feed::<GroundstationCommand>(&byte) {
        throttle_command::set(data.throttle);
    }
}
```

For a low-rate command stream this is perfectly efficient; the per-byte DMA overhead is nothing at these rates.

**Option B — `read_until_idle` (chunked).** Returns `Result<usize, Error>` and yields as soon as the RX line goes idle, giving a whole burst at once. But it is **not** a method on the plain `UarteRx`. It lives on `UarteRxWithIdle`, which you only get from `Uarte::split_with_idle(timer, ppi_ch1, ppi_ch2)` — idle-line detection needs a hardware TIMER and two PPI channels. Worth it for high-throughput or bursty links; overkill for a throttle channel.

We used A for the uplink (low-rate commands). The high-rate direction here is telemetry, which is TX from the remote, so byte-at-a-time RX is fine and needs no extra peripherals.

The matching `UarteTx::write(slice)` does send the whole slice (DMA), so the send side has no equivalent surprise.

## 4. `serialport::try_clone()` deadlocks on Windows for concurrent read+write

On the host, the obvious full-duplex design is: open the port once, `try_clone()` it, read in one thread and write in the other. On Linux (`dup` of the fd) this works. **On Windows it deadlocks.**

`serialport-rs` on Windows uses overlapped I/O, and both cloned handles refer to the same COM port with shared timeout/event state. Windows serializes overlapped operations on that port, so while the RX thread is parked in a blocking `ReadFile` (its read timeout was 2000 ms), the TX thread's `WriteFile` cannot proceed — it queues behind the in-flight read. The write "gets stuck" for up to the read timeout.

Symptom: the framed bytes are built correctly (you can print them), but `port.write()` blocks. Dropping the read timeout to ~50 ms makes the write start succeeding intermittently — the smoking gun, but racy.

Fix: **one I/O thread, not two.** Alternate non-blocking write-drain and short-timeout read on a single handle. No `try_clone`, no shared-handle contention.

```rust
loop {
    // 1. drain pending commands, write them (non-blocking)
    while let Ok(value) = rx.try_recv() {
        let framed = postcard::to_slice_cobs(&GroundstationCommand { throttle }, &mut buf)?;
        port.write_all(framed)?;
    }
    // 2. read whatever telemetry arrived (short timeout, then loop back to writes)
    match port.read(&mut raw) {
        Ok(n)  => { /* feed CobsAccumulator */ }
        Err(e) if e.kind() == ErrorKind::TimedOut => {}
        Err(e) => break,
    }
}
```

Set the port timeout short (~50 ms): the read releases quickly so the loop cycles back to writes promptly. Worst-case command latency is one timeout period, invisible for a throttle slider. Use `write_all`, not `write` — `write` may do a partial write and return early.

## 5. Single-field structs serialize identically to their field (a latent trap)

postcard adds no struct wrapper and no field names. A single-field struct serializes to exactly the same bytes as its inner field:

```
GroundstationCommand { throttle: Throttle(x) }   ==(on the wire)==   Throttle(x)
```

So sending a bare `Throttle` while the receiver decodes `GroundstationCommand` *works by accident*. The moment a second field is added to `GroundstationCommand`, the two sides silently diverge with no compile error. The safe habit: send and receive the **same named type** on both ends, mirroring how the telemetry direction uses `TelemetryState` both ways.

## 6. A `Watch` consumer that does `get().await` stalls until the *first* publish — seed neutral values at startup

This one bit twice, in two different crates, before the pattern was obvious. The link is a chain of `Watch` signals: each producer task `set()`s its slice, each consumer `get().await`s it. The trap is that `embassy_sync::watch::Receiver::get().await` **blocks until the watch has ever been published** — on a watch that no one has `set()` yet, the consumer parks indefinitely. It is non-blocking for the *CPU* (it yields to the executor), but it absolutely blocks that task's forward progress.

The concrete failure: the drone's `telemetry_aggregator` builds each frame by `get().await`-ing several producer watches in sequence, one of them `pilot_command`. The remote only published a `PilotCommand` *after the PC sent its first stick command* — so until you touched a control, `pilot_command` had never been set, the aggregator parked on that `get().await`, and **no telemetry was ever assembled**. The whole downlink looked dead, intermittently, for a reason that had nothing to do with the serial link itself.

The same shape had already appeared once: telemetry only started flowing once a throttle command was sent. Two instances of one hazard — *a downstream `get().await` is gated on an upstream producer's first publish.*

The fix is to publish a **neutral default at task startup**, before the loop, so the watch always holds a value:

```rust
// serial_link_rx, first thing in the task — before reading any byte from the PC
throttle_command::set(Throttle::from_normalised(0.0));
roll_command::set(Roll::from_normalised(0.0));
pitch_command::set(Pitch::from_normalised(0.0));
yaw_command::set(Yaw::from_normalised(0.0));
```

Now the remote sends a (neutral) `PilotCommand` from the first radio round, the drone publishes `pilot_command` immediately, and `telemetry_aggregator` unblocks at cold start with no operator input. Two properties make this the right layer to fix it:

- **Fix at the producer, not the consumer.** Seeding the source means every downstream `get().await` in the chain resolves early, without each consumer having to defend itself with `try_get` + a local default. One publish, many consumers unblocked.
- **The default must be the *safe* value, not just any value.** For a flight control input that is zero throttle and centred sticks — the same value you would want if the link dropped. Seeding `0.0` doubles as the fail-safe rest state, so it is correct, not merely convenient.

The systems-programming analogy: `get().await` is a read on a condition variable that is only ever signalled by the first write. If the writer is late, the reader sleeps forever. Seeding at startup is publishing an initial value so the very first read has something to return — like initialising shared state before spawning the threads that read it.

If you genuinely want "latest if present, else carry on" semantics instead, `Receiver::try_get() -> Option<T>` is the non-blocking read — but prefer seeding the producer so the data is real and safe, rather than papering over a missing publish in every consumer.

## Takeaways

- Packet transport (radio) frames for you; stream transport (UART) does not — re-add framing (COBS) and decode/re-encode through a shared type.
- postcard = what the bytes mean; COBS = where the message ends. Different layers, compose cleanly. COBS is framing, not compression.
- `embassy_nrf::uarte::UarteRx::read` fills the *whole* buffer. For unknown-length frames, read one byte at a time, or use `split_with_idle` + `read_until_idle` (costs a TIMER + 2 PPI channels).
- `serialport::try_clone()` for concurrent read+write deadlocks on Windows. Use a single I/O thread with a short read timeout.
- A single-field struct is wire-identical to its field in postcard — send the same named type on both ends to avoid a silent future divergence.
- `Watch::get().await` blocks until the *first* publish, so a downstream consumer stalls forever if its producer is late. Seed a neutral, *safe* default at the producer's startup (zero throttle, centred sticks) so the whole chain unblocks at cold start.