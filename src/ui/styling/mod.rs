//! Styling system for UI elements
//!
//! This module provides a comprehensive styling framework including:
//! - Color definitions and palettes
//! - Layout primitives (spacing, padding, border radius)
//! - Style configuration for individual elements
//! - Global theme management
//!
//! # Organization
//!
//! The styling system is split into logical modules:
//! - [`colors`] - Color constants and palette management
//! - [`layout`] - Spacing, padding, and border radius
//! - [`style`] - Style configuration and button variants
//! - [`theme`] - Global theme combining all styling parameters
//!
//! # Examples
//!
//! ```ignore
//! use ui::styling::*;
//!
//! // Create a themed element style
//! let theme = Theme::default();
//! let style = Style::new()
//!     .with_background(theme.palette.surface)
//!     .with_foreground(theme.palette.text_primary)
//!     .with_padding(Padding::all(theme.spacing.medium));
//!
//! // Use predefined button variants
//! let button_style = ButtonVariant::Primary.to_style(&theme.palette);
//! ```

// Module declarations
pub mod colors;
pub mod layout;
pub mod style;
pub mod theme;

// Re-export commonly used items for convenience
pub use colors::{
    COLOR_BACKGROUND, COLOR_BAD_FOREGROUND, COLOR_EXCELLENT_FOREGROUND, COLOR_FOREGROUND,
    COLOR_GOOD_FOREGROUND, COLOR_POOR_BACKGROUND, COLOR_STROKE, ColorPalette, DARK_GRAY,
    LIGHT_GRAY, WHITE,
};
pub use layout::{BorderRadius, Padding, Spacing};
pub use style::{ButtonVariant, Style};
pub use theme::Theme;
