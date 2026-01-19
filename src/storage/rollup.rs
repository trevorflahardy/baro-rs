//! Rollup data structures for time-series storage.
//!
//! Implements the binary format specifications from STORAGE.md:
//! - RawSample: 96 bytes (raw sensor readings, 24-hour retention)
//! - Rollup: 256 bytes (aggregated data for 5m/1h/daily tiers)
//! - LifetimeStats: 256 bytes (lifetime counters and extrema)

/// Maximum number of sensor values per sample
pub const MAX_SENSORS: usize = 20;

/// Raw sensor sample for short-term storage.
///
/// Size: 96 bytes (padded)
/// Retention: 24 hours (ring buffer)
/// Used for: 5 minute and 1 hour graphs
///
/// Binary format (little-endian):
/// - timestamp: 4 bytes (u32)
/// - values: 80 bytes (20 × i32)
/// - padding: 12 bytes
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RawSample {
    /// Seconds since epoch or boot
    pub timestamp: u32,
    /// Sensor readings in fixed-point format (e.g., milli-units)
    pub values: [i32; MAX_SENSORS],
    /// Padding to ensure 96-byte alignment
    _padding: [u8; 12],
}

impl RawSample {
    /// Creates a new raw sample with the given timestamp and sensor values.
    pub fn new(timestamp: u32, values: [i32; MAX_SENSORS]) -> Self {
        Self {
            timestamp,
            values,
            _padding: [0; 12],
        }
    }

    /// Returns the size of this structure in bytes (96).
    pub const fn size() -> usize {
        core::mem::size_of::<Self>()
    }

    /// Converts the sample to a byte array for storage.
    pub fn to_bytes(&self) -> [u8; 96] {
        let mut bytes = [0u8; 96];
        
        // Write timestamp (little-endian)
        bytes[0..4].copy_from_slice(&self.timestamp.to_le_bytes());
        
        // Write sensor values (little-endian)
        for (i, &value) in self.values.iter().enumerate() {
            let offset = 4 + (i * 4);
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        
        // Padding is already zero
        bytes
    }

    /// Creates a sample from a byte array.
    pub fn from_bytes(bytes: &[u8; 96]) -> Self {
        let mut timestamp_bytes = [0u8; 4];
        timestamp_bytes.copy_from_slice(&bytes[0..4]);
        let timestamp = u32::from_le_bytes(timestamp_bytes);

        let mut values = [0i32; MAX_SENSORS];
        for i in 0..MAX_SENSORS {
            let offset = 4 + (i * 4);
            let mut value_bytes = [0u8; 4];
            value_bytes.copy_from_slice(&bytes[offset..offset + 4]);
            values[i] = i32::from_le_bytes(value_bytes);
        }

        Self {
            timestamp,
            values,
            _padding: [0; 12],
        }
    }
}

/// Aggregated rollup record for multi-scale time series.
///
/// Size: 256 bytes (padded)
/// Used for: 5-minute, hourly, and daily rollups
/// Retention: Forever (append-only)
///
/// Binary format (little-endian):
/// - start_ts: 4 bytes (u32)
/// - avg: 80 bytes (20 × i32)
/// - min: 80 bytes (20 × i32)
/// - max: 80 bytes (20 × i32)
/// - padding: 12 bytes
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Rollup {
    /// Window start timestamp (seconds since epoch)
    pub start_ts: u32,
    /// Average values for each sensor
    pub avg: [i32; MAX_SENSORS],
    /// Minimum values for each sensor
    pub min: [i32; MAX_SENSORS],
    /// Maximum values for each sensor
    pub max: [i32; MAX_SENSORS],
    /// Padding to ensure 256-byte alignment
    _padding: [u8; 12],
}

impl Rollup {
    /// Creates a new rollup with the given timestamp and aggregated values.
    pub fn new(
        start_ts: u32,
        avg: [i32; MAX_SENSORS],
        min: [i32; MAX_SENSORS],
        max: [i32; MAX_SENSORS],
    ) -> Self {
        Self {
            start_ts,
            avg,
            min,
            max,
            _padding: [0; 12],
        }
    }

    /// Returns the size of this structure in bytes (256).
    pub const fn size() -> usize {
        core::mem::size_of::<Self>()
    }

    /// Converts the rollup to a byte array for storage.
    pub fn to_bytes(&self) -> [u8; 256] {
        let mut bytes = [0u8; 256];
        let mut offset = 0;

        // Write timestamp (little-endian)
        bytes[offset..offset + 4].copy_from_slice(&self.start_ts.to_le_bytes());
        offset += 4;

        // Write avg values
        for &value in &self.avg {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            offset += 4;
        }

        // Write min values
        for &value in &self.min {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            offset += 4;
        }

        // Write max values
        for &value in &self.max {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            offset += 4;
        }

        // Padding is already zero
        bytes
    }

    /// Creates a rollup from a byte array.
    pub fn from_bytes(bytes: &[u8; 256]) -> Self {
        let mut offset = 0;

        // Read timestamp
        let mut timestamp_bytes = [0u8; 4];
        timestamp_bytes.copy_from_slice(&bytes[offset..offset + 4]);
        let start_ts = u32::from_le_bytes(timestamp_bytes);
        offset += 4;

        // Read avg values
        let mut avg = [0i32; MAX_SENSORS];
        for i in 0..MAX_SENSORS {
            let mut value_bytes = [0u8; 4];
            value_bytes.copy_from_slice(&bytes[offset..offset + 4]);
            avg[i] = i32::from_le_bytes(value_bytes);
            offset += 4;
        }

        // Read min values
        let mut min = [0i32; MAX_SENSORS];
        for i in 0..MAX_SENSORS {
            let mut value_bytes = [0u8; 4];
            value_bytes.copy_from_slice(&bytes[offset..offset + 4]);
            min[i] = i32::from_le_bytes(value_bytes);
            offset += 4;
        }

        // Read max values
        let mut max = [0i32; MAX_SENSORS];
        for i in 0..MAX_SENSORS {
            let mut value_bytes = [0u8; 4];
            value_bytes.copy_from_slice(&bytes[offset..offset + 4]);
            max[i] = i32::from_le_bytes(value_bytes);
            offset += 4;
        }

        Self {
            start_ts,
            avg,
            min,
            max,
            _padding: [0; 12],
        }
    }

    /// Calculates a rollup from a slice of raw samples.
    ///
    /// Computes avg/min/max for each sensor across all samples.
    /// Returns None if the samples slice is empty.
    pub fn from_samples(samples: &[RawSample]) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }

        let start_ts = samples[0].timestamp;
        let mut sum = [0i64; MAX_SENSORS]; // Use i64 to avoid overflow
        let mut min = [i32::MAX; MAX_SENSORS];
        let mut max = [i32::MIN; MAX_SENSORS];

        // Calculate min/max and sum for average
        for sample in samples {
            for i in 0..MAX_SENSORS {
                let value = sample.values[i];
                min[i] = min[i].min(value);
                max[i] = max[i].max(value);
                sum[i] = sum[i].saturating_add(value as i64);
            }
        }

        // Calculate averages from sums
        let mut avg = [0i32; MAX_SENSORS];
        for i in 0..MAX_SENSORS {
            avg[i] = (sum[i] / samples.len() as i64) as i32;
        }

        Some(Self::new(start_ts, avg, min, max))
    }

    /// Calculates a rollup from a slice of other rollups.
    ///
    /// Aggregates avg/min/max across multiple rollup periods.
    /// Returns None if the rollups slice is empty.
    pub fn from_rollups(rollups: &[Rollup]) -> Option<Self> {
        if rollups.is_empty() {
            return None;
        }

        let start_ts = rollups[0].start_ts;
        let mut sum = [0i64; MAX_SENSORS]; // Use i64 to avoid overflow
        let mut min = [i32::MAX; MAX_SENSORS];
        let mut max = [i32::MIN; MAX_SENSORS];

        // Calculate min/max across rollups and sum averages
        for rollup in rollups {
            for i in 0..MAX_SENSORS {
                min[i] = min[i].min(rollup.min[i]);
                max[i] = max[i].max(rollup.max[i]);
                sum[i] = sum[i].saturating_add(rollup.avg[i] as i64);
            }
        }

        // Calculate averages from sums
        let mut avg = [0i32; MAX_SENSORS];
        for i in 0..MAX_SENSORS {
            avg[i] = (sum[i] / rollups.len() as i64) as i32;
        }

        Some(Self::new(start_ts, avg, min, max))
    }
}

/// Lifetime statistics for long-term tracking.
///
/// Size: 336 bytes (with natural alignment)
/// Retention: Forever (single record, periodically overwritten)
/// Used for: All-time stats, uptime, exposure metrics
///
/// Note: STORAGE.md specified 256 bytes, but with 20 sensors and proper alignment,
/// the natural size is 336 bytes. This is still acceptable for single-record storage.
///
/// Binary format (little-endian):
/// - boot_time: 4 bytes (u32)
/// - _align_pad: 4 bytes (alignment padding)
/// - total_samples: 8 bytes (u64)
/// - sensor_integrals: 160 bytes (20 × i64)
/// - sensor_max: 80 bytes (20 × i32)
/// - sensor_min: 80 bytes (20 × i32)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LifetimeStats {
    /// Boot time (seconds since epoch)
    pub boot_time: u32,
    /// Alignment padding (automatically added by compiler for u64 alignment)
    _align_pad: u32,
    /// Total number of samples recorded
    pub total_samples: u64,
    /// Running integral (sum) of each sensor over lifetime
    pub sensor_integrals: [i64; MAX_SENSORS],
    /// Maximum value ever recorded for each sensor
    pub sensor_max: [i32; MAX_SENSORS],
    /// Minimum value ever recorded for each sensor
    pub sensor_min: [i32; MAX_SENSORS],
}

impl LifetimeStats {
    /// Creates new lifetime stats with default values.
    pub fn new(boot_time: u32) -> Self {
        Self {
            boot_time,
            _align_pad: 0,
            total_samples: 0,
            sensor_integrals: [0; MAX_SENSORS],
            sensor_max: [i32::MIN; MAX_SENSORS],
            sensor_min: [i32::MAX; MAX_SENSORS],
        }
    }

    /// Returns the size of this structure in bytes (336).
    pub const fn size() -> usize {
        core::mem::size_of::<Self>()
    }

    /// Updates lifetime stats with a new sample.
    pub fn update(&mut self, sample: &RawSample) {
        self.total_samples += 1;

        for i in 0..MAX_SENSORS {
            let value = sample.values[i];
            self.sensor_integrals[i] = self.sensor_integrals[i].saturating_add(value as i64);
            self.sensor_max[i] = self.sensor_max[i].max(value);
            self.sensor_min[i] = self.sensor_min[i].min(value);
        }
    }

    /// Converts the lifetime stats to a byte array for storage.
    pub fn to_bytes(&self) -> [u8; 336] {
        let mut bytes = [0u8; 336];
        let mut offset = 0;

        // Write boot_time
        bytes[offset..offset + 4].copy_from_slice(&self.boot_time.to_le_bytes());
        offset += 4;

        // Write alignment padding
        bytes[offset..offset + 4].copy_from_slice(&self._align_pad.to_le_bytes());
        offset += 4;

        // Write total_samples
        bytes[offset..offset + 8].copy_from_slice(&self.total_samples.to_le_bytes());
        offset += 8;

        // Write sensor_integrals
        for &value in &self.sensor_integrals {
            bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
            offset += 8;
        }

        // Write sensor_max
        for &value in &self.sensor_max {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            offset += 4;
        }

        // Write sensor_min
        for &value in &self.sensor_min {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            offset += 4;
        }

        bytes
    }

    /// Creates lifetime stats from a byte array.
    pub fn from_bytes(bytes: &[u8; 336]) -> Self {
        let mut offset = 0;

        // Read boot_time
        let mut boot_time_bytes = [0u8; 4];
        boot_time_bytes.copy_from_slice(&bytes[offset..offset + 4]);
        let boot_time = u32::from_le_bytes(boot_time_bytes);
        offset += 4;

        // Read alignment padding
        let mut align_pad_bytes = [0u8; 4];
        align_pad_bytes.copy_from_slice(&bytes[offset..offset + 4]);
        let _align_pad = u32::from_le_bytes(align_pad_bytes);
        offset += 4;

        // Read total_samples
        let mut total_samples_bytes = [0u8; 8];
        total_samples_bytes.copy_from_slice(&bytes[offset..offset + 8]);
        let total_samples = u64::from_le_bytes(total_samples_bytes);
        offset += 8;

        // Read sensor_integrals
        let mut sensor_integrals = [0i64; MAX_SENSORS];
        for i in 0..MAX_SENSORS {
            let mut value_bytes = [0u8; 8];
            value_bytes.copy_from_slice(&bytes[offset..offset + 8]);
            sensor_integrals[i] = i64::from_le_bytes(value_bytes);
            offset += 8;
        }

        // Read sensor_max
        let mut sensor_max = [0i32; MAX_SENSORS];
        for i in 0..MAX_SENSORS {
            let mut value_bytes = [0u8; 4];
            value_bytes.copy_from_slice(&bytes[offset..offset + 4]);
            sensor_max[i] = i32::from_le_bytes(value_bytes);
            offset += 4;
        }

        // Read sensor_min
        let mut sensor_min = [0i32; MAX_SENSORS];
        for i in 0..MAX_SENSORS {
            let mut value_bytes = [0u8; 4];
            value_bytes.copy_from_slice(&bytes[offset..offset + 4]);
            sensor_min[i] = i32::from_le_bytes(value_bytes);
            offset += 4;
        }

        Self {
            boot_time,
            _align_pad,
            total_samples,
            sensor_integrals,
            sensor_max,
            sensor_min,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_sample_size() {
        assert_eq!(RawSample::size(), 96, "RawSample must be exactly 96 bytes");
    }

    #[test]
    fn test_rollup_size() {
        assert_eq!(Rollup::size(), 256, "Rollup must be exactly 256 bytes");
    }

    #[test]
    fn test_lifetime_stats_size() {
        assert_eq!(
            LifetimeStats::size(),
            336,
            "LifetimeStats must be exactly 336 bytes (with alignment)"
        );
    }

    #[test]
    fn test_raw_sample_serialization() {
        let values = [42; MAX_SENSORS];
        let sample = RawSample::new(12345, values);
        
        let bytes = sample.to_bytes();
        let deserialized = RawSample::from_bytes(&bytes);
        
        assert_eq!(sample.timestamp, deserialized.timestamp);
        assert_eq!(sample.values, deserialized.values);
    }

    #[test]
    fn test_rollup_serialization() {
        let avg = [100; MAX_SENSORS];
        let min = [50; MAX_SENSORS];
        let max = [150; MAX_SENSORS];
        let rollup = Rollup::new(12345, avg, min, max);
        
        let bytes = rollup.to_bytes();
        let deserialized = Rollup::from_bytes(&bytes);
        
        assert_eq!(rollup.start_ts, deserialized.start_ts);
        assert_eq!(rollup.avg, deserialized.avg);
        assert_eq!(rollup.min, deserialized.min);
        assert_eq!(rollup.max, deserialized.max);
    }

    #[test]
    fn test_lifetime_stats_serialization() {
        let mut stats = LifetimeStats::new(12345);
        stats.total_samples = 1000;
        stats.sensor_integrals[0] = 50000;
        stats.sensor_max[0] = 200;
        stats.sensor_min[0] = -50;
        
        let bytes = stats.to_bytes();
        let deserialized = LifetimeStats::from_bytes(&bytes);
        
        assert_eq!(stats.boot_time, deserialized.boot_time);
        assert_eq!(stats.total_samples, deserialized.total_samples);
        assert_eq!(stats.sensor_integrals[0], deserialized.sensor_integrals[0]);
        assert_eq!(stats.sensor_max[0], deserialized.sensor_max[0]);
        assert_eq!(stats.sensor_min[0], deserialized.sensor_min[0]);
    }

    #[test]
    fn test_rollup_from_samples() {
        let samples = [
            RawSample::new(1000, [10; MAX_SENSORS]),
            RawSample::new(1010, [20; MAX_SENSORS]),
            RawSample::new(1020, [30; MAX_SENSORS]),
        ];
        
        let rollup = Rollup::from_samples(&samples).unwrap();
        
        assert_eq!(rollup.start_ts, 1000);
        assert_eq!(rollup.avg[0], 20); // (10+20+30)/3 = 20
        assert_eq!(rollup.min[0], 10);
        assert_eq!(rollup.max[0], 30);
    }

    #[test]
    fn test_rollup_from_rollups() {
        let rollups = [
            Rollup::new(1000, [15; MAX_SENSORS], [10; MAX_SENSORS], [20; MAX_SENSORS]),
            Rollup::new(1300, [25; MAX_SENSORS], [20; MAX_SENSORS], [30; MAX_SENSORS]),
        ];
        
        let aggregated = Rollup::from_rollups(&rollups).unwrap();
        
        assert_eq!(aggregated.start_ts, 1000);
        assert_eq!(aggregated.avg[0], 20); // (15+25)/2 = 20
        assert_eq!(aggregated.min[0], 10);
        assert_eq!(aggregated.max[0], 30);
    }

    #[test]
    fn test_lifetime_stats_update() {
        let mut stats = LifetimeStats::new(0);
        let sample = RawSample::new(1000, [42; MAX_SENSORS]);
        
        stats.update(&sample);
        
        assert_eq!(stats.total_samples, 1);
        assert_eq!(stats.sensor_integrals[0], 42);
        assert_eq!(stats.sensor_max[0], 42);
        assert_eq!(stats.sensor_min[0], 42);
    }
}
