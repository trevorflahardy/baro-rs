// src/ui/components/option_selector.rs
//! Generic radio-button style selector for choosing one of N options.
//!
//! Displays a titled section with option cards, each showing a radio button,
//! label, and subtitle. Exactly one option is selected at a time.
//!
//! # Usage
//! ```ignore
//! let mut selector = OptionSelector::<2>::new(Point::new(8, 20), 304, "Temperature Unit", 0);
//! selector.add_option("Celsius", "Metric (°C)");
//! selector.add_option("Fahrenheit", "Imperial (°F)");
//!
//! // In touch handler — returns Some(index) when selection changes:
//! if let Some(new_index) = selector.handle_touch(event) {
//!     // Map index to your enum/action
//! }
//! ```

use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle,
};
use embedded_graphics::text::{Alignment, Text};
use heapless::Vec;

use crate::ui::Drawable;
use crate::ui::core::{DirtyRegion, TouchEvent};
use crate::ui::styling::{COLOR_FOREGROUND, WHITE};

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

/// Height of each option card in pixels
const OPTION_CARD_HEIGHT_PX: u32 = 36;

/// Vertical gap between option cards
const OPTION_CARD_GAP_PX: u32 = 2;

/// Corner radius for option card pills
const PILL_CORNER_RADIUS: u32 = 6;

/// Radio button outer diameter
const RADIO_OUTER_DIAMETER: u32 = 12;

/// Radio button inner diameter (filled when selected)
const RADIO_INNER_DIAMETER: u32 = 6;

/// Height of the section title above the cards
const TITLE_HEIGHT_PX: u32 = 14;

/// Horizontal inset for the title text from the selector origin
const TITLE_INSET_X: i32 = 4;

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

/// Accent color for the selected option card background
const COLOR_ACCENT: Rgb565 = Rgb565::new(8, 40, 12);

/// Darkened accent for pressed-and-selected feedback
const COLOR_ACCENT_PRESSED: Rgb565 = Rgb565::new(6, 32, 9);

/// Highlight for pressed-but-unselected feedback
const COLOR_PRESSED: Rgb565 = Rgb565::new(4, 12, 5);

/// Muted text for subtitles on unselected cards
const COLOR_MUTED_TEXT: Rgb565 = Rgb565::new(18, 36, 18);

/// Muted text for subtitles on selected (accent) cards
const COLOR_SELECTED_SUBTITLE: Rgb565 = Rgb565::new(20, 40, 20);

// ---------------------------------------------------------------------------
// SelectOption
// ---------------------------------------------------------------------------

/// A single option with a label and subtitle.
pub struct SelectOption {
    label: heapless::String<24>,
    subtitle: heapless::String<32>,
}

// ---------------------------------------------------------------------------
// OptionSelector
// ---------------------------------------------------------------------------

/// Generic radio-button style selector for choosing one of N options.
///
/// The component is positioned via [`set_origin`](Self::set_origin) and
/// sized by the `width` given at construction. Height is determined
/// automatically from the number of options added via
/// [`add_option`](Self::add_option).
///
/// # Type Parameters
/// - `N`: Maximum number of options (const generic).
pub struct OptionSelector<const N: usize> {
    origin: Point,
    width: u32,
    title: heapless::String<24>,
    options: Vec<SelectOption, N>,
    selected: usize,
    pressed: Option<usize>,
    dirty: bool,
}

impl<const N: usize> OptionSelector<N> {
    /// Create a new option selector.
    ///
    /// # Parameters
    /// - `origin`: Top-left position in screen space (title starts here).
    /// - `width`: Width of the option cards.
    /// - `title`: Section title displayed above the option cards.
    /// - `selected`: Index of the initially selected option.
    pub fn new(origin: Point, width: u32, title: &str, selected: usize) -> Self {
        let mut title_str = heapless::String::new();
        title_str.push_str(title).ok();

        Self {
            origin,
            width,
            title: title_str,
            options: Vec::new(),
            selected,
            pressed: None,
            dirty: true,
        }
    }

    /// Add an option with a label and subtitle.
    ///
    /// Options are displayed in the order they are added.
    /// Returns `&mut Self` for chaining.
    pub fn add_option(&mut self, label: &str, subtitle: &str) -> &mut Self {
        let mut label_str = heapless::String::new();
        label_str.push_str(label).ok();
        let mut subtitle_str = heapless::String::new();
        subtitle_str.push_str(subtitle).ok();

        self.options
            .push(SelectOption {
                label: label_str,
                subtitle: subtitle_str,
            })
            .ok();

        self
    }

    /// Get the currently selected index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Set the selected index programmatically. Marks dirty if changed.
    pub fn set_selected(&mut self, index: usize) {
        if self.selected != index && index < self.options.len() {
            self.selected = index;
            self.dirty = true;
        }
    }

    /// Update the screen-space origin (e.g. after scroll offset changes).
    pub fn set_origin(&mut self, origin: Point) {
        if self.origin != origin {
            self.origin = origin;
            self.dirty = true;
        }
    }

    /// Total height of this selector (title + all option cards + gaps).
    pub fn total_height(&self) -> u32 {
        let n = self.options.len() as u32;
        if n == 0 {
            return TITLE_HEIGHT_PX;
        }
        TITLE_HEIGHT_PX + n * OPTION_CARD_HEIGHT_PX + (n - 1) * OPTION_CARD_GAP_PX
    }

    /// Handle a touch event.
    ///
    /// Returns `Some(index)` if the selection **changed** (a previously
    /// unselected option was tapped). Returns `None` otherwise.
    ///
    /// Even when `None` is returned, the internal `pressed` state may have
    /// changed — check [`is_dirty`](Drawable::is_dirty) to decide whether a
    /// redraw is needed.
    pub fn handle_touch(&mut self, event: TouchEvent) -> Option<usize> {
        match event {
            TouchEvent::Press(point) => {
                let pt = point.to_point();
                for i in 0..self.options.len() {
                    if self.option_bounds(i).contains(pt) {
                        self.pressed = Some(i);
                        self.dirty = true;
                        if i != self.selected {
                            self.selected = i;
                            return Some(i);
                        }
                        // Tapped already-selected option — show press feedback only.
                        return None;
                    }
                }
                None
            }
            TouchEvent::Release(_) => {
                if self.pressed.is_some() {
                    self.pressed = None;
                    self.dirty = true;
                }
                None
            }
            TouchEvent::Drag(_) => {
                // Clear pressed state on drag (user is scrolling).
                if self.pressed.is_some() {
                    self.pressed = None;
                    self.dirty = true;
                }
                None
            }
        }
    }

    // -----------------------------------------------------------------------
    // Layout helpers
    // -----------------------------------------------------------------------

    /// Screen-space bounds of the option card at `index`.
    fn option_bounds(&self, index: usize) -> Rectangle {
        let cards_y = self.origin.y + TITLE_HEIGHT_PX as i32;
        let y = cards_y + (index as u32 * (OPTION_CARD_HEIGHT_PX + OPTION_CARD_GAP_PX)) as i32;
        Rectangle::new(
            Point::new(self.origin.x, y),
            Size::new(self.width, OPTION_CARD_HEIGHT_PX),
        )
    }

    // -----------------------------------------------------------------------
    // Drawing helpers
    // -----------------------------------------------------------------------

    fn draw_option_card<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        card_bounds: Rectangle,
        is_selected: bool,
        is_pressed: bool,
        option: &SelectOption,
    ) -> Result<(), D::Error> {
        // Card background with press/selection feedback
        let bg_color = match (is_selected, is_pressed) {
            (true, true) => COLOR_ACCENT_PRESSED,
            (true, false) => COLOR_ACCENT,
            (false, true) => COLOR_PRESSED,
            (false, false) => COLOR_FOREGROUND,
        };

        RoundedRectangle::with_equal_corners(
            card_bounds,
            Size::new(PILL_CORNER_RADIUS, PILL_CORNER_RADIUS),
        )
        .into_styled(PrimitiveStyle::with_fill(bg_color))
        .draw(display)?;

        // Radio button
        let radio_x = card_bounds.top_left.x + 16;
        let radio_y = card_bounds.top_left.y + (OPTION_CARD_HEIGHT_PX / 2) as i32;

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

        // Inner fill when selected
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

        // Label
        let label_x = radio_x + (RADIO_OUTER_DIAMETER / 2) as i32 + 10;
        let label_y = card_bounds.top_left.y + 14;
        Text::with_alignment(
            &option.label,
            Point::new(label_x, label_y),
            MonoTextStyle::new(&FONT_6X10, WHITE),
            Alignment::Left,
        )
        .draw(display)?;

        // Subtitle
        let subtitle_color = if is_selected {
            COLOR_SELECTED_SUBTITLE
        } else {
            COLOR_MUTED_TEXT
        };
        Text::with_alignment(
            &option.subtitle,
            Point::new(label_x, label_y + 12),
            MonoTextStyle::new(&FONT_6X10, subtitle_color),
            Alignment::Left,
        )
        .draw(display)?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Drawable
// ---------------------------------------------------------------------------

impl<const N: usize> Drawable for OptionSelector<N> {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        // Title
        Text::with_alignment(
            &self.title,
            Point::new(self.origin.x + TITLE_INSET_X, self.origin.y + 10),
            MonoTextStyle::new(&FONT_6X10, WHITE),
            Alignment::Left,
        )
        .draw(display)?;

        // Option cards
        for (i, option) in self.options.iter().enumerate() {
            let card_bounds = self.option_bounds(i);
            let is_selected = i == self.selected;
            let is_pressed = self.pressed == Some(i);
            self.draw_option_card(display, card_bounds, is_selected, is_pressed, option)?;
        }

        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        Rectangle::new(self.origin, Size::new(self.width, self.total_height()))
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

    fn dirty_region(&self) -> Option<DirtyRegion> {
        if self.dirty {
            Some(DirtyRegion::new(self.bounds()))
        } else {
            None
        }
    }
}
