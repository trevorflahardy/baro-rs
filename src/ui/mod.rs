// src/ui/mod.rs
//! Baro UI system
//!
//! A small UI toolkit for embedded displays built on top of `embedded-graphics`.
//! The design goal is to make UI code readable and predictable on
//! resource-constrained targets.
//!
//! ## Modules
//! - [`core`] — foundational traits and events (`Drawable`, `Touchable`, `PageEvent`, …)
//! - [`styling`] — `Style`, `Theme`, padding/spacing helpers
//! - [`components`] — concrete widgets (text, buttons)
//! - [`elements`] — a concrete `Element` enum used for heterogeneous layout
//! - [`layouts`] — layout primitives (`Container`, `ScrollableContainer`)
//!
//! ## The important mental model
//! 1. **Widgets are responsible for drawing themselves** within their bounds.
//! 2. **Layouts are responsible for assigning bounds** to widgets.
//! 3. Dirty tracking (`is_dirty` + `dirty_region`) lets you redraw only what changed.
//!
//! ## Layout: `Container` (flex-like)
//! `Container` owns its children (as [`Element`]) and will:
//! - compute child bounds (horizontal or vertical)
//! - set each child's bounds
//! - draw itself + all children
//! - forward touch events to children
//!
//! ### Evenly-spaced row ("3 children in a horizontal container")
//! ```ignore
//! use crate::ui::{Alignment, Container, Direction, Element, MainAxisAlignment, SizeConstraint, TextSize};
//! use embedded_graphics::prelude::*;
//! use embedded_graphics::primitives::Rectangle;
//!
//! let bounds = Rectangle::new(Point::new(0, 0), Size::new(320, 40));
//! let mut row = Container::<3>::new(bounds, Direction::Horizontal)
//!     .with_alignment(Alignment::Center)
//!     .with_main_axis_alignment(MainAxisAlignment::SpaceEvenly);
//!
//! let hint = Rectangle::new(Point::zero(), Size::new(320, 1));
//! row.add_child(Element::text(hint, "A", TextSize::Medium), SizeConstraint::Fit).ok();
//! row.add_child(Element::text(hint, "B", TextSize::Medium), SizeConstraint::Fit).ok();
//! row.add_child(Element::text(hint, "C", TextSize::Medium), SizeConstraint::Fit).ok();
//! ```
//!
//! ### Flex-grow style sizing
//! ```ignore
//! // left fits, right grows to fill remaining space
//! row.add_child(left, SizeConstraint::Fit).ok();
//! row.add_child(right, SizeConstraint::Grow(1)).ok();
//! ```

pub mod components;
pub mod core;
pub mod elements;
pub mod layouts;
pub mod styling;

// Re-export commonly used items.
pub use components::{Button, MultiLineText, TextComponent, TextSize};
pub use core::{
    Action, DirtyRegion, Drawable, Interactive, PageEvent, PageId, SensorData, StorageEvent,
    SystemEvent, TouchEvent, TouchPoint, TouchResult, Touchable,
};
pub use elements::{Element, MAX_CONTAINER_CHILDREN};
pub use layouts::{
    Alignment, Container, Direction, MainAxisAlignment, ScrollDirection, ScrollableContainer,
    SizeConstraint,
};
pub use styling::{
    BorderRadius, ButtonVariant, ColorPalette, Padding, Spacing, Style, Theme, WHITE,
};
