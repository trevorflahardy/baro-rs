use super::MAX_SENSORS;

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
    pub values: [i32; MAX_SENSORS],
    /// Padding to reach 96 bytes for efficient SD card I/O
    _padding: [u8; 12],
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

    pub fn to_slice(&self) -> &[u8] {
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
        self.to_slice()
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
}
