use super::MAX_SENSORS;
use crate::sensors::{CO2, HUMIDITY, TEMPERATURE};
use core::fmt::Display;

/// Raw sensor sample, recorded every 10 seconds
///
/// This is the highest-resolution data tier, retained for 24 hours only.
/// Raw samples are stored in a ring buffer that overwrites itself daily.
///
/// Binary size: 96 bytes (padded for alignment)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct RawSample {
    /// Timestamp in seconds since epoch (or boot time)
    pub timestamp: u32,
    /// Sensor readings in fixed-point format (e.g., milli-units)
    ///
    /// Each sensor value is stored as a signed 32-bit integer. For example:
    /// - Temperature: 25.3°C → 25300 (milli-degrees)
    /// - Humidity: 45.2% → 45200 (milli-percent)
    /// - CO2: 415 ppm → 415000 (milli-ppm)
    pub values: [i32; MAX_SENSORS],
    /// Padding to reach 96 bytes for efficient SD card I/O
    _padding: [u8; 12],
}

impl Display for RawSample {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let temp_c = self.values[TEMPERATURE] as f32 / 1000.0;
        let humidity_pct = self.values[HUMIDITY] as f32 / 1000.0;
        let co2_ppm = self.values[CO2] as f32 / 1000.0;

        write!(
            f,
            "[RawSample] timestamp: {}, temperature: {:.2}°C, humidity: {:.2}%, co2: {:.2} ppm",
            self.timestamp, temp_c, humidity_pct, co2_ppm
        )
    }
}

/// Aggregated rollup record containing average, minimum, and maximum values
///
/// Used for 5-minute, hourly, and daily rollups. Each rollup summarizes
/// multiple lower-tier records into statistical aggregates.
///
/// Binary size: 256 bytes (padded for alignment)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Rollup {
    /// Start timestamp of the aggregation window (seconds since epoch)
    pub start_ts: u32,
    /// Average value for each sensor over the window
    pub avg: [i32; MAX_SENSORS],
    /// Minimum value for each sensor over the window
    pub min: [i32; MAX_SENSORS],
    /// Maximum value for each sensor over the window
    pub max: [i32; MAX_SENSORS],
    /// Padding to reach 256 bytes for efficient SD card I/O
    _padding: [u8; 12],
}

impl Display for Rollup {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Only displaying averages here as others aren't vital for debugging.
        let temp_avg = self.avg[TEMPERATURE] as f32 / 1000.0;
        let humidity_avg = self.avg[HUMIDITY] as f32 / 1000.0;
        let co2_avg = self.avg[CO2] as f32 / 1000.0;

        write!(
            f,
            "[Rollup] start_ts: {}, avg: {:.2}°C, {:.2}%, {:.2} ppm",
            self.start_ts, temp_avg, humidity_avg, co2_avg
        )
    }
}

impl AsMut<[u8]> for Rollup {
    fn as_mut(&mut self) -> &mut [u8] {
        // Safety: Rollup is #[repr(C)] and contains only plain data types
        unsafe {
            core::slice::from_raw_parts_mut(
                (self as *mut Rollup) as *mut u8,
                core::mem::size_of::<Rollup>(),
            )
        }
    }
}

/// Lifetime statistics tracking cumulative metrics across all time
///
/// This single record is periodically overwritten to track long-term trends,
/// extrema, and cumulative exposure metrics.
///
/// Binary size: 256 bytes (padded for alignment)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct LifetimeStats {
    /// Timestamp when the device first booted (seconds since epoch)
    pub boot_time: u32,
    /// Total number of samples recorded since boot
    pub total_samples: u64,
    /// Cumulative integral of each sensor value (for exposure metrics)
    ///
    /// Example: Total degree-hours, total humidity exposure, etc.
    pub sensor_integrals: [i64; MAX_SENSORS],
    /// Maximum value ever recorded for each sensor
    pub sensor_max: [i32; MAX_SENSORS],
    /// Minimum value ever recorded for each sensor
    pub sensor_min: [i32; MAX_SENSORS],
    /// Padding to reach 256 bytes for efficient SD card I/O
    _padding: [u8; 24],
}

impl Display for LifetimeStats {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let temp_max = self.sensor_max[TEMPERATURE] as f32 / 1000.0;
        let temp_min = self.sensor_min[TEMPERATURE] as f32 / 1000.0;
        let humidity_max = self.sensor_max[HUMIDITY] as f32 / 1000.0;
        let humidity_min = self.sensor_min[HUMIDITY] as f32 / 1000.0;
        let co2_max = self.sensor_max[CO2] as f32 / 1000.0;
        let co2_min = self.sensor_min[CO2] as f32 / 1000.0;

        write!(
            f,
            "[LifetimeStats] boot_time: {}, total_samples: {}, temp_max: {:.2}°C, temp_min: {:.2}°C, humidity_max: {:.2}%, humidity_min: {:.2}%, co2_max: {:.2} ppm, co2_min: {:.2} ppm",
            self.boot_time,
            self.total_samples,
            temp_max,
            temp_min,
            humidity_max,
            humidity_min,
            co2_max,
            co2_min
        )
    }
}

impl RawSample {
    /// Create a new raw sample with the given timestamp and sensor values
    pub fn new(timestamp: u32, values: &[i32; MAX_SENSORS]) -> Self {
        Self {
            timestamp,
            values: *values,
            _padding: [0; 12],
        }
    }
}

impl Rollup {
    /// Create a new rollup record with the given timestamp and aggregates
    pub fn new(
        start_ts: u32,
        avg: &[i32; MAX_SENSORS],
        min: &[i32; MAX_SENSORS],
        max: &[i32; MAX_SENSORS],
    ) -> Self {
        Self {
            start_ts,
            avg: *avg,
            min: *min,
            max: *max,
            _padding: [0; 12],
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        // Safety: Rollup is #[repr(C)] and contains only plain data types
        unsafe {
            core::slice::from_raw_parts(
                (self as *const Rollup) as *const u8,
                core::mem::size_of::<Rollup>(),
            )
        }
    }
}

impl AsRef<[u8]> for Rollup {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl LifetimeStats {
    /// Create a new lifetime stats record
    pub fn new(boot_time: u32) -> Self {
        Self {
            boot_time,
            total_samples: 0,
            sensor_integrals: [0; MAX_SENSORS],
            sensor_max: [i32::MIN; MAX_SENSORS],
            sensor_min: [i32::MAX; MAX_SENSORS],
            _padding: [0; 24],
        }
    }

    /// Update lifetime statistics with a new sample
    pub fn update(&mut self, sample: &RawSample) {
        self.total_samples += 1;

        for i in 0..MAX_SENSORS {
            // Update integrals (for exposure metrics)
            self.sensor_integrals[i] =
                self.sensor_integrals[i].saturating_add(sample.values[i] as i64);

            // Update extrema
            self.sensor_max[i] = self.sensor_max[i].max(sample.values[i]);
            self.sensor_min[i] = self.sensor_min[i].min(sample.values[i]);
        }
    }

    fn as_slice(&self) -> &[u8] {
        // Safety: LifetimeStats is #[repr(C)] and contains only plain data types
        unsafe {
            core::slice::from_raw_parts(
                (self as *const LifetimeStats) as *const u8,
                core::mem::size_of::<LifetimeStats>(),
            )
        }
    }
}

impl AsMut<[u8]> for LifetimeStats {
    fn as_mut(&mut self) -> &mut [u8] {
        // Safety: LifetimeStats is #[repr(C)] and contains only plain data types
        unsafe {
            core::slice::from_raw_parts_mut(
                (self as *mut LifetimeStats) as *mut u8,
                core::mem::size_of::<LifetimeStats>(),
            )
        }
    }
}

impl AsRef<[u8]> for LifetimeStats {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl From<&[u8]> for LifetimeStats {
    fn from(bytes: &[u8]) -> Self {
        // Safety: We copy only up to the size of LifetimeStats
        let mut stats = LifetimeStats::default();
        let len = core::mem::size_of::<LifetimeStats>().min(bytes.len());
        stats.as_mut()[..len].copy_from_slice(&bytes[..len]);
        stats
    }
}

impl<const N: usize> From<&mut [u8; N]> for LifetimeStats {
    fn from(bytes: &mut [u8; N]) -> Self {
        // Verify that N is at least the size of LifetimeStats
        assert!(N >= core::mem::size_of::<LifetimeStats>());

        // Safety: We copy only up to the size of LifetimeStats
        let mut stats = LifetimeStats::default();
        let len = core::mem::size_of::<LifetimeStats>().min(bytes.len());
        stats.as_mut()[..len].copy_from_slice(&bytes[..len]);
        stats
    }
}
