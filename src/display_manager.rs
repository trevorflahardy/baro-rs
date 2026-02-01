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
use log::{debug, error, info};

use crate::pages::page_manager::{Page, PageWrapper};
use crate::pages::{home::HomePage, settings::SettingsPage};
use crate::sensors::SensorType;
use crate::sensors::{
    CO2 as SENSOR_CO2_INDEX, HUMIDITY as SENSOR_HUMIDITY_INDEX,
    TEMPERATURE as SENSOR_TEMPERATURE_INDEX,
};
use crate::storage::TimeWindow;
use crate::storage::accumulator::RollupEvent;
use crate::ui::{Action, PageEvent, PageId, SensorData, TouchEvent};

extern crate alloc;
use alloc::boxed::Box;

// Sensor indices from sensors module
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
    current_page: PageWrapper,
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
            current_page: PageWrapper::Home(Box::new(home_page)),
            bounds,
            needs_redraw: true,
        }
    }

    /// Navigate to a new page
    fn navigate_to(&mut self, page_id: PageId) {
        debug!(" Navigating to page: {:?}", page_id);
        match page_id {
            PageId::Home => {
                let mut page = HomePage::new(self.bounds);
                page.init();
                self.current_page = PageWrapper::Home(Box::new(page));
            }
            PageId::Settings => {
                let mut page = SettingsPage::new(self.bounds);
                page.init();
                self.current_page = PageWrapper::Settings(Box::new(page));
            }
            PageId::Graphs => {
                // TODO: Create graphs page when implemented
                debug!(" Graphs page not yet implemented");
            }
            PageId::TrendPage => {
                // Generic trend page requires parameters
                debug!(" TrendPage requires sensor/window parameters");
            }
            PageId::TrendTemperature => {
                let page = crate::pages::TrendPage::new(
                    self.bounds,
                    SensorType::Temperature,
                    TimeWindow::FiveMinutes,
                );
                self.current_page = PageWrapper::TrendPage(Box::new(page));
            }
            PageId::TrendHumidity => {
                let page = crate::pages::TrendPage::new(
                    self.bounds,
                    SensorType::Humidity,
                    TimeWindow::FiveMinutes,
                );
                self.current_page = PageWrapper::TrendPage(Box::new(page));
            }
            PageId::TrendCo2 => {
                let page = crate::pages::TrendPage::new(
                    self.bounds,
                    SensorType::Co2,
                    TimeWindow::ThirtyMinutes,
                );
                self.current_page = PageWrapper::TrendPage(Box::new(page));
            }
            PageId::WifiError => {
                let page = crate::pages::WifiErrorPage::new();
                self.current_page = PageWrapper::WifiError(Box::new(page));
            }
        }
        self.needs_redraw = true;
    }

    /// Handle a touch event on the current page
    fn handle_touch(&mut self, event: TouchEvent) {
        debug!(" Received touch event: {:?}", event);
        if let Some(action) = Page::handle_touch(&mut self.current_page, event) {
            debug!(" Touch resulted in action: {:?}", action);
            match action {
                Action::NavigateToPage(page_id) => {
                    self.navigate_to(page_id);
                }
                _ => {
                    debug!(" Unhandled action: {:?}", action);
                }
            }
        } else {
            debug!(" Touch event not handled by page");
        }
    }

    /// Update the current page with new data
    fn update_data(&mut self, event: Box<RollupEvent>) {
        debug!(" Received data update: {:?}", event);

        // Dispatch raw RollupEvent to pages that need it (like TrendPage)
        let rollup_page_event = PageEvent::RollupEvent(event.clone());
        let needs_redraw_rollup = Page::on_event(&mut self.current_page, &rollup_page_event);

        // Convert RollupEvent to PageEvent and dispatch to current page
        match *event {
            RollupEvent::RawSample(sample) => {
                // Extract sensor values from the raw sample (in milli-units)
                let temperature_mc = sample.values[SENSOR_TEMPERATURE_INDEX];
                let humidity_mp = sample.values[SENSOR_HUMIDITY_INDEX];
                let co2_mp = sample.values[SENSOR_CO2_INDEX];

                // Convert to float values (divide by 1000)
                let temp_c = temperature_mc as f32 / 1000.0;
                let humidity_pct = humidity_mp as f32 / 1000.0;
                let co2_ppm = co2_mp as f32 / 1000.0;

                debug!("{}", sample);

                let sensor_data = SensorData {
                    temperature: Some(temp_c),
                    humidity: Some(humidity_pct),
                    co2: Some(co2_ppm),
                    timestamp: sample.timestamp as u64,
                };

                let page_event = PageEvent::SensorUpdate(sensor_data);
                let needs_redraw = Page::on_event(&mut self.current_page, &page_event);

                if needs_redraw || needs_redraw_rollup {
                    debug!(" Page marked for redraw after sensor update");
                    self.needs_redraw = true;
                }
            }
            RollupEvent::Rollup5m(rollup)
            | RollupEvent::Rollup1h(rollup)
            | RollupEvent::RollupDaily(rollup) => {
                // For rollups, use the average values
                let temperature_mc = rollup.avg[SENSOR_TEMPERATURE_INDEX];
                let humidity_mp = rollup.avg[SENSOR_HUMIDITY_INDEX];
                let co2_mp = rollup.avg[SENSOR_CO2_INDEX];

                let temp_c = temperature_mc as f32 / 1000.0;
                let humidity_pct = humidity_mp as f32 / 1000.0;
                let co2_ppm = co2_mp as f32 / 1000.0;

                debug!("{}", rollup);

                let sensor_data = SensorData {
                    temperature: Some(temp_c),
                    humidity: Some(humidity_pct),
                    co2: Some(co2_ppm),
                    timestamp: rollup.start_ts as u64,
                };

                let page_event = PageEvent::SensorUpdate(sensor_data);
                let needs_redraw = Page::on_event(&mut self.current_page, &page_event);

                if needs_redraw || needs_redraw_rollup {
                    debug!(" Page marked for redraw after rollup update");
                    self.needs_redraw = true;
                }
            }
        }
    }

    /// Render the current page if needed
    fn render(&mut self) -> Result<(), D::Error> {
        if self.needs_redraw {
            debug!(" Rendering page");
            // Clear the display
            self.bounds
                .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                .draw(&mut self.display)?;

            // Draw the current page
            let current_page = &self.current_page;
            current_page.draw_page(&mut self.display)?;

            self.needs_redraw = false;
        }
        Ok(())
    }

    /// Process a display request
    fn process_request(&mut self, request: DisplayRequest) -> Result<(), D::Error> {
        debug!(" Processing request: {:?}", request);
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
        info!(" Display manager task started");

        // Initial render
        if let Err(e) = self.render() {
            error!(" Display render error: {:?}", e);
        }

        loop {
            // Wait for a display request
            let request = receiver.receive().await;

            // Process the request
            if let Err(e) = self.process_request(request) {
                error!(" Error processing request: {:?}", e);
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
