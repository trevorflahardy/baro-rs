use crate::sensors::{SensorError, SensorReadings};

use super::Sensor;
use embedded_hal_async::i2c::I2c;
use log::{error, info};
use scd41_embedded::r#async::Scd41Async;

const CO2_MEASUREMENT_INTERVAL_MS: u32 = 5000;

/// Typed readings from the SCD41 sensor.
/// This provides named access to sensor values and ensures type safety.
pub struct SCD41Readings {
    pub co2_ppm: i32,
}

impl SensorReadings<1> for SCD41Readings {
    fn to_array(self) -> [i32; 1] {
        [self.co2_ppm]
    }
}

pub struct SCD41Sensor<I> {
    sensor: Scd41Async<I, embassy_time::Delay>,
    calibrated: bool,
}

impl<I: I2c> SCD41Sensor<I> {
    pub fn new(i2c: I) -> Self {
        Self {
            sensor: Scd41Async::<I, embassy_time::Delay>::new(i2c, embassy_time::Delay),
            calibrated: false,
        }
    }

    /// Perform calibration and start periodic measurement.
    /// This should be called once during initialization.
    async fn initialize(&mut self) -> Result<(), SensorError> {
        // Stop any ongoing measurement first
        if let Err(_e) = self.sensor.stop_periodic_measurement().await {
            // Ignore error if measurement wasn't running
        }

        // Enable automatic self-calibration (ASC)
        // ASC continuously calibrates the sensor over time (requires 7 days of operation)
        self.sensor
            .set_automatic_self_calibration(true)
            .await
            .map_err(|_| SensorError::ReadError)?;

        info!("SCD41: Automatic self-calibration enabled");

        // Start periodic measurement
        self.sensor
            .start_periodic_measurement()
            .await
            .map_err(|_| SensorError::ReadError)?;

        self.calibrated = true;
        info!("SCD41: Periodic measurement started");

        Ok(())
    }
}

// Implementation for actual I2c devices
impl<I: I2c> Sensor<1> for SCD41Sensor<I> {
    type Readings = SCD41Readings;

    async fn read(&mut self) -> Result<SCD41Readings, super::SensorError> {
        // Initialize sensor on first read
        if !self.calibrated {
            if let Err(e) = self.initialize().await {
                error!("SCD41 initialization failed: {:?}", e);
                return Err(e);
            }

            // Wait for first measurement to be ready (5 seconds)
            embassy_time::Timer::after_millis(CO2_MEASUREMENT_INTERVAL_MS as u64).await;
        }

        // Check if data is ready
        let ready = self
            .sensor
            .data_ready()
            .await
            .map_err(|_| SensorError::ReadError)?;

        if !ready {
            // Data not ready yet, this is expected during startup
            return Err(SensorError::ReadError);
        }

        // Read measurement
        let measurement = self
            .sensor
            .measurement()
            .await
            .map_err(|_| SensorError::ReadError)?;

        let co2_ppm = measurement.co2_ppm as i32;

        Ok(SCD41Readings { co2_ppm })
    }
}
