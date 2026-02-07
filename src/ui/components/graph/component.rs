//! Main graph component with Drawable trait implementation
//!
//! The Graph component orchestrates all rendering and manages data series.

use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Alignment, Text};
use heapless::String;

use crate::ui::core::Drawable;

use super::axis::{AxisConfig, XAxisConfig, YAxisConfig, draw_x_axis_labels, draw_y_axis_labels};
use super::constants::AUTO_SCALE_MARGIN_FACTOR;
use super::grid::{GridConfig, draw_grid};
use super::interpolation::{draw_linear_series, draw_smooth_series};
use super::series::{DataPoint, DataSeries, InterpolationType, SeriesCollection};
use super::viewport::{DataBounds, Viewport, ViewportPadding};
use super::{GraphError, GraphResult};

/// Position for current value display
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurrentValuePosition {
    /// Top right corner with offset
    TopRight {
        /// Horizontal offset from right edge in pixels
        offset_x: u32,
        /// Vertical offset from top edge in pixels
        offset_y: u32,
    },
    /// Top left corner with offset
    TopLeft {
        /// Horizontal offset from left edge in pixels
        offset_x: u32,
        /// Vertical offset from top edge in pixels
        offset_y: u32,
    },
}

/// Current value display configuration
pub struct CurrentValueDisplay {
    /// Value to display
    pub value: f32,
    /// Small label text (e.g., "temp", "co2")
    pub label: String<8>,
    /// Position on the graph
    pub position: CurrentValuePosition,
    /// Text style for the value
    pub value_style: MonoTextStyle<'static, Rgb565>,
    /// Text style for the label
    pub label_style: MonoTextStyle<'static, Rgb565>,
}

/// Main graph component
///
/// Generic over MAX_SERIES (number of data series) and MAX_POINTS (points per series).
pub struct Graph<const MAX_SERIES: usize, const MAX_POINTS: usize> {
    /// Bounding rectangle for the entire graph
    bounds: Rectangle,
    /// Collection of data series
    series_collection: SeriesCollection<MAX_SERIES, MAX_POINTS>,
    /// Grid configuration
    grid_config: GridConfig,
    /// Axis configuration
    axis_config: AxisConfig,
    /// Viewport for coordinate transformation
    viewport: Viewport,
    /// Optional current value display
    current_value_display: Option<CurrentValueDisplay>,
    /// Background color
    background_color: Rgb565,
    /// Dirty flag for rendering optimization
    dirty: bool,
}

impl<const MAX_SERIES: usize, const MAX_POINTS: usize> Graph<MAX_SERIES, MAX_POINTS> {
    /// Create a new graph with default configuration
    pub fn new(bounds: Rectangle) -> Self {
        // Initialize with placeholder data bounds (will be recalculated from data)
        let data_bounds = DataBounds::new(0.0, 1.0, 0.0, 1.0);
        let viewport = Viewport::new(data_bounds, bounds);

        Self {
            bounds,
            series_collection: SeriesCollection::new(),
            grid_config: GridConfig::default(),
            axis_config: AxisConfig::default(),
            viewport,
            current_value_display: None,
            background_color: Rgb565::BLACK,
            dirty: true,
        }
    }

    /// Set background color
    pub fn with_background(mut self, color: Rgb565) -> Self {
        self.background_color = color;
        self
    }

    /// Set grid configuration
    pub fn with_grid(mut self, config: GridConfig) -> Self {
        self.grid_config = config;
        self
    }

    /// Set X-axis configuration
    pub fn with_x_axis(mut self, config: XAxisConfig) -> Self {
        self.axis_config.x_axis = Some(config);
        self
    }

    /// Set Y-axis configuration
    pub fn with_y_axis(mut self, config: YAxisConfig) -> Self {
        self.axis_config.y_axis = Some(config);
        self
    }

    /// Set viewport padding
    pub fn with_padding(mut self, padding: ViewportPadding) -> Self {
        self.viewport = self.viewport.with_padding(padding);
        self
    }

    /// Add a data series to the graph
    ///
    /// Returns the series index on success, or error if at capacity.
    pub fn add_series(&mut self, series: DataSeries<MAX_POINTS>) -> GraphResult<usize> {
        let result = self.series_collection.add(series);
        if result.is_ok() {
            // Recalculate viewport to fit the new series data
            let _ = self.recalculate_viewport();
            self.dirty = true;
        }
        result
    }

    /// Push a data point to a specific series
    ///
    /// Automatically recalculates viewport bounds.
    pub fn push_point(&mut self, series_idx: usize, point: DataPoint) -> GraphResult<()> {
        let series = self
            .series_collection
            .get_mut(series_idx)
            .ok_or(GraphError::InvalidSeriesIndex { index: series_idx })?;

        series
            .push(point)
            .map_err(|_| GraphError::PointCapacityExceeded { max: MAX_POINTS })?;

        self.recalculate_viewport()?;
        self.dirty = true;
        Ok(())
    }

    /// Set current value display
    pub fn set_current_value(&mut self, display: CurrentValueDisplay) {
        self.current_value_display = Some(display);
        self.dirty = true;
    }

    /// Clear current value display
    pub fn clear_current_value(&mut self) {
        self.current_value_display = None;
        self.dirty = true;
    }

    /// Recalculate viewport bounds from all series data
    fn recalculate_viewport(&mut self) -> GraphResult<()> {
        // Collect all points from all series
        // Note: We use a large fixed capacity since const generic expressions
        // are not yet stable in Rust
        const MAX_TOTAL_POINTS: usize = 512;
        let mut all_points: heapless::Vec<DataPoint, MAX_TOTAL_POINTS> = heapless::Vec::new();

        for series in self.series_collection.iter() {
            for point in series.points() {
                if all_points.push(*point).is_err() {
                    // If we hit capacity, stop collecting and work with what we have
                    break;
                }
            }
        }

        if all_points.is_empty() {
            return Err(GraphError::NoData);
        }

        // Calculate bounds with margin
        let bounds = DataBounds::from_points(&all_points, AUTO_SCALE_MARGIN_FACTOR)
            .ok_or(GraphError::NoData)?;

        self.viewport.set_data_bounds(bounds);
        Ok(())
    }

    /// Draw background
    fn draw_background<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        self.bounds
            .into_styled(PrimitiveStyle::with_fill(self.background_color))
            .draw(display)
    }

    /// Draw all data series
    fn draw_series<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        for series in self.series_collection.iter() {
            if !series.is_visible() || series.points().is_empty() {
                continue;
            }

            match series.interpolation() {
                InterpolationType::Linear => {
                    draw_linear_series(series.points(), &self.viewport, series.style(), display)?;
                }
                InterpolationType::Smooth { tension } => {
                    draw_smooth_series(
                        series.points(),
                        &self.viewport,
                        series.style(),
                        tension,
                        display,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Draw current value display if configured
    fn draw_current_value<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        if let Some(ref config) = self.current_value_display {
            let (x, y, alignment) = match config.position {
                CurrentValuePosition::TopRight { offset_x, offset_y } => (
                    self.bounds.top_left.x + self.bounds.size.width as i32 - offset_x as i32,
                    self.bounds.top_left.y + offset_y as i32,
                    Alignment::Right,
                ),
                CurrentValuePosition::TopLeft { offset_x, offset_y } => (
                    self.bounds.top_left.x + offset_x as i32,
                    self.bounds.top_left.y + offset_y as i32,
                    Alignment::Left,
                ),
            };

            // Draw value (large)
            let mut value_str = String::<16>::new();
            let _ = core::fmt::write(&mut value_str, format_args!("{:.0}", config.value));

            Text::with_alignment(
                value_str.as_str(),
                Point::new(x, y),
                config.value_style,
                alignment,
            )
            .draw(display)?;

            // Draw label (small, below value)
            Text::with_alignment(
                config.label.as_str(),
                Point::new(x, y + 15),
                config.label_style,
                alignment,
            )
            .draw(display)?;
        }

        Ok(())
    }
}

impl<const MAX_SERIES: usize, const MAX_POINTS: usize> Drawable for Graph<MAX_SERIES, MAX_POINTS> {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        // Layered rendering: background → grid → series → labels → annotations
        self.draw_background(display)?;
        draw_grid(&self.grid_config, &self.viewport, display)?;
        self.draw_series(display)?;

        if let Some(ref x_axis) = self.axis_config.x_axis {
            draw_x_axis_labels(x_axis, &self.viewport, display)?;
        }

        if let Some(ref y_axis) = self.axis_config.y_axis {
            draw_y_axis_labels(y_axis, &self.viewport, display)?;
        }

        self.draw_current_value(display)?;

        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}
