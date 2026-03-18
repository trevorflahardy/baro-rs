use crate::sensors::{SensorError, SensorReadings};

use super::Sensor;
use bmp388_embedded::r#async::Bmp388Async;
use bmp388_embedded::{Address, Measurement, Oversampling};
use embedded_hal_async::i2c::I2c;
use log::{debug, error};

pub struct BMP388Readings {
    /// Pressure in milli-Pascals (Pa × 1000) for i32 storage precision
    pub milli_pa: i32,
}

impl SensorReadings<1> for BMP388Readings {
    fn to_array(self) -> [i32; 1] {
        [self.milli_pa]
    }
}

/// BMP388 sensor wrapper.
///
/// Unlike other sensors, `Bmp388Async::new()` is async and fallible, so the
/// driver is constructed and configured on each `read()` call. This matches
/// the firmware pattern where a fresh `BMP388Sensor` is created per read
/// cycle (the I2C mux channel is re-selected each time).
pub struct BMP388Sensor<I> {
    i2c: Option<I>,
}

impl<I: I2c> BMP388Sensor<I> {
    pub fn new(i2c: I) -> Self {
        Self { i2c: Some(i2c) }
    }
}

impl<I: I2c> Sensor<1> for BMP388Sensor<I> {
    type Readings = BMP388Readings;

    async fn read(&mut self) -> Result<BMP388Readings, SensorError> {
        let i2c = self.i2c.take().ok_or(SensorError::ReadFailed {
            sensor: "BMP388",
            operation: "read",
            details: "I2C device already consumed; create a new BMP388Sensor per read cycle",
        })?;

        let mut sensor = Bmp388Async::new(i2c, embassy_time::Delay, Address::Primary)
            .await
            .map_err(|e| {
                error!("BMP388 initialization failed: {:?}", e);
                SensorError::InitializationFailed {
                    sensor: "BMP388",
                    details: "Failed to initialize BMP388 async driver",
                }
            })?;

        // Configure oversampling: X8 for pressure, X2 for temperature
        sensor
            .set_oversampling(Oversampling::X8, Oversampling::X2)
            .await
            .map_err(|e| {
                error!("BMP388 set_oversampling failed: {:?}", e);
                SensorError::InitializationFailed {
                    sensor: "BMP388",
                    details: "Failed to set oversampling configuration",
                }
            })?;

        sensor
            .forced_measurement()
            .await
            .map(|data: Measurement| {
                // The BMP388 returns pressure in Pascals as f64.
                // We store as milli-Pascals (Pa × 1000) in i32 for precision.
                let milli_pa = (data.pressure * 1000.0) as i32;
                debug!(
                    "BMP388: Measured pressure = {:.2} Pa (stored as {} mPa)",
                    data.pressure, milli_pa
                );

                BMP388Readings { milli_pa }
            })
            .map_err(|e| {
                error!("BMP388 forced_measurement failed: {:?}", e);
                SensorError::ReadFailed {
                    sensor: "BMP388",
                    operation: "forced_measurement",
                    details: "Failed to read pressure value during forced measurement",
                }
            })
    }
}
