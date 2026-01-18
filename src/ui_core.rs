// src/ui_core.rs

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

/// Represents a 2D touch point
#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
    pub x: u16,
    pub y: u16,
}

/// Result of a touch event
#[derive(Debug, Clone, Copy)]
pub enum TouchEvent {
    Press(TouchPoint),
    Release(TouchPoint),
    Drag(TouchPoint),
}

/// Trait for any UI element that can be drawn
pub trait Drawable {
    fn draw<D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        display: &mut D,
        bounds: Rectangle,
    ) -> Result<(), D::Error>;
    fn is_dirty(&self) -> bool;
    fn mark_clean(&mut self);
}

/// Trait for UI elements that respond to touch
pub trait Touchable {
    /// Check if a point is within this element's bounds
    fn contains_point(&self, point: TouchPoint) -> bool;

    /// Handle a touch event, returns true if handled
    fn handle_touch(&mut self, event: TouchEvent) -> bool;
}

/// Trait for clickable buttons/elements
pub trait Clickable: Touchable + Drawable {
    /// Called when the element is clicked
    fn on_click(&mut self) -> Option<Action>;
}

/// Actions that UI elements can trigger
#[derive(Debug, Clone, Copy)]
pub enum Action {
    NavigateToPage(PageId),
    ToggleSetting(u8),
    RefreshData,
    // Add more as needed
}

/// Page identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageId {
    Home,
    Settings,
    Graphs,
    // Add your pages
}
