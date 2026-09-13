use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Ticker};

use crate::{FIRMWARE_VERSION, signals::telemetry};
use firmware_types::{RemoteInfo, TO_GROUNDSTATION_MAX_SIZE_BYTES, ToGroundStation};

use crate::board::UartTx;

#[embassy_executor::task]
pub async fn serial_link_tx(mut uart_tx: UartTx) -> ! {
    defmt::info!("serial_link_tx (to groundstation) task: started");

    let mut receiver = telemetry::subscribe();

    let mut ticker = Ticker::every(Duration::from_secs(2));

    let remote_info = RemoteInfo {
        firmware_version: FIRMWARE_VERSION,
    };

    loop {
        let msg = match select(receiver.changed(), ticker.next()).await {
            Either::First(telemetry) => ToGroundStation::Telemetry(telemetry),
            Either::Second(_) => ToGroundStation::RemoteInfo(remote_info),
        };

        let mut buf = [0u8; TO_GROUNDSTATION_MAX_SIZE_BYTES];

        let framed = postcard::to_slice_cobs(&msg, &mut buf).unwrap();

        uart_tx.write(framed).await.unwrap();
    }
}
