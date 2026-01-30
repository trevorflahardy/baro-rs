// src/ui/styling.rs
//! Styling system for UI elements

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::primitives::{PrimitiveStyle, PrimitiveStyleBuilder};

// Core color palette
// RGB565 format: R(5 bits), G(6 bits), B(5 bits)
// Convert from 8-bit RGB: R>>3, G>>2, B>>3
pub const COLOR_BACKGROUND: Rgb565 = Rgb565::new(18 >> 3, 23 >> 2, 24 >> 3);
pub const COLOR_FOREGROUND: Rgb565 = Rgb565::new(26 >> 3, 32 >> 2, 33 >> 3);
pub const _COLOR_STROKE: Rgb565 = Rgb565::new(43 >> 3, 55 >> 2, 57 >> 3);

/// Colors for status levels - Excellent
pub const COLOR_EXCELLENT_FOREGROUND: Rgb565 = Rgb565::new(95 >> 3, 185 >> 2, 141 >> 3);
pub const _COLOR_EXCELLENT_BACKGROUND: Rgb565 = Rgb565::new(29 >> 3, 47 >> 2, 43 >> 3);

/// Colors for status levels - Good
pub const COLOR_GOOD_FOREGROUND: Rgb565 = Rgb565::new(76 >> 3, 154 >> 2, 113 >> 3);
pub const _COLOR_GOOD_BACKGROUND: Rgb565 = Rgb565::new(24 >> 3, 40 >> 2, 36 >> 3);

/// Colors for status levels - Poor
pub const _COLOR_POOR_FOREGROUND: Rgb565 = Rgb565::new(200 >> 3, 145 >> 2, 85 >> 3);
pub const COLOR_POOR_BACKGROUND: Rgb565 = Rgb565::new(45 >> 3, 37 >> 2, 28 >> 3);

/// Colors for status levels - Bad
pub const COLOR_BAD_FOREGROUND: Rgb565 = Rgb565::new(190 >> 3, 95 >> 2, 95 >> 3);
pub const _COLOR_BAD_BACKGROUND: Rgb565 = Rgb565::new(43 >> 3, 29 >> 2, 29 >> 3);

// Text colors
pub const WHITE: Rgb565 = Rgb565::new(31, 63, 31); // Max brightness in RGB565
pub const LIGHT_GRAY: Rgb565 = Rgb565::new(21, 42, 21);
pub const _GRAY: Rgb565 = Rgb565::new(16, 32, 16);
pub const DARK_GRAY: Rgb565 = Rgb565::new(10, 20, 10);

/// Color palette for the UI
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorPalette {
    pub primary: Rgb565,
    pub secondary: Rgb565,
    pub background: Rgb565,
    pub surface: Rgb565,
    pub error: Rgb565,
    pub text_primary: Rgb565,
    pub text_secondary: Rgb565,
    pub border: Rgb565,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            primary: COLOR_EXCELLENT_FOREGROUND,
            secondary: COLOR_GOOD_FOREGROUND,
            background: COLOR_BACKGROUND,
            surface: COLOR_FOREGROUND,
            error: COLOR_BAD_FOREGROUND,
            text_primary: WHITE,
            text_secondary: LIGHT_GRAY,
            border: _COLOR_STROKE,
        }
    }
}

impl ColorPalette {
    /// Dark theme palette
    pub fn dark() -> Self {
        Self::default()
    }

    /// Light theme palette
    pub fn light() -> Self {
        Self {
            primary: COLOR_EXCELLENT_FOREGROUND,
            secondary: COLOR_GOOD_FOREGROUND,
            background: WHITE,
            surface: COLOR_FOREGROUND,
            error: COLOR_BAD_FOREGROUND,
            text_primary: COLOR_BACKGROUND,
            text_secondary: DARK_GRAY,
            border: _COLOR_STROKE,
        }
    }
}

/// Spacing values for consistent layout
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spacing {
    pub tiny: u32,
    pub small: u32,
    pub medium: u32,
    pub large: u32,
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

/// Border radius options
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderRadius {
    pub none: u32,
    pub small: u32,
    pub medium: u32,
    pub large: u32,
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

/// Style configuration for UI elements
#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub background_color: Option<Rgb565>,
    pub foreground_color: Option<Rgb565>,
    pub border_color: Option<Rgb565>,
    pub border_width: u32,
    pub padding: Padding,
}

impl Default for Style {
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_background(mut self, color: Rgb565) -> Self {
        self.background_color = Some(color);
        self
    }

    pub fn with_foreground(mut self, color: Rgb565) -> Self {
        self.foreground_color = Some(color);
        self
    }

    pub fn with_border(mut self, color: Rgb565, width: u32) -> Self {
        self.border_color = Some(color);
        self.border_width = width;
        self
    }

    pub fn with_padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    pub fn with_corners(mut self, _radius: u32) -> Self {
        // Corner radius handling can be implemented as needed
        self
    }

    /// Convert this style to a PrimitiveStyle for drawing
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

/// Padding around an element
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Padding {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

impl Padding {
    pub fn all(value: u32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub fn symmetric(vertical: u32, horizontal: u32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    pub fn new(top: u32, right: u32, bottom: u32, left: u32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn horizontal(&self) -> u32 {
        self.left + self.right
    }

    pub fn vertical(&self) -> u32 {
        self.top + self.bottom
    }
}

/// Button style variants
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Outline,
    Text,
    Pill(Rgb565),
}

impl ButtonVariant {
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

/// Global theme configuration
#[derive(Default)]
pub struct Theme {
    pub palette: ColorPalette,
    pub spacing: Spacing,
    pub border_radius: BorderRadius,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            palette: ColorPalette::dark(),
            ..Self::default()
        }
    }

    pub fn light() -> Self {
        Self {
            palette: ColorPalette::light(),
            ..Self::default()
        }
    }
}
