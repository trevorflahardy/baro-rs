// src/pages/page_manager.rs
//! Page manager with navigation and event dispatching

use crate::ui::core::{Action, DirtyRegion, Drawable, PageEvent, PageId, TouchEvent};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use heapless::Vec;

extern crate alloc;
use alloc::boxed::Box;

/// Trait for pages that can be rendered and interacted with
pub trait Page {
    /// Get the unique identifier for this page
    fn id(&self) -> PageId;

    /// Get the title of this page
    fn title(&self) -> &str;

    /// Called when page becomes active
    fn on_activate(&mut self) {}

    /// Called when page becomes inactive
    fn on_deactivate(&mut self) {}

    /// Handle touch events, return action if any
    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action>;

    /// Update page state (called in UI loop)
    fn update(&mut self);

    /// Handle page events (sensor updates, storage events, etc.)
    /// Return true if the event was handled and page needs redraw
    fn on_event(&mut self, _event: &PageEvent) -> bool {
        false // Default: don't handle events
    }

    /// Draw the page to display
    fn draw_page<D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        display: &mut D,
    ) -> Result<(), D::Error>;

    /// Get the page bounds
    fn bounds(&self) -> Rectangle;

    /// Check if the page is dirty
    fn is_dirty(&self) -> bool;

    /// Mark the page as clean
    fn mark_clean(&mut self);

    /// Mark the page as dirty
    fn mark_dirty(&mut self);

    /// Get dirty regions for partial updates
    fn dirty_regions(&self) -> Vec<DirtyRegion, 8> {
        if self.is_dirty() {
            let mut regions = Vec::new();
            regions.push(DirtyRegion::new(self.bounds())).ok();
            regions
        } else {
            Vec::new()
        }
    }
}

impl<T: Page> Page for Box<T> {
    fn id(&self) -> PageId {
        (**self).id()
    }

    fn title(&self) -> &str {
        (**self).title()
    }

    fn on_activate(&mut self) {
        (**self).on_activate()
    }

    fn on_deactivate(&mut self) {
        (**self).on_deactivate()
    }

    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action> {
        (**self).handle_touch(event)
    }

    fn update(&mut self) {
        (**self).update()
    }

    fn on_event(&mut self, event: &PageEvent) -> bool {
        (**self).on_event(event)
    }

    fn draw_page<D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        (**self).draw_page(display)
    }

    fn bounds(&self) -> Rectangle {
        (**self).bounds()
    }

    fn is_dirty(&self) -> bool {
        (**self).is_dirty()
    }

    fn mark_clean(&mut self) {
        (**self).mark_clean()
    }

    fn mark_dirty(&mut self) {
        (**self).mark_dirty()
    }
}

/// Page wrapper enum for storing different page types
pub enum PageWrapper {
    Home(Box<crate::pages::home::HomePage>),
    Settings(Box<crate::pages::settings::SettingsPage>),
}

impl Page for PageWrapper {
    fn id(&self) -> PageId {
        match self {
            PageWrapper::Home(page) => page.id(),
            PageWrapper::Settings(page) => page.id(),
        }
    }

    fn title(&self) -> &str {
        match self {
            PageWrapper::Home(page) => page.title(),
            PageWrapper::Settings(page) => page.title(),
        }
    }

    fn on_activate(&mut self) {
        match self {
            PageWrapper::Home(page) => page.on_activate(),
            PageWrapper::Settings(page) => page.on_activate(),
        }
    }

    fn on_deactivate(&mut self) {
        match self {
            PageWrapper::Home(page) => page.on_deactivate(),
            PageWrapper::Settings(page) => page.on_deactivate(),
        }
    }

    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action> {
        match self {
            PageWrapper::Home(page) => page.handle_touch(event),
            PageWrapper::Settings(page) => page.handle_touch(event),
        }
    }

    fn update(&mut self) {
        match self {
            PageWrapper::Home(page) => page.update(),
            PageWrapper::Settings(page) => page.update(),
        }
    }

    fn on_event(&mut self, event: &PageEvent) -> bool {
        match self {
            PageWrapper::Home(page) => page.on_event(event),
            PageWrapper::Settings(page) => page.on_event(event),
        }
    }

    fn draw_page<D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        match self {
            PageWrapper::Home(page) => page.draw(display),
            PageWrapper::Settings(page) => page.draw(display),
        }
    }

    fn bounds(&self) -> Rectangle {
        match self {
            PageWrapper::Home(page) => Page::bounds(page),
            PageWrapper::Settings(page) => Page::bounds(page),
        }
    }

    fn is_dirty(&self) -> bool {
        match self {
            PageWrapper::Home(page) => Page::is_dirty(page),
            PageWrapper::Settings(page) => Page::is_dirty(page),
        }
    }

    fn mark_clean(&mut self) {
        match self {
            PageWrapper::Home(page) => Page::mark_clean(page),
            PageWrapper::Settings(page) => Page::mark_clean(page),
        }
    }

    fn mark_dirty(&mut self) {
        match self {
            PageWrapper::Home(page) => Page::mark_dirty(page),
            PageWrapper::Settings(page) => Page::mark_dirty(page),
        }
    }
}

/// Manages page navigation, rendering, and event dispatching
pub struct PageManager {
    pages: Vec<PageWrapper, 8>,
    current_page: PageId,
    navigation_stack: Vec<PageId, 8>,
    display_bounds: Rectangle,
}

impl PageManager {
    pub fn new(initial_page: PageId, display_bounds: Rectangle) -> Self {
        Self {
            pages: Vec::new(),
            current_page: initial_page,
            navigation_stack: Vec::new(),
            display_bounds,
        }
    }

    /// Register a new page
    pub fn register_page(&mut self, page: PageWrapper) {
        self.pages.push(page).ok();
    }

    /// Navigate to a specific page
    pub fn navigate_to(&mut self, page_id: PageId) {
        if let Some(current) = self.get_current_page_mut() {
            current.on_deactivate();
        }

        // Push current page to stack for back navigation
        self.navigation_stack.push(self.current_page).ok();
        self.current_page = page_id;

        if let Some(new_page) = self.get_current_page_mut() {
            new_page.on_activate();
        }
    }

    /// Go back to previous page
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

    /// Get mutable reference to current page
    fn get_current_page_mut(&mut self) -> Option<&mut PageWrapper> {
        self.pages.iter_mut().find(|p| p.id() == self.current_page)
    }

    /// Get reference to current page
    fn get_current_page(&self) -> Option<&PageWrapper> {
        self.pages.iter().find(|p| p.id() == self.current_page)
    }

    /// Handle touch events, returns action if any
    pub fn handle_touch(&mut self, event: TouchEvent) -> Option<Action> {
        if let Some(page) = self.get_current_page_mut() {
            page.handle_touch(event)
        } else {
            None
        }
    }

    /// Dispatch event to current page
    /// Returns true if page needs redraw
    pub fn dispatch_event(&mut self, event: &PageEvent) -> bool {
        if let Some(page) = self.get_current_page_mut() {
            page.on_event(event)
        } else {
            false
        }
    }

    /// Update current page state
    pub fn update(&mut self) {
        if let Some(page) = self.get_current_page_mut() {
            page.update();
        }
    }

    /// Draw the current page (full redraw)
    pub fn draw<D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &mut self,
        display: &mut D,
    ) -> Result<(), D::Error> {
        if let Some(page) = self.get_current_page_mut() {
            page.draw_page(display)?;
            page.mark_clean();
        }
        Ok(())
    }

    /// Draw only dirty regions for partial updates
    pub fn draw_dirty<D: DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>>(
        &mut self,
        display: &mut D,
    ) -> Result<bool, D::Error> {
        if let Some(page) = self.get_current_page_mut() {
            if page.is_dirty() {
                // For now, do a full redraw
                // In a more advanced implementation, we would:
                // 1. Get dirty regions from page
                // 2. Create a cropped DrawTarget for each region
                // 3. Draw only affected elements
                page.draw_page(display)?;
                page.mark_clean();
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    /// Check if current page is dirty
    pub fn is_dirty(&self) -> bool {
        if let Some(page) = self.get_current_page() {
            page.is_dirty()
        } else {
            false
        }
    }

    /// Get current page ID
    pub fn current_page_id(&self) -> PageId {
        self.current_page
    }

    /// Get display bounds
    pub fn display_bounds(&self) -> Rectangle {
        self.display_bounds
    }
}
