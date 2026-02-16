//! Color definitions and palette management
//!
//! This module provides a comprehensive color system based on RGB565 format,
//! optimized for embedded displays with 16-bit color depth.
//!
//! # RGB565 Format
//! - Red: 5 bits (0-31)
//! - Green: 6 bits (0-63)
//! - Blue: 5 bits (0-31)
//!
//! To convert from 8-bit RGB: R>>3, G>>2, B>>3

use embedded_graphics::pixelcolor::Rgb565;

// ============================================================================
// Base Colors
// ============================================================================

/// Primary background color - very dark gray-blue
pub const COLOR_BACKGROUND: Rgb565 = Rgb565::new(18 >> 3, 23 >> 2, 24 >> 3);

/// Secondary background/surface color - slightly lighter than background
pub const COLOR_FOREGROUND: Rgb565 = Rgb565::new(26 >> 3, 32 >> 2, 33 >> 3);

/// Border/stroke color - medium gray
pub const COLOR_STROKE: Rgb565 = Rgb565::new(43 >> 3, 55 >> 2, 57 >> 3);

// ============================================================================
// Status Level Colors
// ============================================================================

/// Excellent status foreground - bright teal-green
pub const COLOR_EXCELLENT_FOREGROUND: Rgb565 = Rgb565::new(95 >> 3, 185 >> 2, 141 >> 3);

/// Excellent status background - dark teal
pub const COLOR_EXCELLENT_BACKGROUND: Rgb565 = Rgb565::new(29 >> 3, 47 >> 2, 43 >> 3);

/// Good status foreground - moderate green
pub const COLOR_GOOD_FOREGROUND: Rgb565 = Rgb565::new(76 >> 3, 154 >> 2, 113 >> 3);

/// Good status background - dark green
pub const COLOR_GOOD_BACKGROUND: Rgb565 = Rgb565::new(24 >> 3, 40 >> 2, 36 >> 3);

/// Poor status foreground - warm orange
pub const COLOR_POOR_FOREGROUND: Rgb565 = Rgb565::new(200 >> 3, 145 >> 2, 85 >> 3);

/// Poor status background - dark orange-brown
pub const COLOR_POOR_BACKGROUND: Rgb565 = Rgb565::new(45 >> 3, 37 >> 2, 28 >> 3);

/// Bad status foreground - muted red
pub const COLOR_BAD_FOREGROUND: Rgb565 = Rgb565::new(190 >> 3, 95 >> 2, 95 >> 3);

/// Bad status background - dark red
pub const COLOR_BAD_BACKGROUND: Rgb565 = Rgb565::new(43 >> 3, 29 >> 2, 29 >> 3);

// ============================================================================
// Text Colors
// ============================================================================

/// Pure white - maximum brightness in RGB565
pub const WHITE: Rgb565 = Rgb565::new(31, 63, 31);

/// Light gray - for secondary text
pub const LIGHT_GRAY: Rgb565 = Rgb565::new(21, 42, 21);

/// Medium gray - for disabled or tertiary text
pub const GRAY: Rgb565 = Rgb565::new(16, 32, 16);

/// Dark gray - for subtle text
pub const DARK_GRAY: Rgb565 = Rgb565::new(10, 20, 10);

// ============================================================================
// Color Palette
// ============================================================================

/// A cohesive color palette for consistent UI theming.
///
/// This struct groups related colors together to ensure visual consistency
/// across the entire application. It supports both dark and light themes.
///
/// # Examples
///
/// ```ignore
/// // Use the default dark theme
/// let palette = ColorPalette::default();
///
/// // Or create a light theme
/// let light_palette = ColorPalette::light();
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorPalette {
    /// Primary accent color - used for key interactive elements
    pub primary: Rgb565,

    /// Secondary accent color - used for less prominent actions
    pub secondary: Rgb565,

    /// Main background color
    pub background: Rgb565,

    /// Surface color for cards, panels, and elevated elements
    pub surface: Rgb565,

    /// Error and alert color
    pub error: Rgb565,

    /// Primary text color - high contrast
    pub text_primary: Rgb565,

    /// Secondary text color - lower contrast for less important information
    pub text_secondary: Rgb565,

    /// Border color for separators and outlines
    pub border: Rgb565,
}

impl Default for ColorPalette {
    /// Returns the default dark theme palette
    fn default() -> Self {
        Self::dark()
    }
}

impl ColorPalette {
    /// Creates a dark theme palette (default)
    ///
    /// The dark theme uses light text on dark backgrounds, optimized for
    /// low-light viewing and reduced eye strain during extended use.
    pub fn dark() -> Self {
        Self {
            primary: COLOR_EXCELLENT_FOREGROUND,
            secondary: COLOR_GOOD_FOREGROUND,
            background: COLOR_BACKGROUND,
            surface: COLOR_FOREGROUND,
            error: COLOR_BAD_FOREGROUND,
            text_primary: WHITE,
            text_secondary: LIGHT_GRAY,
            border: COLOR_STROKE,
        }
    }

    /// Creates a light theme palette
    ///
    /// The light theme uses dark text on light backgrounds, suitable for
    /// bright environments or user preference.
    pub fn light() -> Self {
        Self {
            primary: COLOR_EXCELLENT_FOREGROUND,
            secondary: COLOR_GOOD_FOREGROUND,
            background: WHITE,
            surface: COLOR_FOREGROUND,
            error: COLOR_BAD_FOREGROUND,
            text_primary: COLOR_BACKGROUND,
            text_secondary: DARK_GRAY,
            border: COLOR_STROKE,
        }
    }
}
