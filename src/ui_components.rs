// src/ui_components.rs

use super::ui_core::{Action, Clickable, Drawable, TouchEvent, TouchPoint, Touchable};
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle, RoundedRectangle},
    text::{Alignment, Text},
    Drawable as EgDrawable,
};

pub struct Button {
    bounds: Rectangle,
    label: heapless::String<32>,
    action: Action,
    is_pressed: bool,
    dirty: bool,
}

impl Button {
    pub fn new(bounds: Rectangle, label: &str, action: Action) -> Self {
        let mut label_string = heapless::String::new();
        label_string.push_str(label).ok();

        Self {
            bounds,
            label: label_string,
            action,
            is_pressed: false,
            dirty: true,
        }
    }
}

impl Drawable for Button {
    fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        _bounds: Rectangle,
    ) -> Result<(), D::Error> {
        // Button background
        let color = if self.is_pressed {
            Rgb565::CSS_DARK_GRAY
        } else {
            Rgb565::CSS_DODGER_BLUE
        };
        let style = PrimitiveStyleBuilder::new()
            .fill_color(color)
            .stroke_color(Rgb565::WHITE)
            .stroke_width(2)
            .build();

        RoundedRectangle::with_equal_corners(self.bounds, Size::new(8, 8))
            .into_styled(style)
            .draw(display)?;

        // Button text
        let text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);
        let center = self.bounds.center();

        Text::with_alignment(&self.label, center, text_style, Alignment::Center)
            .draw(display)?;

        Ok(())
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
    }
}

impl Touchable for Button {
    fn contains_point(&self, point: TouchPoint) -> bool {
        let p = Point::new(point.x as i32, point.y as i32);
        self.bounds.contains(p)
    }

    fn handle_touch(&mut self, event: TouchEvent) -> bool {
        match event {
            TouchEvent::Press(point) if self.contains_point(point) => {
                self.is_pressed = true;
                self.dirty = true;
                true
            }
            TouchEvent::Release(point) if self.is_pressed => {
                self.is_pressed = false;
                self.dirty = true;
                // Only trigger if release is still over button
                self.contains_point(point)
            }
            _ => {
                if self.is_pressed {
                    self.is_pressed = false;
                    self.dirty = true;
                }
                false
            }
        }
    }
}

impl Clickable for Button {
    fn on_click(&mut self) -> Option<Action> {
        Some(self.action)
    }
}
