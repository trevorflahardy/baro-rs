// src/ui/elements.rs
//! Concrete UI element enum.
//!
//! The UI system needs a way for layout containers to own heterogeneous child
//! widgets *without* using trait objects.
//!
//! In embedded-graphics, `DrawTarget` is generic, which makes `Drawable` (our
//! trait) **not object-safe**. This enum is the pragmatic alternative: it
//! supports the built-in widgets (Text, MultiLineText, Button) and can grow as
//! needed.

use crate::ui::components::{Button, MultiLineText, TextComponent, TextSize};
use crate::ui::core::{DirtyRegion, Drawable, TouchEvent, TouchPoint, TouchResult, Touchable};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

/// A concrete, layout-friendly UI element.
pub enum Element {
    Text(TextComponent),
    MultiLineText(MultiLineText),
    Button(Button),
    /// A layout-only element that draws nothing.
    Spacer {
        bounds: Rectangle,
        dirty: bool,
    },
}

impl Element {
    /// Preferred size used by layout when `SizeConstraint::Fit` is selected.
    ///
    /// Today this is derived from the element's current bounds size.
    pub fn preferred_size(&self) -> Size {
        self.bounds().size
    }

    pub fn set_bounds(&mut self, bounds: Rectangle) {
        match self {
            Element::Text(t) => t.set_bounds(bounds),
            Element::MultiLineText(t) => t.set_bounds(bounds),
            Element::Button(b) => b.set_bounds(bounds),
            Element::Spacer { bounds: b, dirty } => {
                if *b != bounds {
                    *b = bounds;
                    *dirty = true;
                }
            }
        }
    }

    /// Convenience constructor: text element.
    pub fn text(bounds: Rectangle, text: &str, size: TextSize) -> Self {
        Self::Text(TextComponent::new(bounds, text, size))
    }

    /// Convenience constructor: multiline text element.
    pub fn multiline(bounds: Rectangle, text: &str, size: TextSize) -> Self {
        Self::MultiLineText(MultiLineText::new(bounds, text, size))
    }

    /// Convenience constructor: button element.
    pub fn button(bounds: Rectangle, label: &str, action: crate::ui::core::Action) -> Self {
        Self::Button(Button::new(bounds, label, action))
    }

    /// Convenience constructor: spacer.
    pub fn spacer(bounds: Rectangle) -> Self {
        Self::Spacer {
            bounds,
            dirty: true,
        }
    }
}

impl Drawable for Element {
    fn draw<D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        match self {
            Element::Text(t) => t.draw(display),
            Element::MultiLineText(t) => t.draw(display),
            Element::Button(b) => b.draw(display),
            Element::Spacer { .. } => Ok(()),
        }
    }

    fn bounds(&self) -> Rectangle {
        match self {
            Element::Text(t) => t.bounds(),
            Element::MultiLineText(t) => t.bounds(),
            Element::Button(b) => b.bounds(),
            Element::Spacer { bounds, .. } => *bounds,
        }
    }

    fn is_dirty(&self) -> bool {
        match self {
            Element::Text(t) => t.is_dirty(),
            Element::MultiLineText(t) => t.is_dirty(),
            Element::Button(b) => b.is_dirty(),
            Element::Spacer { dirty, .. } => *dirty,
        }
    }

    fn mark_clean(&mut self) {
        match self {
            Element::Text(t) => t.mark_clean(),
            Element::MultiLineText(t) => t.mark_clean(),
            Element::Button(b) => b.mark_clean(),
            Element::Spacer { dirty, .. } => *dirty = false,
        }
    }

    fn mark_dirty(&mut self) {
        match self {
            Element::Text(t) => t.mark_dirty(),
            Element::MultiLineText(t) => t.mark_dirty(),
            Element::Button(b) => b.mark_dirty(),
            Element::Spacer { dirty, .. } => *dirty = true,
        }
    }

    fn dirty_region(&self) -> Option<DirtyRegion> {
        match self {
            Element::Text(t) => t.dirty_region(),
            Element::MultiLineText(t) => t.dirty_region(),
            Element::Button(b) => b.dirty_region(),
            Element::Spacer { bounds, dirty } => {
                if *dirty {
                    Some(DirtyRegion::new(*bounds))
                } else {
                    None
                }
            }
        }
    }
}

impl Touchable for Element {
    fn contains_point(&self, point: TouchPoint) -> bool {
        self.bounds().contains(point.to_point())
    }

    fn handle_touch(&mut self, event: TouchEvent) -> TouchResult {
        match self {
            Element::Text(_) => TouchResult::NotHandled,
            Element::MultiLineText(_) => TouchResult::NotHandled,
            Element::Button(b) => b.handle_touch(event),
            Element::Spacer { .. } => TouchResult::NotHandled,
        }
    }
}
