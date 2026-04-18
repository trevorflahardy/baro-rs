#[cfg(feature = "sensor-bh1750")]
mod bh1750;
#[cfg(feature = "sensor-bmp388")]
mod bmp388;
#[cfg(feature = "sensor-scd41")]
mod scd41;
#[cfg(feature = "sensor-sht40")]
mod sht40;

#[cfg(feature = "sensor-bh1750")]
pub use bh1750::*;
#[cfg(feature = "sensor-bmp388")]
pub use bmp388::*;
#[cfg(feature = "sensor-scd41")]
pub use scd41::*;
#[cfg(feature = "sensor-sht40")]
pub use sht40::*;

use super::storage::MAX_SENSORS;
use core::{future::Future, marker::PhantomData};
use embedded_hal::i2c::{Error as I2cError, ErrorKind};
use thiserror_no_std::Error;

extern crate alloc;

/// Categorized I2C fault, derived from [`embedded_hal::i2c::ErrorKind`].
///
/// Kept `Copy` and heap-free so retry/backoff policy can pattern-match on it
/// cheaply and so health trackers can store the last observed fault without
/// allocating. This is deliberately coarser than the raw HAL variants — callers
/// that need the full detail should still log the original error via `Debug`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cFault {
    /// The addressed device did not acknowledge an address or data byte.
    /// Usually means wrong address, wrong mux channel, or device not powered.
    NoAck,
    /// Generic bus error (noise, stuck line, clock-stretch timeout, etc.).
    Bus,
    /// Lost arbitration to another master on the bus.
    Arbitration,
    /// Transaction-level timeout inside the HAL.
    Timeout,
    /// Overrun, underrun, or other flavor not covered above.
    Other,
}

impl I2cFault {
    /// Classify any `embedded_hal::i2c::Error` into a fault bucket.
    pub fn from_err<E: I2cError>(err: &E) -> Self {
        Self::from_kind(err.kind())
    }

    /// Classify an [`ErrorKind`] directly.
    pub const fn from_kind(kind: ErrorKind) -> Self {
        match kind {
            ErrorKind::NoAcknowledge(_) => Self::NoAck,
            ErrorKind::Bus => Self::Bus,
            ErrorKind::ArbitrationLoss => Self::Arbitration,
            ErrorKind::Overrun => Self::Other,
            _ => Self::Other,
        }
    }
}

impl core::fmt::Display for I2cFault {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            Self::NoAck => "no-ack",
            Self::Bus => "bus",
            Self::Arbitration => "arbitration-loss",
            Self::Timeout => "timeout",
            Self::Other => "other",
        };
        f.write_str(s)
    }
}

/// Which step of sensor bring-up failed.
///
/// Helps distinguish "the chip isn't responding at all" (Reset/ChipId) from
/// "chip talks but rejects our config" (Calibration/Config). Retry policy
/// can key off this — for example, NoAck on `ChipId` should trigger longer
/// backoff (likely wiring or address issue) than NoAck on `Config`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitStep {
    /// Soft reset / power-up sequence.
    Reset,
    /// Chip-ID / WHOAMI validation.
    ChipId,
    /// Factory calibration load from NVM.
    Calibration,
    /// Oversampling / filter / ODR / self-calibration configuration.
    Config,
    /// Anything else that happens during initialization.
    Other,
}

impl core::fmt::Display for InitStep {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            Self::Reset => "reset",
            Self::ChipId => "chip-id",
            Self::Calibration => "calibration",
            Self::Config => "config",
            Self::Other => "other",
        };
        f.write_str(s)
    }
}

/// Structured sensor error.
///
/// All variants are `Copy`-friendly and heap-free: the raw HAL error is
/// classified into an [`I2cFault`] at the call site via [`I2cFault::from_err`]
/// so the error can flow through channels, health trackers, and logs without
/// allocation. If the caller needs the raw `Debug` form, it should log it
/// separately before constructing this error.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorError {
    #[error("Sensor '{sensor}' init failed at {step} ({fault})")]
    Init {
        sensor: &'static str,
        step: InitStep,
        fault: I2cFault,
    },
    #[error("Sensor '{sensor}' read failed during '{op}' ({fault})")]
    Read {
        sensor: &'static str,
        op: &'static str,
        fault: I2cFault,
    },
    #[error("Sensor '{sensor}' mux select failed on channel {channel} ({fault})")]
    MuxSelect {
        sensor: &'static str,
        channel: u8,
        fault: I2cFault,
    },
    #[error("Sensor '{sensor}' data not ready (op: {op})")]
    DataNotReady {
        sensor: &'static str,
        op: &'static str,
    },
    #[error("Sensor '{sensor}' timed out during '{op}'")]
    Timeout {
        sensor: &'static str,
        op: &'static str,
    },
    #[error("Sensor '{sensor}' reported invalid state: {reason}")]
    InvalidState {
        sensor: &'static str,
        reason: &'static str,
    },
}

impl SensorError {
    /// Build an [`SensorError::Init`] from any HAL-level I2C error.
    pub fn init<E: I2cError>(sensor: &'static str, step: InitStep, err: &E) -> Self {
        Self::Init {
            sensor,
            step,
            fault: I2cFault::from_err(err),
        }
    }

    /// Build a [`SensorError::Read`] from any HAL-level I2C error.
    pub fn read<E: I2cError>(sensor: &'static str, op: &'static str, err: &E) -> Self {
        Self::Read {
            sensor,
            op,
            fault: I2cFault::from_err(err),
        }
    }

    /// Build a [`SensorError::MuxSelect`] from any HAL-level I2C error.
    pub fn mux_select<E: I2cError>(sensor: &'static str, channel: u8, err: &E) -> Self {
        Self::MuxSelect {
            sensor,
            channel,
            fault: I2cFault::from_err(err),
        }
    }

    /// Name of the sensor this error belongs to.
    pub const fn sensor(&self) -> &'static str {
        match self {
            Self::Init { sensor, .. }
            | Self::Read { sensor, .. }
            | Self::MuxSelect { sensor, .. }
            | Self::DataNotReady { sensor, .. }
            | Self::Timeout { sensor, .. }
            | Self::InvalidState { sensor, .. } => sensor,
        }
    }
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
    #[cfg(any(
        feature = "sensor-sht40",
        feature = "sensor-scd41",
        feature = "sensor-bh1750",
        feature = "sensor-bmp388"
    ))]
    use crate::sensors::IndexedSensor;
    #[cfg(feature = "sensor-bh1750")]
    use crate::sensors::bh1750::BH1750Sensor;
    #[cfg(feature = "sensor-bmp388")]
    use crate::sensors::bmp388::BMP388Sensor;
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

    /// BH1750 sensor configuration:
    /// - Starts at index 3 (lux)
    /// - Produces 1 value (lux)
    /// - Connected to I2C mux channel 2
    #[cfg(feature = "sensor-bh1750")]
    pub type BH1750Indexed<I> = IndexedSensor<BH1750Sensor<I>, 3, 1, 2>;

    /// BMP388 sensor configuration:
    /// - Starts at index 4 (pressure)
    /// - Produces 1 value (pressure in milli-Pascals)
    /// - Connected to I2C mux channel 3
    #[cfg(feature = "sensor-bmp388")]
    pub type BMP388Indexed<I> = IndexedSensor<BMP388Sensor<I>, 4, 1, 3>;

    pub const TEMPERATURE: usize = 0;
    pub const HUMIDITY: usize = 1;
    pub const CO2: usize = 2;
    pub const LUX: usize = 3;
    pub const PRESSURE: usize = 4;
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
    /// Lux sensor (BH1750 index 3)
    Lux,
    /// Pressure sensor (BMP388 index 4)
    Pressure,
}

impl SensorType {
    /// All supported sensor types, in canonical display order.
    ///
    /// Extend this array when adding a new sensor so UI surfaces that
    /// enumerate sensors (e.g. the live monitor) pick it up automatically.
    pub const ALL: &'static [SensorType] = &[
        SensorType::Temperature,
        SensorType::Humidity,
        SensorType::Co2,
        SensorType::Lux,
        SensorType::Pressure,
    ];

    /// Extract this sensor's reading from a [`SensorData`] snapshot.
    pub fn value_from(self, data: &crate::ui::core::SensorData) -> Option<f32> {
        match self {
            Self::Temperature => data.temperature,
            Self::Humidity => data.humidity,
            Self::Co2 => data.co2,
            Self::Lux => data.lux,
            Self::Pressure => data.pressure,
        }
    }

    /// Short unit string without non-ASCII characters — safe for the
    /// FONT_6X10 ASCII-only glyph set used by the monitor log.
    pub const fn ascii_unit(self) -> &'static str {
        match self {
            Self::Temperature => "C",
            Self::Humidity => "%",
            Self::Co2 => "ppm",
            Self::Lux => "lux",
            Self::Pressure => "hPa",
        }
    }

    /// Number of fractional digits to render for a compact log line.
    pub const fn log_precision(self) -> usize {
        match self {
            Self::Temperature | Self::Humidity | Self::Pressure => 1,
            Self::Co2 | Self::Lux => 0,
        }
    }

    /// Get the sensor array index for this sensor type
    pub const fn index(self) -> usize {
        match self {
            Self::Temperature => indices::TEMPERATURE,
            Self::Humidity => indices::HUMIDITY,
            Self::Co2 => indices::CO2,
            Self::Lux => indices::LUX,
            Self::Pressure => indices::PRESSURE,
        }
    }

    /// Get the unit string for display
    pub const fn unit(self) -> &'static str {
        match self {
            Self::Temperature => "°C",
            Self::Humidity => "%",
            Self::Co2 => "ppm",
            Self::Lux => "lux",
            Self::Pressure => "hPa",
        }
    }

    /// Get the display name for this sensor
    pub const fn name(self) -> &'static str {
        match self {
            Self::Temperature => "Temperature",
            Self::Humidity => "Humidity",
            Self::Co2 => "CO2",
            Self::Lux => "Lux",
            Self::Pressure => "Pressure",
        }
    }

    /// Get the short name for compact display
    pub const fn short_name(self) -> &'static str {
        match self {
            Self::Temperature => "Temp",
            Self::Humidity => "Humid",
            Self::Co2 => "CO2",
            Self::Lux => "Lux",
            Self::Pressure => "Pres",
        }
    }
}

pub use indices::*;

// Re-export for convenience
#[cfg(feature = "sensor-bh1750")]
pub use indices::BH1750Indexed;
#[cfg(feature = "sensor-bmp388")]
pub use indices::BMP388Indexed;
#[cfg(feature = "sensor-scd41")]
pub use indices::SCD41Indexed;
#[cfg(feature = "sensor-sht40")]
pub use indices::SHT40Indexed;

#[cfg(feature = "sensor-bh1750")]
pub use bh1750::BH1750Sensor;
#[cfg(feature = "sensor-bmp388")]
pub use bmp388::BMP388Sensor;

#[cfg(feature = "sensor-scd41")]
pub use scd41::SCD41Sensor;
#[cfg(feature = "sensor-sht40")]
pub use sht40::SHT40Sensor;
