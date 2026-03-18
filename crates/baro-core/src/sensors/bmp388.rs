use crate::sensors::{SensorError, SensorReadings};

use super::Sensor;
use bmp388_embedded::r#async::Bmp388Async;
use bmp388_embedded::{Address, Measurement, Oversampling};
use embedded_hal_async::i2c::I2c;
use log::{error, info};

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
/// Unlike other sensors, `Bmp388Async::new()` is async and fallible, so we
/// store the raw I2C device and lazily construct the driver on the first read.
pub struct BMP388Sensor<I> {
    inner: BMP388Inner<I>,
}

enum BMP388Inner<I> {
    /// Not yet initialized — holds the I2C device until first read.
    Uninit(Option<I>),
    /// Initialized driver ready for measurements.
    Ready(Bmp388Async<I, embassy_time::Delay>),
}

impl<I: I2c> BMP388Sensor<I> {
    pub fn new(i2c: I) -> Self {
        Self {
            inner: BMP388Inner::Uninit(Some(i2c)),
        }
    }

    /// Ensure the driver is initialized, returning a mutable reference to it.
    async fn driver(&mut self) -> Result<&mut Bmp388Async<I, embassy_time::Delay>, SensorError> {
        if let BMP388Inner::Uninit(ref mut i2c_opt) = self.inner {
            let i2c = i2c_opt.take().ok_or(SensorError::InitializationFailed {
                sensor: "BMP388",
                details: "I2C device already consumed during init",
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

            // Configure oversampling once: X8 for pressure, X2 for temperature
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

            self.inner = BMP388Inner::Ready(sensor);
        }

        match self.inner {
            BMP388Inner::Ready(ref mut sensor) => Ok(sensor),
            BMP388Inner::Uninit(_) => unreachable!(),
        }
    }
}

impl<I: I2c> Sensor<1> for BMP388Sensor<I> {
    type Readings = BMP388Readings;

    async fn read(&mut self) -> Result<BMP388Readings, SensorError> {
        let sensor = self.driver().await?;

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
