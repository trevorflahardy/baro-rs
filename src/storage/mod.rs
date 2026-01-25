pub mod rollup_storage;
pub mod sd_card;

pub mod accumulator;
pub mod manager;

pub use rollup_storage::*;

/// Maximum number of sensor values stored per sample
pub const MAX_SENSORS: usize = 20;

/// Time window for data aggregation and display
///
/// Defines the different time scales over which sensor data can be viewed.
/// Each window corresponds to specific data tiers and sample counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeWindow {
    /// 1 minute window (6 raw samples at 10s interval)
    OneMinute,
    /// 5 minute window (30 raw samples)
    FiveMinutes,
    /// 30 minute window (6 x 5m rollups)
    ThirtyMinutes,
    /// 1 hour window (12 x 5m rollups)
    OneHour,
    /// 12 hour window (12 x 1h rollups)
    TwelveHours,
    /// 1 day window (24 x 1h rollups)
    OneDay,
    /// 1 week window (7 x 1day rollups)
    OneWeek,
}

impl TimeWindow {
    /// Get a short label for display
    pub const fn label(self) -> &'static str {
        match self {
            Self::OneMinute => "1m",
            Self::FiveMinutes => "5m",
            Self::ThirtyMinutes => "30m",
            Self::OneHour => "1h",
            Self::TwelveHours => "12h",
            Self::OneDay => "1d",
            Self::OneWeek => "1w",
        }
    }

    /// Get the duration of this window in seconds
    pub const fn duration_secs(self) -> u32 {
        match self {
            Self::OneMinute => 60,
            Self::FiveMinutes => 300,
            Self::ThirtyMinutes => 1800,
            Self::OneHour => 3600,
            Self::TwelveHours => 43200,
            Self::OneDay => 86400,
            Self::OneWeek => 604800,
        }
    }

    /// Get the maximum number of data points to store for this window
    pub const fn max_points(self) -> usize {
        match self {
            Self::OneMinute => 6,
            Self::FiveMinutes => 30,
            Self::ThirtyMinutes => 36,
            Self::OneHour => 72,
            Self::TwelveHours => 144,
            Self::OneDay => 288,
            Self::OneWeek => 168,
        }
    }

    /// Determine which rollup tier to use for this time window
    pub const fn preferred_rollup_tier(self) -> RollupTier {
        match self {
            Self::OneMinute | Self::FiveMinutes => RollupTier::RawSample,
            Self::ThirtyMinutes | Self::OneHour => RollupTier::FiveMinute,
            Self::TwelveHours | Self::OneDay => RollupTier::Hourly,
            Self::OneWeek => RollupTier::Daily,
        }
    }
}

/// Rollup tier for identifying which data layer to use
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollupTier {
    /// Raw samples (10s interval)
    RawSample,
    /// 5-minute rollups
    FiveMinute,
    /// Hourly rollups
    Hourly,
    /// Daily rollups
    Daily,
}
