use embedded_graphics::prelude::*;
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::Rgb565,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
    Drawable as EgDrawable,
};
use heapless::Vec;

use crate::{
    page_manager::Page,
    ui_components::Button,
    ui_core::{Action, Clickable, Drawable, PageId, Touchable, TouchEvent},
};

pub struct HomePage {
    buttons: Vec<Button, 6>,
    _dirty: bool,
}

impl HomePage {
    pub fn new() -> Self {
        Self {
            buttons: Vec::new(),
            _dirty: true,
        }
    }

    pub fn init(&mut self) {
        // Use embedded-layout to position buttons
        let button_height = 50;
        let button_width = 280;

        let y_start = 40;

        // Create buttons with proper spacing
        self.buttons
            .push(Button::new(
                Rectangle::new(
                    Point::new(20, y_start as i32),
                    Size::new(button_width, button_height),
                ),
                "Settings",
                Action::NavigateToPage(PageId::Settings),
            ))
            .ok();
    }
}

impl Page for HomePage {
    fn id(&self) -> PageId {
        PageId::Home
    }

    fn title(&self) -> &str {
        "Home"
    }

    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action> {
        for button in &mut self.buttons {
            if button.handle_touch(event) {
                if matches!(event, TouchEvent::Release(_)) {
                    return button.on_click();
                }
            }
        }
        None
    }

    fn update(&mut self) {
        // Update page state if needed
    }
}

impl Drawable for HomePage {
    fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        bounds: Rectangle,
    ) -> Result<(), D::Error> {
        // Clear background
        bounds
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(display)?;

        // Draw title
        let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
        Text::new("Baro Dashboard", Point::new(60, 20), text_style)
            .draw(display)?;

        // Draw all buttons
        for button in &self.buttons {
            button.draw(display, bounds)?;
        }

        Ok(())
    }

    fn is_dirty(&self) -> bool {
        self._dirty || self.buttons.iter().any(|b| b.is_dirty())
    }

    fn mark_clean(&mut self) {
        self._dirty = false;
        for button in &mut self.buttons {
            button.mark_clean();
        }
    }
}
