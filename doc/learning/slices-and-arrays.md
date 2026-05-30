# Slices and arrays

_Captured 2026-05-30 while wrapping up `firmware-remote/src/tasks/comm_link.rs`. The question that prompted it: "do we need both `scratch` and `bytes_to_send`, they seem to do the same thing." Answer: no, they're different — and understanding why is understanding slices._

## The question

```rust
let mut scratch = [0u8; SEND_BUFFER_SIZE];                        // 32 bytes
let bytes_to_send =
    postcard::to_slice(&state, &mut scratch).expect("large enough");
```

`scratch` is a buffer, `bytes_to_send` is also "the bytes." They look redundant. Why both?

Because they carry **different lengths**. `scratch` is always 32. `bytes_to_send` is "how many bytes postcard actually wrote" — usually a handful. The radio needs to transmit only the encoded data, not 32 bytes of mostly-garbage. The slice carries that "actual length" alongside the pointer, in one value.

That answer opens up the whole topic of slices — Rust's first-class "pointer plus length" type — which is worth understanding properly because it shows up everywhere in `no_std` code.

## The C analogy

Every C function that takes "a buffer and a length" is morally a Rust slice:

```c
void hash(const uint8_t *data, size_t len);
```

becomes

```rust
fn hash(data: &[u8]);
```

`data` carries both the pointer and the length. The callee reads `data.len()` to know how many bytes are there. No risk of length/pointer drift because they're parts of the same value.

That's the whole concept. Slices are C's "array and length" pattern made into one type. Everything else follows from that.

## Arrays vs slices

Both are contiguous storage, but they differ in whether the length is in the type:

| Type      | Length        | Owns storage? | Size known at compile time? |
|-----------|---------------|---------------|------------------------------|
| `[T; N]`  | Compile-time  | Yes           | Yes — exactly `N * size_of::<T>()` |
| `&[T]`    | Runtime       | No (borrows)  | Yes — fat pointer is always 2 words |
| `[T]`     | Runtime       | n/a           | **No** — DST, can't be a local |

### `[T; N]` — arrays

A fixed-size, stack-allocated bag of values. Length is part of the type and known to the compiler:

```rust
let scratch: [u8; 32] = [0u8; 32];          // 32 bytes on the stack
let nums:    [i32; 4] = [1, 2, 3, 4];       // 16 bytes (4 × 4)
```

`[u8; 32]` and `[u8; 33]` are different types. You can't assign one to the other. The compiler knows exactly how much stack space to reserve when entering the function.

### `[T]` — bare slice

`[u8]` on its own is a *dynamically-sized type* (DST). The compiler doesn't know how much space it needs, so you can never have one as a local variable:

```rust
let s: [u8] = ...;     // ERROR: the size of [u8] cannot be known at compile time
```

You always interact with `[T]` through a reference.

### `&[T]` and `&mut [T]` — slice references

These are **fat pointers** — two machine words wide:

| Word | Contents |
|------|----------|
| 1    | Pointer to the first element |
| 2    | Number of elements |

On 32-bit ARM, `&[u8]` is 8 bytes. On 64-bit x86, 16 bytes. Always one pointer + one `usize`.

This is the single most important fact about slices. They are `(ptr, len)` bundled into one value that the compiler treats as one thing. You pass them, return them, store them — and the length always rides along with the pointer.

## Where slices come from

A slice always borrows from an owner. The owner is whatever holds the actual bytes — an array, a `Vec`, a static, a `Box<[T]>`, even another slice.

```rust
let scratch = [0u8; 32];                   // owner: array on the stack
let all:  &[u8] = &scratch;                // slice over all 32 bytes
let head: &[u8] = &scratch[0..8];          // first 8 bytes
let tail: &[u8] = &scratch[24..];          // last 8 bytes
let mid:  &[u8] = &scratch[8..16];         // middle 8 bytes
```

All four slices point into the same underlying memory. None of them allocate. Each is just a (ptr, len) pair computed at the slicing site:

```
scratch:  [0][1][2][3] ... [29][30][31]     // 32 bytes in memory
              ^                  ^
all  → ptr=&scratch[0],  len=32
head → ptr=&scratch[0],  len=8
tail → ptr=&scratch[24], len=8
mid  → ptr=&scratch[8],  len=8
```

The `&` is significant. `&scratch[0..8]` is a slice reference (8 bytes, two words). `scratch[0..8]` on its own would be the slice *value* `[u8]`, which is a DST and can't be bound to a variable.

## Implicit coercion: `&[T; N]` → `&[T]`

Rust silently coerces array references to slice references whenever a slice is expected. The compile-time length `N` gets *erased* into the runtime length field of the fat pointer:

```rust
fn hash(data: &[u8]) { ... }

let scratch = [0u8; 32];
hash(&scratch);                  // &[u8; 32] → &[u8] automatic
```

This is why you can usually pass `&array` where the API wants `&slice` and it just works. The compiler is doing one conversion at the call site, free.

## Mapping to the comm_link code

```rust
let mut scratch = [0u8; SEND_BUFFER_SIZE];                       // [1]
let bytes_to_send =
    postcard::to_slice(&state, &mut scratch).expect("...");      // [2]
tx_packet.copy_from_slice(bytes_to_send);                        // [3]
```

**[1]** `scratch: [u8; 32]` — the **owner**. 32 bytes living on `send()`'s stack frame. Reserved by the function prologue.

**[2]** `&mut scratch` is `&mut [u8; 32]`, which coerces to `&mut [u8]` (fat pointer, len = 32) for `postcard::to_slice`'s signature. Postcard writes some encoded bytes into the buffer — say 1 byte for a small `count` — and returns `Ok(&mut [u8])` where that returned slice has length = "actual bytes written":

```
After postcard::to_slice:
  scratch:    [05][??][??][??] ... [??][??]      // byte 0 written, 1..31 untouched
  bytes_to_send → ptr=&scratch[0], len=1
```

**[3]** `tx_packet.copy_from_slice(bytes_to_send)` reads `bytes_to_send.len()` (= 1) and copies exactly that many bytes. The radio frame contains just the encoded data — not the trailing garbage.

If `scratch` had been passed directly to `copy_from_slice`, the radio would have transmitted all 32 bytes: 1 byte of payload + 31 bytes of stack noise. The receiver would have no way to know where the real data ended.

The slice is *the* mechanism by which the encoded length survives the journey from "postcard wrote it" to "radio transmits it." Without slices you'd have to return `(buf, len)` as a tuple and remember to use them together — which is exactly the C pattern, with all its opportunities to get the length wrong.

## Slice methods you'll use constantly

```rust
let s: &[u8] = &buf[..];

s.len()                    // length in elements
s.is_empty()               // s.len() == 0
s[0]                       // index (panics if out of range)
s.get(0)                   // Option<&u8> — safe indexing
s.first()                  // Option<&u8> — element 0
s.last()                   // Option<&u8>
s.iter()                   // iterator over &T
s.split_at(8)              // (&s[..8], &s[8..])
s.chunks(4)                // iterator over &[T] of size 4
&s[a..b]                   // sub-slice (panics if a..b out of range)
s.copy_from_slice(other)   // memcpy when other.len() == s.len()
```

`copy_from_slice` is the one used in `send()`. It's a checked `memcpy`: panics if the source and destination lengths differ, otherwise blasts the bytes across. The check is what makes it safe — you literally cannot copy the wrong number of bytes.

For mutable slices, add:

```rust
s.iter_mut()                                   // iterator over &mut T
s.split_at_mut(8)                              // (&mut [..8], &mut [8..])
s[i] = v                                       // write
s.fill(0)                                      // memset
```

## Mutable vs shared slices

Same rules as any Rust reference:

- `&[T]` — many readers allowed simultaneously; no writers.
- `&mut [T]` — exactly one user; can read and write; no other references (mutable or shared) may exist to the same elements.

The borrow checker tracks slices the same way it tracks references to plain values. So:

```rust
let mut buf = [0u8; 32];
let a = &mut buf[..16];
let b = &mut buf[16..];        // ERROR: cannot borrow buf as mutable twice
```

This is rejected even though `a` and `b` cover non-overlapping ranges — the compiler can't prove disjointness through arbitrary indexing. The escape hatch is `split_at_mut`, which the standard library implements with `unsafe` internally but exposes a safe API:

```rust
let (a, b) = buf.split_at_mut(16);             // OK: two disjoint &mut [u8]
```

`split_at_mut` is the standard tool when you need two mutable views into different parts of one buffer.

## What slices don't do

- **Don't own anything.** Always borrowing. Dropping a slice does nothing; only the owner's drop matters.
- **Don't allocate.** Creating a slice is just computing (ptr, len). Free.
- **Don't bounds-check construction.** `&scratch[0..40]` panics at the *slicing point* because 40 > 32, but once you have a valid slice it doesn't carry any extra check — indexing into a valid slice still panics on out-of-range.
- **No capacity concept.** A slice's length *is* its size. Compare with `Vec<T>` which has separate `len()` and `capacity()`. Slices don't grow.
- **Can't be returned by value.** `fn make() -> [u8]` is invalid (DST). You return `&[u8]` (a borrow — and then someone has to own the storage), or `[u8; N]` (a fixed-size array), or `Vec<u8>` / `Box<[u8]>` (allocator required).

## Special slice: `&str`

`&str` is a slice — specifically a `&[u8]` that the compiler guarantees is valid UTF-8. Same fat-pointer representation, same borrowing rules, same cost model. The relationship is:

| Owned (allocator) | Borrowed view |
|-------------------|---------------|
| `Vec<u8>`         | `&[u8]`       |
| `String`          | `&str`        |
| `Box<[u8]>`       | `&[u8]`       |
| `[u8; N]`         | `&[u8]`       |

The owned column is heap-allocated (or stack for the array case). The borrowed column is a fat pointer into the owned thing. This pattern repeats throughout the standard library.

## Why this matters extra-hard for embedded

In `no_std` / embedded, there's no allocator by default. No `Vec`, no `String`, no `Box`. Your owners are exclusively:

- Stack arrays (`[u8; N]`).
- `static` variables.
- Memory you got from a peripheral / DMA descriptor.

And the "I need to pass a chunk of bytes around" type is *always* `&[u8]` or `&mut [u8]`. There's no other option. Slices aren't a nicety on embedded — they're the only game in town for "here are some bytes, take them."

Get fluent with slices and you've covered ~80% of the data-passing patterns the rest of this project will need.

## The takeaway

| What it is        | Type         | Size              | Owns? |
|-------------------|--------------|-------------------|-------|
| Array             | `[T; N]`     | `N * size_of::<T>()` | Yes  |
| Bare slice (DST)  | `[T]`        | Unknown at compile | n/a |
| Slice reference   | `&[T]`       | 2 words (fat ptr)  | No  |
| Mut slice ref     | `&mut [T]`   | 2 words (fat ptr)  | No  |
| String slice      | `&str`       | 2 words (fat ptr)  | No  |

In `comm_link.rs`:

- `scratch: [u8; 32]` is "the bytes." (Owner.)
- `bytes_to_send: &mut [u8]` is "the bytes that matter, with the length attached." (View.)

Same memory, different types, different lengths. The slice is what makes "postcard wrote 1 byte, radio sends 1 byte" possible without ever passing the length as a separate variable.

That's the whole point of slices: the length and the pointer ride together as one value, so there's nothing for the programmer to keep in sync.

## Sources

- Rust book, "The Slice Type": <https://doc.rust-lang.org/book/ch04-03-slices.html>
- Rust reference, "Array and slice types": <https://doc.rust-lang.org/reference/types/array.html>
- `std::slice` documentation: <https://doc.rust-lang.org/std/slice/index.html>
- `core::slice` (the `no_std` equivalent — same API minus the allocator-requiring methods): <https://doc.rust-lang.org/core/slice/index.html>
