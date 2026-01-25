mod sht40;

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

pub struct IndexedSensor<S, const START: usize, const COUNT: usize>
where
    S: Sensor<COUNT>,
{
    sensor: S,
    _marker: PhantomData<Idx<START>>,
}

impl<S, const START: usize, const COUNT: usize> From<S> for IndexedSensor<S, START, COUNT>
where
    S: Sensor<COUNT>,
{
    fn from(value: S) -> Self {
        Self::new(value)
    }
}

impl<S, const START: usize, const COUNT: usize> IndexedSensor<S, START, COUNT>
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
}

pub mod indices {
    use crate::sensors::IndexedSensor;
    use crate::sensors::sht40::SHT40Sensor;

    // Listen here, mother fucker. You better god damn well use these indices correctly.
    // There is no compile-time checking of sensor indices to actual sensor data except
    // through these types. So, if you have a sensor that produces multiple readings and you
    // mess up the indices, you will fuck up your data in a way that is very hard to debug.
    //
    // I have included an obtuse IndexedSensor and SensorReadings to help combat
    // this as much as possible, but nevertheless, there is no way to stop from
    // shooting yourself.

    pub type SHT40Indexed<I> = IndexedSensor<SHT40Sensor<I>, 0, 2>;

    pub const TEMPERATURE: usize = 0;
    pub const HUMIDITY: usize = 1;
}

/// Sensor type identifier for selecting which sensor data to display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SensorType {
    /// Temperature sensor (SHT40 index 0)
    Temperature,
    /// Humidity sensor (SHT40 index 1)
    Humidity,
    // Future sensors can be added here
}

impl SensorType {
    /// Get the sensor array index for this sensor type
    pub const fn index(self) -> usize {
        match self {
            Self::Temperature => indices::TEMPERATURE,
            Self::Humidity => indices::HUMIDITY,
        }
    }

    /// Get the unit string for display
    pub const fn unit(self) -> &'static str {
        match self {
            Self::Temperature => "°C",
            Self::Humidity => "%",
        }
    }

    /// Get the display name for this sensor
    pub const fn name(self) -> &'static str {
        match self {
            Self::Temperature => "Temperature",
            Self::Humidity => "Humidity",
        }
    }

    /// Get the short name for compact display
    pub const fn short_name(self) -> &'static str {
        match self {
            Self::Temperature => "Temp",
            Self::Humidity => "Humid",
        }
    }
}

// Re-export for convenience
pub use indices::SHT40Indexed;

pub use indices::*;
pub use sht40::SHT40Sensor;
