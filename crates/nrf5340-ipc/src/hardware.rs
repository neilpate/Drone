use core::future::poll_fn;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::Poll;

use embassy_nrf::interrupt::{InterruptExt, Priority};
use embassy_nrf::{interrupt, pac};
use embassy_sync::waitqueue::AtomicWaker;

#[cfg(feature = "app")]
use crate::MAX_FRAME_SIZE;
use crate::{Error, MAGIC, Queue, STARTUP_PROBE, Shared};

static TAKEN: AtomicBool = AtomicBool::new(false);
static WAKER: AtomicWaker = AtomicWaker::new();

unsafe extern "C" {
    static __radio_shared_start: u8;
    static __radio_shared_end: u8;
}

/// Unique local owner of IPC channel 0 and one producer/consumer pair.
#[derive(Debug)]
pub struct Endpoint {
    shared: &'static Shared,
}

fn registers() -> &'static pac::ipc::RegisterBlock {
    // SAFETY: this module is the sole owner of IPC channel/event 0 on each core.
    unsafe { &*pac::IPC::ptr() }
}

fn notify() {
    // Order shared-memory publication before the peripheral doorbell.
    cortex_m::asm::dmb();
    registers().tasks_send[0].write(|w| w.tasks_send().set_bit());
}

#[interrupt]
fn IPC() {
    registers().events_receive[0].reset();
    WAKER.wake();
}

impl Endpoint {
    /// Application core only. Stops the network core before initializing RAM,
    /// grants access to the shared region, then releases the network core.
    #[cfg(feature = "app")]
    pub fn boot_network() -> Self {
        Self::claim();
        // SAFETY: called once by the app BSP, before any network-core work.
        let reset = unsafe { &*pac::RESET::ptr() };
        reset.network.forceoff.write(|w| w.forceoff().hold());
        cortex_m::asm::dsb();
        cortex_m::asm::isb();

        // The last eight 8-KiB SPU RAM regions are reserved by both linkers.
        // SAFETY: only the app core configures SPU during startup.
        let spu = unsafe { &*pac::SPU::ptr() };
        spu.extdomain[0]
            .perm
            .modify(|_, w| w.secattr().non_secure());
        for region in &spu.ramregion[56..64] {
            region.perm.modify(|_, w| {
                w.secattr()
                    .non_secure()
                    .read()
                    .enable()
                    .write()
                    .enable()
                    .execute()
                    .disable()
            });
        }
        // SAFETY: the network core is held in reset; no other code owns this RAM.
        let shared = unsafe {
            let pointer = Self::shared_pointer();
            pointer.write(Shared::new());
            &*pointer
        };
        shared.magic.store(MAGIC, Ordering::Release);
        let endpoint = Self { shared };
        Self::configure_interrupt();
        cortex_m::asm::dsb();
        reset.network.forceoff.write(|w| w.forceoff().release());
        endpoint
    }

    /// Network core only. The app must initialize shared RAM before releasing us.
    #[cfg(feature = "net")]
    pub fn attach() -> Result<Self, Error> {
        Self::claim();
        Self::configure_interrupt();
        // SAFETY: app startup initializes this region before releasing this core.
        let shared = unsafe { &*Self::shared_pointer() };
        if shared.magic.load(Ordering::Acquire) != MAGIC {
            return Err(Error::IncompatibleVersion);
        }
        Ok(Self { shared })
    }

    fn claim() {
        assert!(!TAKEN.swap(true, Ordering::AcqRel), "IPC already taken");
    }

    fn shared_pointer() -> *mut Shared {
        let start = core::ptr::addr_of!(__radio_shared_start) as usize;
        let end = core::ptr::addr_of!(__radio_shared_end) as usize;
        assert_eq!(start, 0x2007_0000, "IPC linker address mismatch");
        assert_eq!(end - start, 64 * 1024, "IPC linker size mismatch");
        assert!(core::mem::size_of::<Shared>() <= end - start);
        start as *mut Shared
    }

    fn configure_interrupt() {
        let ipc = registers();
        // SAFETY: channel mask 1 selects the reserved IPC channel 0.
        ipc.send_cnf[0].write(|w| unsafe { w.bits(1) });
        // SAFETY: event 0 listens only to IPC channel 0.
        ipc.receive_cnf[0].write(|w| unsafe { w.bits(1) });
        ipc.events_receive[0].reset();
        ipc.intenset.write(|w| w.receive0().set());
        interrupt::IPC.set_priority(Priority::P3);
        interrupt::IPC.unpend();
        // SAFETY: handler and state have been initialized before unmasking.
        unsafe { interrupt::IPC.enable() };
    }

    fn outgoing(&self) -> &Queue {
        #[cfg(feature = "app")]
        return &self.shared.to_net;
        #[cfg(feature = "net")]
        return &self.shared.to_app;
    }

    fn incoming(&self) -> &Queue {
        #[cfg(feature = "app")]
        return &self.shared.to_app;
        #[cfg(feature = "net")]
        return &self.shared.to_net;
    }

    /// Nonblocking enqueue. Success means queued, not transmitted over the air.
    pub fn try_send(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.outgoing().push(bytes)?;
        notify();
        Ok(())
    }

    /// Wait for queue space. Cancellation never publishes a partial frame.
    pub async fn send(&mut self, bytes: &[u8]) -> Result<(), Error> {
        poll_fn(|cx| {
            WAKER.register(cx.waker());
            match self.try_send(bytes) {
                Err(Error::QueueFull) => Poll::Pending,
                result => Poll::Ready(result),
            }
        })
        .await
    }

    /// Application core only. Send the startup probe and wait for the network
    /// core's echo through the real frame queues. The caller owns the timeout.
    #[cfg(feature = "app")]
    pub async fn probe(&mut self) -> Result<(), Error> {
        self.send(STARTUP_PROBE).await?;
        let mut buffer = [0; MAX_FRAME_SIZE];
        let length = self.receive(&mut buffer).await?;
        if &buffer[..length] != STARTUP_PROBE {
            return Err(Error::StartupMismatch);
        }
        Ok(())
    }

    /// Cancel-safe: the frame is consumed only in the poll that returns it.
    #[cfg(feature = "app")]
    pub async fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Error> {
        self.receive_frame(buffer).await
    }

    /// Cancel-safe: the frame is consumed only in the poll that returns it.
    /// Startup probes from the app core are echoed whenever they arrive and
    /// never returned, so a network-core-only restart needs no handshake phase.
    #[cfg(feature = "net")]
    pub async fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Error> {
        loop {
            let length = self.receive_frame(buffer).await?;
            if &buffer[..length] != STARTUP_PROBE {
                return Ok(length);
            }
            // No await between dequeue and echo, so cancellation cannot drop a probe.
            match self.try_send(STARTUP_PROBE) {
                Ok(()) => defmt::info!("app-core IPC probe answered"),
                Err(error) => defmt::warn!("app-core IPC probe echo failed: {:?}", error),
            }
        }
    }

    async fn receive_frame(&mut self, buffer: &mut [u8]) -> Result<usize, Error> {
        poll_fn(|cx| {
            WAKER.register(cx.waker());
            match self.incoming().pop(buffer) {
                Ok(Some(length)) => {
                    notify();
                    Poll::Ready(Ok(length))
                }
                Ok(None) => Poll::Pending,
                Err(error) => Poll::Ready(Err(error)),
            }
        })
        .await
    }
}
