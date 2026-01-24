// src/ui/mod.rs
//! Baro UI System - A modular, efficient UI framework for embedded displays
//!
//! This module provides a complete UI system with:
//! - Core traits for drawable and touchable elements
//! - Layout containers (vertical/horizontal with flexible sizing)
//! - Scrollable containers with overflow handling
//! - Styled components (buttons, text)
//! - Dirty region tracking for efficient rendering
//! - Event system for page updates

pub mod components;
pub mod core;
pub mod layouts;
pub mod styling;

// Re-export commonly used items
pub use components::{Button, MultiLineText, TextComponent, TextSize};
pub use core::{
    Action, DirtyRegion, Drawable, Interactive, PageEvent, PageId, SensorData, StorageEvent,
    SystemEvent, TouchEvent, TouchPoint, TouchResult, Touchable,
};
pub use layouts::{
    Alignment, Container, Direction, ScrollDirection, ScrollableContainer, SizeConstraint,
};
pub use styling::{BorderRadius, ButtonVariant, ColorPalette, Padding, Spacing, Style, Theme};
