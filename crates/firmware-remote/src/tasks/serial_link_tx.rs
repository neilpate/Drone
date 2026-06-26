use crate::signals::telemetry;
use firmware_types::TELEMETRY_FRAME_MAX_SIZE_BYTES;

use crate::board::UartTx;

#[embassy_executor::task]
pub async fn serial_link_tx(mut uart_tx: UartTx) -> ! {
    defmt::info!("serial_link_tx (to groundstation) task: started");

    let mut receiver = telemetry::subscribe();

    loop {
        let telemetry = receiver.changed().await;

        let mut buf = [0u8; TELEMETRY_FRAME_MAX_SIZE_BYTES];
        let framed = postcard::to_slice_cobs(&telemetry, &mut buf).unwrap();

        uart_tx.write(framed).await.unwrap();
    }
}
