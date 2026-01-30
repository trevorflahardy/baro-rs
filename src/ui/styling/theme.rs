//! Global theme management
//!
//! Combines color palette, spacing, and layout parameters into a unified
//! theme configuration that can be applied application-wide.

use super::colors::ColorPalette;
use super::layout::{BorderRadius, Spacing};

// ============================================================================
// Theme
// ============================================================================

/// Global theme configuration
///
/// Aggregates all styling parameters (colors, spacing, border radii) into
/// a single cohesive theme that can be passed throughout the UI.
///
/// # Design Philosophy
///
/// Centralizing theme parameters ensures visual consistency and makes it
/// easy to switch between different themes (e.g., dark/light mode) without
/// modifying individual components.
///
/// # Examples
///
/// ```ignore
/// // Use the default dark theme
/// let theme = Theme::default();
///
/// // Access theme properties
/// let padding = theme.spacing.medium;
/// let radius = theme.border_radius.small;
/// let color = theme.palette.primary;
///
/// // Or create a light theme
/// let light_theme = Theme::light();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// The active color palette (dark or light)
    pub palette: ColorPalette,

    /// Spacing scale for consistent layout
    pub spacing: Spacing,

    /// Border radius options for rounded corners
    pub border_radius: BorderRadius,
}

impl Default for Theme {
    /// Returns the default theme (dark mode)
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// Creates a dark theme
    ///
    /// The dark theme is optimized for low-light environments and extended
    /// viewing sessions, reducing eye strain.
    ///
    /// # Returns
    ///
    /// A `Theme` configured with dark color palette and standard spacing/radii.
    pub fn dark() -> Self {
        Self {
            palette: ColorPalette::dark(),
            spacing: Spacing::default(),
            border_radius: BorderRadius::default(),
        }
    }

    /// Creates a light theme
    ///
    /// The light theme is suitable for bright environments and provides
    /// high contrast for outdoor visibility.
    ///
    /// # Returns
    ///
    /// A `Theme` configured with light color palette and standard spacing/radii.
    pub fn light() -> Self {
        Self {
            palette: ColorPalette::light(),
            spacing: Spacing::default(),
            border_radius: BorderRadius::default(),
        }
    }
}
