// src/ui/layouts/scrollable.rs
//! Scrollable container for content that exceeds visible bounds

use crate::ui::core::{DirtyRegion, Drawable, TouchEvent, TouchPoint, TouchResult, Touchable};
use crate::ui::styling::Style;
use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};

/// Direction that can be scrolled
///
/// Controls which directions the scrollable container allows scrolling.
/// Content can be scrolled vertically, horizontally, or in both directions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollDirection {
    Vertical,
    Horizontal,
    Both,
}

/// Scrollable container with viewport and content size tracking
///
/// A container that displays a viewport into larger content, allowing the user
/// to scroll through content that exceeds the visible area. Supports vertical,
/// horizontal, or bidirectional scrolling with touch drag gestures.
///
/// The viewport defines the visible area, while content_size defines the total
/// scrollable area. Scroll offset tracks the current scroll position.
///
/// # Touch Interaction
/// - Press: Begins tracking touch for scrolling
/// - Drag: Scrolls the content (inverted: drag down scrolls content up)
///
/// # Visual Feedback
/// Automatically draws scrollbar indicators when content exceeds viewport size.
///
/// # Examples
/// ```ignore
/// // Create a vertical scrolling container
/// let viewport = Rectangle::new(Point::new(0, 0), Size::new(320, 240));
/// let content_size = Size::new(320, 600); // Content is taller than viewport
/// let mut scrollable = ScrollableContainer::new(
///     viewport,
///     content_size,
///     ScrollDirection::Vertical
/// );
///
/// // Scroll programmatically
/// scrollable.scroll_by(Point::new(0, -50)); // Scroll up by 50 pixels
/// ```
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
    /// Create a new scrollable container.
    ///
    /// # Parameters
    /// - `viewport`: The visible area rectangle
    /// - `content_size`: Total size of the scrollable content
    /// - `direction`: Which directions scrolling is allowed
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

    /// Set the visual style for the container.
    ///
    /// Controls background color and border appearance.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the total content size.
    ///
    /// Updates the scrollable area and constrains the scroll offset
    /// to valid bounds. Marks the container as dirty if size changed.
    pub fn set_content_size(&mut self, size: Size) {
        if self.content_size != size {
            self.content_size = size;
            self.constrain_scroll();
            self.dirty = true;
        }
    }

    /// Get the current scroll offset in content space.
    ///
    /// The offset represents the top-left corner of the visible viewport
    /// within the total content area.
    pub fn scroll_offset(&self) -> Point {
        self.scroll_offset
    }

    /// Scroll by a relative delta amount.
    ///
    /// Positive delta scrolls right/down, negative scrolls left/up.
    /// The scroll position is automatically constrained to valid bounds.
    pub fn scroll_by(&mut self, delta: Point) {
        self.scroll_offset += delta;
        self.constrain_scroll();
        self.dirty = true;
    }

    /// Scroll to a specific absolute offset.
    ///
    /// The offset is automatically constrained to valid bounds.
    pub fn scroll_to(&mut self, offset: Point) {
        self.scroll_offset = offset;
        self.constrain_scroll();
        self.dirty = true;
    }

    /// Constrain scroll to valid bounds
    fn constrain_scroll(&mut self) {
        let max_scroll_x =
            (self.content_size.width as i32 - self.viewport.size.width as i32).max(0);
        let max_scroll_y =
            (self.content_size.height as i32 - self.viewport.size.height as i32).max(0);

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

    /// Get the visible content rectangle in content space.
    ///
    /// Returns a rectangle representing which portion of the total content
    /// is currently visible in the viewport. Useful for clipping child rendering.
    pub fn visible_content_rect(&self) -> Rectangle {
        Rectangle::new(self.scroll_offset, self.viewport.size)
    }

    /// Transform a point from viewport space to content space.
    ///
    /// # Parameters
    /// - `point`: Touch point in viewport coordinates
    ///
    /// # Returns
    /// - `Some(point)`: Transformed point in content space
    /// - `None`: Point is outside the viewport
    ///
    /// Useful for forwarding touch events to child elements that are
    /// positioned in content space.
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

    /// Check if content can scroll vertically.
    ///
    /// Returns true if vertical scrolling is enabled and content height
    /// exceeds viewport height.
    pub fn can_scroll_vertical(&self) -> bool {
        matches!(
            self.direction,
            ScrollDirection::Vertical | ScrollDirection::Both
        ) && self.content_size.height > self.viewport.size.height
    }

    /// Check if content can scroll horizontally.
    ///
    /// Returns true if horizontal scrolling is enabled and content width
    /// exceeds viewport width.
    pub fn can_scroll_horizontal(&self) -> bool {
        matches!(
            self.direction,
            ScrollDirection::Horizontal | ScrollDirection::Both
        ) && self.content_size.width > self.viewport.size.width
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
            let scroll_ratio =
                self.scroll_offset.y as f32 / (content_height - viewport_height) as f32;
            let bar_height = ((viewport_height * viewport_height) / content_height).max(20);
            let bar_y = self.viewport.top_left.y
                + ((viewport_height - bar_height) as f32 * scroll_ratio) as i32;

            let bar = Rectangle::new(
                Point::new(
                    self.viewport.top_left.x + self.viewport.size.width as i32
                        - scrollbar_width as i32,
                    bar_y,
                ),
                Size::new(scrollbar_width, bar_height),
            );

            bar.into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(scrollbar_color)
                    .build(),
            )
            .draw(display)?;
        }

        // Horizontal scrollbar
        if self.can_scroll_horizontal() {
            let viewport_width = self.viewport.size.width;
            let content_width = self.content_size.width;
            let scroll_ratio =
                self.scroll_offset.x as f32 / (content_width - viewport_width) as f32;
            let bar_width = ((viewport_width * viewport_width) / content_width).max(20);
            let bar_x = self.viewport.top_left.x
                + ((viewport_width - bar_width) as f32 * scroll_ratio) as i32;

            let bar = Rectangle::new(
                Point::new(
                    bar_x,
                    self.viewport.top_left.y + self.viewport.size.height as i32
                        - scrollbar_width as i32,
                ),
                Size::new(bar_width, scrollbar_width),
            );

            bar.into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(scrollbar_color)
                    .build(),
            )
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
