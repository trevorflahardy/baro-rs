// src/page_manager.rs

use super::ui_core::{Action, Drawable, PageId, TouchEvent};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use heapless::Vec;

/// Core page trait - extends your existing Page trait
pub trait Page: Drawable {
    fn id(&self) -> PageId;
    fn title(&self) -> &str;

    /// Called when page becomes active
    fn on_activate(&mut self) {}

    /// Called when page becomes inactive
    fn on_deactivate(&mut self) {}

    /// Handle touch events, return action if any
    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action>;

    /// Update page state (called in UI loop)
    fn update(&mut self);
}

/// Page wrapper enum for storing different page types
pub enum PageWrapper {
    Home(crate::pages::home::HomePage),
    // Add other pages here as needed
    // Settings(SettingsPage),
    // Graphs(GraphsPage),
}

impl PageWrapper {
    fn id(&self) -> PageId {
        match self {
            PageWrapper::Home(_) => PageId::Home,
            // Add other pages here
        }
    }

    fn on_activate(&mut self) {
        match self {
            PageWrapper::Home(page) => page.on_activate(),
        }
    }

    fn on_deactivate(&mut self) {
        match self {
            PageWrapper::Home(page) => page.on_deactivate(),
        }
    }

    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action> {
        match self {
            PageWrapper::Home(page) => page.handle_touch(event),
        }
    }

    fn update(&mut self) {
        match self {
            PageWrapper::Home(page) => page.update(),
        }
    }

    fn draw<D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        display: &mut D,
        bounds: Rectangle,
    ) -> Result<(), D::Error> {
        match self {
            PageWrapper::Home(page) => page.draw(display, bounds),
        }
    }

    #[allow(dead_code)]
    fn is_dirty(&self) -> bool {
        match self {
            PageWrapper::Home(page) => page.is_dirty(),
        }
    }
}

/// Manages page navigation and rendering
pub struct PageManager {
    pages: Vec<PageWrapper, 8>,
    current_page: PageId,
    navigation_stack: Vec<PageId, 8>, // History for back navigation
}

impl PageManager {
    pub fn new(initial_page: PageId) -> Self {
        Self {
            pages: Vec::new(),
            current_page: initial_page,
            navigation_stack: Vec::new(),
        }
    }

    pub fn register_page(&mut self, page: PageWrapper) {
        self.pages.push(page).ok();
    }

    pub fn navigate_to(&mut self, page_id: PageId) {
        if let Some(current) = self.get_current_page_mut() {
            current.on_deactivate();
        }

        self.navigation_stack.push(self.current_page).ok();
        self.current_page = page_id;

        if let Some(new_page) = self.get_current_page_mut() {
            new_page.on_activate();
        }
    }

    pub fn go_back(&mut self) -> bool {
        if let Some(prev_page) = self.navigation_stack.pop() {
            if let Some(current) = self.get_current_page_mut() {
                current.on_deactivate();
            }
            self.current_page = prev_page;
            if let Some(page) = self.get_current_page_mut() {
                page.on_activate();
            }
            true
        } else {
            false
        }
    }

    fn get_current_page_mut(&mut self) -> Option<&mut PageWrapper> {
        self.pages.iter_mut().find(|p| p.id() == self.current_page)
    }

    pub fn handle_touch(&mut self, event: TouchEvent) -> Option<Action> {
        if let Some(page) = self.get_current_page_mut() {
            page.handle_touch(event)
        } else {
            None
        }
    }

    pub fn draw<D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &mut self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        if let Some(page) = self.get_current_page_mut() {
            let bounds = Rectangle::new(Point::zero(), Size::new(320, 280));
            page.draw(display, bounds)?;
        }
        Ok(())
    }

    pub fn update(&mut self) {
        if let Some(page) = self.get_current_page_mut() {
            page.update();
        }
    }
}
