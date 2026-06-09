use firmware_types::Throttle;

use crate::{board::Uart, signals::set_throttle_command};

#[embassy_executor::task]
pub async fn serial_link(mut uart: Uart) -> ! {
    defmt::info!("serial_link task: started");

    let mut buffer = [0u8; 32];

    let mut byte = [0u8; 1];

    loop {
        let mut count = 0;
        loop {
            uart.read(&mut byte).await.unwrap();

            if byte[0] == b'\n' || count >= buffer.len() {
                break;
            }

            buffer[count] = byte[0];
            count += 1;
        }

        let throttle = core::str::from_utf8(&buffer[..count]);

        match throttle {
            Ok(v) => {
                let throttle = v.parse::<f32>();

                match throttle {
                    Ok(t) => {
                        set_throttle_command(Throttle::from_normalised(t));
                    }
                    Err(_) => {
                        defmt::warn!("serial_link: parse error: {=str}", v);
                    }
                }
            }
            Err(_) => {
                defmt::warn!("serial_link: non-utf8: {=[u8]:a}", &buffer[..count]);
            }
        }
    }
}
