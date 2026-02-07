//! Data series types for graph rendering
//!
//! Provides data structures for storing and managing time-series data points
//! with associated styling and interpolation settings.

use embedded_graphics::pixelcolor::Rgb565;

extern crate alloc;
use alloc::vec::Vec;
use embedded_graphics::prelude::RgbColor;

use super::constants::DEFAULT_SERIES_LINE_WIDTH_PX;
use super::{GraphError, GraphResult};

/// A single data point with x and y coordinates
///
/// Uses f32 for smooth interpolation calculations.
/// Typically x represents timestamp and y represents sensor value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DataPoint {
    /// X-coordinate (typically timestamp in seconds)
    pub x: f32,
    /// Y-coordinate (sensor value)
    pub y: f32,
}

impl DataPoint {
    /// Create a new data point
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Interpolation type for rendering series
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterpolationType {
    /// Linear interpolation (straight lines between points)
    Linear,
    /// Smooth curve interpolation (Catmull-Rom spline)
    Smooth {
        /// Curve tension (0.0 = loose, 0.5 = balanced, 1.0 = tight)
        tension: f32,
    },
}

/// Visual style configuration for a data series
#[derive(Debug, Clone, Copy)]
pub struct SeriesStyle {
    /// Line color
    pub color: Rgb565,
    /// Line width in pixels
    pub line_width: u32,
    /// Whether to draw dots at data points
    pub show_points: bool,
    /// Optional gradient fill under the line
    pub fill: Option<GradientFill>,
}

impl Default for SeriesStyle {
    fn default() -> Self {
        Self {
            color: Rgb565::WHITE,
            line_width: DEFAULT_SERIES_LINE_WIDTH_PX,
            show_points: false,
            fill: None,
        }
    }
}

/// Gradient fill configuration for the area under a series
#[derive(Debug, Clone, Copy)]
pub struct GradientFill {
    /// Color at the line
    pub start_color: Rgb565,
    /// Color at the bottom of the plot area
    pub end_color: Rgb565,
    /// Number of gradient bands to render
    pub bands: u8,
}

impl GradientFill {
    /// Create a new gradient fill
    pub const fn new(start_color: Rgb565, end_color: Rgb565, bands: u8) -> Self {
        Self {
            start_color,
            end_color,
            bands,
        }
    }
}

/// A data series containing points, style, and interpolation settings
pub struct DataSeries<const MAX_POINTS: usize> {
    /// Data points (x, y) pairs
    pub(super) points: Vec<DataPoint>,
    /// Visual style for rendering
    pub(super) style: SeriesStyle,
    /// Interpolation method
    pub(super) interpolation: InterpolationType,
    /// Whether this series should be rendered
    pub(super) visible: bool,
}

impl<const MAX_POINTS: usize> DataSeries<MAX_POINTS> {
    /// Create an empty data series
    pub fn new() -> Self {
        Self {
            points: Vec::with_capacity(MAX_POINTS),
            style: SeriesStyle::default(),
            interpolation: InterpolationType::Linear,
            visible: true,
        }
    }

    /// Set the visual style
    pub fn with_style(mut self, style: SeriesStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the interpolation type
    pub fn with_interpolation(mut self, interpolation: InterpolationType) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Set visibility
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Push a data point to the series
    ///
    /// Returns error if series is at capacity
    pub fn push(&mut self, point: DataPoint) -> GraphResult<()> {
        if self.points.len() >= MAX_POINTS {
            return Err(GraphError::PointCapacityExceeded { max: MAX_POINTS });
        }

        self.points.push(point);
        Ok(())
    }

    /// Get reference to all points
    pub fn points(&self) -> &[DataPoint] {
        &self.points
    }

    /// Get the style
    pub fn style(&self) -> &SeriesStyle {
        &self.style
    }

    /// Get the interpolation type
    pub fn interpolation(&self) -> InterpolationType {
        self.interpolation
    }

    /// Check if this series is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Clear all data points
    pub fn clear(&mut self) {
        self.points.clear();
    }
}

impl<const MAX_POINTS: usize> Default for DataSeries<MAX_POINTS> {
    fn default() -> Self {
        Self::new()
    }
}

/// Collection of multiple data series
pub struct SeriesCollection<const MAX_SERIES: usize, const MAX_POINTS: usize> {
    /// Vector of data series
    pub(super) series: Vec<DataSeries<MAX_POINTS>>,
}

impl<const MAX_SERIES: usize, const MAX_POINTS: usize> SeriesCollection<MAX_SERIES, MAX_POINTS> {
    /// Create an empty collection
    pub fn new() -> Self {
        Self {
            series: Vec::with_capacity(MAX_SERIES),
        }
    }

    /// Add a series to the collection
    ///
    /// Returns error if at capacity
    pub fn add(&mut self, series: DataSeries<MAX_POINTS>) -> GraphResult<usize> {
        let index = self.series.len();
        if index >= MAX_SERIES {
            return Err(GraphError::SeriesCapacityExceeded { max: MAX_SERIES });
        }

        self.series.push(series);
        Ok(index)
    }

    /// Get a series by index
    pub fn get(&self, index: usize) -> Option<&DataSeries<MAX_POINTS>> {
        self.series.get(index)
    }

    /// Get a mutable series by index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut DataSeries<MAX_POINTS>> {
        self.series.get_mut(index)
    }

    /// Iterate over all series
    pub fn iter(&self) -> impl Iterator<Item = &DataSeries<MAX_POINTS>> {
        self.series.iter()
    }

    /// Number of series in the collection
    pub fn len(&self) -> usize {
        self.series.len()
    }

    /// Check if collection is empty
    pub fn is_empty(&self) -> bool {
        self.series.is_empty()
    }
}

impl<const MAX_SERIES: usize, const MAX_POINTS: usize> Default
    for SeriesCollection<MAX_SERIES, MAX_POINTS>
{
    fn default() -> Self {
        Self::new()
    }
}
