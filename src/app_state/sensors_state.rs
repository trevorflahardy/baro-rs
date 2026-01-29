//! Sensor management and state

use crate::async_i2c_bus::AsyncI2cDevice;
use crate::sensors::{SHT40Indexed, SHT40Sensor, SensorError};

use log::error;
use tca9548a_embedded::r#async::{I2cChannelAsync, Tca9548aAsync};

type AsyncI2cDeviceType<'a> = AsyncI2cDevice<'a, esp_hal::i2c::master::I2c<'a, esp_hal::Async>>;

#[allow(dead_code)]
type I2CChannelAsyncDeviceType<'a> =
    I2cChannelAsync<'a, AsyncI2cDeviceType<'a>, esp_hal::i2c::master::Error>;

#[allow(dead_code)]
type SHT40IndexedAsyncI2CDeviceType<'a> = SHT40Indexed<I2CChannelAsyncDeviceType<'a>>;

/// Container for all sensor instances
///
/// This struct holds all active sensors in the system.
/// Each sensor implements the `Sensor` trait and is wrapped with
/// an `IndexedSensor` to provide compile-time guarantees about
/// where its data is stored in the values array.
pub struct SensorsState<'a> {
    mux: Tca9548aAsync<AsyncI2cDeviceType<'a>>,
    // pub sht40: SHT40IndexedAsyncI2CDeviceType<'a>,
}

impl<'a> SensorsState<'a> {
    /// Create a new sensors state container
    pub fn new(mux: Tca9548aAsync<AsyncI2cDeviceType<'a>>) -> Self {
        // SHT40 lives on channel 0 of the mux
        // let sht40_i2c: I2CChannelAsyncDeviceType<'a> = mux.channel(0).unwrap();

        Self {
            mux,
            // sht40: SHT40Indexed::from(SHT40Sensor::new(sht40_i2c)),
        }
    }

    async fn read_sht40<'b>(
        &mut self,
        into: &mut [i32; crate::storage::MAX_SENSORS],
    ) -> Result<(), SensorError> {
        let sht40_i2c = self.mux.channel(0).unwrap();
        let mut sensor = SHT40Indexed::from(SHT40Sensor::new(sht40_i2c));
        sensor.read_into(into).await
    }

    /// Read all sensors into the provided values array
    ///
    /// This method reads each sensor in sequence and stores the results
    /// at their designated indices in the array.
    pub async fn read_all(&mut self) -> Result<[i32; crate::storage::MAX_SENSORS], SensorError> {
        let mut values = [0_i32; crate::storage::MAX_SENSORS];

        // Read SHT40 (temperature and humidity at indices 0 and 1)
        if let Err(e) = self.read_sht40(&mut values).await {
            error!("SHT40 read error: {:?}", e);
            return Err(e);
        }

        // Future sensors would be added here
        // Example: self.another_sensor.read_into(values).await;

        Ok(values)
    }
}
