// src/pages/settings/display.rs
//! Display settings sub-page with home page mode and temperature unit selectors.
//!
//! Uses [`OptionSelector`] for radio-button style selection of Outdoor vs Home
//! mode and Celsius vs Fahrenheit. Tapping an option emits
//! `Action::UpdateHomePageMode` or `Action::UpdateTemperatureUnit`.

use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle, RoundedRectangle};
use embedded_graphics::text::{Alignment, Text};

use crate::config::{HomePageMode, TemperatureUnit};
use crate::pages::page::Page;
use crate::ui::Drawable;
use crate::ui::components::OptionSelector;
use crate::ui::core::{Action, PageEvent, PageId, TouchEvent, Touchable};
use crate::ui::layouts::{ScrollDirection, ScrollableContainer};
use crate::ui::styling::{COLOR_BACKGROUND, COLOR_FOREGROUND};

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

/// Height of the header bar
const HEADER_HEIGHT_PX: u32 = 36;

/// Corner radius for the header
const HEADER_CORNER_RADIUS: u32 = 12;

/// Horizontal padding inside the scroll viewport
const PADDING_X: u32 = 8;

/// Vertical padding at top of scroll content
const CONTENT_PADDING_TOP: u32 = 8;

/// Gap between the two selector sections
const SECTION_GAP_PX: u32 = 8;

/// Header text color (muted)
const COLOR_HEADER_TEXT: Rgb565 = Rgb565::new(20, 40, 20);

/// Back button touch target width
const BACK_TOUCH_WIDTH: u32 = 44;

// ---------------------------------------------------------------------------
// DisplaySettingsPage
// ---------------------------------------------------------------------------

pub struct DisplaySettingsPage {
    bounds: Rectangle,
    scroll: ScrollableContainer,
    mode_selector: OptionSelector<2>,
    temp_selector: OptionSelector<2>,
    dirty: bool,
}

impl DisplaySettingsPage {
    pub fn new(
        bounds: Rectangle,
        current_mode: HomePageMode,
        current_temp_unit: TemperatureUnit,
    ) -> Self {
        let scroll_viewport = Self::scroll_viewport(bounds);
        let card_width = scroll_viewport.size.width.saturating_sub(PADDING_X * 2);

        let mode_index = match current_mode {
            HomePageMode::Outdoor => 0,
            HomePageMode::Home => 1,
        };
        let temp_index = match current_temp_unit {
            TemperatureUnit::Celsius => 0,
            TemperatureUnit::Fahrenheit => 1,
        };

        let mut mode_selector =
            OptionSelector::<2>::new(Point::zero(), card_width, "Home Page Style", mode_index);
        mode_selector.add_option("Outdoor", "Status dashboard");
        mode_selector.add_option("Home", "Mini-graph grid");

        let mut temp_selector =
            OptionSelector::<2>::new(Point::zero(), card_width, "Temperature Unit", temp_index);
        temp_selector.add_option("Celsius", "Metric (C)");
        temp_selector.add_option("Fahrenheit", "Imperial (F)");

        let total_content_height = CONTENT_PADDING_TOP
            + mode_selector.total_height()
            + SECTION_GAP_PX
            + temp_selector.total_height()
            + SECTION_GAP_PX;

        let scroll = ScrollableContainer::new(
            scroll_viewport,
            Size::new(scroll_viewport.size.width, total_content_height),
            ScrollDirection::Vertical,
        );

        let mut page = Self {
            bounds,
            scroll,
            mode_selector,
            temp_selector,
            dirty: true,
        };
        page.update_selector_origins();
        page
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

    /// Recompute selector origins from the current scroll offset.
    fn update_selector_origins(&mut self) {
        let viewport = self.scroll.viewport();
        let scroll_y = self.scroll.scroll_offset().y;
        let x = viewport.top_left.x + PADDING_X as i32;
        let base_y = viewport.top_left.y - scroll_y;

        let mode_y = base_y + CONTENT_PADDING_TOP as i32;
        self.mode_selector.set_origin(Point::new(x, mode_y));

        let temp_y = mode_y + self.mode_selector.total_height() as i32 + SECTION_GAP_PX as i32;
        self.temp_selector.set_origin(Point::new(x, temp_y));
    }

    /// Back button touch bounds (top-left of header).
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

        RoundedRectangle::with_equal_corners(
            header_rect,
            Size::new(HEADER_CORNER_RADIUS, HEADER_CORNER_RADIUS),
        )
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
            "DISPLAY",
            Point::new(self.bounds.top_left.x + 28, text_y),
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

                self.update_selector_origins();

                // Home page mode selector
                let mode_result = self.mode_selector.handle_touch(event);
                if self.mode_selector.is_dirty() {
                    self.dirty = true;
                    if let Some(index) = mode_result {
                        let mode = match index {
                            0 => HomePageMode::Outdoor,
                            _ => HomePageMode::Home,
                        };
                        return Some(Action::UpdateHomePageMode(mode));
                    }
                    return None; // pressed already-selected mode, just show feedback
                }

                // Temperature unit selector
                let temp_result = self.temp_selector.handle_touch(event);
                if self.temp_selector.is_dirty() {
                    self.dirty = true;
                    if let Some(index) = temp_result {
                        let unit = match index {
                            0 => TemperatureUnit::Celsius,
                            _ => TemperatureUnit::Fahrenheit,
                        };
                        return Some(Action::UpdateTemperatureUnit(unit));
                    }
                    return None; // pressed already-selected unit, just show feedback
                }

                // Nothing matched — forward to scroll
                self.scroll.handle_touch(event);
            }
            TouchEvent::Drag(_) => {
                // Forward to selectors (clears pressed state) and scroll
                self.mode_selector.handle_touch(event);
                self.temp_selector.handle_touch(event);
                self.scroll.handle_touch(event);
                if self.mode_selector.is_dirty() || self.temp_selector.is_dirty() {
                    self.dirty = true;
                }
            }
            TouchEvent::Release(_) => {
                // Clear pressed states on release
                self.mode_selector.handle_touch(event);
                self.temp_selector.handle_touch(event);
                self.scroll.handle_touch(event);
                if self.mode_selector.is_dirty() || self.temp_selector.is_dirty() {
                    self.dirty = true;
                }
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

        self.mode_selector.draw(display)?;
        self.temp_selector.draw(display)?;

        // Scrollbar indicators
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
        self.mode_selector.mark_clean();
        self.temp_selector.mark_clean();
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}
