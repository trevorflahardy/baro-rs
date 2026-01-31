//! Sensor management and state

use crate::async_i2c_bus::AsyncI2cDevice;
use crate::sensors::{SCD41Indexed, SCD41Sensor, SHT40Indexed, SHT40Sensor, SensorError};

use log::error;
use tca9548a_embedded::r#async::{I2cChannelAsync, Tca9548aAsync};

type AsyncI2cDeviceType<'a> = AsyncI2cDevice<'a, esp_hal::i2c::master::I2c<'a, esp_hal::Async>>;

type I2CChannelAsyncDeviceType<'a> =
    I2cChannelAsync<'a, AsyncI2cDeviceType<'a>, esp_hal::i2c::master::Error>;

type SHT40IndexedAsyncI2CDeviceType<'a> = SHT40Indexed<I2CChannelAsyncDeviceType<'a>>;

type SCD41IndexedAsyncI2CDeviceType<'a> = SCD41Indexed<I2CChannelAsyncDeviceType<'a>>;

/// Container for all sensor instances
///
/// This struct holds all active sensors in the system.
/// Each sensor implements the `Sensor` trait and is wrapped with
/// an `IndexedSensor` to provide compile-time guarantees about
/// where its data is stored in the values array and which I2C mux
/// channel they reside on.
pub struct SensorsState<'a> {
    mux: Tca9548aAsync<AsyncI2cDeviceType<'a>>,
}

impl<'a> SensorsState<'a> {
    /// Create a new sensors state container
    ///
    /// The I2C mux is stored and sensors are created on-demand during reads.
    /// Each sensor type knows its own mux channel via compile-time const generics.
    pub fn new(mux: Tca9548aAsync<AsyncI2cDeviceType<'a>>) -> Self {
        Self { mux }
    }

    async fn read_sht40(
        &mut self,
        into: &mut [i32; crate::storage::MAX_SENSORS],
    ) -> Result<(), SensorError> {
        let channel = SHT40IndexedAsyncI2CDeviceType::mux_channel();
        let sht40_i2c = self.mux.channel(channel).unwrap();
        let mut sht40 = SHT40Indexed::from(SHT40Sensor::new(sht40_i2c));

        sht40.read_into(into).await
    }

    async fn read_scd41(
        &mut self,
        into: &mut [i32; crate::storage::MAX_SENSORS],
    ) -> Result<(), SensorError> {
        let channel = SCD41IndexedAsyncI2CDeviceType::mux_channel();
        let scd41_i2c = self.mux.channel(channel).unwrap();
        let mut scd41 = SCD41Indexed::from(SCD41Sensor::new(scd41_i2c));

        scd41.read_into(into).await
    }

    /// Read all sensors into the provided values array
    ///
    /// This method reads each sensor in sequence and stores the results
    /// at their designated indices in the array.
    ///
    /// Each sensor knows its own mux channel and array indices at compile time,
    /// ensuring type-safe sensor management as the system expands.
    pub async fn read_all(&mut self) -> Result<[i32; crate::storage::MAX_SENSORS], SensorError> {
        let mut values = [0_i32; crate::storage::MAX_SENSORS];

        // Read SHT40 using compile-time channel info
        // The sensor type itself knows it's on channel 0
        match self.read_sht40(&mut values).await {
            Ok(_) => {}
            Err(e) => {
                error!("SHT40 read error: {:?}", e);
                return Err(e);
            }
        }

        // Read SCD41 using compile-time channel info
        // The sensor type itself knows it's on channel 1
        match self.read_scd41(&mut values).await {
            Ok(_) => {}
            Err(e) => {
                error!("SCD41 read error: {:?}", e);
                return Err(e);
            }
        }

        Ok(values)
    }
}
