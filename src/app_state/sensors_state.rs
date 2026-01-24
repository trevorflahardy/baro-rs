//! Sensor management and state

use crate::async_i2c_bus::AsyncI2cDevice;
use crate::sensors::{SHT40Indexed, SHT40Sensor};

use log::error;

/// Container for all sensor instances
///
/// This struct holds all active sensors in the system.
/// Each sensor implements the `Sensor` trait and is wrapped with
/// an `IndexedSensor` to provide compile-time guarantees about
/// where its data is stored in the values array.
pub struct SensorsState<'a> {
    pub sht40: SHT40Indexed<AsyncI2cDevice<'a, esp_hal::i2c::master::I2c<'a, esp_hal::Async>>>,
}

impl<'a> SensorsState<'a> {
    /// Create a new sensors state container
    pub fn new(
        sht40_i2c: AsyncI2cDevice<'a, esp_hal::i2c::master::I2c<'a, esp_hal::Async>>,
    ) -> Self {
        Self {
            sht40: SHT40Indexed::from(SHT40Sensor::new(sht40_i2c)),
        }
    }

    /// Read all sensors into the provided values array
    ///
    /// This method reads each sensor in sequence and stores the results
    /// at their designated indices in the array.
    pub async fn read_all(&mut self, values: &mut [i32; crate::storage::MAX_SENSORS]) {
        // Read SHT40 (temperature and humidity at indices 0 and 1)
        if let Err(e) = self.sht40.read_into(values).await {
            error!("SHT40 read error: {:?}", e);
        }

        // Future sensors would be added here
        // Example: self.another_sensor.read_into(values).await;
    }
}
