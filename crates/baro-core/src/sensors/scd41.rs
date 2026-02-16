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
        // Enable automatic self-calibration (ASC)
        // ASC continuously calibrates the sensor over time (requires 7 days of operation)
        self.sensor
            .set_automatic_self_calibration(true)
            .await
            .map_err(|e| {
                error!("SCD41 set_automatic_self_calibration failed: {:?}", e);
                SensorError::InitializationFailed {
                    sensor: "SCD41",
                    details: "Failed to enable automatic self-calibration",
                }
            })?;

        info!("SCD41: Automatic self-calibration enabled");

        self.calibrated = true;

        Ok(())
    }
}

// Implementation for actual I2c devices
impl<I: I2c> Sensor<1> for SCD41Sensor<I> {
    type Readings = SCD41Readings;

    async fn read(&mut self) -> Result<SCD41Readings, super::SensorError> {
        // Initialize sensor on first read
        if !self.calibrated {
            // Need to initialize before reading
            self.initialize().await.map_err(|e| {
                error!("SCD41 initialization failed: {:?}", e);
                SensorError::InitializationFailed {
                    sensor: "SCD41",
                    details: "Failed to initialize sensor before reading",
                }
            })?;
        }

        self.sensor.measure_single_shot().await.map_err(|e| {
            error!("SCD41 single shot measurement failed: {:?}", e);
            SensorError::ReadFailed {
                sensor: "SCD41",
                operation: "initiate single shot measurement",
                details: "I2C communication error",
            }
        })?;

        // Wait for 5s to allow measurement to complete
        embassy_time::Timer::after_millis(CO2_MEASUREMENT_INTERVAL_MS as u64).await;

        // While the sensor data is not ready, continue waiting for it, max of 5 times.
        // If we exceed this, return a timeout error.
        let mut attempts = 0;
        while (!self.sensor.data_ready().await.map_err(|e| {
            error!("SCD41 data_ready check failed: {:?}", e);
            SensorError::ReadFailed {
                sensor: "SCD41",
                operation: "check data ready status",
                details: "I2C communication error",
            }
        })?) && attempts < 5
        {
            embassy_time::Timer::after_millis(1000).await;
            attempts += 1;
        }

        if attempts >= 5 {
            error!("SCD41 data not ready after multiple attempts");
            return Err(SensorError::Timeout {
                sensor: "SCD41",
                operation: "wait for data ready status",
            });
        }

        // Read measurement
        let measurement = self.sensor.measurement().await.map_err(|e| {
            error!("SCD41 measurement read failed: {:?}", e);
            SensorError::ReadFailed {
                sensor: "SCD41",
                operation: "read CO2 measurement",
                details: "I2C communication error or invalid data",
            }
        })?;

        let co2_ppm = measurement.co2_ppm as i32;

        Ok(SCD41Readings { co2_ppm })
    }
}
