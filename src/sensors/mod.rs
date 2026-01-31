#[cfg(feature = "sensor-scd41")]
mod scd41;
#[cfg(feature = "sensor-sht40")]
mod sht40;

#[cfg(feature = "sensor-scd41")]
pub use scd41::*;
#[cfg(feature = "sensor-sht40")]
pub use sht40::*;

use super::storage::MAX_SENSORS;
use core::{future::Future, marker::PhantomData};
use thiserror_no_std::Error;

#[derive(Error, Debug)]
pub enum SensorError {
    #[error("Unknown sensor error")]
    UnknownError,
    #[error("Sensor read error")]
    ReadError,
}

/// Trait for sensor reading data structures.
/// Provides compile-time guarantees about the number of values and their conversion to arrays.
pub trait SensorReadings<const COUNT: usize> {
    /// Convert the readings into a fixed-size array.
    fn to_array(self) -> [i32; COUNT];
}

/// Trait for sensors that produce typed readings.
pub trait Sensor<const COUNT: usize> {
    /// The type of readings this sensor produces.
    type Readings: SensorReadings<COUNT>;

    /// Read the sensor and return typed readings.
    fn read(&mut self) -> impl Future<Output = Result<Self::Readings, SensorError>>;
}

// Type-level index markers
pub struct Idx<const N: usize>;

/// Indexed sensor with compile-time guarantees about storage indices and mux channel.
///
/// Generic parameters:
/// - S: The sensor type implementing Sensor<COUNT>
/// - START: Starting index in the values array where this sensor's data begins
/// - COUNT: Number of values this sensor produces
/// - MUX_CHANNEL: I2C mux channel number (0-7) where this sensor is connected
pub struct IndexedSensor<S, const START: usize, const COUNT: usize, const MUX_CHANNEL: u8>
where
    S: Sensor<COUNT>,
{
    sensor: S,
    _marker: PhantomData<Idx<START>>,
}

impl<S, const START: usize, const COUNT: usize, const MUX_CHANNEL: u8> From<S>
    for IndexedSensor<S, START, COUNT, MUX_CHANNEL>
where
    S: Sensor<COUNT>,
{
    fn from(value: S) -> Self {
        Self::new(value)
    }
}

impl<S, const START: usize, const COUNT: usize, const MUX_CHANNEL: u8>
    IndexedSensor<S, START, COUNT, MUX_CHANNEL>
where
    S: Sensor<COUNT>,
{
    pub const fn new(sensor: S) -> Self {
        Self {
            sensor,
            _marker: PhantomData,
        }
    }

    /// Read and write to the values array at the correct indices.
    /// Type safety ensures the readings are stored at the declared START position.
    pub async fn read_into(&mut self, values: &mut [i32; MAX_SENSORS]) -> Result<(), SensorError> {
        let readings = self.sensor.read().await?;
        let data = readings.to_array();
        values[START..START + COUNT].copy_from_slice(&data);
        Ok(())
    }

    /// Get the starting index where this sensor's data is stored.
    pub const fn start_index() -> usize {
        START
    }

    /// Get the number of values this sensor produces.
    pub const fn value_count() -> usize {
        COUNT
    }

    /// Get the absolute index for a specific reading within this sensor.
    /// This provides compile-time calculation of indices, ensuring they match the sensor's position.
    pub const fn reading_index(offset: usize) -> usize {
        START + offset
    }

    /// Get the I2C mux channel number where this sensor is connected.
    /// This provides compile-time knowledge of sensor location on the mux.
    pub const fn mux_channel() -> u8 {
        MUX_CHANNEL
    }
}

pub mod indices {
    #[cfg(any(feature = "sensor-sht40", feature = "sensor-scd41"))]
    use crate::sensors::IndexedSensor;
    #[cfg(feature = "sensor-scd41")]
    use crate::sensors::scd41::SCD41Sensor;
    #[cfg(feature = "sensor-sht40")]
    use crate::sensors::sht40::SHT40Sensor;

    // Listen here, mother fucker. You better god damn well use these indices correctly.
    // There is no compile-time checking of sensor indices to actual sensor data except
    // through these types. So, if you have a sensor that produces multiple readings and you
    // mess up the indices, you will fuck up your data in a way that is very hard to debug.
    //
    // I have included an obtuse IndexedSensor and SensorReadings to help combat
    // this as much as possible, but nevertheless, there is no way to stop from
    // shooting yourself.

    /// SHT40 sensor configuration:
    /// - Starts at index 0 (temperature)
    /// - Produces 2 values (temperature, humidity)
    /// - Connected to I2C mux channel 0
    #[cfg(feature = "sensor-sht40")]
    pub type SHT40Indexed<I> = IndexedSensor<SHT40Sensor<I>, 0, 2, 0>;

    /// SCD41 sensor configuration:
    /// - Starts at index 2 (CO2)
    /// - Produces 1 value (CO2 ppm)
    /// - Connected to I2C mux channel 1
    #[cfg(feature = "sensor-scd41")]
    pub type SCD41Indexed<I> = IndexedSensor<SCD41Sensor<I>, 2, 1, 1>;

    pub const TEMPERATURE: usize = 0;
    pub const HUMIDITY: usize = 1;
    pub const CO2: usize = 2;
}

/// Sensor type identifier for selecting which sensor data to display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SensorType {
    /// Temperature sensor (SHT40 index 0)
    Temperature,
    /// Humidity sensor (SHT40 index 1)
    Humidity,
    /// CO2 sensor (SCD41 index 2)
    Co2,
}

impl SensorType {
    /// Get the sensor array index for this sensor type
    pub const fn index(self) -> usize {
        match self {
            Self::Temperature => indices::TEMPERATURE,
            Self::Humidity => indices::HUMIDITY,
            Self::Co2 => indices::CO2,
        }
    }

    /// Get the unit string for display
    pub const fn unit(self) -> &'static str {
        match self {
            Self::Temperature => "°C",
            Self::Humidity => "%",
            Self::Co2 => "ppm",
        }
    }

    /// Get the display name for this sensor
    pub const fn name(self) -> &'static str {
        match self {
            Self::Temperature => "Temperature",
            Self::Humidity => "Humidity",
            Self::Co2 => "CO2",
        }
    }

    /// Get the short name for compact display
    pub const fn short_name(self) -> &'static str {
        match self {
            Self::Temperature => "Temp",
            Self::Humidity => "Humid",
            Self::Co2 => "CO2",
        }
    }
}

// Re-export for convenience
#[cfg(feature = "sensor-scd41")]
pub use indices::SCD41Indexed;
#[cfg(feature = "sensor-sht40")]
pub use indices::SHT40Indexed;

pub use indices::*;
#[cfg(feature = "sensor-scd41")]
pub use scd41::SCD41Sensor;
#[cfg(feature = "sensor-sht40")]
pub use sht40::SHT40Sensor;
