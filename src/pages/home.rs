use embedded_graphics::prelude::*;
use embedded_graphics::{
    Drawable as EgDrawable,
    mono_font::{MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::Rgb565,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use heapless::Vec;

use crate::pages::page_manager::Page;
use crate::ui::{
    Action, Button, ButtonVariant, ColorPalette, Drawable, PageId, TouchEvent, TouchResult,
    Touchable,
};

pub struct HomePage {
    bounds: Rectangle,
    buttons: Vec<Button, 4>,
    dirty: bool,
}

impl HomePage {
    pub fn new(bounds: Rectangle) -> Self {
        Self {
            bounds,
            buttons: Vec::new(),
            dirty: true,
        }
    }

    pub fn init(&mut self) {
        let button_height = 50;
        let button_width = 280;
        let y_start = 50;
        let spacing = 10;

        let palette = ColorPalette::default();

        // Settings button
        self.buttons
            .push(
                Button::new(
                    Rectangle::new(
                        Point::new(20, y_start),
                        Size::new(button_width, button_height),
                    ),
                    "Settings",
                    Action::NavigateToPage(PageId::Settings),
                )
                .with_palette(palette)
                .with_variant(ButtonVariant::Primary),
            )
            .ok();

        // Data button
        self.buttons
            .push(
                Button::new(
                    Rectangle::new(
                        Point::new(20, y_start + button_height as i32 + spacing),
                        Size::new(button_width, button_height),
                    ),
                    "View Graphs",
                    Action::NavigateToPage(PageId::Graphs),
                )
                .with_palette(palette)
                .with_variant(ButtonVariant::Secondary),
            )
            .ok();

        self.dirty = true;
    }
}

impl Page for HomePage {
    fn id(&self) -> PageId {
        PageId::Home
    }

    fn title(&self) -> &str {
        "Home"
    }

    fn on_activate(&mut self) {
        self.dirty = true;
    }

    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action> {
        for button in &mut self.buttons {
            match button.handle_touch(event) {
                TouchResult::Action(action) => return Some(action),
                TouchResult::Handled => return None,
                TouchResult::NotHandled => continue,
            }
        }
        None
    }

    fn update(&mut self) {
        // Update page state if needed
    }

    fn draw_page<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        Drawable::draw(self, display)
    }

    fn bounds(&self) -> Rectangle {
        Drawable::bounds(self)
    }

    fn is_dirty(&self) -> bool {
        Drawable::is_dirty(self)
    }

    fn mark_clean(&mut self) {
        Drawable::mark_clean(self)
    }

    fn mark_dirty(&mut self) {
        Drawable::mark_dirty(self)
    }
}

impl Drawable for HomePage {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        // Clear background
        self.bounds
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(display)?;

        // Draw title
        let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
        Text::new("Baro Dashboard", Point::new(60, 20), text_style).draw(display)?;

        // Draw all buttons
        for button in &self.buttons {
            button.draw(display)?;
        }

        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    fn is_dirty(&self) -> bool {
        self.dirty || self.buttons.iter().any(|b| b.is_dirty())
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
        for button in &mut self.buttons {
            button.mark_clean();
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}
