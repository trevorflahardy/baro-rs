//! TrendPage implementation and Page trait

use embedded_charts::prelude::*;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::{MonoTextStyle, ascii::FONT_6X10};
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Alignment, Text};
use embedded_graphics::{Drawable as EgDrawable, pixelcolor::Rgb565};
use heapless::{Vec, index_set::FnvIndexSet};

use crate::metrics::QualityLevel;
use crate::pages::Page;
use crate::sensors::SensorType;
use crate::storage::accumulator::RollupEvent;
use crate::storage::{RawSample, Rollup, RollupTier, TimeWindow};
use crate::ui::core::{Action, DirtyRegion, PageEvent, PageId, TouchEvent};
use crate::ui::{Container, Direction, Drawable, Padding, Style, WHITE};

use core::fmt::Write;

extern crate alloc;
use alloc::{boxed::Box, string::String};

use super::constants::{COLOR_FOREGROUND, LIGHT_GRAY, MAX_DATA_POINTS};
use super::data::TrendDataBuffer;
use super::stats::TrendStats;

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

    // Graph streaming for animation slides
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

        // Draw quality indicator on the right - round pill-shaped with two-tone color
        let quality_text = self.current_quality.label();
        let text_width = quality_text.len() as u32 * 6; // FONT_6X10 is ~6px wide per char
        let indicator_width = text_width + 20; // Tighter padding
        let indicator_height = 20;

        let quality_bounds = Rectangle::new(
            Point::new(
                self.header_bounds.top_left.x + self.header_bounds.size.width as i32
                    - indicator_width as i32
                    - 5,
                self.header_bounds.top_left.y + 10,
            ),
            Size::new(indicator_width, indicator_height),
        );

        // Use two-tone color scheme: darker background, brighter foreground border
        let quality_style = Style::new()
            .with_background(self.current_quality.background_color())
            .with_foreground(WHITE)
            .with_border(self.current_quality.foreground_color(), 2);

        let mut container = Container::<1>::new(quality_bounds, Direction::Horizontal)
            .with_style(quality_style)
            .with_corner_radius(10) // More rounded
            .with_padding(Padding::symmetric(2, 4)) // Tighter padding
            .with_alignment(crate::ui::Alignment::Center);

        let text_bounds =
            Rectangle::new(Point::zero(), Size::new(indicator_width, indicator_height));
        let text = crate::ui::components::TextComponent::new(
            text_bounds,
            quality_text,
            crate::ui::TextSize::Small,
        )
        .with_alignment(embedded_graphics::text::Alignment::Center)
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
        // Clear graph area with quality-based background color
        self.graph_bounds
            .into_styled(PrimitiveStyle::with_fill(
                self.current_quality.background_color(),
            ))
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

        // Draw current reading in top right of graph area
        if let Some((_, current_value)) = self.data_buffer.points.back() {
            // Format value without unit for main display
            let mut value_str = String::new();
            let _ = write!(value_str, "{:.1}", TrendStats::to_float(*current_value));

            // Position for the reading (top right of graph area)
            let reading_x = self.graph_bounds.top_left.x + self.graph_bounds.size.width as i32 - 10;
            let reading_y = self.graph_bounds.top_left.y + 30;

            // Draw the value in large white text
            let value_style = MonoTextStyle::new(&FONT_10X20, WHITE);

            Text::with_alignment(
                &value_str,
                Point::new(reading_x, reading_y),
                value_style,
                Alignment::Right,
            )
            .draw(display)?;

            // Draw the sensor type in small gray text below
            let type_style = MonoTextStyle::new(&FONT_6X10, LIGHT_GRAY);
            Text::with_alignment(
                self.sensor.name().to_lowercase().as_str(),
                Point::new(reading_x, reading_y + 15),
                type_style,
                Alignment::Right,
            )
            .draw(display)?;
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
            temp_series.push(point).unwrap();
        }

        // Calculate bounds from the data to properly configure axes
        let bounds = match temp_series.bounds() {
            Ok(b) => b,
            Err(_) => {
                // If we can't calculate bounds, show error message
                let text_style = MonoTextStyle::new(&FONT_6X10, LIGHT_GRAY);
                Text::with_alignment(
                    "Unable to calculate data bounds",
                    self.graph_bounds.center(),
                    text_style,
                    Alignment::Center,
                )
                .draw(display)?;
                return Ok(());
            }
        };

        let ((x_min, x_max), (y_min, y_max)) =
            calculate_nice_ranges_from_bounds(&bounds, RangeCalculationConfig::default());

        // Create axes with the calculated ranges and minimal styling
        let x_axis = presets::professional_x_axis(x_min, x_max)
            .tick_count(5)
            .show_grid(true)
            .show_labels(false) // Hide axis labels
            .build()
            .unwrap();

        let y_axis = presets::professional_y_axis(y_min, y_max)
            .tick_count(5)
            .show_grid(true)
            .show_labels(false) // Hide axis labels
            .build()
            .unwrap();

        // Build chart with quality-based color scheme
        let line_chart = LineChartBuilder::new()
            .smooth(true)
            .smooth_subdivisions(2)
            .line_width(3)
            .line_color(self.current_quality.foreground_color())
            .with_x_axis(x_axis)
            .with_y_axis(y_axis)
            .build()
            .unwrap();

        // Draw the chart with the data
        line_chart
            .draw(
                &temp_series,
                line_chart.config(),
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
        // Clear background with quality-based color
        self.bounds
            .into_styled(PrimitiveStyle::with_fill(
                self.current_quality.background_color(),
            ))
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
