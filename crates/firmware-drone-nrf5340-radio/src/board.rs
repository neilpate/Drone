//! Physical-to-logical peripheral mapping for the nRF5340 network core.

use embassy_nrf::config::{Config, HfclkSource};
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::radio::ieee802154::Radio;
use embassy_nrf::{bind_interrupts, peripherals, radio};
use nrf5340_ipc::{Endpoint, Error};

bind_interrupts!(struct Irqs {
    RADIO => radio::InterruptHandler<peripherals::RADIO>;
});

pub struct Board {
    pub radio: Radio<'static, peripherals::RADIO>,
    pub status_led: Output<'static>,
    pub app_core_interface: AppCoreInterface,
}

impl Board {
    pub fn new() -> Result<Self, Error> {
        let mut config = Config::default();
        config.hfclk_source = HfclkSource::ExternalXtal;
        let p = embassy_nrf::init(config);

        Ok(Self {
            radio: Radio::new(p.RADIO, Irqs),
            status_led: Output::new(p.P0_31, Level::Low, OutputDrive::Standard),
            app_core_interface: AppCoreInterface::new()?,
        })
    }
}

pub struct AppCoreInterface {
    endpoint: Endpoint,
}

impl AppCoreInterface {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            endpoint: Endpoint::attach()?,
        })
    }

    /// Receive the next frame for transmission. The transport answers the
    /// app core's startup probes itself and never returns them.
    pub async fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Error> {
        self.endpoint.receive(buffer).await
    }

    pub fn try_send(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.endpoint.try_send(bytes)
    }
}
