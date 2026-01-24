// src/ui/styling.rs
//! Styling system for UI elements

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::primitives::{PrimitiveStyle, PrimitiveStyleBuilder};

// Define common colors
const DODGER_BLUE: Rgb565 = Rgb565::new(30 >> 3, 144 >> 2, 255 >> 3);
const STEEL_BLUE: Rgb565 = Rgb565::new(70 >> 3, 130 >> 2, 180 >> 3);
const BLACK: Rgb565 = Rgb565::new(0, 0, 0);
const WHITE: Rgb565 = Rgb565::new(31, 63, 31);
const LIGHT_GRAY: Rgb565 = Rgb565::new(21, 42, 21);
const GRAY: Rgb565 = Rgb565::new(16, 32, 16);
const DARK_GRAY: Rgb565 = Rgb565::new(10, 20, 10);
const CRIMSON: Rgb565 = Rgb565::new(220 >> 3, 20 >> 2, 60 >> 3);
const SURFACE_DARK: Rgb565 = Rgb565::new(0x08 >> 3, 0x10 >> 2, 0x18 >> 3);
const SURFACE_LIGHT: Rgb565 = Rgb565::new(0xF0 >> 3, 0xF0 >> 2, 0xF0 >> 3);

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
            primary: DODGER_BLUE,
            secondary: STEEL_BLUE,
            background: BLACK,
            surface: SURFACE_DARK,
            error: CRIMSON,
            text_primary: WHITE,
            text_secondary: LIGHT_GRAY,
            border: GRAY,
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
            primary: DODGER_BLUE,
            secondary: STEEL_BLUE,
            background: WHITE,
            surface: SURFACE_LIGHT,
            error: CRIMSON,
            text_primary: BLACK,
            text_secondary: DARK_GRAY,
            border: GRAY,
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
