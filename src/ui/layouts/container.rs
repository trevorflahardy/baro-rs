// src/ui/layouts/container.rs
//! Container layout component with flexible sizing and alignment

use crate::ui::core::{DirtyRegion, Drawable, TouchEvent, TouchResult, Touchable};
use crate::ui::styling::Style;
use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Rectangle, RoundedRectangle};
use heapless::Vec;

/// Alignment options for container children
///
/// Determines how children are positioned along the cross-axis
/// (perpendicular to the layout direction).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alignment {
    /// Align to start (left for horizontal, top for vertical)
    Start,
    /// Center alignment
    Center,
    /// Align to end (right for horizontal, bottom for vertical)
    End,
    /// Stretch to fill available space
    Stretch,
}

/// Direction for container layout
///
/// Determines how children are arranged within the container.
/// Children are laid out sequentially along the main axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    /// Horizontal layout (left to right)
    Horizontal,
    /// Vertical layout (top to bottom)
    Vertical,
}

/// Size constraint for container children
///
/// Controls how a child element's size is calculated during layout.
/// - `Fit`: Child uses its natural content size
/// - `Expand`: Child expands to fill available space (shared equally among all Expand children)
/// - `Fixed(n)`: Child has a fixed size of n pixels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeConstraint {
    /// Fit to content size
    Fit,
    /// Expand to fill parent container
    Expand,
    /// Fixed size in pixels
    Fixed(u32),
}

/// Child element with its size constraint
pub struct ChildElement {
    bounds: Rectangle,
    size_constraint: SizeConstraint,
    dirty: bool,
}

impl ChildElement {
    pub fn new(bounds: Rectangle, size_constraint: SizeConstraint) -> Self {
        Self {
            bounds,
            size_constraint,
            dirty: true,
        }
    }

    pub fn bounds(&self) -> Rectangle {
        self.bounds
    }

    pub fn set_bounds(&mut self, bounds: Rectangle) {
        if self.bounds != bounds {
            self.bounds = bounds;
            self.dirty = true;
        }
    }
}

/// Container that arranges children in a direction with alignment
///
/// A flexible layout container that arranges child elements either horizontally
/// or vertically with configurable spacing and alignment. Children can have
/// different size constraints (Fit, Expand, or Fixed) that control how they
/// are sized within the available space.
///
/// # Type Parameters
/// - `N`: Maximum number of child elements (compile-time constant)
///
/// # Examples
/// ```ignore
/// // Create a vertical container with 4 children, centered alignment
/// let mut container = Container::<4>::new(
///     Rectangle::new(Point::new(10, 10), Size::new(300, 200)),
///     Direction::Vertical
/// )
/// .with_alignment(Alignment::Center)
/// .with_spacing(10);
///
/// // Add children with different size constraints
/// container.add_child(Size::new(280, 50), SizeConstraint::Fixed(50)).ok();
/// container.add_child(Size::new(280, 0), SizeConstraint::Expand).ok();
/// ```
pub struct Container<const N: usize> {
    bounds: Rectangle,
    direction: Direction,
    alignment: Alignment,
    spacing: u32,
    style: Style,
    corner_radius: u32,
    children: Vec<ChildElement, N>,
    dirty: bool,
}

impl<const N: usize> Container<N> {
    /// Create a new container with the specified bounds and layout direction.
    ///
    /// By default, spacing is 0 and alignment is Start. Use builder methods
    /// to configure spacing and alignment.
    pub fn new(bounds: Rectangle, direction: Direction) -> Self {
        Self {
            bounds,
            direction,
            alignment: Alignment::Start,
            spacing: 0,
            style: Style::default(),
            corner_radius: 0,
            children: Vec::new(),
            dirty: true,
        }
    }

    /// Set the cross-axis alignment for children.
    ///
    /// Alignment controls how children are positioned perpendicular to the
    /// layout direction (e.g., for vertical layout, alignment controls horizontal positioning).
    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set the spacing between children in pixels.
    ///
    /// Spacing is applied between each child element along the layout direction.
    pub fn with_spacing(mut self, spacing: u32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set the container's visual style.
    ///
    /// Style controls background color, border, and padding.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the padding for the container.
    ///
    /// This is a convenience method equivalent to modifying the style's padding.
    /// Padding creates space between the container's borders and its children.
    ///
    /// # Examples
    /// ```ignore
    /// // All sides equal padding
    /// container.with_padding(Padding::all(8));
    ///
    /// // Different horizontal and vertical padding
    /// container.with_padding(Padding::symmetric(12, 16));
    /// ```
    pub fn with_padding(mut self, padding: crate::ui::styling::Padding) -> Self {
        self.style.padding = padding;
        self.dirty = true;
        self
    }

    /// Set the corner radius for rounded corners.
    ///
    /// A radius of 0 (default) produces square corners.
    /// Higher values produce more rounded corners.
    ///
    /// # Examples
    /// ```ignore
    /// // Subtle rounding
    /// container.with_corner_radius(4);
    ///
    /// // More prominent rounding
    /// container.with_corner_radius(12);
    /// ```
    pub fn with_corner_radius(mut self, radius: u32) -> Self {
        self.corner_radius = radius;
        self.dirty = true;
        self
    }

    /// Add a child element to the container.
    ///
    /// # Parameters
    /// - `size`: Initial size hint for the child (may be overridden by constraint and layout)
    /// - `constraint`: How the child's size should be determined during layout
    ///
    /// # Returns
    /// - `Ok(index)`: Index of the added child
    /// - `Err`: Container is full (reached capacity N)
    ///
    /// After adding a child, the container automatically recalculates its layout.
    pub fn add_child(
        &mut self,
        size: Size,
        constraint: SizeConstraint,
    ) -> Result<usize, &'static str> {
        let child_bounds = Rectangle::new(self.bounds.top_left, size);
        let child = ChildElement::new(child_bounds, constraint);
        self.children.push(child).map_err(|_| "Container full")?;
        self.dirty = true;
        self.layout();
        Ok(self.children.len() - 1)
    }

    /// Get the computed bounds for a child at the given index.
    ///
    /// Returns `None` if the index is out of bounds.
    /// Get the computed bounds for a child at the given index.
    ///
    /// Returns `None` if the index is out of bounds.
    /// The bounds are calculated by the layout system based on
    /// the child's size constraint and the container's layout direction.
    pub fn child_bounds(&self, index: usize) -> Option<Rectangle> {
        self.children.get(index).map(|c| c.bounds)
    }

    /// Recalculate layout for all children
    fn layout(&mut self) {
        if self.children.is_empty() {
            return;
        }

        let padding = self.style.padding;
        let available_width = self.bounds.size.width.saturating_sub(padding.horizontal());
        let available_height = self.bounds.size.height.saturating_sub(padding.vertical());

        let content_start = Point::new(
            self.bounds.top_left.x + padding.left as i32,
            self.bounds.top_left.y + padding.top as i32,
        );

        match self.direction {
            Direction::Horizontal => {
                self.layout_horizontal(content_start, available_width, available_height);
            }
            Direction::Vertical => {
                self.layout_vertical(content_start, available_width, available_height);
            }
        }
    }

    fn layout_horizontal(&mut self, start: Point, available_width: u32, available_height: u32) {
        let total_spacing = self.spacing * (self.children.len().saturating_sub(1)) as u32;
        let mut fixed_width = 0u32;
        let mut expand_count = 0usize;

        // Calculate fixed widths and count expand elements
        for child in &self.children {
            match child.size_constraint {
                SizeConstraint::Fixed(w) => fixed_width += w,
                SizeConstraint::Fit => fixed_width += child.bounds.size.width,
                SizeConstraint::Expand => expand_count += 1,
            }
        }

        let remaining_width = available_width
            .saturating_sub(fixed_width)
            .saturating_sub(total_spacing);
        let expand_width = if expand_count > 0 {
            remaining_width / expand_count as u32
        } else {
            0
        };

        let mut current_x = start.x;

        for child in &mut self.children {
            let child_width = match child.size_constraint {
                SizeConstraint::Fixed(w) => w,
                SizeConstraint::Fit => child.bounds.size.width,
                SizeConstraint::Expand => expand_width,
            };

            let child_height = match self.alignment {
                Alignment::Stretch => available_height,
                _ => child.bounds.size.height.min(available_height),
            };

            let child_y = match self.alignment {
                Alignment::Start => start.y,
                Alignment::Center => start.y + ((available_height - child_height) / 2) as i32,
                Alignment::End => start.y + (available_height - child_height) as i32,
                Alignment::Stretch => start.y,
            };

            child.set_bounds(Rectangle::new(
                Point::new(current_x, child_y),
                Size::new(child_width, child_height),
            ));

            current_x += child_width as i32 + self.spacing as i32;
        }
    }

    fn layout_vertical(&mut self, start: Point, available_width: u32, available_height: u32) {
        let total_spacing = self.spacing * (self.children.len().saturating_sub(1)) as u32;
        let mut fixed_height = 0u32;
        let mut expand_count = 0usize;

        // Calculate fixed heights and count expand elements
        for child in &self.children {
            match child.size_constraint {
                SizeConstraint::Fixed(h) => fixed_height += h,
                SizeConstraint::Fit => fixed_height += child.bounds.size.height,
                SizeConstraint::Expand => expand_count += 1,
            }
        }

        let remaining_height = available_height
            .saturating_sub(fixed_height)
            .saturating_sub(total_spacing);
        let expand_height = if expand_count > 0 {
            remaining_height / expand_count as u32
        } else {
            0
        };

        let mut current_y = start.y;

        for child in &mut self.children {
            let child_height = match child.size_constraint {
                SizeConstraint::Fixed(h) => h,
                SizeConstraint::Fit => child.bounds.size.height,
                SizeConstraint::Expand => expand_height,
            };

            let child_width = match self.alignment {
                Alignment::Stretch => available_width,
                _ => child.bounds.size.width.min(available_width),
            };

            let child_x = match self.alignment {
                Alignment::Start => start.x,
                Alignment::Center => start.x + ((available_width - child_width) / 2) as i32,
                Alignment::End => start.x + (available_width - child_width) as i32,
                Alignment::Stretch => start.x,
            };

            child.set_bounds(Rectangle::new(
                Point::new(child_x, current_y),
                Size::new(child_width, child_height),
            ));

            current_y += child_height as i32 + self.spacing as i32;
        }
    }

    pub fn set_bounds(&mut self, bounds: Rectangle) {
        if self.bounds != bounds {
            self.bounds = bounds;
            self.dirty = true;
            self.layout();
        }
    }
}

impl<const N: usize> Drawable for Container<N> {
    fn draw<D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        // Draw container background if specified
        if self.style.background_color.is_some() || self.style.border_color.is_some() {
            // Always use RoundedRectangle, even with radius 0 for consistency
            let corner_size = Size::new(self.corner_radius, self.corner_radius);
            RoundedRectangle::with_equal_corners(self.bounds, corner_size)
                .into_styled(self.style.to_primitive_style())
                .draw(display)?;
        }

        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    fn is_dirty(&self) -> bool {
        self.dirty || self.children.iter().any(|c| c.dirty)
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
        for child in &mut self.children {
            child.dirty = false;
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn dirty_region(&self) -> Option<DirtyRegion> {
        if self.dirty {
            return Some(DirtyRegion::new(self.bounds));
        }

        // Check for any dirty children
        let mut region: Option<DirtyRegion> = None;
        for child in &self.children {
            if child.dirty {
                if let Some(ref mut r) = region {
                    r.expand_to_include(child.bounds);
                } else {
                    region = Some(DirtyRegion::new(child.bounds));
                }
            }
        }

        region
    }
}

impl<const N: usize> Touchable for Container<N> {
    fn contains_point(&self, point: crate::ui::core::TouchPoint) -> bool {
        let p = point.to_point();
        self.bounds.contains(p)
    }

    fn handle_touch(&mut self, _event: TouchEvent) -> TouchResult {
        // Containers don't handle touch by default
        // Child elements should handle their own touch events
        TouchResult::NotHandled
    }
}
