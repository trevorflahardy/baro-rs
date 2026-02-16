//! WiFi status page
//!
//! Displays a status screen for WiFi connection state — either "Connecting"
//! (with a spinner-like indicator) or "Error" (with a disconnected icon and
//! a non-functional "Connect" button placeholder).
//!
//! The layout is inspired by the reference design:
//!
//! ```text
//! ┌──────────────────────────────────────┐
//! │  ▫  HOME AIR              ≈ (icon)  │  ← header
//! ├──────────────────────────────────────┤
//! │                                      │
//! │           ( n o n )   or  ...        │  ← large status text
//! │                                      │
//! │       No Wi-Fi Connection            │  ← title message
//! │       Data cannot be updated.        │  ← subtitle
//! │                                      │
//! │       [ <-> CONNECT TO WI-FI ]       │  ← action button (noop)
//! │                                      │
//! └──────────────────────────────────────┘
//! ```

use core::cell::Cell;

use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::{FONT_5X7, FONT_6X10, FONT_10X20};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle,
};
use embedded_graphics::text::{Alignment, Text};

use crate::pages::page::Page;
use crate::ui::core::{Action, Drawable, PageId, TouchEvent};
use crate::ui::styling::{
    COLOR_BACKGROUND, COLOR_FOREGROUND, COLOR_STROKE, DISPLAY_HEIGHT_PX, DISPLAY_WIDTH_PX, WHITE,
};

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

/// Height of the top header bar in pixels
const HEADER_HEIGHT_PX: u32 = 36;

/// Vertical position where the large status icon/text is drawn
const STATUS_ICON_CENTER_Y: i32 = 105;

/// Vertical position of the title line ("No Wi-Fi Connection")
const TITLE_Y: i32 = 130;

/// Vertical position of the subtitle line
const SUBTITLE_Y: i32 = 150;

/// Vertical position of the action button
const BUTTON_CENTER_Y: i32 = 180;

/// Button size
const BUTTON_WIDTH: u32 = 130;
const BUTTON_HEIGHT: u32 = 25;

/// Corner radius of the button
const BUTTON_CORNER_RADIUS: u32 = 18;

/// Corner radius of header background
const HEADER_CORNER_RADIUS: u32 = 0;

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

/// Cyan accent used for the connecting state text
const COLOR_ACCENT_CYAN: Rgb565 = Rgb565::new(0, 50, 31); // bright cyan-ish

/// Muted gray for subtitle / secondary text
const COLOR_TEXT_MUTED: Rgb565 = Rgb565::new(14, 28, 14);

/// Light grayish text for the header title
const COLOR_HEADER_TEXT: Rgb565 = Rgb565::new(20, 40, 20);

/// Reddish-pink for the wifi-bad icon
const COLOR_WIFI_BAD: Rgb565 = Rgb565::new(28, 20, 18);

// ---------------------------------------------------------------------------
// WiFi connection state
// ---------------------------------------------------------------------------

/// Describes the current WiFi connection status displayed by the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiState {
    /// WiFi is currently attempting to connect.
    Connecting,
    /// WiFi connection failed or is unavailable.
    Error,
}

impl WifiState {
    /// Large status text rendered in the centre of the page.
    fn status_text(self) -> &'static str {
        match self {
            Self::Connecting => ". . .",
            Self::Error => "( n o n )",
        }
    }

    /// Primary title line beneath the status text.
    fn title(self) -> &'static str {
        match self {
            Self::Connecting => "Connecting to Wi-Fi",
            Self::Error => "No Wi-Fi Connection",
        }
    }

    /// Secondary subtitle.
    fn subtitle(self) -> &'static str {
        match self {
            Self::Connecting => "Please wait...",
            Self::Error => "Data cannot be updated.",
        }
    }

    /// Accent color used for the status text.
    fn accent_color(self) -> Rgb565 {
        match self {
            Self::Connecting => COLOR_ACCENT_CYAN,
            Self::Error => COLOR_TEXT_MUTED,
        }
    }

    /// Color for the wifi icon in the header.
    fn icon_color(self) -> Rgb565 {
        match self {
            Self::Connecting => COLOR_ACCENT_CYAN,
            Self::Error => COLOR_WIFI_BAD,
        }
    }
}

// ---------------------------------------------------------------------------
// WifiStatusPage
// ---------------------------------------------------------------------------

/// A combined WiFi connecting / error page.
///
/// The page can be constructed in either `Connecting` or `Error` state, and
/// the state can be changed at runtime via [`set_state`](Self::set_state).
pub struct WifiStatusPage {
    dirty: Cell<bool>,
    state: WifiState,
}

impl WifiStatusPage {
    /// Create the page in the given initial state.
    pub fn new(state: WifiState) -> Self {
        Self {
            dirty: Cell::new(true),
            state,
        }
    }

    /// Update the displayed state, marking the page dirty if it changed.
    pub fn set_state(&mut self, state: WifiState) {
        if self.state != state {
            self.state = state;
            self.dirty.set(true);
        }
    }

    /// Current state.
    pub fn state(&self) -> WifiState {
        self.state
    }

    // -- drawing helpers ---------------------------------------------------

    /// Draw the header bar ("HOME AIR" + wifi icon).
    fn draw_header<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        let header_rect = Rectangle::new(
            Point::new(0, 0),
            Size::new(DISPLAY_WIDTH_PX as u32, HEADER_HEIGHT_PX),
        );

        // Header background (rounded top corners)
        RoundedRectangle::with_equal_corners(
            header_rect,
            Size::new(HEADER_CORNER_RADIUS, HEADER_CORNER_RADIUS),
        )
        .into_styled(PrimitiveStyle::with_fill(COLOR_FOREGROUND))
        .draw(display)?;

        // Grid icon (4 small squares)
        let grid_x: i32 = 12;
        let grid_y: i32 = 10;
        let sq = 6u32;
        let gap: i32 = 2;
        let grid_color = COLOR_TEXT_MUTED;
        let sq_style = PrimitiveStyle::with_fill(grid_color);

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

        // "HOME AIR" title
        Text::with_alignment(
            "HOME AIR",
            Point::new(36, (HEADER_HEIGHT_PX / 2 + 4) as i32),
            MonoTextStyle::new(&FONT_6X10, COLOR_HEADER_TEXT),
            Alignment::Left,
        )
        .draw(display)?;

        Ok(())
    }

    /// Draw the large status text in the centre of the page.
    fn draw_status_text<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        let center_x = (DISPLAY_WIDTH_PX / 2) as i32;

        Text::with_alignment(
            self.state.status_text(),
            Point::new(center_x, STATUS_ICON_CENTER_Y),
            MonoTextStyle::new(&FONT_10X20, self.state.accent_color()),
            Alignment::Center,
        )
        .draw(display)?;

        Ok(())
    }

    /// Draw the title and subtitle lines.
    fn draw_captions<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        let center_x = (DISPLAY_WIDTH_PX / 2) as i32;

        // Title
        Text::with_alignment(
            self.state.title(),
            Point::new(center_x, TITLE_Y),
            MonoTextStyle::new(&FONT_10X20, WHITE),
            Alignment::Center,
        )
        .draw(display)?;

        // Subtitle
        Text::with_alignment(
            self.state.subtitle(),
            Point::new(center_x, SUBTITLE_Y),
            MonoTextStyle::new(&FONT_6X10, COLOR_TEXT_MUTED),
            Alignment::Center,
        )
        .draw(display)?;

        Ok(())
    }

    /// Draw the "CONNECT TO WI-FI" button (noop for now).
    fn draw_button<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        // Only show button in error state
        if self.state != WifiState::Error {
            return Ok(());
        }

        let center_x = (DISPLAY_WIDTH_PX / 2) as i32;
        let btn_x = center_x - (BUTTON_WIDTH as i32 / 2);
        let btn_y = BUTTON_CENTER_Y - (BUTTON_HEIGHT as i32 / 2);

        let btn_rect = Rectangle::new(
            Point::new(btn_x, btn_y),
            Size::new(BUTTON_WIDTH, BUTTON_HEIGHT),
        );

        // Button outline
        let outline_style = PrimitiveStyleBuilder::new()
            .stroke_color(COLOR_STROKE)
            .stroke_width(1)
            .fill_color(COLOR_FOREGROUND)
            .build();

        RoundedRectangle::with_equal_corners(
            btn_rect,
            Size::new(BUTTON_CORNER_RADIUS, BUTTON_CORNER_RADIUS),
        )
        .into_styled(outline_style)
        .draw(display)?;

        // Button label
        Text::with_alignment(
            "<-> CONNECT TO WI-FI",
            Point::new(center_x, BUTTON_CENTER_Y + 2),
            MonoTextStyle::new(&FONT_5X7, COLOR_ACCENT_CYAN),
            Alignment::Center,
        )
        .draw(display)?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Page trait
// ---------------------------------------------------------------------------

impl Page for WifiStatusPage {
    fn id(&self) -> PageId {
        PageId::WifiStatus
    }

    fn title(&self) -> &str {
        match self.state {
            WifiState::Connecting => "WiFi Connecting",
            WifiState::Error => "WiFi Error",
        }
    }

    fn on_activate(&mut self) {
        self.dirty.set(true);
    }

    fn handle_touch(&mut self, _event: TouchEvent) -> Option<Action> {
        // Button does nothing for now
        None
    }

    fn update(&mut self) {
        // No periodic updates needed
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
// Drawable trait
// ---------------------------------------------------------------------------

impl Drawable for WifiStatusPage {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        if !self.dirty.get() {
            return Ok(());
        }

        // Full-screen dark background
        display.clear(COLOR_BACKGROUND)?;

        self.draw_header(display)?;
        self.draw_status_text(display)?;
        self.draw_captions(display)?;
        self.draw_button(display)?;

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
