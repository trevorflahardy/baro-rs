//! Viewport and coordinate transformation utilities
//!
//! Handles transformation between data space (sensor values, timestamps)
//! and screen space (pixel coordinates).

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

use super::constants::{
    DEFAULT_VIEWPORT_PADDING_BOTTOM_PX, DEFAULT_VIEWPORT_PADDING_LEFT_PX,
    DEFAULT_VIEWPORT_PADDING_RIGHT_PX, DEFAULT_VIEWPORT_PADDING_TOP_PX, MIN_DATA_RANGE,
};
use super::series::DataPoint;

/// Data space bounds (min/max x and y values)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DataBounds {
    /// Minimum X value in data space
    pub x_min: f32,
    /// Maximum X value in data space
    pub x_max: f32,
    /// Minimum Y value in data space
    pub y_min: f32,
    /// Maximum Y value in data space
    pub y_max: f32,
}

impl DataBounds {
    /// Create new data bounds
    pub const fn new(x_min: f32, x_max: f32, y_min: f32, y_max: f32) -> Self {
        Self {
            x_min,
            x_max,
            y_min,
            y_max,
        }
    }

    /// Calculate bounds from a slice of data points with optional margin
    pub fn from_points(points: &[DataPoint], margin_factor: f32) -> Option<Self> {
        if points.is_empty() {
            return None;
        }

        let mut x_min = points[0].x;
        let mut x_max = points[0].x;
        let mut y_min = points[0].y;
        let mut y_max = points[0].y;

        for point in points.iter().skip(1) {
            x_min = x_min.min(point.x);
            x_max = x_max.max(point.x);
            y_min = y_min.min(point.y);
            y_max = y_max.max(point.y);
        }

        // Add margin
        let x_range = (x_max - x_min).max(MIN_DATA_RANGE);
        let y_range = (y_max - y_min).max(MIN_DATA_RANGE);
        let x_margin = x_range * margin_factor;
        let y_margin = y_range * margin_factor;

        Some(Self {
            x_min: x_min - x_margin,
            x_max: x_max + x_margin,
            y_min: y_min - y_margin,
            y_max: y_max + y_margin,
        })
    }

    /// Get the X range (width)
    pub fn x_range(&self) -> f32 {
        self.x_max - self.x_min
    }

    /// Get the Y range (height)
    pub fn y_range(&self) -> f32 {
        self.y_max - self.y_min
    }
}

/// Padding around the plot area for labels and margins
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportPadding {
    /// Top padding in pixels
    pub top: u32,
    /// Right padding in pixels
    pub right: u32,
    /// Bottom padding in pixels
    pub bottom: u32,
    /// Left padding in pixels
    pub left: u32,
}

impl Default for ViewportPadding {
    fn default() -> Self {
        Self {
            top: DEFAULT_VIEWPORT_PADDING_TOP_PX,
            right: DEFAULT_VIEWPORT_PADDING_RIGHT_PX,
            bottom: DEFAULT_VIEWPORT_PADDING_BOTTOM_PX,
            left: DEFAULT_VIEWPORT_PADDING_LEFT_PX,
        }
    }
}

impl ViewportPadding {
    /// Create uniform padding on all sides
    pub const fn uniform(padding: u32) -> Self {
        Self {
            top: padding,
            right: padding,
            bottom: padding,
            left: padding,
        }
    }

    /// Create padding with specific values
    pub const fn new(top: u32, right: u32, bottom: u32, left: u32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}

/// Viewport for transforming data coordinates to screen coordinates
pub struct Viewport {
    /// Data space bounds
    data_bounds: DataBounds,
    /// Screen space bounds (full area including padding)
    screen_bounds: Rectangle,
    /// Padding around the plotting area
    padding: ViewportPadding,
}

impl Viewport {
    /// Create a new viewport
    pub fn new(data_bounds: DataBounds, screen_bounds: Rectangle) -> Self {
        Self {
            data_bounds,
            screen_bounds,
            padding: ViewportPadding::default(),
        }
    }

    /// Create viewport with custom padding
    pub fn with_padding(mut self, padding: ViewportPadding) -> Self {
        self.padding = padding;
        self
    }

    /// Get the plot area (screen bounds minus padding)
    pub fn plot_area(&self) -> Rectangle {
        let top_left = Point::new(
            self.screen_bounds.top_left.x + self.padding.left as i32,
            self.screen_bounds.top_left.y + self.padding.top as i32,
        );

        let width = self
            .screen_bounds
            .size
            .width
            .saturating_sub(self.padding.left + self.padding.right);
        let height = self
            .screen_bounds
            .size
            .height
            .saturating_sub(self.padding.top + self.padding.bottom);

        Rectangle::new(top_left, Size::new(width, height))
    }

    /// Transform a data point to screen coordinates
    ///
    /// Returns None if the point is outside data bounds or plot area
    pub fn data_to_screen(&self, point: DataPoint) -> Option<Point> {
        let plot_area = self.plot_area();

        // Normalize to 0.0-1.0 range
        let x_norm = (point.x - self.data_bounds.x_min) / self.data_bounds.x_range();
        let y_norm = (point.y - self.data_bounds.y_min) / self.data_bounds.y_range();

        // Check if point is within normalized bounds
        if !x_norm.is_finite() || !y_norm.is_finite() {
            return None;
        }

        // Convert to screen coordinates
        // Note: y-axis is inverted (screen Y increases downward)
        let screen_x = plot_area.top_left.x + (x_norm * plot_area.size.width as f32) as i32;
        let screen_y =
            plot_area.top_left.y + ((1.0 - y_norm) * plot_area.size.height as f32) as i32;

        // Bounds check
        if screen_x < plot_area.top_left.x
            || screen_x > plot_area.top_left.x + plot_area.size.width as i32
            || screen_y < plot_area.top_left.y
            || screen_y > plot_area.top_left.y + plot_area.size.height as i32
        {
            return None;
        }

        Some(Point::new(screen_x, screen_y))
    }

    /// Get the data bounds
    pub fn data_bounds(&self) -> &DataBounds {
        &self.data_bounds
    }

    /// Get the screen bounds
    pub fn screen_bounds(&self) -> Rectangle {
        self.screen_bounds
    }

    /// Get the padding
    pub fn padding(&self) -> &ViewportPadding {
        &self.padding
    }

    /// Update data bounds
    pub fn set_data_bounds(&mut self, bounds: DataBounds) {
        self.data_bounds = bounds;
    }
}
