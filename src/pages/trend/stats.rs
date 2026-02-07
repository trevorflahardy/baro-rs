//! Statistics calculations for trend data

/// Statistics for a time window
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct TrendStats {
    /// Average value in milli-units
    pub(super) avg: i32,
    /// Minimum value in milli-units
    pub(super) min: i32,
    /// Maximum value in milli-units
    pub(super) max: i32,
    /// Number of samples
    pub(super) count: usize,
}

impl TrendStats {
    /// Convert from milli-units to float for display
    pub(super) fn to_float(value: i32) -> f32 {
        value as f32 / 1000.0
    }

    /// Get average as float
    pub(super) fn avg_f32(&self) -> f32 {
        Self::to_float(self.avg)
    }

    /// Get minimum as float
    pub(super) fn min_f32(&self) -> f32 {
        Self::to_float(self.min)
    }

    /// Get maximum as float
    pub(super) fn max_f32(&self) -> f32 {
        Self::to_float(self.max)
    }
}
