// src/pages/home_grid.rs
//! Home Grid page — a 2×2 grid of sensor cards with mini-graphs.
//!
//! Designed for stationary indoor use. Each card shows the sensor name,
//! current value, quality level, and a small trend sparkline. Tapping
//! a card navigates to its full TrendPage.

use core::fmt::Write;

use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle, RoundedRectangle};
use embedded_graphics::text::{Alignment, Text};

use crate::metrics::QualityLevel;
use crate::pages::page::Page;
use crate::sensors::SensorType;
use crate::ui::Drawable;
use crate::ui::core::{Action, PageEvent, PageId, TouchEvent};
use crate::ui::styling::{COLOR_BACKGROUND, COLOR_FOREGROUND, WHITE};

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

/// Height of the top header bar
const HEADER_HEIGHT_PX: u32 = 36;

/// Corner radius for cards
const CORNER_RADIUS: u32 = 12;

/// Gap between the header and the grid
const GRID_Y_OFFSET: u32 = HEADER_HEIGHT_PX + 4;

/// Horizontal gap between grid cards
const GRID_GAP_X: u32 = 4;

/// Vertical gap between grid cards
const GRID_GAP_Y: u32 = 4;

/// Horizontal padding around the grid
const GRID_PADDING_X: u32 = 4;

/// Pill corner radius for cards
const CARD_CORNER_RADIUS: u32 = 8;

/// Settings gear icon touch target width
const SETTINGS_TOUCH_WIDTH: u32 = 44;

/// Maximum number of sparkline points per card
const SPARKLINE_MAX_POINTS: usize = 30;

/// Height allocated for the sparkline within a card
const SPARKLINE_HEIGHT_PX: u32 = 40;

/// Sparkline horizontal padding within card
const SPARKLINE_PADDING_X: u32 = 6;

/// Sparkline bottom margin within card
const SPARKLINE_BOTTOM_MARGIN: u32 = 4;

/// Header text color (muted)
const COLOR_HEADER_TEXT: Rgb565 = Rgb565::new(20, 40, 20);

/// Muted text for labels
const COLOR_MUTED_TEXT: Rgb565 = Rgb565::new(18, 36, 18);

/// Number of sensors displayed in the grid
const GRID_SENSOR_COUNT: usize = 4;

// ---------------------------------------------------------------------------
// Sensor assignment (same order as HomePage)
// ---------------------------------------------------------------------------

const GRID_SENSORS: [SensorType; GRID_SENSOR_COUNT] = [
    SensorType::Temperature,
    SensorType::Humidity,
    SensorType::Co2,
    SensorType::Lux,
];

// ---------------------------------------------------------------------------
// SensorCard
// ---------------------------------------------------------------------------

/// A single card in the 2×2 grid showing sensor data and a sparkline.
struct SensorCard {
    sensor: SensorType,
    quality: QualityLevel,
    latest_value: Option<f32>,
    /// Ring buffer of recent values for sparkline rendering
    sparkline: [Option<f32>; SPARKLINE_MAX_POINTS],
    sparkline_count: usize,
    sparkline_head: usize,
    dirty: bool,
}

impl SensorCard {
    fn new(sensor: SensorType) -> Self {
        Self {
            sensor,
            quality: QualityLevel::Good,
            latest_value: None,
            sparkline: [None; SPARKLINE_MAX_POINTS],
            sparkline_count: 0,
            sparkline_head: 0,
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

        // Push into sparkline ring buffer
        self.sparkline[self.sparkline_head] = Some(value);
        self.sparkline_head = (self.sparkline_head + 1) % SPARKLINE_MAX_POINTS;
        if self.sparkline_count < SPARKLINE_MAX_POINTS {
            self.sparkline_count += 1;
        }
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

    /// Draw the card at the given bounds
    fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        bounds: Rectangle,
    ) -> Result<(), D::Error> {
        // Card background with quality-tinted color
        RoundedRectangle::with_equal_corners(
            bounds,
            Size::new(CARD_CORNER_RADIUS, CARD_CORNER_RADIUS),
        )
        .into_styled(PrimitiveStyle::with_fill(COLOR_FOREGROUND))
        .draw(display)?;

        // Sensor name (top-left)
        let name_y = bounds.top_left.y + 14;
        Text::with_alignment(
            self.sensor.short_name(),
            Point::new(bounds.top_left.x + 8, name_y),
            MonoTextStyle::new(&FONT_6X10, COLOR_MUTED_TEXT),
            Alignment::Left,
        )
        .draw(display)?;

        // Quality label (top-right)
        Text::with_alignment(
            self.quality.short_label(),
            Point::new(bounds.top_left.x + bounds.size.width as i32 - 8, name_y),
            MonoTextStyle::new(&FONT_6X10, self.quality.foreground_color()),
            Alignment::Right,
        )
        .draw(display)?;

        // Current value (large, centered below name)
        if let Some(val) = self.latest_value {
            let mut buf = heapless::String::<16>::new();
            let _ = match self.sensor {
                SensorType::Temperature | SensorType::Humidity => {
                    write!(buf, "{:.1}", val)
                }
                SensorType::Co2 | SensorType::Lux => {
                    write!(buf, "{:.0}", val)
                }
            };

            let val_y = name_y + 16;
            Text::with_alignment(
                &buf,
                Point::new(bounds.top_left.x + 8, val_y),
                MonoTextStyle::new(&FONT_6X10, WHITE),
                Alignment::Left,
            )
            .draw(display)?;

            // Unit
            Text::with_alignment(
                self.sensor.unit(),
                Point::new(bounds.top_left.x + bounds.size.width as i32 - 8, val_y),
                MonoTextStyle::new(&FONT_6X10, COLOR_MUTED_TEXT),
                Alignment::Right,
            )
            .draw(display)?;
        }

        // Sparkline
        self.draw_sparkline(display, bounds)?;

        Ok(())
    }

    /// Draw a simple sparkline at the bottom of the card
    fn draw_sparkline<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        card_bounds: Rectangle,
    ) -> Result<(), D::Error> {
        if self.sparkline_count < 2 {
            return Ok(());
        }

        let spark_x = card_bounds.top_left.x + SPARKLINE_PADDING_X as i32;
        let spark_width = card_bounds
            .size
            .width
            .saturating_sub(SPARKLINE_PADDING_X * 2) as i32;
        let spark_y_bottom = card_bounds.top_left.y + card_bounds.size.height as i32
            - SPARKLINE_BOTTOM_MARGIN as i32;
        let _spark_y_top = spark_y_bottom - SPARKLINE_HEIGHT_PX as i32;

        // Collect valid values in order (oldest first)
        let mut values: heapless::Vec<f32, SPARKLINE_MAX_POINTS> = heapless::Vec::new();
        for i in 0..self.sparkline_count {
            let idx = if self.sparkline_count < SPARKLINE_MAX_POINTS {
                i
            } else {
                (self.sparkline_head + i) % SPARKLINE_MAX_POINTS
            };
            if let Some(v) = self.sparkline[idx] {
                let _ = values.push(v);
            }
        }

        if values.len() < 2 {
            return Ok(());
        }

        // Find min/max for scaling
        let mut min_val = values[0];
        let mut max_val = values[0];
        for &v in &values {
            if v < min_val {
                min_val = v;
            }
            if v > max_val {
                max_val = v;
            }
        }

        let range = max_val - min_val;
        let range = if range < 0.001 { 1.0 } else { range };

        let line_color = self.quality.foreground_color();

        // Draw line segments between consecutive points
        let point_count = values.len();
        for i in 0..(point_count - 1) {
            let x1 = spark_x + (i as i32 * spark_width) / (point_count as i32 - 1);
            let x2 = spark_x + ((i + 1) as i32 * spark_width) / (point_count as i32 - 1);

            let y1 = spark_y_bottom
                - ((values[i] - min_val) / range * SPARKLINE_HEIGHT_PX as f32) as i32;
            let y2 = spark_y_bottom
                - ((values[i + 1] - min_val) / range * SPARKLINE_HEIGHT_PX as f32) as i32;

            // Simple line drawing using Bresenham-style pixel plotting
            draw_line(display, x1, y1, x2, y2, line_color)?;
        }

        Ok(())
    }
}

/// Draw a 1px line between two points using incremental steps.
fn draw_line<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: Rgb565,
) -> Result<(), D::Error> {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut cx = x0;
    let mut cy = y0;

    let style = PrimitiveStyle::with_fill(color);

    loop {
        Rectangle::new(Point::new(cx, cy), Size::new(1, 1))
            .into_styled(style)
            .draw(display)?;

        if cx == x1 && cy == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            if cx == x1 {
                break;
            }
            err += dy;
            cx += sx;
        }
        if e2 <= dx {
            if cy == y1 {
                break;
            }
            err += dx;
            cy += sy;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// HomeGridPage
// ---------------------------------------------------------------------------

/// Home Grid page showing a 2×2 grid of sensor cards with mini sparklines.
pub struct HomeGridPage {
    bounds: Rectangle,
    cards: [SensorCard; GRID_SENSOR_COUNT],
    settings_touch_bounds: Rectangle,
    dirty: bool,
}

impl HomeGridPage {
    pub fn new(bounds: Rectangle) -> Self {
        let cards = [
            SensorCard::new(GRID_SENSORS[0]),
            SensorCard::new(GRID_SENSORS[1]),
            SensorCard::new(GRID_SENSORS[2]),
            SensorCard::new(GRID_SENSORS[3]),
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
            cards,
            settings_touch_bounds,
            dirty: true,
        }
    }

    /// Calculate the bounding rectangle for a card at grid position (row, col).
    fn card_bounds(&self, row: usize, col: usize) -> Rectangle {
        let available_width = self
            .bounds
            .size
            .width
            .saturating_sub(GRID_PADDING_X * 2 + GRID_GAP_X);
        let card_width = available_width / 2;

        let available_height = self
            .bounds
            .size
            .height
            .saturating_sub(GRID_Y_OFFSET + GRID_GAP_Y);
        let card_height = available_height / 2;

        let x = self.bounds.top_left.x
            + GRID_PADDING_X as i32
            + (col as u32 * (card_width + GRID_GAP_X)) as i32;
        let y = self.bounds.top_left.y
            + GRID_Y_OFFSET as i32
            + (row as u32 * (card_height + GRID_GAP_Y)) as i32;

        Rectangle::new(Point::new(x, y), Size::new(card_width, card_height))
    }

    /// Map a flat card index (0–3) to (row, col)
    fn card_grid_position(index: usize) -> (usize, usize) {
        (index / 2, index % 2)
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
            "HOME",
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
            MonoTextStyle::new(
                &embedded_graphics::mono_font::ascii::FONT_10X20,
                COLOR_HEADER_TEXT,
            ),
            Alignment::Center,
        )
        .draw(display)?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Page trait
// ---------------------------------------------------------------------------

impl Page for HomeGridPage {
    fn id(&self) -> PageId {
        PageId::HomeGrid
    }

    fn title(&self) -> &str {
        "Home Grid"
    }

    fn on_activate(&mut self) {
        self.dirty = true;
    }

    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action> {
        if let TouchEvent::Press(point) = event {
            let pt = point.to_point();

            // Settings gear
            if self.settings_touch_bounds.contains(pt) {
                return Some(Action::NavigateToPage(PageId::Settings));
            }

            // Check each card
            for i in 0..GRID_SENSOR_COUNT {
                let (row, col) = Self::card_grid_position(i);
                let card_rect = self.card_bounds(row, col);
                if card_rect.contains(pt) {
                    return Some(Action::NavigateToPage(self.cards[i].trend_page_id()));
                }
            }
        }
        None
    }

    fn update(&mut self) {}

    fn on_event(&mut self, event: &PageEvent) -> bool {
        match event {
            PageEvent::SensorUpdate(data) => {
                if let Some(temp) = data.temperature {
                    self.cards[0].update_value(temp);
                }
                if let Some(hum) = data.humidity {
                    self.cards[1].update_value(hum);
                }
                if let Some(co2) = data.co2 {
                    self.cards[2].update_value(co2);
                }
                if let Some(lux) = data.lux {
                    self.cards[3].update_value(lux);
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

impl Drawable for HomeGridPage {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        if !self.dirty {
            return Ok(());
        }

        display.clear(COLOR_BACKGROUND)?;

        self.draw_header(display)?;

        // Draw 2×2 grid of sensor cards
        for i in 0..GRID_SENSOR_COUNT {
            let (row, col) = Self::card_grid_position(i);
            let card_rect = self.card_bounds(row, col);
            self.cards[i].draw(display, card_rect)?;
        }

        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    fn is_dirty(&self) -> bool {
        self.dirty || self.cards.iter().any(|c| c.dirty)
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
        for card in &mut self.cards {
            card.dirty = false;
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        for card in &mut self.cards {
            card.dirty = true;
        }
    }
}
