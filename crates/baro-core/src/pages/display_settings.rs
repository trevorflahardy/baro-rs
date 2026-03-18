// src/pages/display_settings.rs
//! Display settings sub-page with home page mode selector.
//!
//! Shows a radio-button style selector for Hiking vs Home mode.
//! Tapping an option emits `Action::UpdateHomePageMode`.

use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle,
};
use embedded_graphics::text::{Alignment, Text};

use crate::config::HomePageMode;
use crate::pages::page::Page;
use crate::ui::Drawable;
use crate::ui::core::{Action, PageEvent, PageId, TouchEvent};
use crate::ui::styling::{COLOR_BACKGROUND, COLOR_FOREGROUND, WHITE};

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

/// Height of the header bar
const HEADER_HEIGHT_PX: u32 = 36;

/// Corner radius for rounded elements
const CORNER_RADIUS: u32 = 12;

/// Pill corner radius for option cards
const PILL_CORNER_RADIUS: u32 = 6;

/// Height of each option card
const OPTION_HEIGHT_PX: u32 = 44;

/// Vertical gap between option cards
const OPTION_GAP_PX: u32 = 4;

/// Horizontal padding
const PADDING_X: u32 = 8;

/// Y offset for the section label
const SECTION_LABEL_Y_OFFSET: u32 = HEADER_HEIGHT_PX + 16;

/// Y offset for the first option card
const OPTIONS_Y_OFFSET: u32 = SECTION_LABEL_Y_OFFSET + 20;

/// Radio button outer diameter
const RADIO_OUTER_DIAMETER: u32 = 12;

/// Radio button inner diameter (filled when selected)
const RADIO_INNER_DIAMETER: u32 = 6;

/// Header text color (muted)
const COLOR_HEADER_TEXT: Rgb565 = Rgb565::new(20, 40, 20);

/// Muted text for secondary labels
const COLOR_MUTED_TEXT: Rgb565 = Rgb565::new(18, 36, 18);

/// Accent color for selected option
const COLOR_ACCENT: Rgb565 = Rgb565::new(8, 40, 12);

/// Back button touch target width
const BACK_TOUCH_WIDTH: u32 = 44;

// ---------------------------------------------------------------------------
// DisplaySettingsPage
// ---------------------------------------------------------------------------

pub struct DisplaySettingsPage {
    bounds: Rectangle,
    selected_mode: HomePageMode,
    dirty: bool,
}

impl DisplaySettingsPage {
    pub fn new(bounds: Rectangle, current_mode: HomePageMode) -> Self {
        Self {
            bounds,
            selected_mode: current_mode,
            dirty: true,
        }
    }

    /// Calculate the bounding rectangle of an option card by index.
    fn option_bounds(&self, index: usize) -> Rectangle {
        let x = self.bounds.top_left.x + PADDING_X as i32;
        let y = self.bounds.top_left.y
            + OPTIONS_Y_OFFSET as i32
            + (index as u32 * (OPTION_HEIGHT_PX + OPTION_GAP_PX)) as i32;
        let width = self.bounds.size.width.saturating_sub(PADDING_X * 2);

        Rectangle::new(Point::new(x, y), Size::new(width, OPTION_HEIGHT_PX))
    }

    /// Back button touch bounds (top-left of header)
    fn back_touch_bounds(&self) -> Rectangle {
        Rectangle::new(
            self.bounds.top_left,
            Size::new(BACK_TOUCH_WIDTH, HEADER_HEIGHT_PX),
        )
    }

    fn draw_header<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        let header_rect = Rectangle::new(
            self.bounds.top_left,
            Size::new(self.bounds.size.width, HEADER_HEIGHT_PX),
        );

        RoundedRectangle::with_equal_corners(header_rect, Size::new(CORNER_RADIUS, CORNER_RADIUS))
            .into_styled(PrimitiveStyle::with_fill(COLOR_FOREGROUND))
            .draw(display)?;

        // Back arrow
        let text_y = self.bounds.top_left.y + (HEADER_HEIGHT_PX / 2 + 4) as i32;
        Text::with_alignment(
            "<",
            Point::new(self.bounds.top_left.x + 12, text_y),
            MonoTextStyle::new(&FONT_6X10, COLOR_HEADER_TEXT),
            Alignment::Left,
        )
        .draw(display)?;

        // Title
        Text::with_alignment(
            "DISPLAY",
            Point::new(self.bounds.top_left.x + 28, text_y),
            MonoTextStyle::new(&FONT_6X10, COLOR_HEADER_TEXT),
            Alignment::Left,
        )
        .draw(display)?;

        Ok(())
    }

    fn draw_option<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        index: usize,
        mode: HomePageMode,
        label: &str,
        subtitle: &str,
    ) -> Result<(), D::Error> {
        let bounds = self.option_bounds(index);
        let is_selected = self.selected_mode == mode;

        // Card background — accent tint when selected
        let bg_color = if is_selected {
            COLOR_ACCENT
        } else {
            COLOR_FOREGROUND
        };

        RoundedRectangle::with_equal_corners(
            bounds,
            Size::new(PILL_CORNER_RADIUS, PILL_CORNER_RADIUS),
        )
        .into_styled(PrimitiveStyle::with_fill(bg_color))
        .draw(display)?;

        // Radio button
        let radio_x = bounds.top_left.x + 16;
        let radio_y = bounds.top_left.y + (OPTION_HEIGHT_PX / 2) as i32;

        // Outer circle
        Circle::new(
            Point::new(
                radio_x - (RADIO_OUTER_DIAMETER / 2) as i32,
                radio_y - (RADIO_OUTER_DIAMETER / 2) as i32,
            ),
            RADIO_OUTER_DIAMETER,
        )
        .into_styled(
            PrimitiveStyleBuilder::new()
                .stroke_color(WHITE)
                .stroke_width(1)
                .build(),
        )
        .draw(display)?;

        // Inner circle (filled when selected)
        if is_selected {
            Circle::new(
                Point::new(
                    radio_x - (RADIO_INNER_DIAMETER / 2) as i32,
                    radio_y - (RADIO_INNER_DIAMETER / 2) as i32,
                ),
                RADIO_INNER_DIAMETER,
            )
            .into_styled(PrimitiveStyle::with_fill(WHITE))
            .draw(display)?;
        }

        // Label text
        let label_x = radio_x + (RADIO_OUTER_DIAMETER / 2) as i32 + 10;
        let label_y = bounds.top_left.y + 18;
        Text::with_alignment(
            label,
            Point::new(label_x, label_y),
            MonoTextStyle::new(&FONT_6X10, WHITE),
            Alignment::Left,
        )
        .draw(display)?;

        // Subtitle
        let subtitle_y = label_y + 14;
        Text::with_alignment(
            subtitle,
            Point::new(label_x, subtitle_y),
            MonoTextStyle::new(&FONT_6X10, COLOR_MUTED_TEXT),
            Alignment::Left,
        )
        .draw(display)?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Page trait
// ---------------------------------------------------------------------------

impl Page for DisplaySettingsPage {
    fn id(&self) -> PageId {
        PageId::DisplaySettings
    }

    fn title(&self) -> &str {
        "Display"
    }

    fn on_activate(&mut self) {
        self.dirty = true;
    }

    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action> {
        if let TouchEvent::Press(point) = event {
            let pt = point.to_point();

            // Back button
            if self.back_touch_bounds().contains(pt) {
                return Some(Action::GoBack);
            }

            // Hiking option (index 0)
            if self.option_bounds(0).contains(pt) && self.selected_mode != HomePageMode::Outdoor {
                self.selected_mode = HomePageMode::Outdoor;
                self.dirty = true;
                return Some(Action::UpdateHomePageMode(HomePageMode::Outdoor));
            }

            // Home option (index 1)
            if self.option_bounds(1).contains(pt) && self.selected_mode != HomePageMode::Home {
                self.selected_mode = HomePageMode::Home;
                self.dirty = true;
                return Some(Action::UpdateHomePageMode(HomePageMode::Home));
            }
        }
        None
    }

    fn update(&mut self) {}

    fn on_event(&mut self, _event: &PageEvent) -> bool {
        false
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

impl Drawable for DisplaySettingsPage {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        if !self.dirty {
            return Ok(());
        }

        display.clear(COLOR_BACKGROUND)?;

        self.draw_header(display)?;

        // Section label
        Text::with_alignment(
            "Home Page Style",
            Point::new(
                self.bounds.top_left.x + PADDING_X as i32 + 4,
                self.bounds.top_left.y + SECTION_LABEL_Y_OFFSET as i32,
            ),
            MonoTextStyle::new(&FONT_6X10, WHITE),
            Alignment::Left,
        )
        .draw(display)?;

        // Option cards
        self.draw_option(
            display,
            0,
            HomePageMode::Outdoor,
            "Outdoor",
            "Status dashboard",
        )?;
        self.draw_option(display, 1, HomePageMode::Home, "Home", "Mini-graph grid")?;

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
