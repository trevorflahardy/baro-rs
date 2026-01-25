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
    Action, Alignment, Button, ButtonVariant, ColorPalette, Container, Direction, Drawable, PageId,
    SizeConstraint, TouchEvent, TouchResult, Touchable,
};

const BUTTON_HEIGHT: u32 = 50;
const BUTTON_SPACING: u32 = 10;
const TOP_MARGIN: u32 = 50;
const SIDE_MARGIN: u32 = 20;

pub struct HomePage {
    bounds: Rectangle,
    container: Container<4>,
    buttons: Vec<Button, 4>,
    dirty: bool,
}

impl HomePage {
    pub fn new(bounds: Rectangle) -> Self {
        // Create container for button layout
        // Positioned below title with margins
        let container_bounds = Rectangle::new(
            Point::new(SIDE_MARGIN as i32, TOP_MARGIN as i32),
            Size::new(
                bounds.size.width.saturating_sub(SIDE_MARGIN * 2),
                bounds.size.height.saturating_sub(TOP_MARGIN),
            ),
        );

        let container = Container::<4>::new(container_bounds, Direction::Vertical)
            .with_alignment(Alignment::Start)
            .with_spacing(BUTTON_SPACING);

        Self {
            bounds,
            container,
            buttons: Vec::new(),
            dirty: true,
        }
    }

    pub fn init(&mut self) {
        let palette = ColorPalette::default();
        let button_width = self.bounds.size.width.saturating_sub(SIDE_MARGIN * 2);

        // Settings button
        let settings_button = Button::new(
            Rectangle::new(Point::zero(), Size::new(button_width, BUTTON_HEIGHT)),
            "Settings",
            Action::NavigateToPage(PageId::Settings),
        )
        .with_palette(palette)
        .with_variant(ButtonVariant::Primary);

        // Data button
        let data_button = Button::new(
            Rectangle::new(Point::zero(), Size::new(button_width, BUTTON_HEIGHT)),
            "View Graphs",
            Action::NavigateToPage(PageId::Graphs),
        )
        .with_palette(palette)
        .with_variant(ButtonVariant::Secondary);

        // Add buttons to container with fixed height constraint
        self.container
            .add_child(
                Size::new(button_width, BUTTON_HEIGHT),
                SizeConstraint::Fixed(BUTTON_HEIGHT),
            )
            .ok();

        self.container
            .add_child(
                Size::new(button_width, BUTTON_HEIGHT),
                SizeConstraint::Fixed(BUTTON_HEIGHT),
            )
            .ok();

        // Store buttons with their positions from container
        if let Some(bounds) = self.container.child_bounds(0) {
            self.buttons.push(settings_button.with_bounds(bounds)).ok();
        }

        if let Some(bounds) = self.container.child_bounds(1) {
            self.buttons.push(data_button.with_bounds(bounds)).ok();
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
