//! Centralized sensor data store.
//!
//! Persists the latest sensor readings and per-sensor sparkline ring buffers
//! across page navigations so that home pages and grid pages can be
//! initialized with existing data instead of starting from scratch.

use crate::ui::core::SensorData;

/// Number of sparkline data points retained per sensor.
pub const SPARKLINE_CAPACITY: usize = 30;

/// Number of sensors tracked (Temperature, Humidity, CO2, Lux, Pressure).
const SENSOR_COUNT: usize = 5;

/// Centralized store for sensor data that outlives individual page instances.
///
/// Owned by the display manager (or simulator main loop). Pages read from
/// this store when they are created so they start with current data.
pub struct SensorDataStore {
    /// Most recent sensor reading.
    latest: Option<SensorData>,
    /// Per-sensor ring buffers of recent float values (for sparklines).
    sparklines: [[Option<f32>; SPARKLINE_CAPACITY]; SENSOR_COUNT],
    sparkline_counts: [usize; SENSOR_COUNT],
    sparkline_heads: [usize; SENSOR_COUNT],
}

impl Default for SensorDataStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SensorDataStore {
    /// Create an empty store.
    pub const fn new() -> Self {
        Self {
            latest: None,
            sparklines: [[None; SPARKLINE_CAPACITY]; SENSOR_COUNT],
            sparkline_counts: [0; SENSOR_COUNT],
            sparkline_heads: [0; SENSOR_COUNT],
        }
    }

    /// Record a new sensor reading, updating latest values and sparklines.
    pub fn push(&mut self, data: &SensorData) {
        self.latest = Some(*data);
        if let Some(temp) = data.temperature {
            self.push_sparkline(0, temp);
        }
        if let Some(hum) = data.humidity {
            self.push_sparkline(1, hum);
        }
        if let Some(co2) = data.co2 {
            self.push_sparkline(2, co2);
        }
        if let Some(lux) = data.lux {
            self.push_sparkline(3, lux);
        }
        if let Some(pressure) = data.pressure {
            self.push_sparkline(4, pressure);
        }
    }

    /// Get the most recent sensor reading, if any.
    pub fn latest(&self) -> Option<&SensorData> {
        self.latest.as_ref()
    }

    /// Get sparkline ring buffer data for a sensor index (0–4).
    ///
    /// Returns `(buffer, count, head)` matching the layout used by
    /// `HomeGridPage::SensorCard`.
    pub fn sparkline(
        &self,
        sensor_idx: usize,
    ) -> (&[Option<f32>; SPARKLINE_CAPACITY], usize, usize) {
        debug_assert!(sensor_idx < SENSOR_COUNT);
        let idx = sensor_idx.min(SENSOR_COUNT - 1);
        (
            &self.sparklines[idx],
            self.sparkline_counts[idx],
            self.sparkline_heads[idx],
        )
    }

    fn push_sparkline(&mut self, sensor_idx: usize, value: f32) {
        let head = self.sparkline_heads[sensor_idx];
        self.sparklines[sensor_idx][head] = Some(value);
        self.sparkline_heads[sensor_idx] = (head + 1) % SPARKLINE_CAPACITY;
        if self.sparkline_counts[sensor_idx] < SPARKLINE_CAPACITY {
            self.sparkline_counts[sensor_idx] += 1;
        }
    }
}
