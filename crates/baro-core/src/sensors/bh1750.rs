use crate::sensors::{SensorError, SensorReadings};

use super::Sensor;
use bh1750_embedded::{Address, Resolution, r#async::Bh1750Async};
use embedded_hal_async::i2c::I2c;
use log::{error, info};

pub struct BH1750Readings {
    pub milli_lux: i32,
}

impl SensorReadings<1> for BH1750Readings {
    fn to_array(self) -> [i32; 1] {
        [self.milli_lux]
    }
}

pub struct BH1750Sensor<I> {
    sensor: Bh1750Async<I, embassy_time::Delay>,
}

impl<I: I2c> BH1750Sensor<I> {
    pub fn new(i2c: I) -> Self {
        Self {
            sensor: Bh1750Async::<I, embassy_time::Delay>::new(
                i2c,
                embassy_time::Delay,
                Address::Low,
            ),
        }
    }
}

impl<I: I2c> Sensor<1> for BH1750Sensor<I> {
    type Readings = BH1750Readings;

    async fn read(&mut self) -> Result<BH1750Readings, SensorError> {
        self.sensor
            .one_time_measurement(Resolution::High)
            .await
            .map(|lux| {
                // The BH1750 gives us the lux value as f32, but we want to store it as i32 in our values array.
                // We can multiply by 1000 to preserve three decimal places of precision, and then convert to i32.
                let lux_i32 = (lux * 1000.0) as i32;
                info!("BH1750: Measured lux = {} (stored as {})", lux, lux_i32);

                BH1750Readings { milli_lux: lux_i32 }
            })
            .map_err(|e| {
                error!("BH1750 one_time_measurement failed: {:?}", e);
                SensorError::ReadFailed {
                    sensor: "BH1750",
                    operation: "one_time_measurement",
                    details: "Failed to read lux value during a single one-time measurement",
                }
            })
    }
}
