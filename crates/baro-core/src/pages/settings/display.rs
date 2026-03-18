// src/pages/settings/display.rs
//! Display settings sub-page with home page mode and temperature unit selectors.
//!
//! Shows radio-button style selectors for Outdoor vs Home mode and Celsius vs Fahrenheit.
//! Tapping an option emits `Action::UpdateHomePageMode` or `Action::UpdateTemperatureUnit`.

use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle,
};
use embedded_graphics::text::{Alignment, Text};

use crate::config::{HomePageMode, TemperatureUnit};
use crate::pages::page::Page;
use crate::ui::Drawable;
use crate::ui::core::{Action, PageEvent, PageId, TouchEvent, Touchable};
use crate::ui::layouts::{ScrollDirection, ScrollableContainer};
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
const OPTION_HEIGHT_PX: u32 = 36;

/// Vertical gap between option cards
const OPTION_GAP_PX: u32 = 2;

/// Horizontal padding
const PADDING_X: u32 = 8;

/// Vertical padding at top of scroll content
const CONTENT_PADDING_TOP: u32 = 8;

/// Section label height (label text + gap before first card)
const SECTION_LABEL_HEIGHT: u32 = 14;

/// Gap between sections
const SECTION_GAP: u32 = 8;

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
// Section layout helpers
// ---------------------------------------------------------------------------

/// Y offset in content space for the "Home Page Style" section label.
const fn mode_section_label_y() -> u32 {
    CONTENT_PADDING_TOP
}

/// Y offset in content space for the first mode option card.
const fn mode_options_y() -> u32 {
    mode_section_label_y() + SECTION_LABEL_HEIGHT
}

/// Y offset in content space for the "Temperature Unit" section label.
const fn temp_section_label_y() -> u32 {
    mode_options_y() + 2 * (OPTION_HEIGHT_PX + OPTION_GAP_PX) + SECTION_GAP
}

/// Y offset in content space for the first temperature unit option card.
const fn temp_options_y() -> u32 {
    temp_section_label_y() + SECTION_LABEL_HEIGHT
}

/// Total content height for scrolling.
const fn total_content_height() -> u32 {
    temp_options_y() + 2 * (OPTION_HEIGHT_PX + OPTION_GAP_PX) + SECTION_GAP
}

// ---------------------------------------------------------------------------
// DisplaySettingsPage
// ---------------------------------------------------------------------------

pub struct DisplaySettingsPage {
    bounds: Rectangle,
    scroll: ScrollableContainer,
    selected_mode: HomePageMode,
    selected_temp_unit: TemperatureUnit,
    dirty: bool,
}

impl DisplaySettingsPage {
    pub fn new(
        bounds: Rectangle,
        current_mode: HomePageMode,
        current_temp_unit: TemperatureUnit,
    ) -> Self {
        let scroll_viewport = Self::scroll_viewport(bounds);
        let scroll = ScrollableContainer::new(
            scroll_viewport,
            Size::new(scroll_viewport.size.width, total_content_height()),
            ScrollDirection::Vertical,
        );

        Self {
            bounds,
            scroll,
            selected_mode: current_mode,
            selected_temp_unit: current_temp_unit,
            dirty: true,
        }
    }

    /// The scrollable viewport below the header.
    fn scroll_viewport(bounds: Rectangle) -> Rectangle {
        Rectangle::new(
            Point::new(
                bounds.top_left.x,
                bounds.top_left.y + HEADER_HEIGHT_PX as i32,
            ),
            Size::new(
                bounds.size.width,
                bounds.size.height.saturating_sub(HEADER_HEIGHT_PX),
            ),
        )
    }

    /// Calculate the screen-space bounds of an option card.
    fn option_screen_bounds(&self, index: usize, base_content_y: u32) -> Rectangle {
        let viewport = self.scroll.viewport();
        let scroll_y = self.scroll.scroll_offset().y;
        let x = viewport.top_left.x + PADDING_X as i32;
        let content_y =
            base_content_y as i32 + (index as u32 * (OPTION_HEIGHT_PX + OPTION_GAP_PX)) as i32;
        let y = viewport.top_left.y + content_y - scroll_y;
        let width = viewport.size.width.saturating_sub(PADDING_X * 2);
        Rectangle::new(Point::new(x, y), Size::new(width, OPTION_HEIGHT_PX))
    }

    /// Home page mode option screen bounds.
    fn mode_option_screen_bounds(&self, index: usize) -> Rectangle {
        self.option_screen_bounds(index, mode_options_y())
    }

    /// Temperature unit option screen bounds.
    fn temp_option_screen_bounds(&self, index: usize) -> Rectangle {
        self.option_screen_bounds(index, temp_options_y())
    }

    /// Section label screen Y position.
    fn section_label_screen_y(&self, content_y: u32) -> i32 {
        let viewport = self.scroll.viewport();
        let scroll_y = self.scroll.scroll_offset().y;
        viewport.top_left.y + content_y as i32 - scroll_y
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

    fn draw_option_card<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        bounds: Rectangle,
        is_selected: bool,
        label: &str,
        subtitle: &str,
    ) -> Result<(), D::Error> {
        // Skip if entirely outside viewport
        let viewport = self.scroll.viewport();
        let card_bottom = bounds.top_left.y + OPTION_HEIGHT_PX as i32;
        let vp_top = viewport.top_left.y;
        let vp_bottom = vp_top + viewport.size.height as i32;
        if card_bottom <= vp_top || bounds.top_left.y >= vp_bottom {
            return Ok(());
        }

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
        let label_y = bounds.top_left.y + 14;
        Text::with_alignment(
            label,
            Point::new(label_x, label_y),
            MonoTextStyle::new(&FONT_6X10, WHITE),
            Alignment::Left,
        )
        .draw(display)?;

        // Subtitle — use lighter color on selected (accent) background for contrast
        let subtitle_color = if is_selected {
            COLOR_HEADER_TEXT
        } else {
            COLOR_MUTED_TEXT
        };
        let subtitle_y = label_y + 12;
        Text::with_alignment(
            subtitle,
            Point::new(label_x, subtitle_y),
            MonoTextStyle::new(&FONT_6X10, subtitle_color),
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
        match event {
            TouchEvent::Press(point) => {
                let pt = point.to_point();

                // Back button (in header, not scrollable)
                if self.back_touch_bounds().contains(pt) {
                    return Some(Action::GoBack);
                }

                // Home page mode: Outdoor (index 0)
                if self.mode_option_screen_bounds(0).contains(pt)
                    && self.selected_mode != HomePageMode::Outdoor
                {
                    self.selected_mode = HomePageMode::Outdoor;
                    self.dirty = true;
                    return Some(Action::UpdateHomePageMode(HomePageMode::Outdoor));
                }

                // Home page mode: Home (index 1)
                if self.mode_option_screen_bounds(1).contains(pt)
                    && self.selected_mode != HomePageMode::Home
                {
                    self.selected_mode = HomePageMode::Home;
                    self.dirty = true;
                    return Some(Action::UpdateHomePageMode(HomePageMode::Home));
                }

                // Temperature unit: Celsius (index 0)
                if self.temp_option_screen_bounds(0).contains(pt)
                    && self.selected_temp_unit != TemperatureUnit::Celsius
                {
                    self.selected_temp_unit = TemperatureUnit::Celsius;
                    self.dirty = true;
                    return Some(Action::UpdateTemperatureUnit(TemperatureUnit::Celsius));
                }

                // Temperature unit: Fahrenheit (index 1)
                if self.temp_option_screen_bounds(1).contains(pt)
                    && self.selected_temp_unit != TemperatureUnit::Fahrenheit
                {
                    self.selected_temp_unit = TemperatureUnit::Fahrenheit;
                    self.dirty = true;
                    return Some(Action::UpdateTemperatureUnit(TemperatureUnit::Fahrenheit));
                }

                // Start tracking for potential drag
                self.scroll.handle_touch(event);
            }
            TouchEvent::Drag(_) => {
                self.scroll.handle_touch(event);
                self.dirty = true;
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

        // "Home Page Style" section label
        let label_x = self.bounds.top_left.x + PADDING_X as i32 + 4;
        Text::with_alignment(
            "Home Page Style",
            Point::new(label_x, self.section_label_screen_y(mode_section_label_y())),
            MonoTextStyle::new(&FONT_6X10, WHITE),
            Alignment::Left,
        )
        .draw(display)?;

        // Home page mode option cards
        self.draw_option_card(
            display,
            self.mode_option_screen_bounds(0),
            self.selected_mode == HomePageMode::Outdoor,
            "Outdoor",
            "Status dashboard",
        )?;
        self.draw_option_card(
            display,
            self.mode_option_screen_bounds(1),
            self.selected_mode == HomePageMode::Home,
            "Home",
            "Mini-graph grid",
        )?;

        // "Temperature Unit" section label
        Text::with_alignment(
            "Temperature Unit",
            Point::new(label_x, self.section_label_screen_y(temp_section_label_y())),
            MonoTextStyle::new(&FONT_6X10, WHITE),
            Alignment::Left,
        )
        .draw(display)?;

        // Temperature unit option cards
        self.draw_option_card(
            display,
            self.temp_option_screen_bounds(0),
            self.selected_temp_unit == TemperatureUnit::Celsius,
            "Celsius",
            "Metric (C)",
        )?;
        self.draw_option_card(
            display,
            self.temp_option_screen_bounds(1),
            self.selected_temp_unit == TemperatureUnit::Fahrenheit,
            "Fahrenheit",
            "Imperial (F)",
        )?;

        // Draw scrollbar indicators
        self.scroll.draw(display)?;

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
