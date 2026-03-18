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

use crate::pages::page::Page;
use crate::sensor_store::SensorDataStore;
use crate::ui::Drawable;
use crate::ui::core::{Action, PageEvent, PageId, StorageEvent, TouchEvent};
use crate::ui::styling::{COLOR_BACKGROUND, COLOR_FOREGROUND, WHITE};

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

/// Height of the sensor values section
const SENSOR_SECTION_HEIGHT: u32 = 48;

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

/// Header text color (muted)
const COLOR_HEADER_TEXT: Rgb565 = Rgb565::new(20, 40, 20);

/// Muted text color
const COLOR_MUTED_TEXT: Rgb565 = Rgb565::new(18, 36, 18);

// ---------------------------------------------------------------------------
// LogEntry
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct LogEntry {
    message: HeaplessString<64>,
}

// ---------------------------------------------------------------------------
// MonitorPage
// ---------------------------------------------------------------------------

pub struct MonitorPage {
    bounds: Rectangle,
    log_entries: Vec<LogEntry, MAX_LOG_ENTRIES>,
    last_temperature: Option<f32>,
    last_humidity: Option<f32>,
    last_co2: Option<f32>,
    last_lux: Option<f32>,
    dirty: bool,
}

impl MonitorPage {
    pub fn new(bounds: Rectangle) -> Self {
        Self {
            bounds,
            log_entries: Vec::new(),
            last_temperature: None,
            last_humidity: None,
            last_co2: None,
            last_lux: None,
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
            self.last_temperature = data.temperature;
            self.last_humidity = data.humidity;
            self.last_co2 = data.co2;
            self.last_lux = data.lux;
            self.dirty = true;
        }
    }

    fn back_touch_bounds(&self) -> Rectangle {
        Rectangle::new(
            self.bounds.top_left,
            Size::new(BACK_TOUCH_WIDTH, HEADER_HEIGHT_PX),
        )
    }

    fn add_log_entry(&mut self, message: &str) {
        let mut entry_text = HeaplessString::<64>::new();
        entry_text.push_str(message).ok();

        if self.log_entries.len() >= MAX_LOG_ENTRIES {
            for i in 0..(MAX_LOG_ENTRIES - 1) {
                let next = self.log_entries.get(i + 1).cloned();
                if let (Some(dst), Some(next)) = (self.log_entries.get_mut(i), next) {
                    *dst = next;
                }
            }
            if let Some(last) = self.log_entries.get_mut(MAX_LOG_ENTRIES - 1) {
                last.message = entry_text;
            }
        } else {
            self.log_entries
                .push(LogEntry {
                    message: entry_text,
                })
                .ok();
        }
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
        let _label_style = MonoTextStyle::new(&FONT_6X10, COLOR_MUTED_TEXT);

        // Row 1: Temperature + Humidity
        let mut buf = HeaplessString::<32>::new();
        if let Some(t) = self.last_temperature {
            let _ = write!(buf, "T: {:.1}C", t);
        } else {
            let _ = write!(buf, "T: --");
        }
        Text::new(&buf, Point::new(x, y_base + 12), text_style).draw(display)?;

        buf.clear();
        if let Some(h) = self.last_humidity {
            let _ = write!(buf, "H: {:.1}%", h);
        } else {
            let _ = write!(buf, "H: --");
        }
        Text::new(&buf, Point::new(x + 120, y_base + 12), text_style).draw(display)?;

        // Row 2: CO2 + Lux
        buf.clear();
        if let Some(c) = self.last_co2 {
            let _ = write!(buf, "CO2: {:.0}ppm", c);
        } else {
            let _ = write!(buf, "CO2: --");
        }
        Text::new(&buf, Point::new(x, y_base + 28), text_style).draw(display)?;

        buf.clear();
        if let Some(l) = self.last_lux {
            let _ = write!(buf, "Lux: {:.0}", l);
        } else {
            let _ = write!(buf, "Lux: --");
        }
        Text::new(&buf, Point::new(x + 120, y_base + 28), text_style).draw(display)?;

        // Separator line
        let sep_y = y_base + SENSOR_SECTION_HEIGHT as i32 - 2;
        Rectangle::new(
            Point::new(x, sep_y),
            Size::new(self.bounds.size.width.saturating_sub(PADDING_X * 2), 1),
        )
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

        let text_style = MonoTextStyle::new(&FONT_6X10, WHITE);
        let mut y = log_area.top_left.y + LOG_LINE_HEIGHT;
        let max_y = log_area.top_left.y + log_area.size.height as i32 - 2;

        for entry in self.log_entries.iter().rev() {
            if y > max_y {
                break;
            }
            Text::new(
                entry.message.as_str(),
                Point::new(log_area.top_left.x + LOG_TEXT_PADDING_LEFT, y),
                text_style,
            )
            .draw(display)?;
            y += LOG_LINE_HEIGHT;
        }

        Ok(())
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
                if let Some(temp) = data.temperature {
                    self.last_temperature = Some(temp);
                }
                if let Some(hum) = data.humidity {
                    self.last_humidity = Some(hum);
                }
                if let Some(co2) = data.co2 {
                    self.last_co2 = Some(co2);
                }
                if let Some(lux) = data.lux {
                    self.last_lux = Some(lux);
                }

                let mut log_msg = HeaplessString::<64>::new();
                if let Some(temp) = data.temperature {
                    let _ = write!(
                        log_msg,
                        "[Sensor] T:{:.1} H:{:.1} CO2:{:.0} L:{:.0}",
                        temp,
                        data.humidity.unwrap_or(0.0),
                        data.co2.unwrap_or(0.0),
                        data.lux.unwrap_or(0.0),
                    );
                }
                self.add_log_entry(&log_msg);

                self.dirty = true;
                true
            }
            PageEvent::StorageEvent(storage_event) => {
                match storage_event {
                    StorageEvent::RawSample { sensor, value, .. } => {
                        let mut log_msg = HeaplessString::<64>::new();
                        let _ = write!(log_msg, "[Raw] {}: {:.2}", sensor, value);
                        self.add_log_entry(&log_msg);
                    }
                    StorageEvent::Rollup {
                        interval, count, ..
                    } => {
                        let mut log_msg = HeaplessString::<64>::new();
                        let _ = write!(log_msg, "[Rollup] {}: {}", interval, count);
                        self.add_log_entry(&log_msg);
                    }
                }
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
