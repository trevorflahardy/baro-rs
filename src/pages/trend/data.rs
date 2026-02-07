//! Data buffer management for trend data

use heapless::{Deque, Vec};

use crate::sensors::SensorType;
use crate::storage::{RawSample, Rollup, TimeWindow};

use super::constants::{DataPoint, MAX_DATA_POINTS};
use super::stats::TrendStats;

/// Ring buffer for storing time-series data points
pub(super) struct TrendDataBuffer {
    /// Ring buffer of (timestamp, value) pairs using Deque
    pub(super) points: Deque<DataPoint, MAX_DATA_POINTS>,
    /// Index of the sensor in the MAX_SENSORS array
    sensor_index: usize,
}

impl TrendDataBuffer {
    /// Create a new data buffer for a specific sensor
    pub(super) fn new(sensor_type: SensorType) -> Self {
        Self {
            points: Deque::new(),
            sensor_index: sensor_type.index(),
        }
    }

    /// Add a data point from a raw sample
    pub(super) fn push_from_raw_sample(&mut self, sample: &RawSample) {
        let value = sample.values[self.sensor_index];
        // If buffer is full, remove oldest
        if self.points.is_full() {
            self.points.pop_front();
        }
        let _ = self.points.push_back((sample.timestamp, value));
    }

    /// Add a data point from a rollup (using average)
    pub(super) fn push_from_rollup(&mut self, rollup: &Rollup) {
        let value = rollup.avg[self.sensor_index];
        // If buffer is full, remove oldest
        if self.points.is_full() {
            self.points.pop_front();
        }
        let _ = self.points.push_back((rollup.start_ts, value));
    }

    /// Bulk load multiple rollups into the buffer (for initialization)
    /// This is more efficient than calling push_from_rollup repeatedly
    pub(super) fn load_rollups(&mut self, rollups: &[Rollup]) {
        for rollup in rollups {
            self.push_from_rollup(rollup);
        }
    }

    /// Bulk load multiple raw samples into the buffer (for initialization)
    /// This is more efficient than calling push_from_raw_sample repeatedly
    pub(super) fn load_raw_samples(&mut self, samples: &[RawSample]) {
        for sample in samples {
            self.push_from_raw_sample(sample);
        }
    }

    /// Get data points within the specified time window
    pub(super) fn get_window_data(
        &self,
        window: TimeWindow,
        now: u32,
    ) -> Vec<DataPoint, MAX_DATA_POINTS> {
        let window_start = now.saturating_sub(window.duration_secs());

        self.points
            .iter()
            .filter(|(ts, _)| *ts >= window_start)
            .copied()
            .collect()
    }

    /// Calculate statistics for the current time window
    pub(super) fn calculate_stats(&self, window: TimeWindow, now: u32) -> TrendStats {
        let data = self.get_window_data(window, now);

        if data.is_empty() {
            return TrendStats::default();
        }

        let mut sum = 0i64;
        let mut min = i32::MAX;
        let mut max = i32::MIN;

        for (_, value) in data.iter() {
            sum += *value as i64;
            min = min.min(*value);
            max = max.max(*value);
        }

        let count = data.len();
        let avg = (sum / count as i64) as i32;

        TrendStats {
            avg,
            min,
            max,
            count,
        }
    }

    /// Check if there's any data in the buffer
    pub(super) fn is_empty(&self) -> bool {
        self.points.len() == 0
    }
}
