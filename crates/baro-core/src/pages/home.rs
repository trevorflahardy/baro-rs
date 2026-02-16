//! Home page with 2×2 sensor mini-graph grid
//!
//! When sensor data is flowing, this page renders a 2×2 grid of small graph
//! cards — one per sensor type — each showing a compact trend line whose
//! color reflects the sensor's current quality level.
//!
//! Tapping a card navigates to that sensor's full trend page.

use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle,
};
use embedded_graphics::text::{Alignment, Text};

use crate::metrics::QualityLevel;
use crate::pages::page::Page;
use crate::sensors::SensorType;
use crate::ui::components::graph::{DataPoint, DataSeries, Graph, InterpolationType, SeriesStyle};
use crate::ui::core::{Action, Drawable, PageEvent, PageId, TouchEvent};
use crate::ui::styling::{COLOR_BACKGROUND, COLOR_FOREGROUND, WHITE};

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

/// Height of the top header bar
const HEADER_HEIGHT_PX: u32 = 36;

/// Padding around the grid area
const GRID_PADDING_PX: u32 = 8;

/// Gap between grid cells
const GRID_GAP_PX: u32 = 8;

/// Corner radius of each card
const CARD_CORNER_RADIUS: u32 = 12;

/// Border width of each card
const CARD_BORDER_WIDTH: u32 = 1;

/// Vertical inset inside each card for the graph
const CARD_GRAPH_TOP_INSET: u32 = 22;

/// Left inset inside each card for the graph
const CARD_GRAPH_LEFT_INSET: u32 = 4;

/// Right inset inside each card for the graph
const CARD_GRAPH_RIGHT_INSET: u32 = 4;

/// Bottom inset inside each card for the graph
const CARD_GRAPH_BOTTOM_INSET: u32 = 4;

/// Maximum data points stored per mini-graph
const MINI_GRAPH_MAX_POINTS: usize = 64;

/// Header corner radius
const HEADER_CORNER_RADIUS: u32 = 12;

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

/// Header text color (muted)
const COLOR_HEADER_TEXT: Rgb565 = Rgb565::new(20, 40, 20);

/// Muted text for sensor labels
const COLOR_LABEL_TEXT: Rgb565 = Rgb565::new(18, 36, 18);

// ---------------------------------------------------------------------------
// SensorCard
// ---------------------------------------------------------------------------

/// A single sensor card in the 2×2 grid.
struct SensorCard {
    sensor: SensorType,
    bounds: Rectangle,
    graph: Graph<1, MINI_GRAPH_MAX_POINTS>,
    quality: QualityLevel,
    latest_value: Option<f32>,
    point_count: u32,
    dirty: bool,
}

impl SensorCard {
    fn new(sensor: SensorType, bounds: Rectangle) -> Self {
        let graph_bounds = Rectangle::new(
            Point::new(
                bounds.top_left.x + CARD_GRAPH_LEFT_INSET as i32,
                bounds.top_left.y + CARD_GRAPH_TOP_INSET as i32,
            ),
            Size::new(
                bounds
                    .size
                    .width
                    .saturating_sub(CARD_GRAPH_LEFT_INSET + CARD_GRAPH_RIGHT_INSET),
                bounds
                    .size
                    .height
                    .saturating_sub(CARD_GRAPH_TOP_INSET + CARD_GRAPH_BOTTOM_INSET),
            ),
        );

        let quality = QualityLevel::Good;
        let mut graph = Graph::<1, MINI_GRAPH_MAX_POINTS>::new(graph_bounds)
            .with_background(quality.background_color());

        let series = DataSeries::new()
            .with_style(SeriesStyle {
                color: quality.foreground_color(),
                line_width: 2,
                show_points: false,
                fill: None,
            })
            .with_interpolation(InterpolationType::Smooth { tension: 0.5 });

        let _ = graph.add_series(series);

        Self {
            sensor,
            bounds,
            graph,
            quality,
            latest_value: None,
            point_count: 0,
            dirty: true,
        }
    }

    /// Push a new sensor reading.
    fn push_value(&mut self, value: f32, timestamp: u64) {
        let new_quality = QualityLevel::assess(self.sensor, value);

        if new_quality != self.quality {
            self.quality = new_quality;
            self.graph.set_background(new_quality.background_color());
            let _ = self.graph.set_series_style(
                0,
                SeriesStyle {
                    color: new_quality.foreground_color(),
                    line_width: 2,
                    show_points: false,
                    fill: None,
                },
            );
        }

        let point = DataPoint {
            x: timestamp as f32,
            y: value,
        };
        let _ = self.graph.push_point(0, point);

        self.latest_value = Some(value);
        self.point_count += 1;
        self.dirty = true;
    }

    /// Draw the card.
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        let card_fill = PrimitiveStyleBuilder::new()
            .fill_color(COLOR_FOREGROUND)
            .stroke_color(self.quality.foreground_color())
            .stroke_width(CARD_BORDER_WIDTH)
            .build();

        RoundedRectangle::with_equal_corners(
            self.bounds,
            Size::new(CARD_CORNER_RADIUS, CARD_CORNER_RADIUS),
        )
        .into_styled(card_fill)
        .draw(display)?;

        // Sensor label (top-left of card)
        let label_x = self.bounds.top_left.x + 8;
        let label_y = self.bounds.top_left.y + 14;
        Text::with_alignment(
            self.sensor.short_name(),
            Point::new(label_x, label_y),
            MonoTextStyle::new(&FONT_6X10, self.quality.foreground_color()),
            Alignment::Left,
        )
        .draw(display)?;

        // Current value (top-right of card)
        if let Some(val) = self.latest_value {
            let mut buf = heapless::String::<16>::new();
            use core::fmt::Write;
            let _ = write!(buf, "{:.1}{}", val, self.sensor.unit());

            let val_x = self.bounds.top_left.x + self.bounds.size.width as i32 - 8;
            Text::with_alignment(
                &buf,
                Point::new(val_x, label_y),
                MonoTextStyle::new(&FONT_6X10, WHITE),
                Alignment::Right,
            )
            .draw(display)?;
        }

        // Mini graph
        if self.point_count > 0 {
            self.graph.draw(display)?;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Sensor assignment for the 2×2 grid
// ---------------------------------------------------------------------------

const DEFAULT_SENSORS: [SensorType; 4] = [
    SensorType::Temperature,
    SensorType::Humidity,
    SensorType::Co2,
    SensorType::Lux,
];

// ---------------------------------------------------------------------------
// HomePage
// ---------------------------------------------------------------------------

/// Home page showing a 2×2 grid of sensor mini-graphs.
pub struct HomePage {
    bounds: Rectangle,
    cards: [SensorCard; 4],
    dirty: bool,
}

impl HomePage {
    /// Calculate the bounding rectangles for each of the four cards.
    fn card_bounds(page_bounds: Rectangle) -> [Rectangle; 4] {
        let content_y = page_bounds.top_left.y + HEADER_HEIGHT_PX as i32 + GRID_PADDING_PX as i32;
        let content_x = page_bounds.top_left.x + GRID_PADDING_PX as i32;
        let usable_width = page_bounds
            .size
            .width
            .saturating_sub(GRID_PADDING_PX * 2 + GRID_GAP_PX);
        let usable_height = page_bounds
            .size
            .height
            .saturating_sub(HEADER_HEIGHT_PX + GRID_PADDING_PX * 2 + GRID_GAP_PX);

        let card_w = usable_width / 2;
        let card_h = usable_height / 2;

        [
            Rectangle::new(Point::new(content_x, content_y), Size::new(card_w, card_h)),
            Rectangle::new(
                Point::new(content_x + card_w as i32 + GRID_GAP_PX as i32, content_y),
                Size::new(card_w, card_h),
            ),
            Rectangle::new(
                Point::new(content_x, content_y + card_h as i32 + GRID_GAP_PX as i32),
                Size::new(card_w, card_h),
            ),
            Rectangle::new(
                Point::new(
                    content_x + card_w as i32 + GRID_GAP_PX as i32,
                    content_y + card_h as i32 + GRID_GAP_PX as i32,
                ),
                Size::new(card_w, card_h),
            ),
        ]
    }

    pub fn new(bounds: Rectangle) -> Self {
        let card_rects = Self::card_bounds(bounds);

        let cards = [
            SensorCard::new(DEFAULT_SENSORS[0], card_rects[0]),
            SensorCard::new(DEFAULT_SENSORS[1], card_rects[1]),
            SensorCard::new(DEFAULT_SENSORS[2], card_rects[2]),
            SensorCard::new(DEFAULT_SENSORS[3], card_rects[3]),
        ];

        Self {
            bounds,
            cards,
            dirty: true,
        }
    }

    /// Kept for API compatibility — does nothing extra now.
    pub fn init(&mut self) {
        self.dirty = true;
    }

    fn draw_header<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        let header_rect = Rectangle::new(
            self.bounds.top_left,
            Size::new(self.bounds.size.width, HEADER_HEIGHT_PX),
        );

        RoundedRectangle::with_equal_corners(
            header_rect,
            Size::new(HEADER_CORNER_RADIUS, HEADER_CORNER_RADIUS),
        )
        .into_styled(PrimitiveStyle::with_fill(COLOR_FOREGROUND))
        .draw(display)?;

        // Grid icon (4 small squares)
        let grid_x = self.bounds.top_left.x + 12;
        let grid_y = self.bounds.top_left.y + 10;
        let sq = 6u32;
        let gap: i32 = 2;
        let sq_style = PrimitiveStyle::with_fill(COLOR_LABEL_TEXT);

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
            "HOME AIR",
            Point::new(
                self.bounds.top_left.x + 36,
                self.bounds.top_left.y + (HEADER_HEIGHT_PX / 2 + 4) as i32,
            ),
            MonoTextStyle::new(&FONT_6X10, COLOR_HEADER_TEXT),
            Alignment::Left,
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

        for card in &self.cards {
            if card.bounds.contains(point.to_point()) {
                let page_id = match card.sensor {
                    SensorType::Temperature => PageId::TrendTemperature,
                    SensorType::Humidity => PageId::TrendHumidity,
                    SensorType::Co2 => PageId::TrendCo2,
                    SensorType::Lux => PageId::Settings, // No lux trend page yet
                };
                return Some(Action::NavigateToPage(page_id));
            }
        }

        None
    }

    fn update(&mut self) {}

    fn on_event(&mut self, event: &PageEvent) -> bool {
        match event {
            PageEvent::SensorUpdate(data) => {
                let ts = data.timestamp;

                if let Some(temp) = data.temperature {
                    self.cards[0].push_value(temp, ts);
                }
                if let Some(hum) = data.humidity {
                    self.cards[1].push_value(hum, ts);
                }
                if let Some(co2) = data.co2 {
                    self.cards[2].push_value(co2, ts);
                }
                // Lux not in SensorData yet — card[3] stays empty

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

        self.draw_header(display)?;

        for card in &self.cards {
            card.draw(display)?;
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
