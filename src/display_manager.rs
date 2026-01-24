//! Display Manager for handling screen rendering and page management
//!
//! This module provides an async task-based display management system that:
//! - Manages the current active page
//! - Handles page transitions
//! - Renders updates to the display asynchronously
//! - Receives page change requests via channels

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};

use crate::pages::home::HomePage;
use crate::pages::page_manager::Page;
use crate::storage::accumulator::RollupEvent;
use crate::ui::{Action, Drawable, PageId, TouchEvent};

extern crate alloc;
use alloc::boxed::Box;

const DISPLAY_WIDTH: u16 = 320;
const DISPLAY_HEIGHT: u16 = 240;

/// Channel capacity for page change requests
const PAGE_CHANGE_CAPACITY: usize = 4;

/// Request to change the current page or update the display
#[derive(Debug, Clone)]
pub enum DisplayRequest {
    /// Navigate to a specific page
    NavigateToPage(PageId),
    /// Force a redraw of the current page
    Redraw,
    /// Handle a touch event on the current page
    HandleTouch(TouchEvent),
    /// Update the display with new rollup data
    UpdateData(Box<RollupEvent>),
}

/// Global channel for display requests
pub static DISPLAY_CHANNEL: Channel<CriticalSectionRawMutex, DisplayRequest, PAGE_CHANGE_CAPACITY> =
    Channel::new();

/// Display manager that owns the display and manages page rendering
pub struct DisplayManager<D>
where
    D: DrawTarget<Color = Rgb565>,
{
    display: D,
    current_page: HomePage,
    bounds: Rectangle,
    needs_redraw: bool,
}

impl<D> DisplayManager<D>
where
    D: DrawTarget<Color = Rgb565>,
{
    /// Create a new display manager with the given display
    pub fn new(display: D) -> Self {
        let bounds = Rectangle::new(
            Point::zero(),
            Size::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32),
        );

        // Start with the home page
        let mut home_page = HomePage::new(bounds);
        home_page.init();

        Self {
            display,
            current_page: home_page,
            bounds,
            needs_redraw: true,
        }
    }

    /// Navigate to a new page
    fn navigate_to(&mut self, page_id: PageId) {
        match page_id {
            PageId::Home => {
                let mut page = HomePage::new(self.bounds);
                page.init();
                self.current_page = page;
            }
            PageId::Settings => {
                // TODO: Create settings page when implemented
                rtt_target::rprintln!("Settings page not yet implemented");
            }
            PageId::Graphs => {
                // TODO: Create graphs page when implemented
                rtt_target::rprintln!("Graphs page not yet implemented");
            }
        }
        self.needs_redraw = true;
    }

    /// Handle a touch event on the current page
    fn handle_touch(&mut self, event: TouchEvent) {
        if let Some(action) = Page::handle_touch(&mut self.current_page, event) {
            match action {
                Action::NavigateToPage(page_id) => {
                    self.navigate_to(page_id);
                }
                _ => {
                    // Other actions can be handled here as needed
                }
            }
        }
    }

    /// Update the current page with new data
    fn update_data(&mut self, _event: Box<RollupEvent>) {
        // For now, just mark as needing redraw
        // Future: pass event to page for data updates
        self.needs_redraw = true;
    }

    /// Render the current page if needed
    fn render(&mut self) -> Result<(), D::Error> {
        if self.needs_redraw {
            // Clear the display
            self.bounds
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(&mut self.display)?;

            // Draw the current page
            Drawable::draw(&self.current_page, &mut self.display)?;

            self.needs_redraw = false;
        }
        Ok(())
    }

    /// Process a display request
    fn process_request(&mut self, request: DisplayRequest) -> Result<(), D::Error> {
        match request {
            DisplayRequest::NavigateToPage(page_id) => {
                self.navigate_to(page_id);
            }
            DisplayRequest::Redraw => {
                self.needs_redraw = true;
            }
            DisplayRequest::HandleTouch(event) => {
                self.handle_touch(event);
            }
            DisplayRequest::UpdateData(event) => {
                self.update_data(event);
            }
        }

        // Render if needed
        self.render()
    }

    /// Run the display manager task
    ///
    /// This async function processes display requests from the channel
    /// and updates the display accordingly.
    pub async fn run(
        &mut self,
        receiver: Receiver<'_, CriticalSectionRawMutex, DisplayRequest, PAGE_CHANGE_CAPACITY>,
    ) where
        <D as DrawTarget>::Error: core::fmt::Debug,
    {
        rtt_target::rprintln!("Display manager task started");

        // Initial render
        if let Err(e) = self.render() {
            rtt_target::rprintln!("Display render error: {:?}", e);
        }

        loop {
            // Wait for a display request
            let request = receiver.receive().await;

            // Process the request
            if let Err(e) = self.process_request(request) {
                rtt_target::rprintln!("Display render error: {:?}", e);
            }
        }
    }
}

/// Helper to get a display request sender
pub fn get_display_sender()
-> Sender<'static, CriticalSectionRawMutex, DisplayRequest, PAGE_CHANGE_CAPACITY> {
    DISPLAY_CHANNEL.sender()
}

/// Helper to get a display request receiver
pub fn get_display_receiver()
-> Receiver<'static, CriticalSectionRawMutex, DisplayRequest, PAGE_CHANGE_CAPACITY> {
    DISPLAY_CHANNEL.receiver()
}
