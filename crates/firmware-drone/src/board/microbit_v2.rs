//! BBC micro:bit v2 (nRF52833) board support.
//!
//! Phase 1–3 development target. Sensors and actuators are breadboarded onto
//! the edge connector; only the on-board 5x5 LED matrix is used directly.
//!
//! See [ADR 0010](../../../../doc/decisions/0010-board-support-package.md) for
//! the contract this module satisfies.
//!
//!

use core::sync::atomic::{AtomicU16, Ordering};

use embassy_nrf::config::{Config, HfclkSource};
use embassy_nrf::gpio::{Level, Output, OutputDrive, Pin};
use embassy_nrf::pwm;
use embassy_nrf::spim::{self, Spim};
use embassy_nrf::{bind_interrupts, peripherals, radio, temp};
use firmware_types::{Acceleration, AngularRate, ImuData, MotorCommand, Throttle};

pub const NAME: &str = "BBC micro:bit v2";

/// BSP-typed alias for the embassy IEEE 802.15.4 radio driver bound to this board.
pub type Radio = radio::ieee802154::Radio<'static, peripherals::RADIO>;

pub type TemperatureSensor = temp::Temp<'static>;
pub type Spim3 = spim::Spim<'static, peripherals::SPI3>;

bind_interrupts!(struct Irqs {
    RADIO => radio::InterruptHandler<peripherals::RADIO>;
    TEMP => temp::InterruptHandler;
    SPIM3 => spim::InterruptHandler<peripherals::SPI3>;
});

pub struct Board {
    pub status_led: StatusLed,
    pub motors: Motors,
    pub radio: Radio,
    pub temperature_sensor: TemperatureSensor,
    pub imu: Imu,
}

// Pin map — BBC micro:bit v2 / nRF52833
//
// | Function       | nRF pin | Edge | Peripheral | Notes               |
// |----------------|---------|------|------------|---------------------|
// | Motor M1       | P0.10   | 8    | PWM0 ch0   | rear-right (CCW), NFC2 -> GPIO |
// | Motor M2       | P0.09   | 9    | PWM0 ch1   | front-right (CW), NFC1 -> GPIO |
// | Motor M3       | P0.12   | 12   | PWM0 ch2   | rear-left (CW)                 |
// | Motor M4       | P0.02   | 0    | PWM0 ch3   | front-left (CCW), large pad    |
// | IMU SCK        | P0.17   | 13   | SPIM3      | SPI_EXT block       |
// | IMU MISO       | P0.01   | 14   | SPIM3      |                     |
// | IMU MOSI       | P0.13   | 15   | SPIM3      |                     |
// | IMU CS         | P1.02   | 16   | GPIO out   | manual, idle high   |
// | Status LED row | P0.21   | -    | GPIO out   | onboard 5x5 matrix  |
// | Status LED col | P0.28   | -    | GPIO out   | onboard 5x5 matrix  |
// | Radio          | -       | -    | RADIO      | internal, 802.15.4  |
// | Temp sensor    | -       | -    | TEMP       | internal            |
//
// Motors share PWM0 (one frequency, independent duty). The NFC pins
// (P0.09 / P0.10) require the `nfc-pins-as-gpio` embassy-nrf feature.

impl Board {
    pub fn new() -> Self {
        let mut config = Config::default();
        config.hfclk_source = HfclkSource::ExternalXtal;
        let p = embassy_nrf::init(config);

        let mut spi_config = spim::Config::default();
        spi_config.frequency = spim::Frequency::M1;
        spi_config.mode = spim::MODE_0;

        let imu_spi = Spim::new(p.SPI3, Irqs, p.P0_17, p.P0_01, p.P0_13, spi_config);
        let imu_cs = Output::new(p.P1_02.degrade(), Level::High, OutputDrive::Standard);

        Self {
            status_led: StatusLed::new(p.P0_21, p.P0_28),
            motors: Motors::new(p.PWM0, p.P0_10, p.P0_09, p.P0_12, p.P0_02),
            radio: Radio::new(p.RADIO, Irqs),
            temperature_sensor: TemperatureSensor::new(p.TEMP, Irqs),
            imu: Imu {
                spi: imu_spi,
                cs: imu_cs,
            },
        }
    }
}

pub struct StatusLed {
    _row: Output<'static>,
    col: Output<'static>,
}

impl StatusLed {
    pub fn new(row: impl Pin, col: impl Pin) -> Self {
        Self {
            _row: Output::new(row.degrade(), Level::High, OutputDrive::Standard),
            col: Output::new(col.degrade(), Level::High, OutputDrive::Standard),
        }
    }

    pub fn on(&mut self) {
        self.col.set_low();
    }
    pub fn off(&mut self) {
        self.col.set_high();
    }
}

pub struct Motors {
    // Kept alive so its `Drop` (which resets the output pins) never runs. The
    // waveform is generated entirely by EasyDMA looping over `MOTOR_DUTIES`;
    // nothing in this struct is touched per period. Same kept-alive convention
    // as `StatusLed::_row`.
    _pwm: pwm::SequencePwm<'static, peripherals::PWM0>,
}

/// Per-channel PWM compare values in "off-ticks" (0 = full on, `PERIOD_TICKS` =
/// full off), read continuously by EasyDMA. Written by [`Motors::set_all_motors`]
/// and picked up on the next PWM period. Interior mutability is required because
/// the DMA loops over this buffer forever while the CPU updates it in place; see
/// [`Motors::new`].
///
/// The array index is the motor, in the Betaflight-style X layout (props-out
/// rotation). Viewed from above, front between the two forward arms:
///
/// ```text
///           front
///     M4 (FL)     M2 (FR)
///       CCW         CW
///         \        /
///          \      /
///            hub
///          /      \
///         /        \
///     M3 (RL)     M1 (RR)
///       CW          CCW
///           rear
/// ```
///
/// - `[0]` = M1: ch0 / P0.10, rear-right, CCW
/// - `[1]` = M2: ch1 / P0.09, front-right, CW
/// - `[2]` = M3: ch2 / P0.12, rear-left, CW
/// - `[3]` = M4: ch3 / P0.02, front-left, CCW
///
/// Adjacent corners spin opposite, diagonals the same. Rotations are set and
/// confirmed props-off on the bench; the mixer's yaw sign depends on them (see
/// ADR 0023 and the pin map above). The order matches `MotorCommand`'s
/// `motor1..motor4` fields, which [`Motors::set_all_motors`] writes here in turn.
static MOTOR_DUTIES: [AtomicU16; 4] = [
    AtomicU16::new(Motors::PERIOD_TICKS),
    AtomicU16::new(Motors::PERIOD_TICKS),
    AtomicU16::new(Motors::PERIOD_TICKS),
    AtomicU16::new(Motors::PERIOD_TICKS),
];

impl Motors {
    /// ESC frame rate. 400 Hz is the practical ceiling for standard 1-2 ms
    /// servo PWM (2 ms pulse plus a low gap); see the Blueson A1 / AM32 notes.
    const REFRESH_HZ: u32 = 400;

    /// PWM tick rate. `Div16` -> 16 MHz / 16 = 1 MHz, so 1 tick = 1 us.
    const TICK_HZ: u32 = 1_000_000;

    /// PWM period (COUNTERTOP) in ticks: one frame at [`Self::REFRESH_HZ`].
    const PERIOD_TICKS: u16 = (Self::TICK_HZ / Self::REFRESH_HZ) as u16;

    /// ESC pulse endpoints, in ticks == microseconds (holds while 1 tick = 1 us).
    const MIN_PULSE_TICKS: u16 = 1000; // 1 ms - idle / min throttle
    const MAX_PULSE_TICKS: u16 = 2000; // 2 ms - full throttle

    pub fn new(
        peripherals: peripherals::PWM0,
        motor1: impl Pin,
        motor2: impl Pin,
        motor3: impl Pin,
        motor4: impl Pin,
    ) -> Self {
        let mut config = pwm::Config::default();
        config.prescaler = pwm::Prescaler::Div16; // 16 MHz / 16 = 1 MHz (1 tick = 1 us)
        config.max_duty = Self::PERIOD_TICKS; // COUNTERTOP: one frame at REFRESH_HZ
        config.sequence_load = pwm::SequenceLoad::Individual; // 4 words/period, one compare per channel
        config.counter_mode = pwm::CounterMode::Up;

        let mut pwm =
            pwm::SequencePwm::new_4ch(peripherals, motor1, motor2, motor3, motor4, config).unwrap();

        // EasyDMA reads the compare values straight out of `MOTOR_DUTIES`. The
        // driver wants a `&[u16]`; `AtomicU16` is layout-compatible with `u16`,
        // so expose the buffer as a raw slice for the DMA pointer while we keep
        // writing through the atomics. This `&[u16]` is only live until the
        // sequencer is forgotten below; afterwards the only accessors are the
        // hardware DMA pointer and the atomic stores in `set_throttle`, so no
        // live `&[u16]` aliases the memory we mutate.
        // SAFETY: the pointer is valid, aligned and spans exactly `len()` u16s.
        let words: &[u16] = unsafe {
            core::slice::from_raw_parts(MOTOR_DUTIES.as_ptr().cast(), MOTOR_DUTIES.len())
        };

        // Start the sequence looping forever. From here the hardware
        // `loopsdone -> seqstart0` short re-triggers it every period with no CPU
        // involvement, so `set_throttle` never has to wait on the peripheral.
        let sequencer = pwm::SingleSequencer::new(&mut pwm, words, pwm::SequenceConfig::default());
        sequencer.start(pwm::SingleSequenceMode::Infinite).unwrap();

        // The sequencer only borrows `pwm`; drop it without stopping the running
        // sequence so we can move `pwm` into the returned struct.
        core::mem::forget(sequencer);

        Self { _pwm: pwm }
    }

    // API symmetry / failsafe path: drive every channel to idle (full off).
    #[expect(dead_code)]
    pub fn disable(&mut self) {
        for duty in &MOTOR_DUTIES {
            duty.store(Self::PERIOD_TICKS, Ordering::Relaxed);
        }
    }

    fn calc_off_ticks(throttle: Throttle) -> u16 {
        // ESCs inherit the classic RC servo protocol: throttle is encoded in the
        // *width* of the high pulse.
        // 1 ms (MIN_PULSE_TICKS) = idle / min throttle
        // 2 ms (MAX_PULSE_TICKS) = full throttle, and the
        // 1-2 ms band in between maps linearly to 0..1. The pulse width is
        // independent of the frame period (REFRESH_HZ); the rest of each frame is
        // just low dead time the ESC ignores.
        let span = f32::from(Self::MAX_PULSE_TICKS - Self::MIN_PULSE_TICKS);
        let on_ticks = f32::from(Self::MIN_PULSE_TICKS) + throttle.as_normalised() * span;

        // The compare value is "off-ticks": 0 = full on, PERIOD_TICKS = full off.
        Self::PERIOD_TICKS - on_ticks as u16
    }

    pub fn set_all_motors(&mut self, command: MotorCommand) {
        let motor1_off_ticks = Self::calc_off_ticks(command.motor1);
        let motor2_off_ticks = Self::calc_off_ticks(command.motor2);
        let motor3_off_ticks = Self::calc_off_ticks(command.motor3);
        let motor4_off_ticks = Self::calc_off_ticks(command.motor4);

        // There is no simple way to atomically update the buffer the EasyDMA is using
        // The "proper" way to do this would be to use a double-buffered sequence, but that is not supported by the embassy-nrf PWM driver.
        // Instead, we just update the values in the buffer directly. This is safe because the EasyDMA is reading the values in a loop, and the worst that can happen is that one of the motors gets a slightly incorrect value for one PWM period, which is not a big deal.
        MOTOR_DUTIES[0].store(motor1_off_ticks, Ordering::Relaxed);
        MOTOR_DUTIES[1].store(motor2_off_ticks, Ordering::Relaxed);
        MOTOR_DUTIES[2].store(motor3_off_ticks, Ordering::Relaxed);
        MOTOR_DUTIES[3].store(motor4_off_ticks, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum ImuError {
    Spi(spim::Error),
    UnexpectedIdentity { found: u8 },
}

pub struct Imu {
    spi: Spim3,
    cs: Output<'static>,
}

impl Imu {
    const ACCEL_LSB_PER_G: f32 = 2048.0; // ±16g  range
    const GYRO_LSB_PER_DPS: f32 = 16.4; // ±2000 degrees per second range

    pub async fn check_identity(&mut self) -> Result<(), ImuError> {
        self.cs.set_low(); // Drop CS low to select the IMU

        let who_am_i_register = 0x75; // WHO_AM_I register address for ICM-42688-P

        let mut buf = [who_am_i_register | 0x80, 0x00]; // Read command for WHO_AM_I register
        // The first byte is the register address with the read bit set (0x80).
        // The second byte is a dummy byte to clock out the response.

        let result = self.spi.transfer_in_place(&mut buf).await;

        self.cs.set_high();

        match result {
            Ok(_) => {
                match buf[1] {
                    0x47 => Ok(()), // Expect 0x47 for the ICM-42688-P IMU
                    found => Err(ImuError::UnexpectedIdentity { found }), // Unexpected identity value
                }
            }
            Err(e) => Err(ImuError::Spi(e)),
        }
    }

    pub async fn configure(&mut self) -> Result<(), ImuError> {
        self.cs.set_low(); // Drop CS low to select the IMU

        let config = 0x0f; // Enables both sensors in the low noise mode

        let pwm_mgmt0_register = 0x4E; // PWM_MGMT0 register address for ICM-42688-P
        let mut buf = [pwm_mgmt0_register, config]; // Write command for setting the PWM_MGMT0 register
        // The first byte is the register address with the read bit cleared
        // The second byte is the data we will write

        let result = self.spi.transfer_in_place(&mut buf).await;

        self.cs.set_high();

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(ImuError::Spi(e)),
        }
    }

    fn convert_bytes(data: &[u8]) -> ImuData {
        let acceleration_x = i16::from_be_bytes([data[0], data[1]]);
        let acceleration_y = i16::from_be_bytes([data[2], data[3]]);
        let acceleration_z = i16::from_be_bytes([data[4], data[5]]);

        let gyro_x = i16::from_be_bytes([data[6], data[7]]);
        let gyro_y = i16::from_be_bytes([data[8], data[9]]);
        let gyro_z = i16::from_be_bytes([data[10], data[11]]);

        ImuData {
            acceleration_x: Acceleration::from_g(acceleration_x as f32 / Self::ACCEL_LSB_PER_G),
            acceleration_y: Acceleration::from_g(acceleration_y as f32 / Self::ACCEL_LSB_PER_G),
            acceleration_z: Acceleration::from_g(acceleration_z as f32 / Self::ACCEL_LSB_PER_G),
            angular_rate_x: AngularRate::from_degrees_per_second(
                gyro_x as f32 / Self::GYRO_LSB_PER_DPS,
            ),
            angular_rate_y: AngularRate::from_degrees_per_second(
                gyro_y as f32 / Self::GYRO_LSB_PER_DPS,
            ),
            angular_rate_z: AngularRate::from_degrees_per_second(
                gyro_z as f32 / Self::GYRO_LSB_PER_DPS,
            ),
        }
    }

    pub async fn read_all(&mut self) -> Result<ImuData, ImuError> {
        self.cs.set_low(); // Drop CS low to select the IMU

        let accel_data_x1_register = 0x1F; // ACCEL_DATA_X1 register address for ICM-42688-P

        let mut buf = [0u8; 13]; // Buffer to hold the read command and the 12 bytes of response
        //If we do a continuous read starting from ACCEL_DATA_X1, we will get 12 bytes of data: 6 bytes for accelerometer (X, Y, Z) and 6 bytes for gyroscope (X, Y, Z)

        buf[0] = accel_data_x1_register | 0x80; // Read command for ACCEL_DATA_X1 register

        let result = self.spi.transfer_in_place(&mut buf).await;

        self.cs.set_high();

        match result {
            Ok(_) => Ok(Self::convert_bytes(&buf[1..13])),
            Err(e) => Err(ImuError::Spi(e)),
        }
    }
}
