// src/ui/components/text.rs
//! Text component for displaying text with styling

use crate::ui::core::{DirtyRegion, Drawable};
use crate::ui::styling::Style;
use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::mono_font::{MonoFont, MonoTextStyle, ascii::FONT_6X10};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::{Alignment, Text as EgText};

/// Text size variants
///
/// Provides three preset text sizes with corresponding embedded-graphics fonts:
/// - `Small`: 5x8 font
/// - `Medium`: 6x10 font (default)
/// - `Large`: 10x20 font
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextSize {
    Small,
    Medium,
    Large,
}

impl TextSize {
    pub fn font(&self) -> &'static MonoFont<'static> {
        match self {
            TextSize::Small => &embedded_graphics::mono_font::ascii::FONT_5X8,
            TextSize::Medium => &FONT_6X10,
            TextSize::Large => &embedded_graphics::mono_font::ascii::FONT_10X20,
        }
    }
}

/// Text component for displaying styled text
///
/// A simple text display component with configurable size, alignment, and styling.
/// Supports up to 128 characters of text content.
///
/// # Features
/// - Three size presets (Small, Medium, Large)
/// - Left, Center, or Right alignment
/// - Optional background and border styling
/// - Automatic dirty tracking when text changes
///
/// # Examples
/// ```ignore
/// let text = TextComponent::new(
///     Rectangle::new(Point::new(20, 60), Size::new(280, 20)),
///     "Temperature: 22.5°C",
///     TextSize::Medium
/// )
/// .with_alignment(Alignment::Center);
/// ```
pub struct TextComponent {
    bounds: Rectangle,
    text: heapless::String<128>,
    size: TextSize,
    alignment: Alignment,
    style: Style,
    dirty: bool,
}

impl TextComponent {
    pub fn new(bounds: Rectangle, text: &str, size: TextSize) -> Self {
        let mut text_string = heapless::String::new();
        text_string.push_str(text).ok();

        Self {
            bounds,
            text: text_string,
            size,
            alignment: Alignment::Left,
            style: Style::default(),
            dirty: true,
        }
    }

    /// Set the text alignment (Left, Center, or Right).
    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Update the displayed text.
    ///
    /// Automatically marks the component as dirty if the text changed.
    pub fn set_text(&mut self, text: &str) {
        let mut new_text = heapless::String::new();
        new_text.push_str(text).ok();

        if self.text != new_text {
            self.text = new_text;
            self.dirty = true;
        }
    }

    /// Get the current text content.
    pub fn text(&self) -> &str {
        &self.text
    }

    fn text_position(&self) -> Point {
        match self.alignment {
            Alignment::Left => Point::new(
                self.bounds.top_left.x + self.style.padding.left as i32,
                self.bounds.top_left.y + self.style.padding.top as i32,
            ),
            Alignment::Center => Point::new(
                self.bounds.center().x,
                self.bounds.top_left.y + self.style.padding.top as i32,
            ),
            Alignment::Right => Point::new(
                self.bounds.top_left.x + self.bounds.size.width as i32
                    - self.style.padding.right as i32,
                self.bounds.top_left.y + self.style.padding.top as i32,
            ),
        }
    }
}

impl Drawable for TextComponent {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        // Draw background if specified
        if self.style.background_color.is_some() {
            self.bounds
                .into_styled(self.style.to_primitive_style())
                .draw(display)?;
        }

        // Draw text
        let text_color = self.style.foreground_color.unwrap_or(Rgb565::WHITE);
        let text_style = MonoTextStyle::new(self.size.font(), text_color);

        let position = self.text_position();

        EgText::with_alignment(&self.text, position, text_style, self.alignment).draw(display)?;

        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn dirty_region(&self) -> Option<DirtyRegion> {
        if self.dirty {
            Some(DirtyRegion::new(self.bounds))
        } else {
            None
        }
    }
}

/// Multi-line text component with word wrapping
pub struct MultiLineText {
    bounds: Rectangle,
    lines: heapless::Vec<heapless::String<64>, 16>,
    size: TextSize,
    line_spacing: u32,
    style: Style,
    dirty: bool,
}

impl MultiLineText {
    pub fn new(bounds: Rectangle, text: &str, size: TextSize) -> Self {
        let mut component = Self {
            bounds,
            lines: heapless::Vec::new(),
            size,
            line_spacing: 2,
            style: Style::default(),
            dirty: true,
        };

        component.set_text(text);
        component
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn with_line_spacing(mut self, spacing: u32) -> Self {
        self.line_spacing = spacing;
        self
    }

    pub fn set_text(&mut self, text: &str) {
        self.lines.clear();

        // Simple line breaking by newlines and character limit
        let max_chars =
            (self.bounds.size.width / (self.size.font().character_size.width + 1)) as usize;

        for line in text.split('\n') {
            if line.len() <= max_chars {
                let mut line_string = heapless::String::new();
                line_string.push_str(line).ok();
                self.lines.push(line_string).ok();
            } else {
                // Simple word wrapping
                let mut current_line = heapless::String::<64>::new();
                for word in line.split_whitespace() {
                    if current_line.len() + word.len() < max_chars {
                        if !current_line.is_empty() {
                            current_line.push(' ').ok();
                        }
                        current_line.push_str(word).ok();
                    } else {
                        if !current_line.is_empty() {
                            self.lines.push(current_line.clone()).ok();
                        }
                        current_line.clear();
                        current_line.push_str(word).ok();
                    }
                }
                if !current_line.is_empty() {
                    self.lines.push(current_line).ok();
                }
            }
        }

        self.dirty = true;
    }
}

impl Drawable for MultiLineText {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        // Draw background if specified
        if self.style.background_color.is_some() {
            self.bounds
                .into_styled(self.style.to_primitive_style())
                .draw(display)?;
        }

        // Draw each line
        let text_color = self.style.foreground_color.unwrap_or(Rgb565::WHITE);
        let text_style = MonoTextStyle::new(self.size.font(), text_color);
        let line_height = self.size.font().character_size.height + self.line_spacing;

        let mut y = self.bounds.top_left.y + self.style.padding.top as i32;
        let x = self.bounds.top_left.x + self.style.padding.left as i32;

        for line in &self.lines {
            EgText::new(line, Point::new(x, y), text_style).draw(display)?;
            y += line_height as i32;

            // Stop if we exceed bounds
            if y > self.bounds.top_left.y + self.bounds.size.height as i32 {
                break;
            }
        }

        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn dirty_region(&self) -> Option<DirtyRegion> {
        if self.dirty {
            Some(DirtyRegion::new(self.bounds))
        } else {
            None
        }
    }
}
