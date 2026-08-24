use crate::signals::{
    attitude, control_mode_update, control_system_parameter_update, controller_demand, cpu_load,
    esc_telemetry_sample, imu_data, motor_command, pilot_command, sensors, status, telemetry,
};
use embassy_time::{Duration, Ticker};
use firmware_types::{ESCTelemetry, MotorID, TelemetryFrame, TelemetryFrameHighRate};

/// One low-rate frame every N aggregator cycles; 10 cycles at the 100 Hz cycle gives ~10 Hz.
const CYCLES_PER_LOW_RATE_FRAME: u32 = 10;

#[embassy_executor::task]
pub async fn telemetry_aggregator() -> ! {
    defmt::info!("telemetry aggregator task: started");

    let mut sequence_count: u32 = 0;

    let mut status_receiver = status::subscribe();
    let mut sensors_receiver = sensors::subscribe();
    let mut pilot_command_receiver = pilot_command::subscribe();
    let mut cpu_load_receiver = cpu_load::subscribe();
    let mut attitude_receiver = attitude::subscribe();
    let mut controller_demand_receiver = controller_demand::subscribe();
    let mut motor_command_receiver = motor_command::subscribe();
    let mut imu_processed_receiver = imu_data::subscribe_processed();
    let mut control_mode_receiver = control_mode_update::subscribe();
    let mut esc_telemetry_sample_receiver = esc_telemetry_sample::subscribe();
    let mut control_system_parameter_receiver = control_system_parameter_update::subscribe();

    let mut ticker = Ticker::every(Duration::from_millis(10));

    let mut esc_telemetry = ESCTelemetry::default();

    loop {
        ticker.next().await;

        sequence_count = sequence_count.wrapping_add(1);

        let drone_state = status_receiver.get().await;
        let sensors = sensors_receiver.get().await;
        let pilot_command = pilot_command_receiver.get().await;
        let cpu_load = cpu_load_receiver.get().await;
        let attitude = attitude_receiver.get().await;
        let controller_demand = controller_demand_receiver.get().await;
        let motor_command = motor_command_receiver.get().await;
        let imu_processed = imu_processed_receiver.get().await;
        let control_mode = control_mode_receiver.get().await;
        let esc_telemetry_sample = esc_telemetry_sample_receiver.get().await;
        let control_system_parameters = control_system_parameter_receiver.get().await;

        esc_telemetry.temperature = esc_telemetry_sample.temperature;
        esc_telemetry.battery_state = esc_telemetry_sample.battery_state;
        match esc_telemetry_sample.motor_id {
            MotorID::Motor1 => esc_telemetry.motor1_rpm = esc_telemetry_sample.motor_rpm,
            MotorID::Motor2 => esc_telemetry.motor2_rpm = esc_telemetry_sample.motor_rpm,
            MotorID::Motor3 => esc_telemetry.motor3_rpm = esc_telemetry_sample.motor_rpm,
            MotorID::Motor4 => esc_telemetry.motor4_rpm = esc_telemetry_sample.motor_rpm,
            MotorID::None => {}
        }

        let telemetry_frame_high_rate = TelemetryFrameHighRate {
            sequence_number: sequence_count,
            imu: sensors.imu,
            filtered_angular_rate_x: imu_processed.angular_rate_x,
            filtered_angular_rate_y: imu_processed.angular_rate_y,
            filtered_angular_rate_z: imu_processed.angular_rate_z,
            pilot_command,
            attitude,
            controller_demand,
            motor_command,
            drone_state,
        };

        let telemetry_frame_low_rate = firmware_types::TelemetryFrameLowRate {
            sequence_number: sequence_count,
            cpu_load,
            control_mode,
            temperature: sensors.temperature,
            esc_telemetry,
            control_parameters: control_system_parameters,
        };

        let telemetry_frame_high_rate = TelemetryFrame::HighRate(telemetry_frame_high_rate);
        let telemetry_frame_low_rate = TelemetryFrame::LowRate(telemetry_frame_low_rate);

        if sequence_count % CYCLES_PER_LOW_RATE_FRAME == 0 {
            telemetry::set(telemetry_frame_low_rate);
        } else {
            telemetry::set(telemetry_frame_high_rate);
        }
    }
}
