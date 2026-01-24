// src/ui/core.rs
//! Core UI traits and types for the Baro UI system

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

/// Represents a 2D touch point on the display
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchPoint {
    pub x: u16,
    pub y: u16,
}

impl TouchPoint {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }

    pub fn to_point(&self) -> Point {
        Point::new(self.x as i32, self.y as i32)
    }
}

/// Touch events that can occur on the UI
#[derive(Debug, Clone, Copy)]
pub enum TouchEvent {
    /// Initial touch press at a point
    Press(TouchPoint),
    /// Touch drag to a new point
    Drag(TouchPoint),
}

/// Result from handling a touch event
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TouchResult {
    /// Event was handled by this element
    Handled,
    /// Event was not handled, pass to next element
    NotHandled,
    /// Event triggered an action
    Action(Action),
}

/// Actions that UI elements can trigger
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    /// Navigate to a specific page
    NavigateToPage(PageId),
    /// Go back to previous page
    GoBack,
    /// Toggle a setting
    ToggleSetting(u8),
    /// Refresh data display
    RefreshData,
    /// Custom action with ID
    Custom(u16),
}

/// Page identifier for navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageId {
    Home,
    Settings,
    Graphs,
}

/// Dirty region tracking for efficient rendering
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirtyRegion {
    pub bounds: Rectangle,
    pub is_dirty: bool,
}

impl DirtyRegion {
    pub fn new(bounds: Rectangle) -> Self {
        Self {
            bounds,
            is_dirty: true,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    pub fn mark_clean(&mut self) {
        self.is_dirty = false;
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    /// Expand this dirty region to include another region
    pub fn expand_to_include(&mut self, other: Rectangle) {
        if !self.is_dirty {
            self.bounds = other;
            self.is_dirty = true;
        } else {
            // Calculate the bounding box that includes both rectangles
            let min_x = self.bounds.top_left.x.min(other.top_left.x);
            let min_y = self.bounds.top_left.y.min(other.top_left.y);

            let max_x = (self.bounds.top_left.x + self.bounds.size.width as i32)
                .max(other.top_left.x + other.size.width as i32);
            let max_y = (self.bounds.top_left.y + self.bounds.size.height as i32)
                .max(other.top_left.y + other.size.height as i32);

            self.bounds = Rectangle::new(
                Point::new(min_x, min_y),
                Size::new((max_x - min_x) as u32, (max_y - min_y) as u32),
            );
        }
    }
}

/// Trait for any UI element that can be drawn
pub trait Drawable {
    /// Draw the element to the display within the given bounds
    fn draw<D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        display: &mut D,
    ) -> Result<(), D::Error>;

    /// Get the bounds of this drawable element
    fn bounds(&self) -> Rectangle;

    /// Check if this element needs to be redrawn
    fn is_dirty(&self) -> bool;

    /// Mark this element as clean (already drawn)
    fn mark_clean(&mut self);

    /// Mark this element as dirty (needs redraw)
    fn mark_dirty(&mut self);

    /// Get the dirty region for partial updates
    fn dirty_region(&self) -> Option<DirtyRegion> {
        if self.is_dirty() {
            Some(DirtyRegion::new(self.bounds()))
        } else {
            None
        }
    }
}

/// Trait for UI elements that respond to touch events
pub trait Touchable {
    /// Check if a point is within this element's bounds
    fn contains_point(&self, point: TouchPoint) -> bool;

    /// Handle a touch event, returns result indicating if handled and any action
    fn handle_touch(&mut self, event: TouchEvent) -> TouchResult;
}

/// Combined trait for interactive drawable elements
pub trait Interactive: Drawable + Touchable {}

/// Implement Interactive for any type that implements both Drawable and Touchable
impl<T: Drawable + Touchable> Interactive for T {}

/// Events that pages can subscribe to for updates
#[derive(Debug, Clone)]
pub enum PageEvent {
    /// Sensor data updated
    SensorUpdate(SensorData),
    /// Storage event (rollup, sample, etc.)
    StorageEvent(StorageEvent),
    /// System event
    SystemEvent(SystemEvent),
}

/// Sensor data for event system
#[derive(Debug, Clone)]
pub struct SensorData {
    pub temperature: Option<f32>,
    pub humidity: Option<f32>,
    pub timestamp: u64,
}

/// Storage events for live monitoring
#[derive(Debug, Clone)]
pub enum StorageEvent {
    RawSample {
        sensor: &'static str,
        value: f32,
        timestamp: u64,
    },
    Rollup {
        interval: &'static str,
        count: usize,
        timestamp: u64,
    },
}

/// System events
#[derive(Debug, Clone)]
pub enum SystemEvent {
    LowMemory,
    NetworkConnected,
    NetworkDisconnected,
}
