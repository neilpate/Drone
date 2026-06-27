//! BBC micro:bit v2 (nRF52833) board support.
//!
//! Phase 1–3 development target. Sensors and actuators are breadboarded onto
//! the edge connector; only the on-board 5x5 LED matrix is used directly.
//!
//! See [ADR 0010](../../../../doc/decisions/0010-board-support-package.md) for
//! the contract this module satisfies.
//!
//!

use embassy_nrf::config::{Config, HfclkSource};
use embassy_nrf::gpio::{Level, Output, OutputDrive, Pin};
use embassy_nrf::pwm::SimplePwm;
use embassy_nrf::spim::{self, Spim};
use embassy_nrf::{bind_interrupts, peripherals, radio, temp};
use firmware_types::{Acceleration, AngularRate, ImuData, Throttle};

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
// | Motor M0       | P0.10   | 8    | PWM0 ch0   | NFC2 -> GPIO        |
// | Motor M1       | P0.09   | 9    | PWM0 ch1   | NFC1 -> GPIO        |
// | Motor M2       | P0.12   | 12   | PWM0 ch2   | plain GPIO          |
// | Motor M3       | P0.02   | 0    | PWM0 ch3   | large pad / ADC     |
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
    motors: SimplePwm<'static, peripherals::PWM0>,
}

pub enum Motor {
    Motor0,
    Motor1,
    Motor2,
    Motor3,
}

impl Motors {
    const MAX_DUTY: u16 = 1000;

    pub fn new(
        peripherals: peripherals::PWM0,
        motor0: impl Pin,
        motor1: impl Pin,
        motor2: impl Pin,
        motor3: impl Pin,
    ) -> Self {
        let mut motors = SimplePwm::new_4ch(peripherals, motor0, motor1, motor2, motor3);
        motors.set_prescaler(embassy_nrf::pwm::Prescaler::Div1);
        motors.set_max_duty(Self::MAX_DUTY);

        for motor in [Motor::Motor0, Motor::Motor1, Motor::Motor2, Motor::Motor3] {
            let channel = Self::motor_to_channel(motor);
            motors.set_duty(channel, Self::MAX_DUTY); //Default to off (100% duty means always off, 0% duty means always on)
        }

        Self { motors }
    }

    pub fn enable(&mut self) {
        self.motors.enable();
    }

    // API symmetry with enable(); will be used by the failsafe path.
    #[expect(dead_code)]
    pub fn disable(&mut self) {
        self.motors.disable();
    }

    fn motor_to_channel(motor: Motor) -> usize {
        match motor {
            Motor::Motor0 => 0,
            Motor::Motor1 => 1,
            Motor::Motor2 => 2,
            Motor::Motor3 => 3,
        }
    }

    pub fn set_throttle(&mut self, motor: Motor, throttle: Throttle) {
        let on_ticks = (throttle.as_normalised() * f32::from(Self::MAX_DUTY)) as u16;

        // Note, duty is a bit counterintuitive: 0 is full on, max_duty is full off.
        // Duty means off ticks per period
        let duty = Self::MAX_DUTY - on_ticks; //Invert to get duty (off ticks)

        let channel = Self::motor_to_channel(motor);
        self.motors.set_duty(channel, duty);
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
