# Rust value semantics on Cortex-M

_Captured 2026-05-30 while refactoring `firmware-remote/src/tasks/drone_link.rs` (then named `comm_link.rs`) into `send` / `receive` helpers._

## The question

In C you generally pass big things by pointer because returning a struct by value implies a copy. Rust code returns things by value everywhere — `Option<T>`, `Result<T, E>`, whole structs — without anyone seeming to worry. Is this actually free, or is the abstraction hiding cost?

Short answer: on ARM Cortex-M with `--release`, it's free. The compiler and the ARM calling convention conspire to make "return by value" compile to either register moves (small types) or a single in-place write (large types). No intermediate copies.

## The mechanics

### AAPCS register roles

ARM has 16 general-purpose 32-bit registers `r0`–`r15`. The ARM Architecture Procedure Call Standard (AAPCS) assigns roles:

| Register | Role | Save responsibility |
|---|---|---|
| `r0`–`r3`  | First 4 arguments, return value(s) | Caller-saved (scratch) |
| `r4`–`r11` | General-purpose locals             | Callee-saved |
| `r12`      | Intra-procedure scratch            | Caller-saved |
| `r13` (SP) | Stack pointer                      | Special |
| `r14` (LR) | Link register (return address)     | Special |
| `r15` (PC) | Program counter                    | Special |

Caller-saved means short-lived values that the caller spills only if it needs them across a call. Callee-saved means values the callee must push on entry and pop on exit if it wants to use them. The split exists so neither side pays for saves it doesn't need.

### Return value rules

- ≤ 4 bytes → returned in `r0`.
- ≤ 8 bytes → returned in `r0`–`r1`.
- &gt; 8 bytes → **sret** (see below).

### sret ("structure return")

When the return type is too big for the return registers, the caller pre-allocates a slot in its own stack frame and passes the address of that slot to the callee as a **hidden first argument** in `r0`. Real arguments shift to `r1`–`r3`. The callee writes the value directly into that slot and returns.

In C terms, a function declared as:

```c
Packet make_packet(int x);
```

…is compiled as if it were:

```c
void make_packet(Packet *out, int x);
```

No intermediate `Packet` exists. The bytes are written once, into their final destination.

### Guaranteed copy elision (RVO)

Rust's spec guarantees that when a function returns a value, the compiler must write it directly into the caller's slot — no intermediate temporary, no copy. Combined with sret, this means a chain like:

```rust
fn outer() -> Packet { inner() }
fn inner() -> Packet { Packet::new() }
let p = outer();
```

…allocates `p` once in the outermost frame, and `Packet::new` writes its bytes directly there through the sret pointer threaded down the call chain. Zero copies, regardless of how many layers of "returning by value" are stacked.

C++ has the same optimisation but it's only mandatory since C++17. C has no equivalent — you write the out-pointer manually.

## Worked example from `drone_link.rs`

```rust
async fn receive(radio: &mut Radio) -> Option<TelemetryState> {
    let mut rx_packet = Packet::new();          // [1]
    // ... timeout match, decode ...
    match postcard::from_bytes(&rx_packet) {
        Ok(telemetry) => Some(telemetry),       // [2]
        Err(_) => None,
    }
}
```

**[1] `let mut rx_packet = Packet::new()`** — `Packet` is ~130 bytes (max 802.15.4 PHY payload + length + padding). Too big for registers, so sret kicks in:

1. `receive`'s prologue reserves 130 bytes on the stack (`SP -= 130`).
2. `Packet::new` is called with `r0 = &rx_packet` as the hidden out-pointer.
3. `Packet::new` zero-fills those 130 bytes directly. Returns.

Net cost: one stack reservation + one zero-fill. Identical to `Packet rx_packet; packet_init(&rx_packet);` in C, but written as a single expression and without the out-pointer being visible.

`mut` is purely a compile-time annotation. It does not change generated code; it just tells the borrow checker that `&mut` references to this binding are allowed.

**[2] `Some(telemetry)`** — `TelemetryState` is 4 bytes (`u32`). `Option<TelemetryState>` is 8 bytes (1-byte discriminant + 3 bytes padding + 4-byte payload). 8 bytes fits in `r0`–`r1`, so no sret. The whole return path:

```
; r0 holds &Result from postcard::from_bytes (sret on the way back)
ldr   r1, [r0, #4]    ; load TelemetryState.count from Result payload
movs  r0, #1          ; Some discriminant
bx    lr              ; return — Option<TelemetryState> in (r0, r1)
```

At the call site, `if let Some(telemetry) = receive(...).await` becomes:

```
cmp   r0, #0
beq   .skip           ; r0 == 0 (None) → skip
                       ; otherwise r1 holds telemetry.count
```

No `Option` ever exists in memory. It lives in two registers across the call boundary, gets pattern-matched, and is gone.

### Struct construction is free

`ControlState::new(count)` where `ControlState { pub count: u32 }`:

```rust
let state = ControlState::new(count);
```

Compiles to **nothing**. After inlining, `state` is just an alias for the same register that already held `count`. The "struct wrapping" is a type-system fiction the compiler sees through.

This is why Rust newtype patterns (`struct Celsius(f32); struct Fahrenheit(f32);`) cost zero at runtime — you get type-checked unit safety for free.

## The general rule

| Return size | Mechanism | Cost |
|---|---|---|
| ≤ 8 bytes | Registers (`r0`/`r1`) | Free — bytes are register moves |
| &gt; 8 bytes | sret + guaranteed copy elision | One in-place write, no copy |

Idiomatic Rust — `Option`, `Result`, small structs, newtypes — compiles to the same machine code you'd write by hand in C. The abstraction is genuinely free.

## When it isn't free

The "free return-by-value" story breaks when:

- The struct is large **and** you're not in a position to chain sret cleanly (e.g. through dynamic dispatch via `dyn Trait`, where the compiler can't see the call target).
- The constructor does real work (allocation, validation, calling non-inlined code).
- You actually want to keep the buffer around in the caller's frame — in which case pass `&mut` rather than returning, exactly as we do with `Packet` in `radio.receive(&mut rx_packet)`. Returning a 130-byte buffer would burn 130 bytes of caller stack for nothing if all the caller wants is to extract a 4-byte field from it.

## Comparison: C, x86-64, RISC-V

| Architecture     | Arg registers                          | Return register(s)            | Total GPRs |
|------------------|----------------------------------------|-------------------------------|------------|
| ARM Cortex-M     | `r0`–`r3`                              | `r0` (≤4B), `r0`–`r1` (≤8B)   | 16         |
| x86-64 Windows   | `rcx`, `rdx`, `r8`, `r9`               | `rax` (`rdx:rax` for 16B)     | 16         |
| x86-64 SysV      | `rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9` | `rax` (`rdx:rax` for 16B)     | 16         |
| RISC-V           | `a0`–`a7`                              | `a0`–`a1`                     | 32         |

All of them use sret for large returns. The threshold differs (8B on ARM, 16B on x86-64) but the mechanism is the same.

## How to verify

To see what the compiler actually emitted for a specific function, build release and disassemble:

```pwsh
cargo build --release
arm-none-eabi-objdump -d target/thumbv7em-none-eabihf/release/firmware-remote `
    | Select-String -Context 0,20 "<firmware_remote.*receive"
```

The "shape" of the Rust source — match arms, `Option`, `Result` — vanishes. What's left is `bl` (branch-with-link) calls plus a small amount of register shuffling on the return path.

## Sources

- AAPCS: <https://github.com/ARM-software/abi-aa/blob/main/aapcs32/aapcs32.rst>
- Rust reference, "Destructors and copy elision": <https://doc.rust-lang.org/reference/destructors.html>
- Rust RFC 1909 (unsized rvalues / placement) for background on the value-semantics model.
