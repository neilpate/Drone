# Sizing serialization buffers from the wire type: postcard `MaxSize`

_Captured 2026-06-26 after a real `SerializeBufferFull` panic: adding one field to the telemetry struct silently overflowed a hand-picked `[0u8; 32]` buffer and hard-faulted the chip in flight._

## The bug that prompted this

`postcard` is a non-self-describing serializer: you hand it a `&mut [u8]` and it writes the bytes into it. The buffer is yours to size:

```rust
let mut buf = [0u8; 32];
let framed = postcard::to_slice_cobs(&telemetry, &mut buf).unwrap();
```

That `32` was a guess that happened to fit. Then this session's CPU-load profiler added a `cpu_load` field to `Telemetry`. The serialized form grew past 32 bytes, `to_slice_cobs` returned `Err(SerializeBufferFull)`, the `.unwrap()` panicked, and on `no_std` a panic is a `HardFault` — the remote board just died mid-link.

The footgun: **the buffer size and the struct definition were in different files, with no compiler link between them.** Nothing made the buffer grow when the struct did. This is exactly the C mistake of `char buf[32]` next to a `sprintf` whose output you have not actually bounded — except here the wire format is the thing being bounded, not a string.

## Why it did not crash immediately — the serialized size grows at runtime

The nastiest part: the panic did not happen at startup. The link ran fine for a while and only hard-faulted **after the drone had been powered up for some time.** That delay is what made it look intermittent.

The cause is varint encoding. postcard LEB128-encodes integers, so a `u32` is *not* a fixed 4 bytes on the wire — it is 1 byte while the value is small and grows to 5 bytes as the value gets large. `Telemetry` carries a `sequence_number: u32` that increments every frame. Early on, the sequence number fit in 1–2 bytes, the whole frame squeezed under 32 bytes, and everything serialized cleanly. As the counter climbed past the varint thresholds (128, 16384, …) the frame grew a byte at a time, until eventually it tipped over the buffer and `to_slice_cobs` returned `SerializeBufferFull`.

So the actual byte that overflowed the buffer was an incrementing counter quietly widening over minutes of flight — a buffer that "worked in testing" and only failed once a runtime value grew large enough. This is exactly why a worst-case bound matters: `MaxSize` sizes for the *largest* the value can ever be (a `u32` at its full 5-byte varint width), not the size it happens to have at boot.

## The fix: let the compiler compute the worst-case size

postcard ships a `MaxSize` trait that gives the worst-case serialized length of a type as an associated const, computed at compile time:

```rust
use postcard::experimental::max_size::MaxSize;

#[derive(Serialize, Deserialize, MaxSize)]
pub struct Telemetry { /* ... */ }

// Telemetry::POSTCARD_MAX_SIZE is now a usize const.
```

Enable it with the Cargo feature (the derive lives behind it):

```toml
postcard = { version = "1", default-features = false, features = ["experimental-derive"] }
```

`MaxSize` derives compositionally — a struct's max size is the sum of its fields' max sizes — so every type *inside* `Telemetry` must derive it too (`Sensors`, `Temperature`, `DroneState`, `PilotCommand`, the axis newtypes, `CpuLoad`). Derive it from the leaves up.

## How postcard counts the bytes

`POSTCARD_MAX_SIZE` reflects postcard's actual wire rules, so the numbers are worth knowing:

| Type | Worst-case bytes | Why |
|------|------------------|-----|
| `u8` / `i8` | 1 | one byte |
| `u32` | 5 | LEB128 varint; a 32-bit value needs up to ⌈32/7⌉ = 5 groups |
| `f32` | 4 | fixed IEEE-754, no varint |
| `bool` | 1 | |
| `struct` | Σ fields | concatenation, no padding, no tags |
| `enum` | 1 (+ largest variant) | varint discriminant, then the biggest variant's payload |
| single-field newtype | = its field | postcard has no struct wrapper overhead |

That last row is why `Throttle(f32)` is wire-identical to a bare `f32` — the newtype is free on the wire, only the type system sees it.

## Don't forget the framing overhead

`POSTCARD_MAX_SIZE` is the *serialized* size. We send it COBS-framed, and COBS adds its own bytes: one overhead byte per 254 bytes of payload, plus a single `0x00` terminator. So the buffer that holds the *framed* frame is larger:

```rust
// COBS-padded upper bound on a serialized Telemetry frame.
pub const FRAME_MAX_SIZE_BYTES: usize =
    Telemetry::POSTCARD_MAX_SIZE + Telemetry::POSTCARD_MAX_SIZE / 254 + 2;
```

The `+ 2` is one for the per-254 overhead byte (integer division floors, so `/254` is 0 for our ~35-byte payload — the `+1` covers that first block) and one for the sentinel terminator. For a small frame this is a generous-but-cheap upper bound.

Export it once and have every buffer reference it, so the panic class cannot come back:

```rust
// firmware-types/src/lib.rs
pub use telemetry::FRAME_MAX_SIZE_BYTES as TELEMETRY_FRAME_MAX_SIZE_BYTES;
```

```rust
// every TX/RX site, on both boards and the groundstation:
let mut buf = [0u8; TELEMETRY_FRAME_MAX_SIZE_BYTES];
```

Now adding a field to `Telemetry` automatically grows every buffer that carries it, at compile time, with no human in the loop.

## The lesson

A serialization buffer should never be a magic number sitting next to a `.unwrap()`. Derive its size from the type it has to hold, in the same crate that owns the type, so the two move together. This is `sizeof` for the wire format — the embedded equivalent of computing a worst-case MTU at compile time instead of hoping 32 is enough.

The general shape: **shared wire types are the single source of truth; everything that touches the wire — both firmware boards and the host groundstation — derives its sizes from them, never restates them.**
