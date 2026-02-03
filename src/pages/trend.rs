//! Trend page for displaying time-series sensor data with graphs
//!
//! This page provides a generic interface for visualizing any sensor's data
//! over configurable time windows, with quality assessment and statistics.

use embedded_charts::prelude::*;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::{MonoTextStyle, ascii::FONT_6X10};
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Alignment, Text};
use embedded_graphics::{Drawable as EgDrawable, pixelcolor::Rgb565};
use heapless::{Deque, Vec, index_set::FnvIndexSet};

use crate::metrics::QualityLevel;
use crate::pages::Page;
use crate::sensors::SensorType;
use crate::storage::accumulator::RollupEvent;
use crate::storage::{RawSample, Rollup, RollupTier, TimeWindow};
use crate::ui::core::{Action, DirtyRegion, PageEvent, PageId, TouchEvent};
use crate::ui::{Container, Direction, Drawable, Padding, Style, WHITE};

extern crate alloc;
use alloc::{boxed::Box, string::String};

// Color constants from styling
// RGB565 format: R(5 bits), G(6 bits), B(5 bits)
// Convert from 8-bit RGB: R>>3, G>>2, B>>3
const COLOR_BACKGROUND: Rgb565 = Rgb565::new(18 >> 3, 23 >> 2, 24 >> 3);
const COLOR_FOREGROUND: Rgb565 = Rgb565::new(26 >> 3, 32 >> 2, 33 >> 3);
const _COLOR_STROKE: Rgb565 = Rgb565::new(43 >> 3, 55 >> 2, 57 >> 3);
const LIGHT_GRAY: Rgb565 = Rgb565::new(21, 42, 21);

/// Maximum data points for the largest time window (limited by embedded_charts)
const MAX_DATA_POINTS: usize = 256;

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

    /// Bulk load multiple rollups into the buffer (for initialization)
    /// This is more efficient than calling push_from_rollup repeatedly
    fn load_rollups(&mut self, rollups: &[Rollup]) {
        for rollup in rollups {
            self.push_from_rollup(rollup);
        }
    }

    /// Bulk load multiple raw samples into the buffer (for initialization)
    /// This is more efficient than calling push_from_raw_sample repeatedly
    fn load_raw_samples(&mut self, samples: &[RawSample]) {
        for sample in samples {
            self.push_from_raw_sample(sample);
        }
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

    // Graph repr for animation slides
    line_chart: LineChart<Rgb565>,
    line_stream: StreamingAnimator<Point2D>,

    // Cached state
    stats: TrendStats,
    current_quality: QualityLevel,
    current_timestamp: u32,

    // Flag to track if initial data has been requested
    initial_data_loaded: bool,
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

        // TODO: line color should be that of the current quality FG color
        let line_chart = LineChartBuilder::new()
            .smooth(true)
            .smooth_subdivisions(2)
            .line_width(2)
            .line_color(Rgb565::WHITE)
            .build()
            .unwrap(); // We want this to fail at run time if it can't be built

        Self {
            bounds,
            sensor,
            window,
            data_buffer: TrendDataBuffer::new(sensor),
            dirty: true,
            header_bounds,
            graph_bounds,
            stats_bounds,
            line_stream: StreamingAnimator::new(),
            line_chart,
            stats: TrendStats::default(),
            current_quality: QualityLevel::Good,
            current_timestamp: 0,
            initial_data_loaded: false,
        }
    }

    /// Load historical data into the trend page buffer
    /// This should be called once when the page is created or activated
    pub fn load_historical_data(&mut self, rollups: &[Rollup], current_time: u32) {
        self.data_buffer.load_rollups(rollups);
        self.current_timestamp = current_time;
        self.update_stats();
        self.initial_data_loaded = true;
        self.mark_dirty();
    }

    /// Load historical raw samples into the trend page buffer
    /// This should be called for short time windows (1m, 5m)
    pub fn load_historical_raw_samples(&mut self, samples: &[RawSample], current_time: u32) {
        self.data_buffer.load_raw_samples(samples);
        self.current_timestamp = current_time;
        self.update_stats();
        self.initial_data_loaded = true;
        self.mark_dirty();
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
        let _quality_style = MonoTextStyle::new(&FONT_10X20, WHITE);

        // Render quality indicator as a styled container that *owns* its text.
        let quality_bounds = Rectangle::new(
            Point::new(
                self.header_bounds.top_left.x + self.header_bounds.size.width as i32 - 120,
                self.header_bounds.top_left.y + 2,
            ),
            Size::new(118, 28),
        );

        let quality_style = Style::new()
            .with_background(self.current_quality.background_color())
            .with_foreground(WHITE)
            .with_border(self.current_quality.foreground_color(), 2);

        let mut container = Container::<1>::new(quality_bounds, Direction::Horizontal)
            .with_style(quality_style)
            .with_corner_radius(5)
            .with_padding(Padding::symmetric(3, 5))
            .with_alignment(crate::ui::Alignment::Center);

        let text_bounds = Rectangle::new(Point::zero(), Size::new(quality_bounds.size.width, 20));
        let text = crate::ui::components::TextComponent::new(
            text_bounds,
            self.current_quality.label(),
            crate::ui::TextSize::Medium,
        )
        .with_alignment(embedded_graphics::text::Alignment::Right)
        .with_style(Style::new().with_foreground(WHITE));

        container
            .add_child(
                crate::ui::Element::Text(Box::new(text)),
                crate::ui::SizeConstraint::Grow(1),
            )
            .ok();

        container.draw(display)?;

        Ok(())
    }

    /// Draw the graph using embedded_charts with interpolation
    fn draw_graph<D>(&mut self, display: &mut D) -> Result<(), D::Error>
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

        // Our goal here is to push new points to the inner_graph for smooth sliding, however,
        // we do not want to rebuild the entire graph from scratch each time. Because events
        // come in chronologically, as we walk through the data points, we'll also be walking
        // chronologically.
        let existing: FnvIndexSet<u32, MAX_DATA_POINTS> = self
            .line_stream
            .current_data()
            .map(|item| item.x as u32)
            .collect();

        let stream = &mut self.line_stream;

        for (ts, value) in data.iter() {
            if !existing.contains(ts) {
                let point = Point2D::new(*ts as f32, *value as f32);
                stream.push_data(point);
            }
        }

        let mut temp_series = StaticDataSeries::<Point2D, MAX_DATA_POINTS>::new();
        for point in stream.current_data() {
            // TODO: Remove unwrap here, impl custom Error type - just base impl for now
            let _ = temp_series.push(point).unwrap();
        }

        self.line_chart
            .draw(
                &temp_series,
                self.line_chart.config(),
                self.graph_bounds,
                display,
            )
            .unwrap();

        Ok(())
    }

    /// Draw the statistics bar at the bottom
    fn draw_stats<D>(&mut self, display: &mut D) -> Result<(), D::Error>
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

        // TODO: Request initial data load from storage manager
        // This would require a new PageEvent type or DisplayRequest to fetch
        // historical data from the storage manager based on this page's
        // sensor type and time window preferences.
        // For now, this is handled by the display manager sending the data
        // via DisplayRequest::LoadHistoricalData when the page is created.
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

                // Always update timestamp from the event to keep window sliding forward
                // This ensures get_window_data() uses the correct time reference
                let new_timestamp = match rollup_event.as_ref() {
                    RollupEvent::RawSample(sample) => {
                        self.data_buffer.push_from_raw_sample(sample);
                        sample.timestamp
                    }
                    RollupEvent::Rollup5m(rollup)
                    | RollupEvent::Rollup1h(rollup)
                    | RollupEvent::RollupDaily(rollup) => {
                        self.data_buffer.push_from_rollup(rollup);
                        // Use rollup end time (start_ts + window duration) for better accuracy
                        // This ensures we're always looking at "now" not "5 minutes ago"
                        rollup.start_ts
                    }
                };

                // Only update timestamp if it's newer (monotonically increasing)
                if new_timestamp > self.current_timestamp {
                    self.current_timestamp = new_timestamp;
                }

                // Recalculate statistics with updated timestamp
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

    fn draw_page<D: DrawTarget<Color = Rgb565>>(
        &mut self,
        display: &mut D,
    ) -> Result<(), D::Error> {
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
