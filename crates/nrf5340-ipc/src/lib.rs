#![cfg_attr(not(test), no_std)]

//! nRF5340-only, length-preserving IPC. No postcard or application protocol.

#[cfg(any(test, feature = "app", feature = "net"))]
use core::sync::atomic::{AtomicU32, Ordering};

#[cfg(all(feature = "app", feature = "net"))]
compile_error!("build the application and network cores separately");

#[cfg(any(feature = "app", feature = "net"))]
mod hardware;
#[cfg(any(feature = "app", feature = "net"))]
pub use hardware::Endpoint;

pub const MAX_FRAME_SIZE: usize = 125;
pub const RADIO_CHANNEL: u8 = 20;
#[cfg(any(feature = "app", feature = "net"))]
const STARTUP_PROBE: &[u8] = b"drone-ipc-v1";
#[cfg(any(test, feature = "app", feature = "net"))]
const QUEUE_DEPTH: usize = 4;
#[cfg(any(test, feature = "app", feature = "net"))]
const WORDS: usize = MAX_FRAME_SIZE.div_ceil(4);
#[cfg(any(feature = "app", feature = "net"))]
const MAGIC: u32 = 0x4950_4301;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(any(feature = "app", feature = "net"), derive(defmt::Format))]
pub enum Error {
    FrameTooLong,
    BufferTooSmall,
    QueueFull,
    CorruptFrame,
    IncompatibleVersion,
    Timeout,
    StartupMismatch,
}

#[repr(C)]
#[derive(Debug)]
#[cfg(any(test, feature = "app", feature = "net"))]
struct Slot {
    length: AtomicU32,
    data: [AtomicU32; WORDS],
}

#[cfg(any(test, feature = "app"))]
impl Slot {
    const fn new() -> Self {
        Self {
            length: AtomicU32::new(0),
            data: [const { AtomicU32::new(0) }; WORDS],
        }
    }
}

// Each queue has exactly one producer and one consumer, on opposite cores.
// Atomic payload words also avoid creating Rust references to concurrently
// modified non-atomic shared memory.
#[repr(C)]
#[derive(Debug)]
#[cfg(any(test, feature = "app", feature = "net"))]
struct Queue {
    head: AtomicU32,
    tail: AtomicU32,
    slots: [Slot; QUEUE_DEPTH],
}

#[cfg(any(test, feature = "app", feature = "net"))]
impl Queue {
    #[cfg(any(test, feature = "app"))]
    const fn new() -> Self {
        Self {
            head: AtomicU32::new(0),
            tail: AtomicU32::new(0),
            slots: [const { Slot::new() }; QUEUE_DEPTH],
        }
    }

    fn push(&self, bytes: &[u8]) -> Result<(), Error> {
        if bytes.len() > MAX_FRAME_SIZE {
            return Err(Error::FrameTooLong);
        }
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        if head.wrapping_sub(tail) >= QUEUE_DEPTH as u32 {
            return Err(Error::QueueFull);
        }
        let slot = &self.slots[head as usize % QUEUE_DEPTH];
        for (destination, chunk) in slot.data.iter().zip(bytes.chunks(4)) {
            let mut word = [0; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            destination.store(u32::from_le_bytes(word), Ordering::Relaxed);
        }
        slot.length.store(bytes.len() as u32, Ordering::Relaxed);
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    fn pop(&self, bytes: &mut [u8]) -> Result<Option<usize>, Error> {
        let tail = self.tail.load(Ordering::Relaxed);
        if tail == self.head.load(Ordering::Acquire) {
            return Ok(None);
        }
        let slot = &self.slots[tail as usize % QUEUE_DEPTH];
        let length = slot.length.load(Ordering::Relaxed) as usize;
        if length > MAX_FRAME_SIZE {
            return Err(Error::CorruptFrame);
        }
        if bytes.len() < length {
            return Err(Error::BufferTooSmall);
        }
        for (destination, word) in bytes[..length].chunks_mut(4).zip(&slot.data) {
            let source = word.load(Ordering::Relaxed).to_le_bytes();
            destination.copy_from_slice(&source[..destination.len()]);
        }
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Ok(Some(length))
    }
}

#[repr(C)]
#[derive(Debug)]
#[cfg(any(feature = "app", feature = "net"))]
struct Shared {
    magic: AtomicU32,
    to_app: Queue,
    to_net: Queue,
}

#[cfg(any(feature = "app", feature = "net"))]
const _: () = {
    assert!(core::mem::size_of::<Shared>() == 1076);
    assert!(core::mem::align_of::<Shared>() == 4);
};

#[cfg(feature = "app")]
impl Shared {
    const fn new() -> Self {
        Self {
            magic: AtomicU32::new(0),
            to_app: Queue::new(),
            to_net: Queue::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_lengths_and_payloads_across_reuse() {
        let queue = Queue::new();
        let mut output = [0xAA; MAX_FRAME_SIZE];
        for length in [125, 1, 0, 4, 5, 124, 3] {
            let input: Vec<u8> = (0..length).map(|i| i as u8).collect();
            queue.push(&input).unwrap();
            assert_eq!(queue.pop(&mut output), Ok(Some(length)));
            assert_eq!(&output[..length], input);
            assert_eq!(queue.pop(&mut output), Ok(None));
        }
    }

    #[test]
    fn rejects_oversize_and_retains_frame_for_small_buffer() {
        let queue = Queue::new();
        assert_eq!(queue.push(&[0; 126]), Err(Error::FrameTooLong));
        queue.push(&[1, 2, 3]).unwrap();
        assert_eq!(queue.pop(&mut [0; 2]), Err(Error::BufferTooSmall));
        let mut output = [0; 3];
        assert_eq!(queue.pop(&mut output), Ok(Some(3)));
        assert_eq!(output, [1, 2, 3]);
    }

    #[test]
    fn full_queue_does_not_overwrite_unread_frames_and_counters_wrap() {
        let queue = Queue::new();
        queue.head.store(u32::MAX - 1, Ordering::Relaxed);
        queue.tail.store(u32::MAX - 1, Ordering::Relaxed);
        for i in 0..QUEUE_DEPTH {
            queue.push(&[i as u8]).unwrap();
        }
        assert_eq!(queue.push(&[99]), Err(Error::QueueFull));
        for i in 0..QUEUE_DEPTH {
            let mut output = [0];
            assert_eq!(queue.pop(&mut output), Ok(Some(1)));
            assert_eq!(output, [i as u8]);
        }
        queue.push(&[42]).unwrap();
        let mut output = [0];
        assert_eq!(queue.pop(&mut output), Ok(Some(1)));
        assert_eq!(output, [42]);
    }

    #[test]
    fn concurrent_producer_and_consumer_preserve_frame_order() {
        let queue = Queue::new();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                for i in 0u32..10_000 {
                    while queue.push(&i.to_le_bytes()) == Err(Error::QueueFull) {
                        std::thread::yield_now();
                    }
                }
            });
            let mut output = [0; 4];
            for i in 0u32..10_000 {
                loop {
                    match queue.pop(&mut output).unwrap() {
                        Some(4) => break,
                        None => std::thread::yield_now(),
                        other => panic!("unexpected frame: {other:?}"),
                    }
                }
                assert_eq!(u32::from_le_bytes(output), i);
            }
        });
    }
}
