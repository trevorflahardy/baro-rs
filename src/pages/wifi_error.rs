//! WiFi Error page
//!
//! Displays a centered error message when WiFi connection fails

use crate::pages::Page;
use crate::ui::core::{Action, Drawable, PageId, TouchEvent};
use crate::ui::{DISPLAY_HEIGHT_PX, DISPLAY_WIDTH_PX, FONT_10X20_CHAR_HEIGHT_PX};
use core::cell::Cell;
use embedded_graphics::{
    Drawable as EgDrawable,
    geometry::{Point, Size},
    mono_font::{MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::Rectangle,
    text::{Alignment, Text},
};

/// WiFi error page that displays a centered error message
pub struct WifiErrorPage {
    /// Whether the page needs to be redrawn
    dirty: Cell<bool>,
    /// The error message to display
    error_message: &'static str,
}

impl WifiErrorPage {
    /// Create a new WiFi error page with default error message
    pub fn new() -> Self {
        Self {
            dirty: Cell::new(true),
            error_message: "WiFi Connection Failed",
        }
    }

    /// Create a new WiFi error page with a custom error message
    pub fn with_message(message: &'static str) -> Self {
        Self {
            dirty: Cell::new(true),
            error_message: message,
        }
    }
}

impl Default for WifiErrorPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Page for WifiErrorPage {
    fn id(&self) -> PageId {
        PageId::WifiError
    }

    fn title(&self) -> &str {
        "WiFi Error"
    }

    fn on_activate(&mut self) {
        self.dirty.set(true);
    }

    fn handle_touch(&mut self, _event: TouchEvent) -> Option<Action> {
        // WiFi error page doesn't respond to touch
        None
    }

    fn update(&mut self) {
        // No updates needed for static error page
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

impl Drawable for WifiErrorPage {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        if !self.dirty.get() {
            return Ok(());
        }

        // Clear screen to black
        display.clear(Rgb565::BLACK)?;

        // Calculate center position
        let center_x = (DISPLAY_WIDTH_PX / 2) as i32;
        let center_y = (DISPLAY_HEIGHT_PX / 2) as i32;

        // Draw main error message one line-height above center
        EgDrawable::draw(
            &Text::with_alignment(
                self.error_message,
                Point::new(center_x, center_y - FONT_10X20_CHAR_HEIGHT_PX as i32),
                MonoTextStyle::new(&FONT_10X20, Rgb565::RED),
                Alignment::Center,
            ),
            display,
        )?;

        // Draw additional help text one line-height below center
        EgDrawable::draw(
            &Text::with_alignment(
                "Check WiFi credentials",
                Point::new(center_x, center_y + FONT_10X20_CHAR_HEIGHT_PX as i32),
                MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE),
                Alignment::Center,
            ),
            display,
        )?;

        self.dirty.set(false);
        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        Rectangle::new(
            Point::zero(),
            Size::new(DISPLAY_WIDTH_PX as u32, DISPLAY_HEIGHT_PX as u32),
        )
    }

    fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    fn mark_clean(&mut self) {
        self.dirty.set(false);
    }

    fn mark_dirty(&mut self) {
        self.dirty.set(true);
    }
}
