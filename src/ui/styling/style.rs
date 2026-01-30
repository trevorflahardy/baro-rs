//! Style configuration for UI elements
//!
//! Provides the core `Style` struct and builder methods for defining the
//! visual appearance of UI components (colors, borders, padding).

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::primitives::{PrimitiveStyle, PrimitiveStyleBuilder};

use super::colors::{ColorPalette, WHITE};
use super::layout::Padding;

// ============================================================================
// Style
// ============================================================================

/// Visual style configuration for a UI element
///
/// Defines appearance properties such as colors, borders, and padding.
/// Use the builder pattern to construct styles incrementally.
///
/// # Examples
///
/// ```ignore
/// use ui::styling::*;
///
/// // Simple text style
/// let text_style = Style::new()
///     .with_foreground(WHITE);
///
/// // Card with border and padding
/// let card_style = Style::new()
///     .with_background(COLOR_FOREGROUND)
///     .with_border(COLOR_STROKE, 2)
///     .with_padding(Padding::all(8));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// Background fill color (if any)
    pub background_color: Option<Rgb565>,

    /// Foreground/text color (if any)
    pub foreground_color: Option<Rgb565>,

    /// Border color (if any)
    pub border_color: Option<Rgb565>,

    /// Border width in pixels (0 = no border)
    pub border_width: u32,

    /// Internal padding around content
    pub padding: Padding,
}

impl Default for Style {
    /// Returns a minimal default style with white text and no background or border
    fn default() -> Self {
        Self {
            background_color: None,
            foreground_color: Some(WHITE),
            border_color: None,
            border_width: 0,
            padding: Padding::default(),
        }
    }
}

impl Style {
    /// Creates a new empty style with defaults
    ///
    /// Prefer using builder methods to configure:
    ///
    /// ```ignore
    /// Style::new()
    ///     .with_background(COLOR_FOREGROUND)
    ///     .with_padding(Padding::all(8))
    ///     .with_border(COLOR_STROKE, 2)
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the background color
    ///
    /// # Arguments
    /// * `color` - RGB565 color value for the background
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let style = Style::new().with_background(COLOR_FOREGROUND);
    /// ```
    pub fn with_background(mut self, color: Rgb565) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Sets the foreground (text) color
    ///
    /// # Arguments
    /// * `color` - RGB565 color value for text/foreground elements
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let style = Style::new().with_foreground(WHITE);
    /// ```
    pub fn with_foreground(mut self, color: Rgb565) -> Self {
        self.foreground_color = Some(color);
        self
    }

    /// Sets the border color and width
    ///
    /// A width of 0 effectively disables the border.
    ///
    /// # Arguments
    /// * `color` - RGB565 color value for the border
    /// * `width` - Border width in pixels
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // 2px gray border
    /// let style = Style::new().with_border(COLOR_STROKE, 2);
    ///
    /// // No border
    /// let style = Style::new().with_border(COLOR_STROKE, 0);
    /// ```
    pub fn with_border(mut self, color: Rgb565, width: u32) -> Self {
        self.border_color = Some(color);
        self.border_width = width;
        self
    }

    /// Sets the padding around the element
    ///
    /// # Arguments
    /// * `padding` - Padding configuration
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Equal padding on all sides
    /// let style = Style::new().with_padding(Padding::all(8));
    ///
    /// // Different vertical and horizontal
    /// let style = Style::new().with_padding(Padding::symmetric(12, 16));
    ///
    /// // Individual sides
    /// let style = Style::new().with_padding(Padding::new(8, 16, 8, 16));
    /// ```
    pub fn with_padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    /// Converts this style to a `PrimitiveStyle` for embedded-graphics drawing
    ///
    /// This method is used internally when rendering styled shapes and backgrounds.
    ///
    /// # Returns
    ///
    /// A `PrimitiveStyle<Rgb565>` compatible with embedded-graphics drawing operations.
    pub fn to_primitive_style(&self) -> PrimitiveStyle<Rgb565> {
        let mut builder = PrimitiveStyleBuilder::new();

        if let Some(bg) = self.background_color {
            builder = builder.fill_color(bg);
        }

        if let Some(border) = self.border_color
            && self.border_width > 0
        {
            builder = builder.stroke_color(border).stroke_width(self.border_width);
        }

        builder.build()
    }
}

// ============================================================================
// Button Variants
// ============================================================================

/// Predefined button style variants for consistent button appearances
///
/// Each variant provides a semantically meaningful button style that
/// automatically adapts to the current color palette.
///
/// # Examples
///
/// ```ignore
/// let palette = ColorPalette::default();
///
/// // Primary action button
/// let primary_style = ButtonVariant::Primary.to_style(&palette);
///
/// // Subtle text-only button
/// let text_style = ButtonVariant::Text.to_style(&palette);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    /// Primary action button - most prominent, used for main actions
    Primary,

    /// Secondary action button - less prominent than primary
    Secondary,

    /// Outlined button - subtle emphasis with border
    Outline,

    /// Text-only button - minimal visual weight
    Text,

    /// Pill-shaped button with custom background color
    Pill(Rgb565),
}

impl ButtonVariant {
    /// Converts the variant to a concrete style based on a color palette
    ///
    /// # Arguments
    /// * `palette` - The color palette to derive colors from
    ///
    /// # Returns
    ///
    /// A `Style` configured according to the variant's visual language.
    pub fn to_style(&self, palette: &ColorPalette) -> Style {
        match self {
            ButtonVariant::Primary => Style::new()
                .with_background(palette.primary)
                .with_foreground(WHITE)
                .with_padding(Padding::symmetric(8, 16)),

            ButtonVariant::Secondary => Style::new()
                .with_background(palette.secondary)
                .with_foreground(WHITE)
                .with_padding(Padding::symmetric(8, 16)),

            ButtonVariant::Outline => Style::new()
                .with_background(palette.surface)
                .with_foreground(palette.text_primary)
                .with_border(palette.border, 2)
                .with_padding(Padding::symmetric(8, 16)),

            ButtonVariant::Text => Style::new()
                .with_foreground(palette.primary)
                .with_padding(Padding::symmetric(4, 8)),

            ButtonVariant::Pill(fg_color) => Style::new().with_background(*fg_color),
        }
    }
}
