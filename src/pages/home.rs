use embedded_graphics::prelude::*;
use embedded_graphics::{
    Drawable as EgDrawable,
    pixelcolor::Rgb565,
    primitives::{PrimitiveStyle, Rectangle},
};
use heapless::Vec;

use crate::pages::page_manager::Page;
use crate::ui::{
    Action, Alignment, Button, ButtonVariant, ColorPalette, Container, Direction, Drawable, PageId,
    SizeConstraint, TextComponent, TextSize, TouchEvent, TouchResult, Touchable,
};

pub struct HomePage {
    bounds: Rectangle,
    container: Container<5>,
    buttons: Vec<Button, 3>,
    title: TextComponent,
    dirty: bool,
}

impl HomePage {
    pub fn new(bounds: Rectangle) -> Self {
        // Create a vertical container with stretch alignment and spacing
        let container = Container::new(bounds, Direction::Vertical)
            .with_alignment(Alignment::Stretch)
            .with_spacing(10);

        // Create title text component
        let title = TextComponent::new(
            Rectangle::new(Point::zero(), Size::new(bounds.size.width, 30)),
            "Baro Dashboard",
            TextSize::Large,
        )
        .with_alignment(embedded_graphics::text::Alignment::Center);

        Self {
            bounds,
            container,
            buttons: Vec::new(),
            title,
            dirty: true,
        }
    }

    pub fn init(&mut self) {
        let palette = ColorPalette::default();

        // Add title with fixed height
        self.container
            .add_child(
                Size::new(self.bounds.size.width, 30),
                SizeConstraint::Fixed(30),
            )
            .ok();

        // Add spacer
        self.container
            .add_child(
                Size::new(self.bounds.size.width, 10),
                SizeConstraint::Fixed(10),
            )
            .ok();

        // Add buttons with expanding size - they will share remaining space equally
        // Temperature Graph button
        let temp_bounds = self
            .container
            .add_child(Size::new(0, 0), SizeConstraint::Expand)
            .ok()
            .and_then(|idx| self.container.child_bounds(idx))
            .unwrap_or(Rectangle::new(Point::zero(), Size::zero()));

        let temp_button = Button::new(
            temp_bounds,
            "Temperature Graph",
            Action::NavigateToPage(PageId::TrendTemperature),
        )
        .with_palette(palette)
        .with_variant(ButtonVariant::Primary);

        self.buttons.push(temp_button).ok();

        // Humidity Graph button
        let humidity_bounds = self
            .container
            .add_child(Size::new(0, 0), SizeConstraint::Expand)
            .ok()
            .and_then(|idx| self.container.child_bounds(idx))
            .unwrap_or(Rectangle::new(Point::zero(), Size::zero()));

        let humidity_button = Button::new(
            humidity_bounds,
            "Humidity Graph",
            Action::NavigateToPage(PageId::TrendHumidity),
        )
        .with_palette(palette)
        .with_variant(ButtonVariant::Secondary);

        self.buttons.push(humidity_button).ok();

        // Settings button
        let settings_bounds = self
            .container
            .add_child(Size::new(0, 0), SizeConstraint::Expand)
            .ok()
            .and_then(|idx| self.container.child_bounds(idx))
            .unwrap_or(Rectangle::new(Point::zero(), Size::zero()));

        let settings_button = Button::new(
            settings_bounds,
            "Settings",
            Action::NavigateToPage(PageId::Settings),
        )
        .with_palette(palette)
        .with_variant(ButtonVariant::Secondary);

        self.buttons.push(settings_button).ok();

        // Update title bounds from container
        if let Some(title_bounds) = self.container.child_bounds(0) {
            self.title.set_bounds(title_bounds);
        }

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
        self.title.draw(display)?;

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
        self.dirty || self.buttons.iter().any(|b| b.is_dirty()) || self.title.is_dirty()
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
        self.title.mark_clean();
        for button in &mut self.buttons {
            button.mark_clean();
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}
