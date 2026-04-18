use crate::sensors::{I2cFault, SensorError, SensorReadings};

use super::Sensor;
use embedded_hal_async::i2c::I2c;
use sht4x::{Error as Sht4xError, Sht4xAsync};

/// Typed readings from the SHT40 sensor.
/// This provides named access to sensor values and ensures type safety.
pub struct SHT40Readings {
    pub temperature_milli_celsius: i32,
    pub humidity_milli_percent: i32,
}

impl SensorReadings<2> for SHT40Readings {
    fn to_array(self) -> [i32; 2] {
        [self.temperature_milli_celsius, self.humidity_milli_percent]
    }
}

pub struct SHT40Sensor<I> {
    sensor: Sht4xAsync<I, embassy_time::Delay>,
}

impl<I: I2c> SHT40Sensor<I> {
    pub fn new(i2c: I) -> Self {
        Self {
            sensor: Sht4xAsync::<I, embassy_time::Delay>::new(i2c),
        }
    }
}

// Implementation for actual I2c devices
impl<I: I2c> Sensor<2> for SHT40Sensor<I> {
    type Readings = SHT40Readings;

    async fn read(&mut self) -> Result<SHT40Readings, super::SensorError> {
        let measurement = self
            .sensor
            .measure(sht4x::Precision::High, &mut embassy_time::Delay)
            .await
            .map_err(|e| {
                log::error!("SHT40 measurement failed: {:?}", e);
                // `sht4x::Error` does not implement `embedded_hal::i2c::Error`,
                // so classify manually — I2c variant carries the HAL error, Crc
                // is a data-integrity failure (not an I2C bus fault).
                let fault = match &e {
                    Sht4xError::I2c(inner) => I2cFault::from_err(inner),
                    Sht4xError::Crc => I2cFault::Other,
                    _ => I2cFault::Other,
                };
                SensorError::Read {
                    sensor: "SHT40",
                    op: "measure",
                    fault,
                }
            })?;

        let temperature_milli_celsius =
            (measurement.temperature_celsius().to_num::<f32>() * 1000.0) as i32;
        let humidity_milli_percent =
            (measurement.humidity_percent().to_num::<f32>() * 1000.0) as i32;

        Ok(SHT40Readings {
            temperature_milli_celsius,
            humidity_milli_percent,
        })
    }
}
