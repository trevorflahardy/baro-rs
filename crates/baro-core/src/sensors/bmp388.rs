extern crate alloc;

use crate::sensors::{SensorError, SensorReadings};

use super::Sensor;
use bmp388_embedded::r#async::Bmp388Async;
use bmp388_embedded::{Address, Measurement, Oversampling};
use embedded_hal_async::i2c::I2c;
use log::{debug, error, warn};

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
                    cause: alloc::format!("{:?}", e),
                }
            })?;

        // Post-init diagnostics: chip ID, error register. Non-fatal — log only.
        match sensor.chip_id().await {
            Ok(id) => debug!("BMP388: chip_id = 0x{:02X} (expect 0x50 or 0x60)", id),
            Err(e) => warn!("BMP388: chip_id read failed: {:?}", e),
        }
        if let Err(e) = sensor.check_errors().await {
            warn!("BMP388: error register reports fault: {:?}", e);
        }

        // Configure oversampling: X8 for pressure, X2 for temperature
        sensor
            .set_oversampling(Oversampling::X8, Oversampling::X2)
            .await
            .map_err(|e| {
                error!("BMP388 set_oversampling failed: {:?}", e);
                SensorError::InitializationFailed {
                    sensor: "BMP388",
                    details: "Failed to set oversampling configuration",
                    cause: alloc::format!("{:?}", e),
                }
            })?;

        // Read back oversampling to confirm the write landed.
        match sensor.oversampling().await {
            Ok((p, t)) => debug!("BMP388: oversampling readback press={:?} temp={:?}", p, t),
            Err(e) => warn!("BMP388: oversampling readback failed: {:?}", e),
        }

        let measurement = sensor.forced_measurement().await.map_err(|e| {
            error!("BMP388 forced_measurement failed: {:?}", e);
            SensorError::ReadFailed {
                sensor: "BMP388",
                operation: "forced_measurement",
                details: "Failed to read pressure value during forced measurement",
            }
        })?;

        // Post-measurement diagnostics. The library's `wait_for_data` silently
        // returns Ok on timeout (see bmp388-embedded async.rs), so if the data
        // ready bits never asserted we'd read stale/zero values here. Dump
        // status + power control + error bits so we can distinguish between
        // "sensor never measured" (DRDY still low) and "measurement ran but
        // compensation produced zero".
        match sensor.status().await {
            Ok(s) => debug!(
                "BMP388: post-measure status: cmd_rdy={} press_drdy={} temp_drdy={}",
                s.command_ready, s.pressure_data_ready, s.temperature_data_ready,
            ),
            Err(e) => warn!("BMP388: status readback failed: {:?}", e),
        }
        match sensor.power_control().await {
            Ok(p) => debug!(
                "BMP388: power control: press_en={} temp_en={} mode={:?}",
                p.pressure_enable, p.temperature_enable, p.mode,
            ),
            Err(e) => warn!("BMP388: power_control readback failed: {:?}", e),
        }
        if let Err(e) = sensor.check_errors().await {
            warn!("BMP388: post-measure error register: {:?}", e);
        }

        let Measurement {
            temperature,
            pressure,
        } = measurement;
        let milli_pa = (pressure * 1000.0) as i32;
        debug!(
            "BMP388: measurement temp={:.3} C, pressure={:.3} Pa (stored {} mPa)",
            temperature, pressure, milli_pa,
        );

        // Flag suspicious all-zero output so the log surfaces it explicitly
        // rather than silently passing a 0 Pa reading through to storage.
        if pressure.abs() < f64::EPSILON {
            if temperature.abs() < f64::EPSILON {
                warn!(
                    "BMP388: both temperature and pressure read as 0 — DRDY likely never asserted (library wait_for_data silently times out) or I2C mux channel is wrong",
                );
            } else {
                warn!(
                    "BMP388: pressure is 0 but temperature is {:.2} C — pressure compensation suspect (calibration or raw ADC = 0)",
                    temperature,
                );
            }
        }

        Ok(BMP388Readings { milli_pa })
    }
}
