//! A widget is a reusable component that displays some information from a sensor. There are a few widget types
//! implemented here, with more to come later. Each sensor must implement all the widgets to be dynamically
//! composed into the dashboard.

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

/// A widget must implement this trait to draw itself onto the display.
pub trait Widget {
    fn is_dirty(&self) -> bool;
    fn mark_clean(&mut self);
    fn mark_dirty(&mut self);
}

pub trait WidgetQuadrant: Widget {
    fn draw<D: DrawTarget>(&mut self, display: &mut D, bounds: Rectangle) -> Result<(), D::Error>;
}

pub trait WidgetVerticalQuarter: Widget {
    fn draw<D: DrawTarget>(&mut self, display: &mut D, bounds: Rectangle) -> Result<(), D::Error>;
}
