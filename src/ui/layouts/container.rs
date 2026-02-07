// src/ui/layouts/container.rs
//! Flex-like layout container.
//!
//! `Container` is the primary layout primitive in `baro-rs`'s UI system.
//! It is intentionally small and predictable (embedded constraints), but it
//! aims to feel familiar if you have ever used CSS flexbox.
//!
//! ## Mental model
//! - The container has **bounds** (a `Rectangle`).
//! - It arranges **children** either **horizontally** or **vertically**.
//! - Each child gets a **main-axis size constraint** (`SizeConstraint`) and a
//!   **cross-axis alignment** (controlled by the container).
//! - Layout is computed eagerly when you add children or change bounds.
//!
//! ## What this fixes vs the old design
//! The previous `Container` only tracked child rectangles; it did **not** own
//! any child widgets, so it could draw a background but could not draw its
//! contents (leading to "TODO: Add text to container" style call-sites).
//!
//! This version stores real widgets and will:
//! - compute each child's bounds
//! - **set the child's bounds**
//! - draw the container background + all children
//! - forward touch events to children
//!
//! ## Common patterns
//!
//! ### Horizontal row, 3 children evenly spaced
//! ```ignore
//! use crate::ui::{Container, Direction, MainAxisAlignment, Alignment, SizeConstraint};
//! use crate::ui::elements::Element;
//! use embedded_graphics::primitives::Rectangle;
//! use embedded_graphics::prelude::*;
//!
//! let bounds = Rectangle::new(Point::new(0, 0), Size::new(320, 40));
//! let mut row = Container::<3>::new(bounds, Direction::Horizontal)
//!     .with_main_axis_alignment(MainAxisAlignment::SpaceEvenly)
//!     .with_alignment(Alignment::Center);
//!
//! row.add_child(Element::text(bounds, "A", crate::ui::TextSize::Medium), SizeConstraint::Fit).ok();
//! row.add_child(Element::text(bounds, "B", crate::ui::TextSize::Medium), SizeConstraint::Fit).ok();
//! row.add_child(Element::text(bounds, "C", crate::ui::TextSize::Medium), SizeConstraint::Fit).ok();
//! ```
//!
//! ### Flex-grow style sizing
//! ```ignore
//! // Two children: left fits its content, right grows to fill remaining space.
//! row.add_child(left, SizeConstraint::Fit).ok();
//! row.add_child(right, SizeConstraint::Grow(1)).ok();
//! ```

use crate::ui::core::{
    Action, DirtyRegion, Drawable, TouchEvent, TouchPoint, TouchResult, Touchable,
};
use crate::ui::elements::Element;
use crate::ui::styling::Style;
use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Rectangle, RoundedRectangle};
use heapless::Vec;

/// Alignment options for container children along the cross-axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alignment {
    Start,
    Center,
    End,
    Stretch,
}

/// Direction for container layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

/// How children are distributed along the main axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MainAxisAlignment {
    /// Pack at the start; use `gap` between children.
    Start,
    /// Center as a group; use `gap` between children.
    Center,
    /// Pack at the end; use `gap` between children.
    End,
    /// Space only between items (no leading/trailing space).
    SpaceBetween,
    /// Equal space around items (half-size on edges).
    SpaceAround,
    /// Equal space between items and edges.
    SpaceEvenly,
}

/// Size constraint for a child along the main axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeConstraint {
    /// Use the child's preferred size (typically its current bounds size).
    Fit,
    /// Fixed main-axis size in pixels.
    Fixed(u32),
    /// Flex-grow style sizing.
    ///
    /// Remaining space is distributed proportional to weights.
    Grow(u16),
}

impl SizeConstraint {
    fn grow_weight(&self) -> u16 {
        match *self {
            SizeConstraint::Grow(w) => w.max(1),
            _ => 0,
        }
    }
}

struct ChildElement {
    element: Element,
    bounds: Rectangle,
    size_constraint: SizeConstraint,
    dirty: bool,
}

impl ChildElement {
    fn new(mut element: Element, bounds: Rectangle, size_constraint: SizeConstraint) -> Self {
        element.set_bounds(bounds);
        Self {
            element,
            bounds,
            size_constraint,
            dirty: true,
        }
    }

    fn set_bounds(&mut self, bounds: Rectangle) {
        if self.bounds != bounds {
            self.bounds = bounds;
            self.element.set_bounds(bounds);
            self.dirty = true;
        }
    }

    fn preferred_size(&self) -> Size {
        self.element.preferred_size()
    }
}

/// A flex-like container that owns and lays out child elements.
///
/// `N` is the maximum number of children stored inline (heapless).
pub struct Container<const N: usize> {
    bounds: Rectangle,
    direction: Direction,
    alignment: Alignment,
    main_axis_alignment: MainAxisAlignment,
    gap: u32,
    style: Style,
    corner_radius: u32,
    children: Vec<ChildElement, N>,
    dirty: bool,
}

impl<const N: usize> Container<N> {
    pub fn new(bounds: Rectangle, direction: Direction) -> Self {
        Self {
            bounds,
            direction,
            alignment: Alignment::Start,
            main_axis_alignment: MainAxisAlignment::Start,
            gap: 0,
            style: Style::default(),
            corner_radius: 0,
            children: Vec::new(),
            dirty: true,
        }
    }

    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn with_main_axis_alignment(mut self, alignment: MainAxisAlignment) -> Self {
        self.main_axis_alignment = alignment;
        self
    }

    /// Set the base gap between children (in pixels).
    pub fn with_gap(mut self, gap: u32) -> Self {
        self.gap = gap;
        self
    }

    /// Back-compat builder (historical name).
    pub fn with_spacing(self, spacing: u32) -> Self {
        self.with_gap(spacing)
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn with_padding(mut self, padding: crate::ui::styling::Padding) -> Self {
        self.style.padding = padding;
        self.dirty = true;
        self
    }

    pub fn with_corner_radius(mut self, radius: u32) -> Self {
        self.corner_radius = radius;
        self.dirty = true;
        self
    }

    /// Add a child widget to this container.
    ///
    /// The element's bounds will be overridden by layout.
    pub fn add_child(
        &mut self,
        element: Element,
        constraint: SizeConstraint,
    ) -> Result<usize, &'static str> {
        let child_bounds = Rectangle::new(self.bounds.top_left, element.preferred_size());
        let child = ChildElement::new(element, child_bounds, constraint);
        self.children.push(child).map_err(|_| "Container full")?;
        self.dirty = true;
        self.layout();
        Ok(self.children.len() - 1)
    }

    pub fn child_bounds(&self, index: usize) -> Option<Rectangle> {
        self.children.get(index).map(|c| c.bounds)
    }

    pub fn child(&self, index: usize) -> Option<&Element> {
        self.children.get(index).map(|c| &c.element)
    }

    pub fn child_mut(&mut self, index: usize) -> Option<&mut Element> {
        self.children.get_mut(index).map(|c| {
            c.dirty = true;
            &mut c.element
        })
    }

    pub fn set_bounds(&mut self, bounds: Rectangle) {
        if self.bounds != bounds {
            self.bounds = bounds;
            self.dirty = true;
            self.layout();
        }
    }

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
                self.layout_main_axis(
                    content_start,
                    available_width,
                    available_height,
                    Axis::Horizontal,
                );
            }
            Direction::Vertical => {
                self.layout_main_axis(
                    content_start,
                    available_height,
                    available_width,
                    Axis::Vertical,
                );
            }
        }
    }

    fn layout_main_axis(
        &mut self,
        start: Point,
        available_main: u32,
        available_cross: u32,
        axis: Axis,
    ) {
        let count = self.children.len();
        if count == 0 {
            return;
        }

        // 1) Measure fixed + fit, and sum grow weights.
        let mut fixed_main: u32 = 0;
        let mut total_grow: u32 = 0;

        for child in &self.children {
            match child.size_constraint {
                SizeConstraint::Fixed(px) => fixed_main = fixed_main.saturating_add(px),
                SizeConstraint::Fit => {
                    let pref = child.preferred_size();
                    let main = axis.main(pref);
                    fixed_main = fixed_main.saturating_add(main);
                }
                SizeConstraint::Grow(_) => {
                    total_grow =
                        total_grow.saturating_add(child.size_constraint.grow_weight() as u32)
                }
            }
        }

        // 2) Allocate main sizes.
        let base_gap_total = self.gap.saturating_mul(count.saturating_sub(1) as u32);
        let mut remaining = available_main
            .saturating_sub(fixed_main)
            .saturating_sub(base_gap_total);

        // First pass sizes.
        let mut main_sizes: heapless::Vec<u32, N> = heapless::Vec::new();
        for child in &self.children {
            let s = match child.size_constraint {
                SizeConstraint::Fixed(px) => px,
                SizeConstraint::Fit => axis.main(child.preferred_size()),
                SizeConstraint::Grow(_) => {
                    if total_grow == 0 {
                        0
                    } else {
                        // proportional allocation
                        let w = child.size_constraint.grow_weight() as u64;
                        let share = (remaining as u64 * w) / (total_grow as u64);
                        share as u32
                    }
                }
            };
            main_sizes.push(s).ok();
        }

        // If we allocated grow sizes proportionally, there may be rounding leftover.
        let used_main: u32 = main_sizes.iter().copied().sum();
        remaining = available_main
            .saturating_sub(used_main)
            .saturating_sub(base_gap_total);

        // 3) Determine final gaps + leading offset based on main-axis alignment.
        let (leading, extra_gap) = match self.main_axis_alignment {
            MainAxisAlignment::Start => (0, 0),
            MainAxisAlignment::Center => (remaining / 2, 0),
            MainAxisAlignment::End => (remaining, 0),
            MainAxisAlignment::SpaceBetween => {
                if count <= 1 {
                    (0, 0)
                } else {
                    (0, remaining / (count as u32 - 1))
                }
            }
            MainAxisAlignment::SpaceAround => {
                if count == 0 {
                    (0, 0)
                } else {
                    let gap = remaining / (count as u32);
                    (gap / 2, gap)
                }
            }
            MainAxisAlignment::SpaceEvenly => {
                if count == 0 {
                    (0, 0)
                } else {
                    let gap = remaining / (count as u32 + 1);
                    (gap, gap)
                }
            }
        };

        // 4) Place children.
        let mut cursor: i32 = axis.main_point(start) + leading as i32;

        for (idx, child) in self.children.iter_mut().enumerate() {
            let child_main = main_sizes.get(idx).copied().unwrap_or(0);

            // Compute cross size.
            let pref_cross = axis.cross(child.preferred_size());
            let child_cross = match self.alignment {
                Alignment::Stretch => available_cross,
                _ => pref_cross.min(available_cross),
            };

            // Compute cross position.
            let cross_pos = match self.alignment {
                Alignment::Start | Alignment::Stretch => axis.cross_point(start),
                Alignment::Center => {
                    axis.cross_point(start) + ((available_cross - child_cross) / 2) as i32
                }
                Alignment::End => axis.cross_point(start) + (available_cross - child_cross) as i32,
            };

            let top_left = axis.compose_point(cursor, cross_pos);
            let size = axis.compose_size(child_main, child_cross);
            child.set_bounds(Rectangle::new(top_left, size));

            cursor += child_main as i32;

            // gap after, except last
            if idx + 1 < count {
                cursor += (self.gap + extra_gap) as i32;
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    fn main(&self, size: Size) -> u32 {
        match self {
            Axis::Horizontal => size.width,
            Axis::Vertical => size.height,
        }
    }

    fn cross(&self, size: Size) -> u32 {
        match self {
            Axis::Horizontal => size.height,
            Axis::Vertical => size.width,
        }
    }

    fn main_point(&self, p: Point) -> i32 {
        match self {
            Axis::Horizontal => p.x,
            Axis::Vertical => p.y,
        }
    }

    fn cross_point(&self, p: Point) -> i32 {
        match self {
            Axis::Horizontal => p.y,
            Axis::Vertical => p.x,
        }
    }

    fn compose_point(&self, main: i32, cross: i32) -> Point {
        match self {
            Axis::Horizontal => Point::new(main, cross),
            Axis::Vertical => Point::new(cross, main),
        }
    }

    fn compose_size(&self, main: u32, cross: u32) -> Size {
        match self {
            Axis::Horizontal => Size::new(main, cross),
            Axis::Vertical => Size::new(cross, main),
        }
    }
}

// Additional Container builder methods for ergonomic construction.
impl<const N: usize> Container<N> {
    /// Create a vertical stack container with automatic sizing.
    ///
    /// This is a convenience constructor for the common case of a vertical list
    /// of items. The initial bounds are zero and will be set during layout.
    pub fn vstack() -> Self {
        Self::new(Rectangle::zero(), Direction::Vertical).with_alignment(Alignment::Stretch)
    }

    /// Create a horizontal stack container with automatic sizing.
    ///
    /// This is a convenience constructor for the common case of a horizontal
    /// row of items. The initial bounds are zero and will be set during layout.
    pub fn hstack() -> Self {
        Self::new(Rectangle::zero(), Direction::Horizontal).with_alignment(Alignment::Center)
    }

    /// Builder-style method to add a child with constraint.
    ///
    /// Returns `Result<Self, &'static str>` for chaining. Unlike the silent
    /// behavior of ignoring overflow, this forces callers to explicitly handle
    /// the case where the container is full, making layout bugs more visible.
    ///
    /// # Errors
    ///
    /// Returns `Err("Container full")` if the container has reached its
    /// compile-time capacity `N`.
    pub fn with_child(
        mut self,
        element: Element,
        constraint: SizeConstraint,
    ) -> Result<Self, &'static str> {
        self.add_child(element, constraint)?;
        Ok(self)
    }
}

impl<const N: usize> Drawable for Container<N> {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        // Background/border.
        if self.style.background_color.is_some() || self.style.border_color.is_some() {
            let corner_size = Size::new(self.corner_radius, self.corner_radius);
            RoundedRectangle::with_equal_corners(self.bounds, corner_size)
                .into_styled(self.style.to_primitive_style())
                .draw(display)?;
        }

        // Children.
        for child in &self.children {
            child.element.draw(display)?;
        }

        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    fn is_dirty(&self) -> bool {
        self.dirty
            || self
                .children
                .iter()
                .any(|c| c.dirty || c.element.is_dirty())
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
        for child in &mut self.children {
            child.dirty = false;
            child.element.mark_clean();
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn dirty_region(&self) -> Option<DirtyRegion> {
        if self.dirty {
            return Some(DirtyRegion::new(self.bounds));
        }

        let mut region: Option<DirtyRegion> = None;
        for child in &self.children {
            if child.dirty || child.element.is_dirty() {
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
    fn contains_point(&self, point: TouchPoint) -> bool {
        self.bounds.contains(point.to_point())
    }

    fn handle_touch(&mut self, event: TouchEvent) -> TouchResult {
        // Forward to children (top-most last wins).
        let point = match event {
            TouchEvent::Press(p) | TouchEvent::Drag(p) => p,
        };

        for child in self.children.iter_mut().rev() {
            if child.bounds.contains(point.to_point()) {
                let result = child.element.handle_touch(event);
                match result {
                    TouchResult::NotHandled => continue,
                    TouchResult::Handled | TouchResult::Action(_) => {
                        child.dirty = true;
                        return result;
                    }
                }
            }
        }

        TouchResult::NotHandled
    }
}

// Convenience helpers (small, but reduces boilerplate at call-sites).
impl Element {
    /// A shorthand for a no-op element action (useful for static UI).
    pub fn noop_action() -> Action {
        Action::Custom(0)
    }
}
