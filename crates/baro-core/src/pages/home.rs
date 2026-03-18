//! Home page with status banner and priority-sorted sensor list
//!
//! Displays a status banner showing overall air quality at a glance,
//! followed by a vertical list of sensor rows sorted worst-first.
//! Tapping a sensor row navigates to its trend page; tapping the
//! gear icon navigates to settings.
//!
//! When any sensor reaches `Bad` quality, an alert overlay appears
//! that must be manually dismissed (with a 5-minute per-sensor cooldown).

use core::fmt::Write;

use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::{FONT_6X10, FONT_10X20};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle,
};
use embedded_graphics::text::{Alignment, Text};

use crate::metrics::QualityLevel;
use crate::pages::page::Page;
use crate::sensors::SensorType;
use crate::ui::core::{Action, Drawable, PageEvent, PageId, TouchEvent};
use crate::ui::styling::{COLOR_BACKGROUND, COLOR_FOREGROUND, WHITE};

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

/// Height of the top header bar
const HEADER_HEIGHT_PX: u32 = 36;

/// Height of the status banner
const BANNER_HEIGHT_PX: u32 = 44;

/// Y position of the banner (header + gap)
const BANNER_Y_OFFSET: u32 = HEADER_HEIGHT_PX + 2;

/// Y position of the sensor list (banner bottom + gap)
const LIST_Y_OFFSET: u32 = BANNER_Y_OFFSET + BANNER_HEIGHT_PX + 2;

/// Height of each sensor row
const ROW_HEIGHT_PX: u32 = 36;

/// Vertical gap between rows
const ROW_GAP_PX: u32 = 2;

/// Horizontal padding for list area
const LIST_PADDING_X: u32 = 8;

/// Corner radius for cards and banner
const CORNER_RADIUS: u32 = 12;

/// Maximum number of sensors the home page can display
const MAX_HOME_SENSORS: usize = 8;

/// Alert cooldown period in seconds
const ALERT_COOLDOWN_SECS: u64 = 300;

/// Width/height of the settings gear icon touch target
const SETTINGS_TOUCH_WIDTH: u32 = 44;

/// Pill corner radius
const PILL_CORNER_RADIUS: u32 = 4;

// ---------------------------------------------------------------------------
// Quality bar constants
// ---------------------------------------------------------------------------

/// Number of segments in the quality bar
const QUALITY_BAR_SEGMENTS: usize = 4;

/// Width of each quality bar segment
const QUALITY_BAR_SEG_WIDTH: u32 = 6;

/// Height of each quality bar segment
const QUALITY_BAR_SEG_HEIGHT: u32 = 10;

/// Gap between quality bar segments
const QUALITY_BAR_GAP: u32 = 2;

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

/// Header text color (muted)
const COLOR_HEADER_TEXT: Rgb565 = Rgb565::new(20, 40, 20);

/// Muted text for labels
const COLOR_MUTED_TEXT: Rgb565 = Rgb565::new(18, 36, 18);

/// Semi-transparent overlay (dark)
const COLOR_OVERLAY: Rgb565 = Rgb565::new(5, 10, 5);

// ---------------------------------------------------------------------------
// Default sensor assignment
// ---------------------------------------------------------------------------

const DEFAULT_SENSORS: [SensorType; 4] = [
    SensorType::Temperature,
    SensorType::Humidity,
    SensorType::Co2,
    SensorType::Lux,
];

// ---------------------------------------------------------------------------
// SensorRow
// ---------------------------------------------------------------------------

/// A single sensor row in the priority-sorted list.
struct SensorRow {
    sensor: SensorType,
    quality: QualityLevel,
    latest_value: Option<f32>,
    dirty: bool,
}

impl SensorRow {
    fn new(sensor: SensorType) -> Self {
        Self {
            sensor,
            quality: QualityLevel::Good,
            latest_value: None,
            dirty: true,
        }
    }

    fn update_value(&mut self, value: f32) {
        let new_quality = QualityLevel::assess(self.sensor, value);
        if new_quality != self.quality || self.latest_value != Some(value) {
            self.dirty = true;
        }
        self.quality = new_quality;
        self.latest_value = Some(value);
    }

    /// Map this sensor to its TrendPage PageId
    fn trend_page_id(&self) -> PageId {
        match self.sensor {
            SensorType::Temperature => PageId::TrendTemperature,
            SensorType::Humidity => PageId::TrendHumidity,
            SensorType::Co2 => PageId::TrendCo2,
            SensorType::Lux => PageId::TrendLux,
        }
    }

    /// Draw a quality bar (filled segments based on quality level)
    fn draw_quality_bar<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        x: i32,
        y: i32,
    ) -> Result<(), D::Error> {
        let filled_count = match self.quality {
            QualityLevel::Bad => 1,
            QualityLevel::Poor => 2,
            QualityLevel::Good => 3,
            QualityLevel::Excellent => 4,
        };

        for i in 0..QUALITY_BAR_SEGMENTS {
            let seg_x = x + (i as u32 * (QUALITY_BAR_SEG_WIDTH + QUALITY_BAR_GAP)) as i32;
            let color = if i < filled_count {
                self.quality.foreground_color()
            } else {
                COLOR_MUTED_TEXT
            };

            Rectangle::new(
                Point::new(seg_x, y),
                Size::new(QUALITY_BAR_SEG_WIDTH, QUALITY_BAR_SEG_HEIGHT),
            )
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(display)?;
        }

        Ok(())
    }

    /// Draw the row at the given bounds
    fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        bounds: Rectangle,
    ) -> Result<(), D::Error> {
        // Row background
        RoundedRectangle::with_equal_corners(
            bounds,
            Size::new(PILL_CORNER_RADIUS, PILL_CORNER_RADIUS),
        )
        .into_styled(PrimitiveStyle::with_fill(COLOR_FOREGROUND))
        .draw(display)?;

        let row_center_y = bounds.top_left.y + (ROW_HEIGHT_PX / 2) as i32 + 4;
        let text_style = MonoTextStyle::new(&FONT_6X10, WHITE);

        // Alert indicator for Poor/Bad
        let label_x = bounds.top_left.x + 10;
        if self.quality.sort_key() <= 1 {
            // Poor or Bad — show alert triangle
            Text::with_alignment(
                self.quality.status_icon(),
                Point::new(label_x, row_center_y),
                MonoTextStyle::new(&FONT_6X10, self.quality.foreground_color()),
                Alignment::Left,
            )
            .draw(display)?;
        }

        // Sensor label
        let name_x = label_x + 14;
        Text::with_alignment(
            self.sensor.short_name(),
            Point::new(name_x, row_center_y),
            MonoTextStyle::new(&FONT_6X10, COLOR_MUTED_TEXT),
            Alignment::Left,
        )
        .draw(display)?;

        // Value (large, centered)
        if let Some(val) = self.latest_value {
            let mut buf = heapless::String::<16>::new();
            let _ = match self.sensor {
                SensorType::Temperature | SensorType::Humidity => {
                    write!(buf, "{:.1} {}", val, self.sensor.unit())
                }
                SensorType::Co2 | SensorType::Lux => {
                    write!(buf, "{:.0} {}", val, self.sensor.unit())
                }
            };

            let val_x = bounds.top_left.x + (bounds.size.width / 2) as i32 + 10;
            Text::with_alignment(
                &buf,
                Point::new(val_x, row_center_y),
                text_style,
                Alignment::Center,
            )
            .draw(display)?;
        }

        // Quality bar + label (right side)
        let quality_total_width =
            QUALITY_BAR_SEGMENTS as u32 * (QUALITY_BAR_SEG_WIDTH + QUALITY_BAR_GAP);
        let right_x = bounds.top_left.x + bounds.size.width as i32 - 10;
        let bar_x = right_x - quality_total_width as i32 - 30;
        let bar_y =
            bounds.top_left.y + (ROW_HEIGHT_PX / 2) as i32 - (QUALITY_BAR_SEG_HEIGHT / 2) as i32;

        self.draw_quality_bar(display, bar_x, bar_y)?;

        // Quality text label
        Text::with_alignment(
            self.quality.short_label(),
            Point::new(right_x, row_center_y),
            MonoTextStyle::new(&FONT_6X10, self.quality.foreground_color()),
            Alignment::Right,
        )
        .draw(display)?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// StatusBanner
// ---------------------------------------------------------------------------

/// Draws the overall status banner with color-coded background.
struct StatusBanner {
    overall_quality: QualityLevel,
    worst_sensor_name: &'static str,
    attention_count: u8,
    dirty: bool,
}

impl StatusBanner {
    fn new() -> Self {
        Self {
            overall_quality: QualityLevel::Good,
            worst_sensor_name: "",
            attention_count: 0,
            dirty: true,
        }
    }

    fn update(&mut self, rows: &[SensorRow], row_count: usize) {
        let qualities: heapless::Vec<QualityLevel, MAX_HOME_SENSORS> = rows[..row_count]
            .iter()
            .filter(|r| r.latest_value.is_some())
            .map(|r| r.quality)
            .collect();

        let new_quality = QualityLevel::worst(&qualities);
        let new_count = qualities.iter().filter(|q| q.sort_key() <= 1).count() as u8;

        // Find worst sensor name
        let worst_name = rows[..row_count]
            .iter()
            .filter(|r| r.latest_value.is_some())
            .min_by_key(|r| r.quality.sort_key())
            .map(|r| r.sensor.short_name())
            .unwrap_or("");

        if new_quality != self.overall_quality
            || new_count != self.attention_count
            || worst_name != self.worst_sensor_name
        {
            self.overall_quality = new_quality;
            self.attention_count = new_count;
            self.worst_sensor_name = worst_name;
            self.dirty = true;
        }
    }

    fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        bounds: Rectangle,
    ) -> Result<(), D::Error> {
        // Banner background with quality color
        RoundedRectangle::with_equal_corners(bounds, Size::new(CORNER_RADIUS, CORNER_RADIUS))
            .into_styled(PrimitiveStyle::with_fill(
                self.overall_quality.background_color(),
            ))
            .draw(display)?;

        // Main status line: "● ALL GOOD" or "▲ POOR — CO2"
        let mut status_buf = heapless::String::<32>::new();
        let _ = write!(
            status_buf,
            "{} {}",
            self.overall_quality.status_icon(),
            self.overall_quality.status_text()
        );

        if self.overall_quality.sort_key() <= 1 && !self.worst_sensor_name.is_empty() {
            let _ = write!(status_buf, " - {}", self.worst_sensor_name);
        }

        let center_x = bounds.top_left.x + (bounds.size.width / 2) as i32;
        let line1_y = bounds.top_left.y + 18;
        Text::with_alignment(
            &status_buf,
            Point::new(center_x, line1_y),
            MonoTextStyle::new(&FONT_6X10, self.overall_quality.foreground_color()),
            Alignment::Center,
        )
        .draw(display)?;

        // Subtitle
        let line2_y = line1_y + 16;
        if self.attention_count > 0 {
            let mut sub_buf = heapless::String::<32>::new();
            let _ = write!(
                sub_buf,
                "{} sensor{} need{} attention",
                self.attention_count,
                if self.attention_count > 1 { "s" } else { "" },
                if self.attention_count > 1 { "" } else { "s" },
            );
            Text::with_alignment(
                &sub_buf,
                Point::new(center_x, line2_y),
                MonoTextStyle::new(&FONT_6X10, COLOR_MUTED_TEXT),
                Alignment::Center,
            )
            .draw(display)?;
        } else {
            Text::with_alignment(
                "All sensors nominal",
                Point::new(center_x, line2_y),
                MonoTextStyle::new(&FONT_6X10, COLOR_MUTED_TEXT),
                Alignment::Center,
            )
            .draw(display)?;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// AlertOverlay
// ---------------------------------------------------------------------------

/// Modal overlay shown when a sensor reaches Bad quality.
struct AlertOverlay {
    active: bool,
    sensor: SensorType,
    value: f32,
    quality: QualityLevel,
    dismiss_bounds: Rectangle,
    cooldowns: [u64; MAX_HOME_SENSORS],
}

impl AlertOverlay {
    fn new() -> Self {
        Self {
            active: false,
            sensor: SensorType::Temperature,
            value: 0.0,
            quality: QualityLevel::Bad,
            dismiss_bounds: Rectangle::new(Point::new(120, 150), Size::new(80, 30)),
            cooldowns: [0; MAX_HOME_SENSORS],
        }
    }

    /// Check if an alert should be triggered for a sensor
    fn check_trigger(&mut self, rows: &[SensorRow], row_count: usize, timestamp: u64) {
        if self.active {
            return;
        }

        for row in &rows[..row_count] {
            if row.quality == QualityLevel::Bad
                && let Some(val) = row.latest_value
            {
                let sensor_idx = row.sensor.index();
                if sensor_idx < MAX_HOME_SENSORS
                    && timestamp.saturating_sub(self.cooldowns[sensor_idx]) >= ALERT_COOLDOWN_SECS
                {
                    self.active = true;
                    self.sensor = row.sensor;
                    self.value = val;
                    self.quality = row.quality;
                    return;
                }
            }
        }
    }

    /// Dismiss the current alert
    fn dismiss(&mut self, timestamp: u64) {
        if self.active {
            let idx = self.sensor.index();
            if idx < MAX_HOME_SENSORS {
                self.cooldowns[idx] = timestamp;
            }
            self.active = false;
        }
    }

    fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        page_bounds: Rectangle,
    ) -> Result<(), D::Error> {
        if !self.active {
            return Ok(());
        }

        // Semi-transparent overlay (draw a dark rectangle over the whole page)
        page_bounds
            .into_styled(PrimitiveStyle::with_fill(COLOR_OVERLAY))
            .draw(display)?;

        // Alert box centered on screen
        let box_width: u32 = 240;
        let box_height: u32 = 120;
        let box_x =
            page_bounds.top_left.x + (page_bounds.size.width.saturating_sub(box_width) / 2) as i32;
        let box_y = page_bounds.top_left.y
            + (page_bounds.size.height.saturating_sub(box_height) / 2) as i32;

        let alert_rect = Rectangle::new(Point::new(box_x, box_y), Size::new(box_width, box_height));

        // Alert box background
        RoundedRectangle::with_equal_corners(alert_rect, Size::new(CORNER_RADIUS, CORNER_RADIUS))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(self.quality.background_color())
                    .stroke_color(self.quality.foreground_color())
                    .stroke_width(2)
                    .build(),
            )
            .draw(display)?;

        let center_x = box_x + (box_width / 2) as i32;

        // Title: "⚠ CO₂ LEVEL HIGH"
        let mut title_buf = heapless::String::<32>::new();
        let _ = write!(title_buf, "! {} LEVEL HIGH", self.sensor.short_name());
        Text::with_alignment(
            &title_buf,
            Point::new(center_x, box_y + 30),
            MonoTextStyle::new(&FONT_6X10, self.quality.foreground_color()),
            Alignment::Center,
        )
        .draw(display)?;

        // Value
        let mut val_buf = heapless::String::<16>::new();
        let _ = match self.sensor {
            SensorType::Temperature | SensorType::Humidity => {
                write!(val_buf, "{:.1} {}", self.value, self.sensor.unit())
            }
            SensorType::Co2 | SensorType::Lux => {
                write!(val_buf, "{:.0} {}", self.value, self.sensor.unit())
            }
        };
        Text::with_alignment(
            &val_buf,
            Point::new(center_x, box_y + 58),
            MonoTextStyle::new(&FONT_10X20, WHITE),
            Alignment::Center,
        )
        .draw(display)?;

        // Dismiss button
        let btn_width: u32 = 80;
        let btn_height: u32 = 24;
        let btn_x = center_x - (btn_width / 2) as i32;
        let btn_y = box_y + box_height as i32 - btn_height as i32 - 12;

        let btn_rect = Rectangle::new(Point::new(btn_x, btn_y), Size::new(btn_width, btn_height));

        RoundedRectangle::with_equal_corners(
            btn_rect,
            Size::new(PILL_CORNER_RADIUS, PILL_CORNER_RADIUS),
        )
        .into_styled(
            PrimitiveStyleBuilder::new()
                .fill_color(COLOR_FOREGROUND)
                .stroke_color(self.quality.foreground_color())
                .stroke_width(1)
                .build(),
        )
        .draw(display)?;

        Text::with_alignment(
            "DISMISS",
            Point::new(center_x, btn_y + 16),
            MonoTextStyle::new(&FONT_6X10, WHITE),
            Alignment::Center,
        )
        .draw(display)?;

        // Store dismiss bounds for touch handling (we update via interior state pattern)
        // The dismiss_bounds is set in new() and stays constant since the overlay is centered

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// HomePage
// ---------------------------------------------------------------------------

/// Home page showing status banner and priority-sorted sensor list.
pub struct HomePage {
    bounds: Rectangle,
    banner: StatusBanner,
    rows: [SensorRow; MAX_HOME_SENSORS],
    row_count: usize,
    sort_order: [usize; MAX_HOME_SENSORS],
    alert: AlertOverlay,
    settings_touch_bounds: Rectangle,
    last_timestamp: u64,
    dirty: bool,
}

impl HomePage {
    pub fn new(bounds: Rectangle) -> Self {
        let rows = [
            SensorRow::new(DEFAULT_SENSORS[0]),
            SensorRow::new(DEFAULT_SENSORS[1]),
            SensorRow::new(DEFAULT_SENSORS[2]),
            SensorRow::new(DEFAULT_SENSORS[3]),
            SensorRow::new(SensorType::Temperature), // unused slots
            SensorRow::new(SensorType::Temperature),
            SensorRow::new(SensorType::Temperature),
            SensorRow::new(SensorType::Temperature),
        ];

        let settings_touch_bounds = Rectangle::new(
            Point::new(
                bounds.top_left.x + bounds.size.width as i32 - SETTINGS_TOUCH_WIDTH as i32,
                bounds.top_left.y,
            ),
            Size::new(SETTINGS_TOUCH_WIDTH, HEADER_HEIGHT_PX),
        );

        Self {
            bounds,
            banner: StatusBanner::new(),
            rows,
            row_count: 4,
            sort_order: [0, 1, 2, 3, 4, 5, 6, 7],
            alert: AlertOverlay::new(),
            settings_touch_bounds,
            last_timestamp: 0,
            dirty: true,
        }
    }

    /// Kept for API compatibility.
    pub fn init(&mut self) {
        self.dirty = true;
    }

    /// Recompute the sort order (worst quality first)
    fn recompute_sort_order(&mut self) {
        // Initialize with identity
        for i in 0..self.row_count {
            self.sort_order[i] = i;
        }

        // Simple insertion sort (small N)
        for i in 1..self.row_count {
            let mut j = i;
            while j > 0
                && self.rows[self.sort_order[j]].quality.sort_key()
                    < self.rows[self.sort_order[j - 1]].quality.sort_key()
            {
                self.sort_order.swap(j, j - 1);
                j -= 1;
            }
        }
    }

    /// Calculate the bounding rectangle for a row at the given visual position
    fn row_bounds(&self, visual_index: usize) -> Rectangle {
        let x = self.bounds.top_left.x + LIST_PADDING_X as i32;
        let y = self.bounds.top_left.y
            + LIST_Y_OFFSET as i32
            + (visual_index as u32 * (ROW_HEIGHT_PX + ROW_GAP_PX)) as i32;
        let width = self.bounds.size.width.saturating_sub(LIST_PADDING_X * 2);

        Rectangle::new(Point::new(x, y), Size::new(width, ROW_HEIGHT_PX))
    }

    fn draw_header<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        let header_rect = Rectangle::new(
            self.bounds.top_left,
            Size::new(self.bounds.size.width, HEADER_HEIGHT_PX),
        );

        RoundedRectangle::with_equal_corners(header_rect, Size::new(CORNER_RADIUS, CORNER_RADIUS))
            .into_styled(PrimitiveStyle::with_fill(COLOR_FOREGROUND))
            .draw(display)?;

        // Grid icon (4 small squares)
        let grid_x = self.bounds.top_left.x + 12;
        let grid_y = self.bounds.top_left.y + 10;
        let sq = 6u32;
        let gap: i32 = 2;
        let sq_style = PrimitiveStyle::with_fill(COLOR_HEADER_TEXT);

        for row in 0..2 {
            for col in 0..2 {
                Rectangle::new(
                    Point::new(
                        grid_x + col * (sq as i32 + gap),
                        grid_y + row * (sq as i32 + gap),
                    ),
                    Size::new(sq, sq),
                )
                .into_styled(sq_style)
                .draw(display)?;
            }
        }

        // Title
        Text::with_alignment(
            "AIR AROUND YOU",
            Point::new(
                self.bounds.top_left.x + 36,
                self.bounds.top_left.y + (HEADER_HEIGHT_PX / 2 + 4) as i32,
            ),
            MonoTextStyle::new(&FONT_6X10, COLOR_HEADER_TEXT),
            Alignment::Left,
        )
        .draw(display)?;

        // Settings gear icon (right side)
        let gear_x = self.bounds.top_left.x + self.bounds.size.width as i32 - 24;
        let gear_y = self.bounds.top_left.y + (HEADER_HEIGHT_PX / 2 + 4) as i32;
        Text::with_alignment(
            "*",
            Point::new(gear_x, gear_y),
            MonoTextStyle::new(&FONT_10X20, COLOR_HEADER_TEXT),
            Alignment::Center,
        )
        .draw(display)?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Page trait
// ---------------------------------------------------------------------------

impl Page for HomePage {
    fn id(&self) -> PageId {
        PageId::Home
    }

    fn title(&self) -> &str {
        "Home"
    }

    fn on_activate(&mut self) {
        self.dirty = true;
    }

    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action> {
        let point = match event {
            TouchEvent::Press(p) => p,
            TouchEvent::Drag(_) => return None,
        };

        let pt = point.to_point();

        // If alert overlay is active, only handle dismiss
        if self.alert.active {
            if self.alert.dismiss_bounds.contains(pt) {
                self.alert.dismiss(self.last_timestamp);
                self.dirty = true;
                return None;
            }
            // Block all other touches while alert is shown
            return None;
        }

        // Settings gear
        if self.settings_touch_bounds.contains(pt) {
            return Some(Action::NavigateToPage(PageId::Settings));
        }

        // Sensor rows — check each visual row
        for visual_idx in 0..self.row_count {
            let row_rect = self.row_bounds(visual_idx);
            if row_rect.contains(pt) {
                let data_idx = self.sort_order[visual_idx];
                return Some(Action::NavigateToPage(self.rows[data_idx].trend_page_id()));
            }
        }

        None
    }

    fn update(&mut self) {}

    fn on_event(&mut self, event: &PageEvent) -> bool {
        match event {
            PageEvent::SensorUpdate(data) => {
                self.last_timestamp = data.timestamp;

                if let Some(temp) = data.temperature {
                    self.rows[0].update_value(temp);
                }
                if let Some(hum) = data.humidity {
                    self.rows[1].update_value(hum);
                }
                if let Some(co2) = data.co2 {
                    self.rows[2].update_value(co2);
                }
                if let Some(lux) = data.lux {
                    self.rows[3].update_value(lux);
                }

                self.recompute_sort_order();
                self.banner.update(&self.rows, self.row_count);
                self.alert
                    .check_trigger(&self.rows, self.row_count, data.timestamp);

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

impl Drawable for HomePage {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        if !self.dirty {
            return Ok(());
        }

        display.clear(COLOR_BACKGROUND)?;

        // Header
        self.draw_header(display)?;

        // Status banner
        let banner_rect = Rectangle::new(
            Point::new(
                self.bounds.top_left.x + LIST_PADDING_X as i32,
                self.bounds.top_left.y + BANNER_Y_OFFSET as i32,
            ),
            Size::new(
                self.bounds.size.width.saturating_sub(LIST_PADDING_X * 2),
                BANNER_HEIGHT_PX,
            ),
        );
        self.banner.draw(display, banner_rect)?;

        // Sensor rows (sorted)
        for visual_idx in 0..self.row_count {
            let data_idx = self.sort_order[visual_idx];
            let row_rect = self.row_bounds(visual_idx);
            self.rows[data_idx].draw(display, row_rect)?;
        }

        // Alert overlay (drawn last, on top)
        self.alert.draw(display, self.bounds)?;

        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    fn is_dirty(&self) -> bool {
        self.dirty || self.banner.dirty || self.rows.iter().any(|r| r.dirty)
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
        self.banner.dirty = false;
        for row in &mut self.rows {
            row.dirty = false;
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.banner.dirty = true;
        for row in &mut self.rows {
            row.dirty = true;
        }
    }
}
