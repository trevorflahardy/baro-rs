// src/pages/page_manager.rs
//! Page manager with navigation and event dispatching.

use crate::pages::page::{Page, PageWrapper};
use crate::ui::core::{Action, PageEvent, PageId, TouchEvent};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use heapless::Vec;
use log::debug;

/// Manages page navigation, rendering, and event dispatching.
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
        debug!(" Processing touch event: {:?}", event);
        if let Some(page) = self.get_current_page_mut() {
            let result = page.handle_touch(event);
            debug!(" Touch result: {:?}", result);
            result
        } else {
            debug!(" No current page to handle touch");
            None
        }
    }

    /// Dispatch event to current page
    /// Returns true if page needs redraw
    pub fn dispatch_event(&mut self, event: &PageEvent) -> bool {
        debug!(
            " Dispatching event to page {:?}: {:?}",
            self.current_page, event
        );
        if let Some(page) = self.get_current_page_mut() {
            let handled = page.on_event(event);
            debug!(" Event handled: {}, needs_redraw: {}", handled, handled);
            handled
        } else {
            debug!(" No current page to dispatch event to");
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
