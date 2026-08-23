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

use crate::signals::esc_telemetry_sample;
use embassy_nrf::config::{Config, HfclkSource};
use embassy_nrf::gpio::{Level, Output, OutputDrive, Pin};
use embassy_nrf::pwm;
use embassy_nrf::spim::{self, Spim};
use embassy_nrf::uarte::{self, Baudrate, Parity, UarteRx, UarteRxWithIdle};
use embassy_nrf::{bind_interrupts, peripherals, radio, temp};
use firmware_types::{Acceleration, AngularRate, ImuData, MotorCommand, MotorID, ThrottleCommand};

pub const NAME: &str = "BBC micro:bit v2";

/// BSP-typed alias for the embassy IEEE 802.15.4 radio driver bound to this board.
pub type Radio = radio::ieee802154::Radio<'static, peripherals::RADIO>;

pub type TemperatureSensor = temp::Temp<'static>;
pub type Spim3 = spim::Spim<'static, peripherals::SPI3>;
pub type EscTelemetryRx = UarteRxWithIdle<'static, peripherals::UARTE1, peripherals::TIMER1>;

bind_interrupts!(struct Irqs {
    RADIO => radio::InterruptHandler<peripherals::RADIO>;
    TEMP => temp::InterruptHandler;
    SPIM3 => spim::InterruptHandler<peripherals::SPI3>;
    UARTE1 => uarte::InterruptHandler<peripherals::UARTE1>;
});

pub struct Board {
    pub status_led: StatusLed,
    pub motors: Motors,
    pub radio: Radio,
    pub temperature_sensor: TemperatureSensor,
    pub imu: Imu,
    pub esc_telemetry: ESCTelemetry,
}

// Pin map — BBC micro:bit v2 / nRF52833
//
// | Function       | nRF pin | Edge | Peripheral | Notes               |
// |----------------|---------|------|------------|---------------------|
// | Motor M1       | P0.02   | 0    | PWM0 ch0   | rear-right (CCW), large pad    |
// | Motor M2       | P0.12   | 12   | PWM0 ch1   | front-right (CW)               |
// | Motor M3       | P0.09   | 9    | PWM0 ch2   | rear-left (CW), NFC1 -> GPIO   |
// | Motor M4       | P0.10   | 8    | PWM0 ch3   | front-left (CCW), NFC2 -> GPIO |
// | IMU SCK        | P0.17   | 13   | SPIM3      | SPI_EXT block       |
// | IMU MISO       | P0.01   | 14   | SPIM3      |                     |
// | IMU MOSI       | P0.13   | 15   | SPIM3      |                     |
// | IMU CS         | P1.02   | 16   | GPIO out   | manual, idle high   |
// | ESC telem RX   | P0.26   | 19   | UARTE      | 115200 8N1, RX-only, I2C_EXT SCL pad |
// | Status LED row | P0.21   | -    | GPIO out   | onboard 5x5 matrix  |
// | Status LED col | P0.28   | -    | GPIO out   | onboard 5x5 matrix  |
// | Radio          | -       | -    | RADIO      | internal, 802.15.4  |
// | Temp sensor    | -       | -    | TEMP       | internal            |
//
// Motors share PWM0 (one frequency, independent duty). The NFC pins
// (P0.09 / P0.10) require the `nfc-pins-as-gpio` embassy-nrf feature.
//
// ===========================================================================
// MOTOR WIRING -- bench-verified 2026-07-19. DO NOT reorder without re-testing.
// ===========================================================================
// The pin -> corner mapping above is NOT the "natural" ch0..ch3 order. On this
// airframe the four ESC signal leads landed reversed, so each PWM channel
// physically drives its DIAGONAL-OPPOSITE corner relative to a naive
// ch0=M1 .. ch3=M4 wiring. Determined empirically by the single-motor bench
// test (drive one `MOTOR_DUTIES` index, note which corner spins), giving the
// physical pin -> corner map:
//
//     P0.02 -> rear-right    P0.12 -> front-right
//     P0.09 -> rear-left     P0.10 -> front-left
//
// To make logical M1..M4 reach the correct corners, `Motors::new` is handed
// the pins in REVERSED order (P0.02, P0.12, P0.09, P0.10) -- see `Board::new`.
// The drone is tightly packed and the wiring cannot easily be re-inspected, so
// this bench-determined mapping is the authoritative record. If the constructor
// is ever "tidied" back to natural pin order, roll and pitch invert silently
// (yaw still looks fine -- diagonals share spin direction). Re-run the
// single-motor test before trusting any change here.
// ===========================================================================

impl Board {
    pub fn new() -> Self {
        let mut config = Config::default();
        config.hfclk_source = HfclkSource::ExternalXtal;
        let p = embassy_nrf::init(config);

        Self {
            status_led: StatusLed::new(p.P0_21, p.P0_28),
            motors: Motors::new(p.PWM0, p.P0_02, p.P0_12, p.P0_09, p.P0_10),
            radio: Radio::new(p.RADIO, Irqs),
            temperature_sensor: TemperatureSensor::new(p.TEMP, Irqs),
            imu: Imu::new(p.SPI3, p.P0_17, p.P0_01, p.P0_13, p.P1_02),
            esc_telemetry: ESCTelemetry::new(p.UARTE1, p.P0_26, p.TIMER1, p.PPI_CH0, p.PPI_CH1),
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

pub struct ESCTelemetry {
    esc_telemetry_rx: EscTelemetryRx,
}

impl ESCTelemetry {
    pub fn new(
        uarte: peripherals::UARTE1,
        pin: impl Pin,
        timer: peripherals::TIMER1,
        ppi_ch1: peripherals::PPI_CH0,
        ppi_ch2: peripherals::PPI_CH1,
    ) -> Self {
        let mut esc_telemetry_config = embassy_nrf::uarte::Config::default();
        esc_telemetry_config.baudrate = Baudrate::BAUD115200;
        esc_telemetry_config.parity = Parity::EXCLUDED;

        let rx = UarteRx::new(uarte, Irqs, pin.degrade(), esc_telemetry_config);
        // Idle-line framing: the timer + PPI stop RX on the inter-frame gap, so
        // each read re-anchors to a real frame boundary (self-healing sync).
        let esc_telemetry_rx = rx.with_idle(timer, ppi_ch1, ppi_ch2);

        Self { esc_telemetry_rx }
    }

    /// Read one frame's worth of bytes, returning at the idle gap. Returns the
    /// byte count (10 for a full frame; fewer means a partial/mid-frame read).
    pub async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, uarte::Error> {
        self.esc_telemetry_rx.read_until_idle(buffer).await
    }
}

/// DShot frame construction lives in the host-tested core crate.
use firmware_drone_core::dshot;

/// DShot300 motor driver.
///
/// Drives PWM0 + EasyDMA with a *digital DShot300* bitstream: each PWM period is
/// one DShot bit, the 16-bit frame is followed by a low inter-frame gap, and the
/// whole loop repeats forever so the ESC sees a continuous frame stream (a DShot
/// ESC disarms if the signal stops). Throttle updates rewrite the frame words in
/// place; a torn frame simply fails its CRC at the ESC and is dropped, so no
/// double-buffering is needed.
///
/// The looped buffer uses an "off-ticks" word convention (`word = BIT_TICKS -
/// high_ticks`), giving each bit a HIGH pulse of `high_ticks` at the START of the
/// bit period (a rising edge at each bit boundary). If a scope ever shows it
/// inverted (high at the end of the bit), flip the one line in [`Motors::word`].
pub struct Motors {
    // Kept alive so its `Drop` never runs; the waveform is generated entirely by
    // EasyDMA looping over `DSHOT_WORDS`. Same kept-alive convention as
    // `StatusLed::_row`.
    _pwm: pwm::SequencePwm<'static, peripherals::PWM0>,
}

/// Looped DShot sequence: `FRAME_BITS` bit-periods then `GAP_PERIODS` low
/// periods, four interleaved channel words per period (Individual load). Written
/// in place by [`Motors::set_all_motors`], read continuously by EasyDMA. Init to
/// the all-low word; [`Motors::new`] then lays STOP frames over the bit region.
///
/// The four channels map to the motors in the Betaflight-style X layout
/// (props-out rotation). Viewed from above, front between the two forward arms:
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
/// - channel 0 = M1: P0.02, rear-right, CCW
/// - channel 1 = M2: P0.12, front-right, CW
/// - channel 2 = M3: P0.09, rear-left, CW
/// - channel 3 = M4: P0.10, front-left, CCW
///
/// NB: the pin order looks reversed on purpose -- see the "MOTOR WIRING" note
/// in the pin map at the top of this module.
///
/// Adjacent corners spin opposite, diagonals the same. Rotations are set and
/// confirmed props-off on the bench; the mixer's yaw sign depends on them (see
/// ADR 0023 and the pin map above). The order matches `MotorCommand`'s
/// `motor1..motor4` fields, which [`Motors::set_all_motors`] writes here in turn.
static DSHOT_WORDS: [AtomicU16; Motors::SEQ_LEN] =
    [const { AtomicU16::new(Motors::BIT_TICKS) }; Motors::SEQ_LEN];

impl Motors {
    /// PWM clock: Div1 -> 16 MHz (1 tick = 62.5 ns).
    const CLOCK_HZ: u32 = 16_000_000;
    /// DShot300 bit rate.
    const DSHOT_HZ: u32 = 300_000;
    /// One DShot bit period, in PWM ticks (COUNTERTOP). 16 MHz / 300 kHz ~= 53.
    const BIT_TICKS: u16 = (Self::CLOCK_HZ / Self::DSHOT_HZ) as u16;
    /// High-time of a `1` bit: ~75% of the bit period.
    const ONE_HIGH: u16 = 40; // 0.75 * BIT_TICKS (53)
    /// High-time of a `0` bit: ~37.5% of the bit period.
    const ZERO_HIGH: u16 = 20; // 0.375 * BIT_TICKS (53)
    /// Bits per DShot frame.
    const FRAME_BITS: usize = 16;
    /// Low periods after each frame: gives the ESC an inter-frame gap and sets
    /// the frame rate (one loop = `BIT_TICKS * (FRAME_BITS + GAP_PERIODS)` ticks,
    /// ~4.7 kHz here). Tune if an ESC dislikes the rate.
    const GAP_PERIODS: usize = 48;
    /// Total PWM periods per loop.
    const PERIODS: usize = Self::FRAME_BITS + Self::GAP_PERIODS;
    /// Sequence length: four channel words per period (Individual load).
    const SEQ_LEN: usize = Self::PERIODS * 4;

    pub fn new(
        peripherals: peripherals::PWM0,
        motor1: impl Pin,
        motor2: impl Pin,
        motor3: impl Pin,
        motor4: impl Pin,
    ) -> Self {
        let mut config = pwm::Config::default();
        config.prescaler = pwm::Prescaler::Div1; // 16 MHz: 1 tick = 62.5 ns
        config.max_duty = Self::BIT_TICKS; // COUNTERTOP: one DShot bit period
        config.sequence_load = pwm::SequenceLoad::Individual; // 4 words/period
        config.counter_mode = pwm::CounterMode::Up;

        let mut pwm =
            pwm::SequencePwm::new_4ch(peripherals, motor1, motor2, motor3, motor4, config).unwrap();

        // Lay STOP frames over the bit region so the ESC sees valid DShot from
        // boot; the gap periods keep their low init value.
        for channel in 0..4 {
            Self::write_frame(channel, dshot::STOP);
        }

        // SAFETY: as in the PWM driver -- `AtomicU16` is layout-compatible with
        // `u16`; this `&[u16]` is only live until the sequencer is forgotten,
        // after which the sole accessors are the DMA pointer and the atomic
        // stores in `write_frame`, so no live `&[u16]` aliases the mutated memory.
        let words: &[u16] =
            unsafe { core::slice::from_raw_parts(DSHOT_WORDS.as_ptr().cast(), Self::SEQ_LEN) };

        // Loop the frame+gap forever; the hardware `loopsdone -> seqstart0` short
        // re-triggers with no CPU involvement, so the ESC gets a continuous
        // stream and `set_all_motors` never waits on the peripheral.
        let sequencer = pwm::SingleSequencer::new(&mut pwm, words, pwm::SequenceConfig::default());
        sequencer.start(pwm::SingleSequenceMode::Infinite).unwrap();
        core::mem::forget(sequencer);

        Self { _pwm: pwm }
    }

    // API symmetry / failsafe path: drive every motor to STOP (continuous STOP
    // frames -> motors off).
    #[expect(dead_code)]
    pub fn disable(&mut self) {
        for channel in 0..4 {
            Self::write_frame(channel, dshot::STOP);
        }
    }

    pub fn set_all_motors(&mut self, command: MotorCommand) {
        let req = esc_telemetry_sample::take_request();

        Self::write_frame(
            0,
            Self::frame_for(command.motor1, req == Some(MotorID::Motor1)),
        );
        Self::write_frame(
            1,
            Self::frame_for(command.motor2, req == Some(MotorID::Motor2)),
        );
        Self::write_frame(
            2,
            Self::frame_for(command.motor3, req == Some(MotorID::Motor3)),
        );
        Self::write_frame(
            3,
            Self::frame_for(command.motor4, req == Some(MotorID::Motor4)),
        );
    }

    /// Zero throttle maps to STOP (motor off), preserving the invariant that a
    /// zero command never spins a motor; any positive throttle maps into the
    /// 48..2047 DShot throttle band.
    fn frame_for(throttle: ThrottleCommand, request_telemetry: bool) -> u16 {
        let normalised = throttle.as_normalised();
        if normalised <= 0.0 {
            dshot::frame(dshot::STOP, request_telemetry) // stop the motor, but still request telem
        } else {
            dshot::throttle_frame(normalised, request_telemetry)
        }
    }

    /// PWM compare word for a bit's high-time, in the PWM driver's "off-ticks"
    /// convention. Single point to flip if a scope shows inverted polarity.
    fn word(high_ticks: u16) -> u16 {
        Self::BIT_TICKS - high_ticks
    }

    /// Write the 16 bit-words of `frame` for `channel` into the looped buffer,
    /// MSB first. Leaves the inter-frame gap periods untouched.
    fn write_frame(channel: usize, frame: u16) {
        for bit in 0..Self::FRAME_BITS {
            let is_one = ((frame >> (15 - bit)) & 1) == 1;
            let high = if is_one {
                Self::ONE_HIGH
            } else {
                Self::ZERO_HIGH
            };
            DSHOT_WORDS[bit * 4 + channel].store(Self::word(high), Ordering::Relaxed);
        }
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

    pub fn new(
        spi: peripherals::SPI3,
        sck: impl Pin,
        miso: impl Pin,
        mosi: impl Pin,
        cs: impl Pin,
    ) -> Self {
        let mut spi_config = spim::Config::default();
        spi_config.frequency = spim::Frequency::M1;
        spi_config.mode = spim::MODE_0;

        let spi = Spim::new(spi, Irqs, sck, miso, mosi, spi_config);
        let cs = Output::new(cs.degrade(), Level::High, OutputDrive::Standard);

        Self { spi, cs }
    }

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
