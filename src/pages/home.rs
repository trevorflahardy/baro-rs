use embedded_graphics::prelude::*;
use embedded_graphics::{
    Drawable as EgDrawable,
    pixelcolor::Rgb565,
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::pages::page_manager::Page;
use crate::ui::{
    Action, Alignment, Button, ButtonVariant, ColorPalette, Container, Direction, Drawable,
    Element, PageId, SizeConstraint, TextComponent, TextSize, TouchEvent, TouchResult, Touchable,
};

extern crate alloc;
use alloc::boxed::Box;

pub struct HomePage {
    bounds: Rectangle,
    container: Container<5>,
    dirty: bool,
}

impl HomePage {
    pub fn new(bounds: Rectangle) -> Self {
        // Vertical layout: title, spacer, then 3 buttons that grow.
        let container = Container::new(bounds, Direction::Vertical)
            .with_alignment(Alignment::Stretch)
            .with_gap(10);

        Self {
            bounds,
            container,
            dirty: true,
        }
    }

    pub fn init(&mut self) {
        let palette = ColorPalette::default();

        // NOTE: child bounds are overwritten by layout; these are just initial hints.
        let hint = Rectangle::new(Point::zero(), Size::new(self.bounds.size.width, 1));

        // Title.
        let title = TextComponent::new(hint, "Hello User!", TextSize::Large)
            .with_alignment(embedded_graphics::text::Alignment::Center);

        self.container
            .add_child(Element::Text(Box::new(title)), SizeConstraint::Fixed(30))
            .ok();

        // Spacer
        self.container
            .add_child(Element::spacer(hint), SizeConstraint::Fixed(10))
            .ok();

        // Buttons: share remaining space.
        let temp_btn = Button::new(
            hint,
            "Temperature Graph",
            Action::NavigateToPage(PageId::TrendTemperature),
        )
        .with_palette(palette)
        .with_variant(ButtonVariant::Primary);

        let humidity_btn = Button::new(
            hint,
            "Humidity Graph",
            Action::NavigateToPage(PageId::TrendHumidity),
        )
        .with_palette(palette)
        .with_variant(ButtonVariant::Secondary);

        let co2_btn = Button::new(hint, "CO2 Graph", Action::NavigateToPage(PageId::TrendCo2));

        let settings_btn = Button::new(hint, "Settings", Action::NavigateToPage(PageId::Settings))
            .with_palette(palette)
            .with_variant(ButtonVariant::Secondary);

        self.container
            .add_child(Element::Button(Box::new(temp_btn)), SizeConstraint::Grow(1))
            .ok();
        self.container
            .add_child(
                Element::Button(Box::new(humidity_btn)),
                SizeConstraint::Grow(1),
            )
            .ok();
        self.container
            .add_child(Element::Button(Box::new(co2_btn)), SizeConstraint::Grow(1))
            .ok();
        self.container
            .add_child(
                Element::Button(Box::new(settings_btn)),
                SizeConstraint::Grow(1),
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
        match self.container.handle_touch(event) {
            TouchResult::Action(a) => Some(a),
            TouchResult::Handled | TouchResult::NotHandled => None,
        }
    }

    fn update(&mut self) {}

    fn draw_page<D: DrawTarget<Color = Rgb565>>(
        &mut self,
        display: &mut D,
    ) -> Result<(), D::Error> {
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
        // Clear background.
        self.bounds
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(display)?;

        // Draw container + children.
        self.container.draw(display)?;

        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    fn is_dirty(&self) -> bool {
        self.dirty || self.container.is_dirty()
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
        self.container.mark_clean();
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.container.mark_dirty();
    }
}
