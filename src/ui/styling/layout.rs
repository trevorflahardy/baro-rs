//! Layout primitives for consistent spacing and dimensions
//!
//! This module provides spacing constants, padding utilities, and layout
//! helpers to maintain visual consistency throughout the UI.

// ============================================================================
// Spacing
// ============================================================================

/// Standard spacing scale for consistent layout
///
/// Use these values for margins, padding, gaps, and other layout measurements
/// to maintain a harmonious visual rhythm across the interface.
///
/// # Design Philosophy
///
/// The spacing scale follows a geometric progression to create clear visual
/// hierarchy and prevent awkward "in-between" spacing decisions.
///
/// # Examples
///
/// ```ignore
/// // Use spacing for padding
/// Padding::all(Spacing::default().medium)
///
/// // Or access directly
/// let gap = Spacing::default().large;
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spacing {
    /// Minimal spacing (2px) - for tight elements or fine adjustments
    pub tiny: u32,

    /// Small spacing (4px) - for compact layouts
    pub small: u32,

    /// Medium spacing (8px) - standard spacing for most elements
    pub medium: u32,

    /// Large spacing (16px) - for major sections or breathing room
    pub large: u32,

    /// Extra large spacing (24px) - for page-level separation
    pub xlarge: u32,
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            tiny: 2,
            small: 4,
            medium: 8,
            large: 16,
            xlarge: 24,
        }
    }
}

// ============================================================================
// Border Radius
// ============================================================================

/// Border radius options for rounded corners
///
/// Provides a consistent set of corner radius values to maintain visual
/// coherence across different UI elements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderRadius {
    /// No rounding (0px) - sharp corners
    pub none: u32,

    /// Small rounding (4px) - subtle softening
    pub small: u32,

    /// Medium rounding (8px) - standard rounded corners
    pub medium: u32,

    /// Large rounding (16px) - pronounced curves
    pub large: u32,

    /// Circular (999px) - creates circular or pill-shaped elements
    pub circle: u32,
}

impl Default for BorderRadius {
    fn default() -> Self {
        Self {
            none: 0,
            small: 4,
            medium: 8,
            large: 16,
            circle: 999,
        }
    }
}

// ============================================================================
// Padding
// ============================================================================

/// Padding around an element (top, right, bottom, left)
///
/// Defines internal spacing within UI elements. Padding creates breathing
/// room between content and its container boundaries.
///
/// # Examples
///
/// ```ignore
/// // Equal padding on all sides (8px)
/// let p = Padding::all(8);
///
/// // Different vertical (12px) and horizontal (16px)
/// let p = Padding::symmetric(12, 16);
///
/// // Individual control: top=8, right=16, bottom=8, left=16
/// let p = Padding::new(8, 16, 8, 16);
///
/// // Calculate total space consumed
/// let total_width = p.horizontal();  // left + right
/// let total_height = p.vertical();   // top + bottom
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Padding {
    /// Top padding (pixels)
    pub top: u32,

    /// Right padding (pixels)
    pub right: u32,

    /// Bottom padding (pixels)
    pub bottom: u32,

    /// Left padding (pixels)
    pub left: u32,
}

impl Padding {
    /// Creates equal padding on all sides
    ///
    /// # Arguments
    /// * `value` - Padding value in pixels, applied to all sides
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let p = Padding::all(8);  // 8px on all sides
    /// ```
    pub fn all(value: u32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Creates symmetric padding (vertical and horizontal)
    ///
    /// # Arguments
    /// * `vertical` - Padding for top and bottom (pixels)
    /// * `horizontal` - Padding for left and right (pixels)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let p = Padding::symmetric(12, 16);  // 12px top/bottom, 16px left/right
    /// ```
    pub fn symmetric(vertical: u32, horizontal: u32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Creates padding with individual control for each side
    ///
    /// # Arguments
    /// * `top` - Top padding (pixels)
    /// * `right` - Right padding (pixels)
    /// * `bottom` - Bottom padding (pixels)
    /// * `left` - Left padding (pixels)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let p = Padding::new(8, 16, 8, 16);
    /// ```
    pub fn new(top: u32, right: u32, bottom: u32, left: u32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Returns total horizontal padding (left + right)
    ///
    /// Useful for calculating element widths accounting for padding.
    pub fn horizontal(&self) -> u32 {
        self.left + self.right
    }

    /// Returns total vertical padding (top + bottom)
    ///
    /// Useful for calculating element heights accounting for padding.
    pub fn vertical(&self) -> u32 {
        self.top + self.bottom
    }
}
