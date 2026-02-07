// src/ui/components/button.rs
//! Button component with various styles and states

use crate::ui::core::{
    Action, DirtyRegion, Drawable, TouchEvent, TouchPoint, TouchResult, Touchable,
};
use crate::ui::styling::{ButtonVariant, ColorPalette, Style};
use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::mono_font::{MonoTextStyle, ascii::FONT_6X10};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Rectangle, RoundedRectangle};
use embedded_graphics::text::{Alignment as TextAlignment, Text};

/// Button state
#[derive(Debug, Clone, Copy, PartialEq)]
enum ButtonState {
    Normal,
    Pressed,
    Disabled,
}

/// Button component with label and action
///
/// An interactive button that responds to touch events and can trigger actions.
/// Supports multiple visual variants (Primary, Secondary, etc.) and states
/// (Normal, Pressed, Disabled).
///
/// # Visual Features
/// - Rounded corners (configurable radius)
/// - Color variants based on ColorPalette
/// - Visual feedback on press (darkened background)
/// - Disabled state with dimmed appearance
///
/// # Touch Behavior
/// - Triggers action immediately on press
/// - Provides visual feedback during press
/// - Updates state during drag (pressed if over button, normal if dragged away)
///
/// # Examples
/// ```ignore
/// let button = Button::new(
///     Rectangle::new(Point::new(20, 50), Size::new(280, 50)),
///     "Settings",
///     Action::NavigateToPage(PageId::Settings)
/// )
/// .with_variant(ButtonVariant::Primary)
/// .with_palette(ColorPalette::default());
/// ```
pub struct Button {
    bounds: Rectangle,
    label: heapless::String<32>,
    action: Action,
    state: ButtonState,
    variant: ButtonVariant,
    palette: ColorPalette,
    border_radius: u32,
    dirty: bool,
}

impl Button {
    /// Create a new button with the specified bounds, label, and action.
    ///
    /// # Parameters
    /// - `bounds`: Position and size of the button
    /// - `label`: Text displayed on the button (max 32 characters)
    /// - `action`: Action triggered when button is pressed
    ///
    /// By default, buttons use Primary variant with default ColorPalette.
    pub fn new(bounds: Rectangle, label: &str, action: Action) -> Self {
        let mut label_string = heapless::String::new();
        label_string.push_str(label).ok();

        Self {
            bounds,
            label: label_string,
            action,
            state: ButtonState::Normal,
            variant: ButtonVariant::Primary,
            palette: ColorPalette::default(),
            border_radius: 8,
            dirty: true,
        }
    }

    /// Create a button with automatic sizing based on label.
    ///
    /// The button will size itself to fit the label with standard padding.
    /// Minimum button size is 100x44 for touchability.
    pub fn auto(label: &str, action: Action) -> Self {
        let mut label_string = heapless::String::new();
        label_string.push_str(label).ok();

        // Standard font for buttons
        let font = &FONT_6X10;
        let char_width = font.character_size.width;
        let char_height = font.character_size.height;

        // Calculate content size with padding
        const HORIZONTAL_PADDING: u32 = 20;
        const VERTICAL_PADDING: u32 = 12;
        const MIN_WIDTH: u32 = 100;
        const MIN_HEIGHT: u32 = 44;

        let text_width = (label_string.chars().count() as u32) * char_width;
        let width = (text_width + 2 * HORIZONTAL_PADDING).max(MIN_WIDTH);
        let height = (char_height + 2 * VERTICAL_PADDING).max(MIN_HEIGHT);

        let bounds = Rectangle::new(Point::zero(), Size::new(width, height));

        Self {
            bounds,
            label: label_string,
            action,
            state: ButtonState::Normal,
            variant: ButtonVariant::Primary,
            palette: ColorPalette::default(),
            border_radius: 8,
            dirty: true,
        }
    }

    /// Set the button's visual variant.
    ///
    /// Variants control the button's color scheme (Primary, Secondary, etc.).
    pub fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self.dirty = true;
        self
    }

    /// Set the button's color palette.
    ///
    /// The palette defines the base colors used for rendering.
    pub fn with_palette(mut self, palette: ColorPalette) -> Self {
        self.palette = palette;
        self.dirty = true;
        self
    }

    /// Set the border radius for rounded corners.
    ///
    /// Default is 8 pixels.
    pub fn with_border_radius(mut self, radius: u32) -> Self {
        self.border_radius = radius;
        self.dirty = true;
        self
    }

    /// Update the button's bounds (useful when managed by a layout container)
    pub fn with_bounds(mut self, bounds: Rectangle) -> Self {
        self.bounds = bounds;
        self.dirty = true;
        self
    }

    /// Set the button's bounds (for dynamic repositioning)
    pub fn set_bounds(&mut self, bounds: Rectangle) {
        if self.bounds != bounds {
            self.bounds = bounds;
            self.dirty = true;
        }
    }

    /// Enable or disable the button.
    ///
    /// Disabled buttons don't respond to touch and are rendered with dimmed colors.
    pub fn set_enabled(&mut self, enabled: bool) {
        let new_state = if enabled {
            ButtonState::Normal
        } else {
            ButtonState::Disabled
        };

        if self.state != new_state {
            self.state = new_state;
            self.dirty = true;
        }
    }

    /// Check if the button is enabled.
    pub fn is_enabled(&self) -> bool {
        !matches!(self.state, ButtonState::Disabled)
    }

    /// Get the action that will be triggered when the button is pressed.
    pub fn action(&self) -> Action {
        self.action
    }

    fn get_style(&self) -> Style {
        let base_style = self.variant.to_style(&self.palette);

        match self.state {
            ButtonState::Normal => base_style,
            ButtonState::Pressed => {
                // Darken the background for pressed state
                let bg = base_style.background_color.unwrap_or(self.palette.primary);
                let darkened = Rgb565::new(
                    bg.r().saturating_sub(4),
                    bg.g().saturating_sub(8),
                    bg.b().saturating_sub(4),
                );
                base_style.with_background(darkened)
            }
            ButtonState::Disabled => base_style
                .with_background(self.palette.surface)
                .with_foreground(self.palette.text_secondary),
        }
    }
}

impl Drawable for Button {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        let style = self.get_style();

        // Draw button background with rounded corners
        let corner_radius = Size::new(self.border_radius, self.border_radius);
        RoundedRectangle::with_equal_corners(self.bounds, corner_radius)
            .into_styled(style.to_primitive_style())
            .draw(display)?;

        // Draw button text
        let text_color = style.foreground_color.unwrap_or(Rgb565::WHITE);
        let text_style = MonoTextStyle::new(&FONT_6X10, text_color);
        let center = self.bounds.center();

        Text::with_alignment(&self.label, center, text_style, TextAlignment::Center)
            .draw(display)?;

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

impl Touchable for Button {
    fn contains_point(&self, point: TouchPoint) -> bool {
        let p = point.to_point();
        self.bounds.contains(p)
    }

    fn handle_touch(&mut self, event: TouchEvent) -> TouchResult {
        if !self.is_enabled() {
            return TouchResult::NotHandled;
        }

        match event {
            TouchEvent::Press(point) if self.contains_point(point) => {
                self.state = ButtonState::Pressed;
                self.dirty = true;

                // Trigger action immediately on press
                TouchResult::Action(self.action)
            }
            TouchEvent::Drag(point) => {
                // Update pressed state based on whether drag is still over button
                let new_state = if self.contains_point(point) {
                    ButtonState::Pressed
                } else {
                    ButtonState::Normal
                };

                if self.state != new_state {
                    self.state = new_state;
                    self.dirty = true;
                }
                TouchResult::Handled
            }
            _ => TouchResult::NotHandled,
        }
    }
}
