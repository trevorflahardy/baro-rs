// src/pages/settings/list.rs
//! Settings page — a scrollable list of category rows.
//!
//! Each row navigates to a sub-settings page. Currently implemented:
//! - **Display** → `DisplaySettingsPage` (home page mode selector)
//! - **Monitor** → `MonitorPage` (live sensor feed + storage log)

use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle, RoundedRectangle};
use embedded_graphics::text::{Alignment, Text};

use crate::pages::page::Page;
use crate::ui::Drawable;
use crate::ui::core::{Action, PageEvent, PageId, TouchEvent};
use crate::ui::styling::{COLOR_BACKGROUND, COLOR_FOREGROUND, WHITE};

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

/// Height of the top header bar
const HEADER_HEIGHT_PX: u32 = 36;

/// Corner radius for rounded elements
const CORNER_RADIUS: u32 = 12;

/// Height of each category row
const ROW_HEIGHT_PX: u32 = 40;

/// Vertical gap between rows
const ROW_GAP_PX: u32 = 2;

/// Horizontal padding for the list area
const LIST_PADDING_X: u32 = 8;

/// Y offset where the category list begins (below header + gap)
const LIST_Y_OFFSET: u32 = HEADER_HEIGHT_PX + 4;

/// Pill corner radius for rows
const PILL_CORNER_RADIUS: u32 = 6;

/// Header text color (muted)
const COLOR_HEADER_TEXT: Rgb565 = Rgb565::new(20, 40, 20);

/// Muted text for secondary labels
const COLOR_MUTED_TEXT: Rgb565 = Rgb565::new(18, 36, 18);

// ---------------------------------------------------------------------------
// Category definition
// ---------------------------------------------------------------------------

/// A category row in the settings list.
struct SettingsCategory {
    label: &'static str,
    subtitle: &'static str,
    target: PageId,
}

const CATEGORIES: &[SettingsCategory] = &[
    SettingsCategory {
        label: "Display",
        subtitle: "Home page style",
        target: PageId::DisplaySettings,
    },
    SettingsCategory {
        label: "Monitor",
        subtitle: "Live sensor & log feed",
        target: PageId::Monitor,
    },
];

// ---------------------------------------------------------------------------
// SettingsPage
// ---------------------------------------------------------------------------

/// Settings page displaying a vertical list of tappable category rows.
pub struct SettingsPage {
    bounds: Rectangle,
    dirty: bool,
}

impl SettingsPage {
    pub fn new(bounds: Rectangle) -> Self {
        Self {
            bounds,
            dirty: true,
        }
    }

    /// Kept for API compatibility with existing callers.
    pub fn init(&mut self) {
        self.dirty = true;
    }

    /// Calculate the bounding rectangle of a category row by index.
    fn row_bounds(&self, index: usize) -> Rectangle {
        let x = self.bounds.top_left.x + LIST_PADDING_X as i32;
        let y = self.bounds.top_left.y
            + LIST_Y_OFFSET as i32
            + (index as u32 * (ROW_HEIGHT_PX + ROW_GAP_PX)) as i32;
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

        Text::with_alignment(
            "SETTINGS",
            Point::new(
                self.bounds.top_left.x + 12,
                self.bounds.top_left.y + (HEADER_HEIGHT_PX / 2 + 4) as i32,
            ),
            MonoTextStyle::new(&FONT_6X10, COLOR_HEADER_TEXT),
            Alignment::Left,
        )
        .draw(display)?;

        Ok(())
    }

    fn draw_row<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        index: usize,
        category: &SettingsCategory,
    ) -> Result<(), D::Error> {
        let bounds = self.row_bounds(index);

        // Row background
        RoundedRectangle::with_equal_corners(
            bounds,
            Size::new(PILL_CORNER_RADIUS, PILL_CORNER_RADIUS),
        )
        .into_styled(PrimitiveStyle::with_fill(COLOR_FOREGROUND))
        .draw(display)?;

        // Label (left)
        let label_y = bounds.top_left.y + 16;
        Text::with_alignment(
            category.label,
            Point::new(bounds.top_left.x + 12, label_y),
            MonoTextStyle::new(&FONT_6X10, WHITE),
            Alignment::Left,
        )
        .draw(display)?;

        // Subtitle (below label)
        let subtitle_y = label_y + 14;
        Text::with_alignment(
            category.subtitle,
            Point::new(bounds.top_left.x + 12, subtitle_y),
            MonoTextStyle::new(&FONT_6X10, COLOR_MUTED_TEXT),
            Alignment::Left,
        )
        .draw(display)?;

        // Chevron ">" on right
        let right_x = bounds.top_left.x + bounds.size.width as i32 - 14;
        Text::with_alignment(
            ">",
            Point::new(right_x, bounds.top_left.y + (ROW_HEIGHT_PX / 2 + 4) as i32),
            MonoTextStyle::new(&FONT_6X10, COLOR_MUTED_TEXT),
            Alignment::Right,
        )
        .draw(display)?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Page trait
// ---------------------------------------------------------------------------

impl Page for SettingsPage {
    fn id(&self) -> PageId {
        PageId::Settings
    }

    fn title(&self) -> &str {
        "Settings"
    }

    fn on_activate(&mut self) {
        self.dirty = true;
    }

    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action> {
        if let TouchEvent::Press(point) = event {
            let pt = point.to_point();

            // Check each category row
            for (i, category) in CATEGORIES.iter().enumerate() {
                let row_rect = self.row_bounds(i);
                if row_rect.contains(pt) {
                    return Some(Action::NavigateToPage(category.target));
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

impl Drawable for SettingsPage {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        if !self.dirty {
            return Ok(());
        }

        display.clear(COLOR_BACKGROUND)?;

        self.draw_header(display)?;

        for (i, category) in CATEGORIES.iter().enumerate() {
            self.draw_row(display, i, category)?;
        }

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
