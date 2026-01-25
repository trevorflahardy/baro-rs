//! Metrics and quality assessment for sensor data
//!
//! This module provides quality level assessment and thresholds for
//! determining environmental quality based on sensor readings.

use crate::sensors::SensorType;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::WebColors;

/// Quality level assessment for sensor readings
///
/// Provides standardized quality ratings based on configurable thresholds.
/// Used primarily for display and alerting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityLevel {
    /// Optimal conditions
    Excellent,
    /// Acceptable conditions
    Good,
    /// Sub-optimal conditions
    Poor,
    /// Problematic conditions
    Bad,
}

impl QualityLevel {
    /// Assess quality level for a given sensor reading
    ///
    /// Thresholds are sensor-specific and based on common comfort/safety ranges.
    /// Values are in the same units as the sensor (e.g., °C for temperature).
    pub fn assess(sensor: SensorType, value: f32) -> Self {
        match sensor {
            SensorType::Temperature => {
                // Temperature quality thresholds (°C)
                // Excellent: 20-24°C (comfortable indoor range)
                // Good: 18-26°C (acceptable range)
                // Poor: 15-28°C (uncomfortable but tolerable)
                // Bad: Outside these ranges
                if (20.0..=24.0).contains(&value) {
                    Self::Excellent
                } else if (18.0..=26.0).contains(&value) {
                    Self::Good
                } else if (15.0..=28.0).contains(&value) {
                    Self::Poor
                } else {
                    Self::Bad
                }
            }
            SensorType::Humidity => {
                // Humidity quality thresholds (%)
                // Excellent: 40-60% (optimal indoor humidity)
                // Good: 30-70% (acceptable range)
                // Poor: 20-80% (uncomfortable but tolerable)
                // Bad: Outside these ranges
                if (40.0..=60.0).contains(&value) {
                    Self::Excellent
                } else if (30.0..=70.0).contains(&value) {
                    Self::Good
                } else if (20.0..=80.0).contains(&value) {
                    Self::Poor
                } else {
                    Self::Bad
                }
            }
        }
    }

    /// Get the display color for this quality level
    pub const fn color(self) -> Rgb565 {
        match self {
            Self::Excellent => Rgb565::CSS_GREEN,
            Self::Good => Rgb565::CSS_LIGHT_GREEN,
            Self::Poor => Rgb565::CSS_ORANGE,
            Self::Bad => Rgb565::CSS_RED,
        }
    }

    /// Get the display label for this quality level
    pub const fn label(self) -> &'static str {
        match self {
            Self::Excellent => "Excellent",
            Self::Good => "Good",
            Self::Poor => "Poor",
            Self::Bad => "Bad",
        }
    }
}
