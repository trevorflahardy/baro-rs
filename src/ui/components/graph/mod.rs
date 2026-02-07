//! Custom graph rendering library for embedded displays
//!
//! This module provides a flexible, well-documented graph rendering system
//! optimized for resource-constrained embedded devices. It supports:
//!
//! - Linear and smooth (Catmull-Rom) curve interpolation
//! - Multiple data series with independent styling
//! - Configurable grid lines (vertical/horizontal)
//! - Automatic axis scaling with custom label formatters
//! - Current value display overlays
//!
//! # Memory Characteristics
//!
//! The graph uses const generics for compile-time capacity limits:
//! - `MAX_SERIES`: Maximum number of data series
//! - `MAX_POINTS`: Maximum points per series
//!
//! A `Graph<2, 256>` uses approximately 5KB of stack space.
//!
//! # Examples
//!
//! ```ignore
//! use baro_rs::ui::components::graph::*;
//! use embedded_graphics::prelude::*;
//!
//! let bounds = Rectangle::new(Point::new(0, 40), Size::new(320, 200));
//! let mut graph = Graph::<1, 128>::new(bounds)
//!     .with_background(COLOR_BACKGROUND);
//!
//! let series = DataSeries::new()
//!     .with_style(SeriesStyle {
//!         color: Rgb565::GREEN,
//!         line_width: 2,
//!         show_points: false,
//!     })
//!     .with_interpolation(InterpolationType::Smooth { tension: 0.5 });
//!
//! graph.add_series(series)?;
//! graph.push_point(0, DataPoint { x: 100.0, y: 22.5 })?;
//! ```

use thiserror_no_std::Error;

// Module declarations
mod axis;
mod component;
pub mod constants;
mod grid;
mod interpolation;
pub mod series;
pub mod viewport;

// Re-export main types
pub use axis::{AxisConfig, LabelFormatter, XAxisConfig, YAxisConfig};
pub use component::{CurrentValueDisplay, CurrentValuePosition, Graph};
pub use grid::{GridConfig, HorizontalGridLines, LineStyle, VerticalGridLines};
pub use series::{
    DataPoint, DataSeries, GradientFill, InterpolationType, SeriesCollection, SeriesStyle,
};
pub use viewport::{DataBounds, Viewport, ViewportPadding};

/// Error types for graph operations
#[derive(Debug, Error)]
pub enum GraphError {
    /// Series capacity exceeded
    #[error("Series capacity exceeded (max: {max})")]
    SeriesCapacityExceeded {
        /// Maximum allowed series count
        max: usize,
    },

    /// Point capacity exceeded for a series
    #[error("Point capacity exceeded (max: {max})")]
    PointCapacityExceeded {
        /// Maximum allowed points per series
        max: usize,
    },

    /// Invalid data bounds
    #[error("Invalid data bounds (min >= max)")]
    InvalidDataBounds,

    /// No data points available
    #[error("No data points available")]
    NoData,

    /// Invalid series index
    #[error("Invalid series index: {index}")]
    InvalidSeriesIndex {
        /// The invalid index
        index: usize,
    },

    /// Invalid interpolation parameter
    #[error("Invalid interpolation parameter: {param}")]
    InvalidInterpolationParameter {
        /// Parameter description
        param: &'static str,
    },
}

/// Result type for graph operations
pub type GraphResult<T> = Result<T, GraphError>;
