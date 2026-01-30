//! Trend page for displaying time-series sensor data with graphs
//!
//! This page provides a generic interface for visualizing any sensor's data
//! over configurable time windows, with quality assessment and statistics.

use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::{MonoTextStyle, ascii::FONT_6X10};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Alignment, Text};
use embedded_graphics::{Drawable as EgDrawable, pixelcolor::Rgb565};
use embedded_layout::View;
use heapless::{Deque, Vec};

use crate::metrics::QualityLevel;
use crate::pages::Page;
use crate::sensors::SensorType;
use crate::storage::accumulator::RollupEvent;
use crate::storage::{RawSample, Rollup, RollupTier, TimeWindow};
use crate::ui::core::{Action, DirtyRegion, PageEvent, PageId, TouchEvent};
use crate::ui::{Container, Direction, Drawable, Padding, Style, WHITE};

extern crate alloc;
use alloc::string::String;

// Color constants from styling
// RGB565 format: R(5 bits), G(6 bits), B(5 bits)
// Convert from 8-bit RGB: R>>3, G>>2, B>>3
const COLOR_BACKGROUND: Rgb565 = Rgb565::new(18 >> 3, 23 >> 2, 24 >> 3);
const COLOR_FOREGROUND: Rgb565 = Rgb565::new(26 >> 3, 32 >> 2, 33 >> 3);
const _COLOR_STROKE: Rgb565 = Rgb565::new(43 >> 3, 55 >> 2, 57 >> 3);
const LIGHT_GRAY: Rgb565 = Rgb565::new(21, 42, 21);

/// Maximum data points for the largest time window (1 day = 288 hourly points)
const MAX_DATA_POINTS: usize = 288;

/// Data point for graphing: (timestamp, value)
type DataPoint = (u32, i32);

/// Statistics for a time window
#[derive(Debug, Clone, Copy, Default)]
struct TrendStats {
    /// Average value in milli-units
    avg: i32,
    /// Minimum value in milli-units
    min: i32,
    /// Maximum value in milli-units
    max: i32,
    /// Number of samples
    count: usize,
}

impl TrendStats {
    /// Convert from milli-units to float for display
    fn to_float(value: i32) -> f32 {
        value as f32 / 1000.0
    }

    /// Get average as float
    fn avg_f32(&self) -> f32 {
        Self::to_float(self.avg)
    }

    /// Get minimum as float
    fn min_f32(&self) -> f32 {
        Self::to_float(self.min)
    }

    /// Get maximum as float
    fn max_f32(&self) -> f32 {
        Self::to_float(self.max)
    }
}

/// Ring buffer for storing time-series data points
struct TrendDataBuffer {
    /// Ring buffer of (timestamp, value) pairs using Deque
    points: Deque<DataPoint, MAX_DATA_POINTS>,
    /// Index of the sensor in the MAX_SENSORS array
    sensor_index: usize,
}

impl TrendDataBuffer {
    /// Create a new data buffer for a specific sensor
    fn new(sensor_type: SensorType) -> Self {
        Self {
            points: Deque::new(),
            sensor_index: sensor_type.index(),
        }
    }

    /// Add a data point from a raw sample
    fn push_from_raw_sample(&mut self, sample: &RawSample) {
        let value = sample.values[self.sensor_index];
        // If buffer is full, remove oldest
        if self.points.is_full() {
            self.points.pop_front();
        }
        let _ = self.points.push_back((sample.timestamp, value));
    }

    /// Add a data point from a rollup (using average)
    fn push_from_rollup(&mut self, rollup: &Rollup) {
        let value = rollup.avg[self.sensor_index];
        // If buffer is full, remove oldest
        if self.points.is_full() {
            self.points.pop_front();
        }
        let _ = self.points.push_back((rollup.start_ts, value));
    }

    /// Get data points within the specified time window
    fn get_window_data(&self, window: TimeWindow, now: u32) -> Vec<DataPoint, MAX_DATA_POINTS> {
        let window_start = now.saturating_sub(window.duration_secs());

        self.points
            .iter()
            .filter(|(ts, _)| *ts >= window_start)
            .copied()
            .collect()
    }

    /// Calculate statistics for the current time window
    fn calculate_stats(&self, window: TimeWindow, now: u32) -> TrendStats {
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
    fn is_empty(&self) -> bool {
        self.points.len() == 0
    }
}

/// Trend page displaying time-series graph and statistics
pub struct TrendPage {
    bounds: Rectangle,
    sensor: SensorType,
    window: TimeWindow,
    data_buffer: TrendDataBuffer,
    dirty: bool,

    // Layout sections
    header_bounds: Rectangle,
    graph_bounds: Rectangle,
    stats_bounds: Rectangle,

    // Cached state
    stats: TrendStats,
    current_quality: QualityLevel,
    current_timestamp: u32,
}

impl TrendPage {
    /// Create a new trend page for a specific sensor and time window
    pub fn new(bounds: Rectangle, sensor: SensorType, window: TimeWindow) -> Self {
        const HEADER_HEIGHT: u32 = 40;
        const STATS_HEIGHT: u32 = 55;

        let graph_height = bounds
            .size
            .height
            .saturating_sub(HEADER_HEIGHT + STATS_HEIGHT);

        let header_bounds =
            Rectangle::new(bounds.top_left, Size::new(bounds.size.width, HEADER_HEIGHT));

        let graph_bounds = Rectangle::new(
            Point::new(bounds.top_left.x, bounds.top_left.y + HEADER_HEIGHT as i32),
            Size::new(bounds.size.width, graph_height),
        );

        let stats_bounds = Rectangle::new(
            Point::new(
                bounds.top_left.x,
                bounds.top_left.y + (HEADER_HEIGHT + graph_height) as i32,
            ),
            Size::new(bounds.size.width, STATS_HEIGHT),
        );

        Self {
            bounds,
            sensor,
            window,
            data_buffer: TrendDataBuffer::new(sensor),
            dirty: true,
            header_bounds,
            graph_bounds,
            stats_bounds,
            stats: TrendStats::default(),
            current_quality: QualityLevel::Good,
            current_timestamp: 0,
        }
    }

    /// Update cached statistics and quality level
    fn update_stats(&mut self) {
        self.stats = self
            .data_buffer
            .calculate_stats(self.window, self.current_timestamp);

        // Assess quality based on average value
        if self.stats.count > 0 {
            self.current_quality = QualityLevel::assess(self.sensor, self.stats.avg_f32());
        }
    }

    /// Draw the header with title and quality indicator
    fn draw_header<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        // Clear header area with foreground color
        self.header_bounds
            .into_styled(PrimitiveStyle::with_fill(COLOR_FOREGROUND))
            .draw(display)?;

        let text_style = MonoTextStyle::new(&FONT_6X10, WHITE);

        // Draw sensor name and time window
        let mut title = String::new();
        use core::fmt::Write;
        let _ = write!(title, "{} - {}", self.sensor.name(), self.window.label());

        Text::with_alignment(
            &title,
            Point::new(
                self.header_bounds.top_left.x + 5,
                self.header_bounds.top_left.y + 15,
            ),
            text_style,
            Alignment::Left,
        )
        .draw(display)?;

        // Draw quality indicator on the right
        let quality_style = MonoTextStyle::new(&FONT_10X20, WHITE);

        let text = Text::with_alignment(
            self.current_quality.label(),
            Point::new(
                self.header_bounds.top_left.x + self.header_bounds.size.width as i32 - 5,
                self.header_bounds.top_left.y + 15,
            ),
            quality_style,
            Alignment::Right,
        );
        // .draw(display)?;

        let quality_style = Style::new()
            .with_background(self.current_quality.background_color())
            .with_foreground(WHITE)
            .with_border(self.current_quality.foreground_color(), 2);

        let mut container = Container::<1>::new(text.bounds(), Direction::Horizontal)
            .with_style(quality_style)
            .with_corner_radius(5)
            .with_padding(Padding::symmetric(3, 5));

        container
            .add_child(text.size(), crate::ui::SizeConstraint::Fit)
            .unwrap();

        container.draw(display)?;

        Ok(())
    }

    /// Draw the graph using embedded_charts with interpolation
    fn draw_graph<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        // Clear graph area with background color
        self.graph_bounds
            .into_styled(PrimitiveStyle::with_fill(COLOR_BACKGROUND))
            .draw(display)?;

        // Check if we have data
        if self.data_buffer.is_empty() {
            let text_style = MonoTextStyle::new(&FONT_6X10, LIGHT_GRAY);
            Text::with_alignment(
                "No data available",
                self.graph_bounds.center(),
                text_style,
                Alignment::Center,
            )
            .draw(display)?;
            return Ok(());
        }

        // Get data for current window
        let data = self
            .data_buffer
            .get_window_data(self.window, self.current_timestamp);

        if data.is_empty() {
            let text_style = MonoTextStyle::new(&FONT_6X10, LIGHT_GRAY);
            Text::with_alignment(
                "No data in window",
                self.graph_bounds.center(),
                text_style,
                Alignment::Center,
            )
            .draw(display)?;
            return Ok(());
        }

        // Draw a simple line graph manually using embedded-graphics with interpolation
        // Find min/max values for scaling
        let mut min_val = i32::MAX;
        let mut max_val = i32::MIN;
        for (_, val) in data.iter() {
            min_val = min_val.min(*val);
            max_val = max_val.max(*val);
        }

        // Add some padding to the range
        let range = max_val - min_val;
        let padding = range / 10;
        min_val -= padding;
        max_val += padding;

        // Ensure we have a non-zero range
        if min_val == max_val {
            min_val -= 1000;
            max_val += 1000;
        }

        let graph_width = self.graph_bounds.size.width as i32 - 10; // 5px padding each side
        let graph_height = self.graph_bounds.size.height as i32 - 10;
        let x_offset = self.graph_bounds.top_left.x + 5;
        let y_offset = self.graph_bounds.top_left.y + 5;

        // Use Catmull-Rom spline interpolation for smooth curves
        // We'll draw segments between points with interpolated intermediate points
        let segments_per_interval = 4; // Number of interpolated points between data points

        let mut prev_point: Option<Point> = None;
        let line_color = self.current_quality.foreground_color();

        for i in 0..data.len() {
            let curr_idx = i;

            // Get surrounding points for interpolation (p0, p1, p2, p3)
            let p0_idx = if i > 0 { i - 1 } else { i };
            let p1_idx = i;
            let p2_idx = if i + 1 < data.len() { i + 1 } else { i };
            let p3_idx = if i + 2 < data.len() { i + 2 } else { p2_idx };

            let (_t0, v0) = data[p0_idx];
            let (_t1, v1) = data[p1_idx];
            let (_t2, v2) = data[p2_idx];
            let (_t3, v3) = data[p3_idx];

            // Draw interpolated segments between p1 and p2
            for seg in 0..=segments_per_interval {
                let t = seg as f32 / segments_per_interval as f32;

                // Catmull-Rom spline interpolation
                // Formula: 0.5 * (2*p1 + (-p0 + p2)*t + (2*p0 - 5*p1 + 4*p2 - p3)*t^2 + (-p0 + 3*p1 - 3*p2 + p3)*t^3)
                let t2 = t * t;
                let t3 = t2 * t;

                let v0_f = v0 as f32;
                let v1_f = v1 as f32;
                let v2_f = v2 as f32;
                let v3_f = v3 as f32;

                let interpolated_val = (0.5
                    * (2.0 * v1_f
                        + (-v0_f + v2_f) * t
                        + (2.0 * v0_f - 5.0 * v1_f + 4.0 * v2_f - v3_f) * t2
                        + (-v0_f + 3.0 * v1_f - 3.0 * v2_f + v3_f) * t3))
                    as i32;

                // Map x position (blend between p1 and p2)
                let base_x = (curr_idx as i32 * graph_width) / (data.len() as i32 - 1).max(1);
                let next_x = if curr_idx + 1 < data.len() {
                    ((curr_idx + 1) as i32 * graph_width) / (data.len() as i32 - 1).max(1)
                } else {
                    base_x
                };
                let x = x_offset + base_x + ((next_x - base_x) as f32 * t) as i32;

                // Map y position (invert because screen y grows downward)
                let normalized =
                    ((interpolated_val - min_val) * graph_height) / (max_val - min_val).max(1);
                let y = y_offset + graph_height - normalized;

                let current_point = Point::new(x, y);

                // Draw line from previous point to current point
                if let Some(prev) = prev_point {
                    Line::new(prev, current_point)
                        .into_styled(PrimitiveStyle::with_stroke(line_color, 2))
                        .draw(display)?;
                }

                prev_point = Some(current_point);
            }
        }

        // Draw y-axis labels (min, max)
        let label_style = MonoTextStyle::new(&FONT_6X10, LIGHT_GRAY);

        let mut max_str = String::new();
        let mut min_str = String::new();
        use core::fmt::Write;
        let _ = write!(max_str, "{:.1}", TrendStats::to_float(max_val));
        let _ = write!(min_str, "{:.1}", TrendStats::to_float(min_val));

        // Draw max value at top
        Text::with_alignment(
            &max_str,
            Point::new(self.graph_bounds.top_left.x + 2, y_offset + 8),
            label_style,
            Alignment::Left,
        )
        .draw(display)?;

        // Draw min value at bottom
        Text::with_alignment(
            &min_str,
            Point::new(
                self.graph_bounds.top_left.x + 2,
                y_offset + graph_height - 2,
            ),
            label_style,
            Alignment::Left,
        )
        .draw(display)?;

        Ok(())
    }

    /// Draw the statistics bar at the bottom
    fn draw_stats<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        // Clear stats area with foreground color
        self.stats_bounds
            .into_styled(PrimitiveStyle::with_fill(COLOR_FOREGROUND))
            .draw(display)?;

        if self.stats.count == 0 {
            return Ok(());
        }

        let text_style = MonoTextStyle::new(&FONT_6X10, WHITE);
        let section_width = self.stats_bounds.size.width / 3;

        // Format stats with sensor unit
        let unit = self.sensor.unit();
        let mut avg_str = String::new();
        let mut min_str = String::new();
        let mut max_str = String::new();

        use core::fmt::Write;
        let _ = write!(avg_str, "Avg: {:.1}{}", self.stats.avg_f32(), unit);
        let _ = write!(min_str, "Min: {:.1}{}", self.stats.min_f32(), unit);
        let _ = write!(max_str, "Max: {:.1}{}", self.stats.max_f32(), unit);

        // Draw AVG
        Text::with_alignment(
            &avg_str,
            Point::new(
                self.stats_bounds.top_left.x + section_width as i32 / 2,
                self.stats_bounds.top_left.y + 25,
            ),
            text_style,
            Alignment::Center,
        )
        .draw(display)?;

        // Draw MIN
        Text::with_alignment(
            &min_str,
            Point::new(
                self.stats_bounds.top_left.x + section_width as i32 + section_width as i32 / 2,
                self.stats_bounds.top_left.y + 25,
            ),
            text_style,
            Alignment::Center,
        )
        .draw(display)?;

        // Draw MAX
        Text::with_alignment(
            &max_str,
            Point::new(
                self.stats_bounds.top_left.x + 2 * section_width as i32 + section_width as i32 / 2,
                self.stats_bounds.top_left.y + 25,
            ),
            text_style,
            Alignment::Center,
        )
        .draw(display)?;

        Ok(())
    }
}

impl Page for TrendPage {
    fn id(&self) -> PageId {
        PageId::TrendPage
    }

    fn title(&self) -> &str {
        self.sensor.name()
    }

    fn on_activate(&mut self) {
        self.mark_dirty();
    }

    fn on_event(&mut self, event: &PageEvent) -> bool {
        match event {
            PageEvent::RollupEvent(rollup_event) => {
                // Determine if this event is relevant for our time window
                let tier = self.window.preferred_rollup_tier();

                let should_process = matches!(
                    (tier, rollup_event.as_ref()),
                    (RollupTier::RawSample, RollupEvent::RawSample(_))
                        | (RollupTier::FiveMinute, RollupEvent::Rollup5m(_))
                        | (RollupTier::Hourly, RollupEvent::Rollup1h(_))
                        | (RollupTier::Daily, RollupEvent::RollupDaily(_))
                );

                if !should_process {
                    return false;
                }

                // Add data point to buffer
                match rollup_event.as_ref() {
                    RollupEvent::RawSample(sample) => {
                        self.data_buffer.push_from_raw_sample(sample);
                        self.current_timestamp = sample.timestamp;
                    }
                    RollupEvent::Rollup5m(rollup)
                    | RollupEvent::Rollup1h(rollup)
                    | RollupEvent::RollupDaily(rollup) => {
                        self.data_buffer.push_from_rollup(rollup);
                        self.current_timestamp = rollup.start_ts;
                    }
                }

                // Recalculate statistics
                self.update_stats();
                self.mark_dirty();
                true
            }
            _ => false,
        }
    }

    fn handle_touch(&mut self, _event: TouchEvent) -> Option<Action> {
        // For now, no touch interactions
        // Future: could add pan/zoom, time window selection, etc.
        None
    }

    fn update(&mut self) {
        // Called in UI loop - could be used for animations
    }

    fn draw_page<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        // Clear background
        self.bounds
            .into_styled(PrimitiveStyle::with_fill(COLOR_BACKGROUND))
            .draw(display)?;

        // Draw all sections
        self.draw_header(display)?;
        self.draw_graph(display)?;
        self.draw_stats(display)?;

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

    fn dirty_regions(&self) -> Vec<DirtyRegion, 8> {
        if self.is_dirty() {
            let mut regions = Vec::new();
            regions.push(DirtyRegion::new(self.bounds)).ok();
            regions
        } else {
            Vec::new()
        }
    }
}
