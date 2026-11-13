#![no_std]
#![no_main]

use cortex_m_rt::entry;
use nrf5340_app_pac::{CTRLAP_S, NFCT_S, P0_S, UICR_S};

#[entry]
fn main() -> ! {
    let gpio = unsafe { &*P0_S::ptr() };
    let uicr = unsafe { &*UICR_S::ptr() };
    let ctrlap = unsafe { &*CTRLAP_S::ptr() };
    ctrlap
        .approtect
        .disable
        .write(|w| unsafe { w.bits(uicr.approtect.read().bits()) });
    ctrlap
        .secureapprotect
        .disable
        .write(|w| unsafe { w.bits(uicr.secureapprotect.read().bits()) });

    if uicr.nfcpins.read().protect().is_nfc() {
        gpio.outset.write(|w| unsafe { w.bits(1 << 28) });
        gpio.pin_cnf[28].write(|w| w.dir().output().input().disconnect());
        loop {
            cortex_m::asm::wfi();
        }
    }

    let nfct = unsafe { &*NFCT_S::ptr() };
    nfct.tasks_disable.write(|w| w.tasks_disable().trigger());

    gpio.outclr.write(|w| unsafe { w.bits(1 << 28) });
    gpio.pin_cnf[28].write(|w| w.dir().output().input().disconnect());

    gpio.outclr.write(|w| unsafe { w.bits(1 << 3) });
    gpio.pin_cnf[3].write(|w| w.dir().output().input().connect());

    // Drive P0.03 low then high, about 2 s each, and mirror the read-back on
    // the LED. A working pin makes the LED alternate; a stuck pin does not.
    let mut step: u32 = 0;
    loop {
        if step % 64 < 32 {
            gpio.outclr.write(|w| unsafe { w.bits(1 << 3) });
        } else {
            gpio.outset.write(|w| unsafe { w.bits(1 << 3) });
        }
        cortex_m::asm::delay(10_000);
        if gpio.in_.read().bits() & (1 << 3) != 0 {
            gpio.outset.write(|w| unsafe { w.bits(1 << 28) });
        } else {
            gpio.outclr.write(|w| unsafe { w.bits(1 << 28) });
        }
        cortex_m::asm::delay(4_000_000);
        step = step.wrapping_add(1);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        cortex_m::asm::wfi();
    }
}
