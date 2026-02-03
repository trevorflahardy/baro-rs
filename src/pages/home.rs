use embedded_graphics::prelude::*;
use embedded_graphics::{
    Drawable as EgDrawable,
    pixelcolor::Rgb565,
    primitives::{PrimitiveStyle, Rectangle},
};

use crate::pages::page_manager::Page;
use crate::ui::{
    Action, Alignment, ColorPalette, Container, Direction, Drawable, Element,
    MAX_CONTAINER_CHILDREN, MainAxisAlignment, PageId, SizeConstraint, TextComponent, TextSize,
    TouchEvent, TouchResult, Touchable,
};

extern crate alloc;

/// Home page with nested container architecture.
///
/// Layout structure:
/// - Root container (vertical)
///   - Header container (vertical, centered)
///   - Body container (vertical, stretched buttons)
pub struct HomePage {
    bounds: Rectangle,
    root_container: Container<2>,
    dirty: bool,
}

impl HomePage {
    pub fn new(bounds: Rectangle) -> Self {
        let root_container = Container::new(bounds, Direction::Vertical)
            .with_alignment(Alignment::Stretch)
            .with_gap(10);

        Self {
            bounds,
            root_container,
            dirty: true,
        }
    }

    pub fn init(&mut self) {
        let _palette = ColorPalette::default();

        // Create header container with title
        let mut header = Container::<MAX_CONTAINER_CHILDREN>::new(
            Rectangle::new(Point::zero(), Size::new(self.bounds.size.width, 60)),
            Direction::Vertical,
        )
        .with_alignment(Alignment::Center)
        .with_main_axis_alignment(MainAxisAlignment::Center)
        .with_gap(5);

        let title = TextComponent::auto("Baro Metrics", TextSize::Large)
            .with_alignment(embedded_graphics::text::Alignment::Center);
        header.add_child(title.into(), SizeConstraint::Fit).ok();

        // Create body container with buttons
        let button_height = 50;
        let mut body = Container::<MAX_CONTAINER_CHILDREN>::new(
            Rectangle::new(Point::zero(), Size::new(self.bounds.size.width, 1)),
            Direction::Vertical,
        )
        .with_alignment(Alignment::Stretch)
        .with_gap(10);

        body.add_child(
            Element::button_auto(
                "Temperature Graph",
                Action::NavigateToPage(PageId::TrendTemperature),
            ),
            SizeConstraint::Fixed(button_height),
        )
        .ok();

        body.add_child(
            Element::button_auto(
                "Humidity Graph",
                Action::NavigateToPage(PageId::TrendHumidity),
            ),
            SizeConstraint::Fixed(button_height),
        )
        .ok();

        body.add_child(
            Element::button_auto("CO₂ Graph", Action::NavigateToPage(PageId::TrendCo2)),
            SizeConstraint::Fixed(button_height),
        )
        .ok();

        body.add_child(
            Element::button_auto("Settings", Action::NavigateToPage(PageId::Settings)),
            SizeConstraint::Fixed(button_height),
        )
        .ok();

        // Add containers to root using From trait
        self.root_container
            .add_child(header.into(), SizeConstraint::Fixed(60))
            .ok();

        self.root_container
            .add_child(body.into(), SizeConstraint::Grow(1))
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
        match self.root_container.handle_touch(event) {
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

        // Draw root container and all children.
        self.root_container.draw(display)?;

        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    fn is_dirty(&self) -> bool {
        self.dirty || self.root_container.is_dirty()
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
        self.root_container.mark_clean();
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.root_container.mark_dirty();
    }
}
