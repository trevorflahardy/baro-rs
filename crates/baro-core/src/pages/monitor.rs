// src/pages/monitor.rs
//! Monitor page with live sensor data and log feed.
//!
//! Displays a header with back navigation, current sensor values,
//! and a scrolling log of raw samples and rollup events.

use core::fmt::Write;

use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle,
};
use embedded_graphics::text::{Alignment, Text};
use heapless::{String as HeaplessString, Vec};

use crate::metrics::QualityLevel;
use crate::pages::page::Page;
use crate::sensor_store::SensorDataStore;
use crate::sensors::SensorType;
use crate::ui::Drawable;
use crate::ui::core::{Action, PageEvent, PageId, SensorData, StorageEvent, TouchEvent};
use crate::ui::styling::{COLOR_BACKGROUND, COLOR_FOREGROUND, FONT_6X10_CHAR_WIDTH_PX, WHITE};

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

/// Height of the header bar
const HEADER_HEIGHT_PX: u32 = 36;

/// Corner radius for header
const CORNER_RADIUS: u32 = 12;

/// Back button touch target width
const BACK_TOUCH_WIDTH: u32 = 44;

/// Y offset for sensor section
const SENSOR_SECTION_Y: u32 = HEADER_HEIGHT_PX + 4;

/// Number of columns in the live sensor grid.
const SENSOR_COLUMNS: usize = 2;

/// Vertical spacing between sensor rows, in pixels.
const SENSOR_ROW_HEIGHT_PX: i32 = 16;

/// Y offset of the first sensor row relative to the section top.
const SENSOR_ROW_BASELINE_Y: i32 = 12;

/// Extra pixels below the last sensor row (separator + padding).
const SENSOR_SECTION_TRAILING_PX: u32 = 6;

/// Height of the sensor values section, sized to fit every supported sensor.
const SENSOR_ROW_COUNT: usize = SensorType::ALL.len().div_ceil(SENSOR_COLUMNS);
const SENSOR_SECTION_HEIGHT: u32 = SENSOR_ROW_BASELINE_Y as u32
    + SENSOR_ROW_COUNT as u32 * SENSOR_ROW_HEIGHT_PX as u32
    + SENSOR_SECTION_TRAILING_PX;

/// Y offset for the log feed area
const LOG_Y_OFFSET: u32 = SENSOR_SECTION_Y + SENSOR_SECTION_HEIGHT + 4;

/// Log feed border stroke width
const LOG_BORDER_WIDTH: u32 = 1;

/// Log text left padding
const LOG_TEXT_PADDING_LEFT: i32 = 4;

/// Log line height
const LOG_LINE_HEIGHT: i32 = 12;

/// Horizontal padding
const PADDING_X: u32 = 6;

/// Maximum log entries
const MAX_LOG_ENTRIES: usize = 20;

/// Maximum characters retained per log entry. Must be large enough to hold a
/// summary line with every sensor in [`SensorType::ALL`].
const LOG_ENTRY_CAPACITY: usize = 96;

/// Maximum characters in one `Name:ValueUnit` segment (e.g. `"Pres:1013.2hPa"`).
const LOG_SEGMENT_CAPACITY: usize = 24;

/// Pixels of horizontal space reserved between adjacent sensor segments.
const LOG_SEGMENT_GAP_PX: i32 = FONT_6X10_CHAR_WIDTH_PX as i32;

/// Header text color (muted)
const COLOR_HEADER_TEXT: Rgb565 = Rgb565::new(20, 40, 20);

/// Muted text color
const COLOR_MUTED_TEXT: Rgb565 = Rgb565::new(18, 36, 18);

// ---------------------------------------------------------------------------
// LogEntry
// ---------------------------------------------------------------------------

/// One item in the scrolling monitor log.
///
/// `Sensor` entries are rendered as wrapping, color-coded segments at draw
/// time so their appearance stays in sync with current quality thresholds.
/// `Text` entries are plain single-line messages (raw samples, rollups).
#[derive(Clone)]
enum LogEntry {
    Sensor(SensorData),
    Text(HeaplessString<LOG_ENTRY_CAPACITY>),
}

// ---------------------------------------------------------------------------
// MonitorPage
// ---------------------------------------------------------------------------

pub struct MonitorPage {
    bounds: Rectangle,
    log_entries: Vec<LogEntry, MAX_LOG_ENTRIES>,
    /// Latest value per sensor, indexed by position in [`SensorType::ALL`].
    last_values: [Option<f32>; SensorType::ALL.len()],
    dirty: bool,
}

impl MonitorPage {
    pub fn new(bounds: Rectangle) -> Self {
        Self {
            bounds,
            log_entries: Vec::new(),
            last_values: [None; SensorType::ALL.len()],
            dirty: true,
        }
    }

    /// Kept for API compatibility.
    pub fn init(&mut self) {
        self.dirty = true;
    }

    /// Initialize sensor values from the centralized data store so the
    /// page shows current readings immediately instead of starting blank.
    pub fn load_from_store(&mut self, store: &SensorDataStore) {
        if let Some(data) = store.latest() {
            self.ingest_sensor_data(data);
            self.dirty = true;
        }
    }

    /// Copy every present reading in `data` into [`Self::last_values`].
    fn ingest_sensor_data(&mut self, data: &SensorData) {
        for (slot, sensor) in self.last_values.iter_mut().zip(SensorType::ALL.iter()) {
            if let Some(value) = sensor.value_from(data) {
                *slot = Some(value);
            }
        }
    }

    /// Format a single sensor reading as an ASCII-safe log segment such as
    /// `"Temp:21.3C"`. Returned buffer is at most [`LOG_SEGMENT_CAPACITY`]
    /// characters — enough for any supported sensor's short name + value.
    fn format_sensor_segment(
        sensor: SensorType,
        value: f32,
    ) -> HeaplessString<LOG_SEGMENT_CAPACITY> {
        let mut buf = HeaplessString::<LOG_SEGMENT_CAPACITY>::new();
        let _ = write!(
            buf,
            "{}:{:.*}{}",
            sensor.short_name(),
            sensor.log_precision(),
            value,
            sensor.ascii_unit(),
        );
        buf
    }

    fn back_touch_bounds(&self) -> Rectangle {
        Rectangle::new(
            self.bounds.top_left,
            Size::new(BACK_TOUCH_WIDTH, HEADER_HEIGHT_PX),
        )
    }

    fn push_log_entry(&mut self, entry: LogEntry) {
        if self.log_entries.len() >= MAX_LOG_ENTRIES {
            for i in 0..(MAX_LOG_ENTRIES - 1) {
                let next = self.log_entries.get(i + 1).cloned();
                if let (Some(dst), Some(next)) = (self.log_entries.get_mut(i), next) {
                    *dst = next;
                }
            }
            if let Some(last) = self.log_entries.get_mut(MAX_LOG_ENTRIES - 1) {
                *last = entry;
            }
        } else {
            self.log_entries.push(entry).ok();
        }
    }

    fn add_text_entry(&mut self, message: &str) {
        let mut buf = HeaplessString::<LOG_ENTRY_CAPACITY>::new();
        buf.push_str(message).ok();
        self.push_log_entry(LogEntry::Text(buf));
    }

    fn log_area_bounds(&self) -> Rectangle {
        let x = self.bounds.top_left.x + PADDING_X as i32;
        let y = self.bounds.top_left.y + LOG_Y_OFFSET as i32;
        let width = self.bounds.size.width.saturating_sub(PADDING_X * 2);
        let height = self.bounds.size.height.saturating_sub(LOG_Y_OFFSET + 2);
        Rectangle::new(Point::new(x, y), Size::new(width, height))
    }

    fn draw_header<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        let header_rect = Rectangle::new(
            self.bounds.top_left,
            Size::new(self.bounds.size.width, HEADER_HEIGHT_PX),
        );

        RoundedRectangle::with_equal_corners(header_rect, Size::new(CORNER_RADIUS, CORNER_RADIUS))
            .into_styled(PrimitiveStyle::with_fill(COLOR_FOREGROUND))
            .draw(display)?;

        let text_y = self.bounds.top_left.y + (HEADER_HEIGHT_PX / 2 + 4) as i32;

        // Back arrow
        Text::with_alignment(
            "<",
            Point::new(self.bounds.top_left.x + 12, text_y),
            MonoTextStyle::new(&FONT_6X10, COLOR_HEADER_TEXT),
            Alignment::Left,
        )
        .draw(display)?;

        // Title
        Text::with_alignment(
            "MONITOR",
            Point::new(self.bounds.top_left.x + 28, text_y),
            MonoTextStyle::new(&FONT_6X10, COLOR_HEADER_TEXT),
            Alignment::Left,
        )
        .draw(display)?;

        Ok(())
    }

    fn draw_sensor_values<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        let x = self.bounds.top_left.x + PADDING_X as i32;
        let y_base = self.bounds.top_left.y + SENSOR_SECTION_Y as i32;
        let text_style = MonoTextStyle::new(&FONT_6X10, WHITE);

        let usable_width = self.bounds.size.width.saturating_sub(PADDING_X * 2);
        let column_width = (usable_width / SENSOR_COLUMNS as u32) as i32;

        let mut buf = HeaplessString::<32>::new();
        for (idx, sensor) in SensorType::ALL.iter().enumerate() {
            let col = (idx % SENSOR_COLUMNS) as i32;
            let row = (idx / SENSOR_COLUMNS) as i32;
            let cell_x = x + col * column_width;
            let cell_y = y_base + SENSOR_ROW_BASELINE_Y + row * SENSOR_ROW_HEIGHT_PX;

            buf.clear();
            match self.last_values[idx] {
                Some(value) => {
                    let _ = write!(
                        buf,
                        "{}: {:.*}{}",
                        sensor.short_name(),
                        sensor.log_precision(),
                        value,
                        sensor.ascii_unit(),
                    );
                }
                None => {
                    let _ = write!(buf, "{}: --", sensor.short_name());
                }
            }
            Text::new(&buf, Point::new(cell_x, cell_y), text_style).draw(display)?;
        }

        // Separator line
        let sep_y = y_base + SENSOR_SECTION_HEIGHT as i32 - 2;
        Rectangle::new(Point::new(x, sep_y), Size::new(usable_width, 1))
            .into_styled(PrimitiveStyle::with_fill(COLOR_MUTED_TEXT))
            .draw(display)?;

        Ok(())
    }

    fn draw_log_feed<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        let log_area = self.log_area_bounds();

        // Log area background with border
        let style = PrimitiveStyleBuilder::new()
            .fill_color(COLOR_BACKGROUND)
            .stroke_color(COLOR_MUTED_TEXT)
            .stroke_width(LOG_BORDER_WIDTH)
            .build();
        log_area.into_styled(style).draw(display)?;

        let left_x = log_area.top_left.x + LOG_TEXT_PADDING_LEFT;
        let right_limit = log_area.top_left.x + log_area.size.width as i32 - LOG_TEXT_PADDING_LEFT;
        let mut y = log_area.top_left.y + LOG_LINE_HEIGHT;
        let max_y = log_area.top_left.y + log_area.size.height as i32 - 2;

        for entry in self.log_entries.iter().rev() {
            if y > max_y {
                break;
            }
            match entry {
                LogEntry::Text(message) => {
                    Text::new(
                        message.as_str(),
                        Point::new(left_x, y),
                        MonoTextStyle::new(&FONT_6X10, WHITE),
                    )
                    .draw(display)?;
                    y += LOG_LINE_HEIGHT;
                }
                LogEntry::Sensor(data) => {
                    y = Self::draw_sensor_log_entry(display, data, left_x, right_limit, y, max_y)?;
                }
            }
        }

        Ok(())
    }

    /// Render a sensor log entry, colour-coding each sensor's short name by
    /// its current quality rating and wrapping segments to additional lines
    /// when they would overflow the log area. Returns the next free y.
    fn draw_sensor_log_entry<D: DrawTarget<Color = Rgb565>>(
        display: &mut D,
        data: &SensorData,
        left_x: i32,
        right_limit: i32,
        start_y: i32,
        max_y: i32,
    ) -> Result<i32, D::Error> {
        let mut y = start_y;
        let mut x = left_x;
        let mut first_on_line = true;
        let char_w = FONT_6X10_CHAR_WIDTH_PX as i32;

        for sensor in SensorType::ALL {
            let Some(value) = sensor.value_from(data) else {
                continue;
            };
            let segment = Self::format_sensor_segment(*sensor, value);
            let segment_width = segment.len() as i32 * char_w;
            let gap = if first_on_line { 0 } else { LOG_SEGMENT_GAP_PX };

            if !first_on_line && x + gap + segment_width > right_limit {
                y += LOG_LINE_HEIGHT;
                if y > max_y {
                    return Ok(y);
                }
                x = left_x;
            } else {
                x += gap;
            }

            let name_width = sensor.short_name().len() as i32 * char_w;
            let quality_color = QualityLevel::assess(*sensor, value).foreground_color();

            // Coloured sensor name (e.g. "Temp").
            Text::new(
                sensor.short_name(),
                Point::new(x, y),
                MonoTextStyle::new(&FONT_6X10, quality_color),
            )
            .draw(display)?;

            // Remainder of the segment (":21.3C") in white for readability.
            let remainder = &segment.as_str()[sensor.short_name().len()..];
            Text::new(
                remainder,
                Point::new(x + name_width, y),
                MonoTextStyle::new(&FONT_6X10, WHITE),
            )
            .draw(display)?;

            x += segment_width;
            first_on_line = false;
        }

        Ok(y + LOG_LINE_HEIGHT)
    }
}

// ---------------------------------------------------------------------------
// Page trait
// ---------------------------------------------------------------------------

impl Page for MonitorPage {
    fn id(&self) -> PageId {
        PageId::Monitor
    }

    fn title(&self) -> &str {
        "Monitor"
    }

    fn on_activate(&mut self) {
        self.dirty = true;
    }

    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action> {
        if let TouchEvent::Press(point) = event
            && self.back_touch_bounds().contains(point.to_point())
        {
            return Some(Action::GoBack);
        }
        None
    }

    fn update(&mut self) {}

    fn on_event(&mut self, event: &PageEvent) -> bool {
        match event {
            PageEvent::SensorUpdate(data) => {
                self.ingest_sensor_data(data);
                if SensorType::ALL.iter().any(|s| s.value_from(data).is_some()) {
                    self.push_log_entry(LogEntry::Sensor(*data));
                }
                self.dirty = true;
                true
            }
            PageEvent::StorageEvent(storage_event) => {
                let mut log_msg = HeaplessString::<LOG_ENTRY_CAPACITY>::new();
                match storage_event {
                    StorageEvent::RawSample { sensor, value, .. } => {
                        let _ = write!(log_msg, "[Raw] {}: {:.2}", sensor, value);
                    }
                    StorageEvent::Rollup {
                        interval, count, ..
                    } => {
                        let _ = write!(log_msg, "[Rollup] {}: {}", interval, count);
                    }
                }
                self.add_text_entry(&log_msg);
                self.dirty = true;
                true
            }
            _ => false,
        }
    }

    fn draw_page<D: DrawTarget<Color = Rgb565>>(
        &mut self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        Drawable::draw(self, display)
    }

    fn bounds(&self) -> Rectangle {
        Drawable::bounds(self)
    }

    fn is_dirty(&self) -> bool {
        Drawable::is_dirty(self)
    }

    fn mark_clean(&mut self) {
        Drawable::mark_clean(self)
    }

    fn mark_dirty(&mut self) {
        Drawable::mark_dirty(self)
    }
}

// ---------------------------------------------------------------------------
// Drawable
// ---------------------------------------------------------------------------

impl Drawable for MonitorPage {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        if !self.dirty {
            return Ok(());
        }

        display.clear(COLOR_BACKGROUND)?;
        self.draw_header(display)?;
        self.draw_sensor_values(display)?;
        self.draw_log_feed(display)?;

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
