// src/ui/layouts/scrollable.rs
//! Scrollable container for content that exceeds visible bounds

use crate::ui::core::{Drawable, DirtyRegion, TouchEvent, TouchPoint, TouchResult, Touchable};
use crate::ui::styling::Style;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::Drawable as EgDrawable;

/// Direction that can be scrolled
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollDirection {
    Vertical,
    Horizontal,
    Both,
}

/// Scrollable container with viewport and content size tracking
pub struct ScrollableContainer {
    /// Visible bounds (viewport)
    viewport: Rectangle,
    /// Total content size (may be larger than viewport)
    content_size: Size,
    /// Current scroll offset
    scroll_offset: Point,
    /// Scroll direction
    direction: ScrollDirection,
    /// Style for the container
    style: Style,
    /// Track if dirty
    dirty: bool,
    /// Last touch position for drag scrolling
    last_touch: Option<TouchPoint>,
}

impl ScrollableContainer {
    pub fn new(viewport: Rectangle, content_size: Size, direction: ScrollDirection) -> Self {
        Self {
            viewport,
            content_size,
            scroll_offset: Point::zero(),
            direction,
            style: Style::default(),
            dirty: true,
            last_touch: None,
        }
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the total content size
    pub fn set_content_size(&mut self, size: Size) {
        if self.content_size != size {
            self.content_size = size;
            self.constrain_scroll();
            self.dirty = true;
        }
    }

    /// Get the current scroll offset
    pub fn scroll_offset(&self) -> Point {
        self.scroll_offset
    }

    /// Scroll by a delta amount
    pub fn scroll_by(&mut self, delta: Point) {
        self.scroll_offset += delta;
        self.constrain_scroll();
        self.dirty = true;
    }

    /// Scroll to a specific offset
    pub fn scroll_to(&mut self, offset: Point) {
        self.scroll_offset = offset;
        self.constrain_scroll();
        self.dirty = true;
    }

    /// Constrain scroll to valid bounds
    fn constrain_scroll(&mut self) {
        let max_scroll_x = (self.content_size.width as i32 - self.viewport.size.width as i32).max(0);
        let max_scroll_y = (self.content_size.height as i32 - self.viewport.size.height as i32).max(0);

        match self.direction {
            ScrollDirection::Vertical => {
                self.scroll_offset.x = 0;
                self.scroll_offset.y = self.scroll_offset.y.clamp(0, max_scroll_y);
            }
            ScrollDirection::Horizontal => {
                self.scroll_offset.x = self.scroll_offset.x.clamp(0, max_scroll_x);
                self.scroll_offset.y = 0;
            }
            ScrollDirection::Both => {
                self.scroll_offset.x = self.scroll_offset.x.clamp(0, max_scroll_x);
                self.scroll_offset.y = self.scroll_offset.y.clamp(0, max_scroll_y);
            }
        }
    }

    /// Get the visible content rectangle (in content space)
    pub fn visible_content_rect(&self) -> Rectangle {
        Rectangle::new(self.scroll_offset, self.viewport.size)
    }

    /// Transform a point from viewport space to content space
    pub fn viewport_to_content(&self, point: TouchPoint) -> Option<TouchPoint> {
        let p = point.to_point();
        if !self.viewport.contains(p) {
            return None;
        }

        let relative = p - self.viewport.top_left;
        let content_point = relative + self.scroll_offset;

        Some(TouchPoint::new(
            content_point.x as u16,
            content_point.y as u16,
        ))
    }

    /// Check if content can scroll in a direction
    pub fn can_scroll_vertical(&self) -> bool {
        matches!(self.direction, ScrollDirection::Vertical | ScrollDirection::Both)
            && self.content_size.height > self.viewport.size.height
    }

    pub fn can_scroll_horizontal(&self) -> bool {
        matches!(self.direction, ScrollDirection::Horizontal | ScrollDirection::Both)
            && self.content_size.width > self.viewport.size.width
    }

    /// Draw scrollbar indicators
    fn draw_scrollbars<D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        use embedded_graphics::pixelcolor::Rgb565;

        let scrollbar_width = 4;
        let scrollbar_color = Rgb565::CSS_GRAY;

        // Vertical scrollbar
        if self.can_scroll_vertical() {
            let viewport_height = self.viewport.size.height;
            let content_height = self.content_size.height;
            let scroll_ratio = self.scroll_offset.y as f32 / (content_height - viewport_height) as f32;
            let bar_height = ((viewport_height * viewport_height) / content_height).max(20);
            let bar_y = self.viewport.top_left.y
                + ((viewport_height - bar_height) as f32 * scroll_ratio) as i32;

            let bar = Rectangle::new(
                Point::new(
                    self.viewport.top_left.x + self.viewport.size.width as i32 - scrollbar_width as i32,
                    bar_y,
                ),
                Size::new(scrollbar_width, bar_height),
            );

            bar.into_styled(PrimitiveStyleBuilder::new().fill_color(scrollbar_color).build())
                .draw(display)?;
        }

        // Horizontal scrollbar
        if self.can_scroll_horizontal() {
            let viewport_width = self.viewport.size.width;
            let content_width = self.content_size.width;
            let scroll_ratio = self.scroll_offset.x as f32 / (content_width - viewport_width) as f32;
            let bar_width = ((viewport_width * viewport_width) / content_width).max(20);
            let bar_x = self.viewport.top_left.x
                + ((viewport_width - bar_width) as f32 * scroll_ratio) as i32;

            let bar = Rectangle::new(
                Point::new(
                    bar_x,
                    self.viewport.top_left.y + self.viewport.size.height as i32 - scrollbar_width as i32,
                ),
                Size::new(bar_width, scrollbar_width),
            );

            bar.into_styled(PrimitiveStyleBuilder::new().fill_color(scrollbar_color).build())
                .draw(display)?;
        }

        Ok(())
    }
}

impl Drawable for ScrollableContainer {
    fn draw<D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        // Draw container background
        if self.style.background_color.is_some() || self.style.border_color.is_some() {
            self.viewport
                .into_styled(self.style.to_primitive_style())
                .draw(display)?;
        }

        // Draw scrollbars
        self.draw_scrollbars(display)?;

        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        self.viewport
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
            Some(DirtyRegion::new(self.viewport))
        } else {
            None
        }
    }
}

impl Touchable for ScrollableContainer {
    fn contains_point(&self, point: TouchPoint) -> bool {
        let p = point.to_point();
        self.viewport.contains(p)
    }

    fn handle_touch(&mut self, event: TouchEvent) -> TouchResult {
        match event {
            TouchEvent::Press(point) => {
                if self.contains_point(point) {
                    self.last_touch = Some(point);
                    TouchResult::Handled
                } else {
                    TouchResult::NotHandled
                }
            }
            TouchEvent::Drag(point) => {
                if let Some(last) = self.last_touch {
                    let delta_x = point.x as i32 - last.x as i32;
                    let delta_y = point.y as i32 - last.y as i32;

                    // Invert scroll direction (drag down = scroll up)
                    self.scroll_by(Point::new(-delta_x, -delta_y));

                    self.last_touch = Some(point);
                    TouchResult::Handled
                } else {
                    TouchResult::NotHandled
                }
            }
        }
    }
}
