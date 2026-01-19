//! # Time-Series Rollup Storage System
//!
//! This module implements a multi-tier time-series storage and aggregation system
//! optimized for embedded devices with SD card storage. The design prioritizes:
//!
//! - **Long-term operation** (years of continuous recording)
//! - **SD card wear leveling** (append-only writes, minimal file operations)
//! - **Power-loss resilience** (fixed-size records, atomic writes)
//! - **Fast graph generation** (O(1) seeking, pre-aggregated data)
//! - **Memory efficiency** (no-std compatible, fixed buffers)
//!
//! ## Architecture Overview
//!
//! The system uses a **multi-tier aggregation strategy** to balance granularity with storage:
//!
//! ### Tier 1: Raw Samples (24-hour retention)
//! - Samples every 10 seconds
//! - 8,640 records/day in a ring buffer
//! - Used for: 5-minute and 1-hour graphs
//! - Storage: ~830 KB (fixed, overwritten daily)
//!
//! ### Tier 2: 5-Minute Rollups (permanent)
//! - Aggregate 30 raw samples (5 minutes)
//! - 288 records/day
//! - Used for: 24-hour and 7-day graphs
//! - Storage: ~27 MB/year
//!
//! ### Tier 3: Hourly Rollups (permanent)
//! - Aggregate 12 five-minute rollups (1 hour)
//! - 24 records/day
//! - Used for: 1-month graphs
//! - Storage: ~2.2 MB/year
//!
//! ### Tier 4: Daily Rollups (permanent)
//! - Aggregate 24 hourly rollups (1 day)
//! - 1 record/day
//! - Used for: all-time graphs and trends
//! - Storage: ~94 KB/year
//!
//! ### Tier 5: Lifetime Statistics (permanent)
//! - Single record tracking cumulative stats
//! - Includes total samples, extrema, integrals
//! - Storage: 256 bytes (overwritten periodically)
//!
//! ## Storage Capacity
//!
//! With a 16 GB SD card:
//! - Total usage: ~30 MB/year
//! - **Estimated lifetime: 467 years** of continuous operation
//! - No pruning or compaction required
//!
//! ## Data Structures
//!
//! All structures use `#[repr(C)]` for predictable binary layout and are padded
//! to power-of-2 sizes for efficient SD card I/O.
//!
//! ## File Structure
//!
//! ```text
//! /
//! ├── raw_samples.bin      (ring buffer, 829,440 bytes)
//! ├── rollup_5m.bin        (append-only)
//! ├── rollup_1h.bin        (append-only)
//! ├── rollup_daily.bin     (append-only)
//! └── lifetime.bin         (single record, 256 bytes)
//! ```
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use rollup_storage::RollupAccumulator;
//!
//! let mut accumulator = RollupAccumulator::new();
//!
//! // Every 10 seconds, add a sensor reading
//! let reading = [temperature, humidity, pressure, /* ... */];
//! accumulator.add_sample(timestamp, &reading);
//!
//! // Accumulator automatically generates rollups when thresholds are met
//! if let Some(rollup_5m) = accumulator.get_5m_rollup() {
//!     // Write to rollup_5m.bin
//! }
//! ```
//!
//! ## Design Rationale
//!
//! This is **not** a general-purpose database. It's a specialized instrument design:
//!
//! - **No raw data archival**: Raw samples expire after 24 hours
//! - **Pre-aggregation only**: All historical data is summarized
//! - **Fixed record sizes**: Enables O(1) seeking and validation
//! - **Append-only writes**: Minimizes SD card wear and fragmentation
//! - **No indexing**: File offsets calculated mathematically
//!
//! For full implementation details, see `STORAGE.md` in the project root.

/// Maximum number of sensor values stored per sample
pub const MAX_SENSORS: usize = 20;

/// Raw sensor sample, recorded every 10 seconds
///
/// This is the highest-resolution data tier, retained for 24 hours only.
/// Raw samples are stored in a ring buffer that overwrites itself daily.
///
/// Binary size: 96 bytes (padded for alignment)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy)]
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

/// Lifetime statistics tracking cumulative metrics across all time
///
/// This single record is periodically overwritten to track long-term trends,
/// extrema, and cumulative exposure metrics.
///
/// Binary size: 256 bytes (padded for alignment)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
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
