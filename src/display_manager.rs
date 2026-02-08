//! Display Manager for handling screen rendering and page management
//!
//! This module provides an async task-based display management system that:
//! - Manages the current active page
//! - Handles page transitions
//! - Renders updates to the display asynchronously
//! - Receives page change requests via channels

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_sync::mutex::Mutex as AsyncMutex;
use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use log::{debug, error, info};

use crate::app_state::AppState;
use crate::pages::page::{Page, PageWrapper};
use crate::pages::{home::HomePage, settings::SettingsPage};
use crate::sensors::SensorType;
use crate::sensors::{
    CO2 as SENSOR_CO2_INDEX, HUMIDITY as SENSOR_HUMIDITY_INDEX,
    TEMPERATURE as SENSOR_TEMPERATURE_INDEX,
};
use crate::storage::accumulator::RollupEvent;
use crate::storage::{RollupTier, TimeWindow};
use crate::ui::{Action, DISPLAY_HEIGHT_PX, DISPLAY_WIDTH_PX, PageEvent, PageId, SensorData, TouchEvent};

extern crate alloc;
use alloc::boxed::Box;

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
            Size::new(DISPLAY_WIDTH_PX as u32, DISPLAY_HEIGHT_PX as u32),
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
    async fn navigate_to<SD, DD, TD>(
        &mut self,
        page_id: PageId,
        app_state: &'static AsyncMutex<CriticalSectionRawMutex, AppState<'static, SD, DD, TD>>,
    ) where
        SD: embedded_hal::spi::SpiDevice<u8>,
        DD: embedded_hal::delay::DelayNs,
        TD: embedded_sdmmc::TimeSource,
    {
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
                debug!(" Creating TrendTemperature page with historical data");
                let mut page = crate::pages::TrendPage::new(
                    self.bounds,
                    SensorType::Temperature,
                    TimeWindow::FiveMinutes,
                );

                // Load historical data directly from storage
                Self::load_trend_data(app_state, &mut page, TimeWindow::FiveMinutes).await;

                self.current_page = PageWrapper::TrendPage(Box::new(page));
            }
            PageId::TrendHumidity => {
                debug!(" Creating TrendHumidity page with historical data");
                let mut page = crate::pages::TrendPage::new(
                    self.bounds,
                    SensorType::Humidity,
                    TimeWindow::OneHour,
                );

                // Load historical data directly from storage
                Self::load_trend_data(app_state, &mut page, TimeWindow::OneHour).await;

                self.current_page = PageWrapper::TrendPage(Box::new(page));
            }
            PageId::TrendCo2 => {
                debug!(" Creating TrendCo2 page with historical data");
                let mut page = crate::pages::TrendPage::new(
                    self.bounds,
                    SensorType::Co2,
                    TimeWindow::ThirtyMinutes,
                );

                // Load historical data directly from storage
                Self::load_trend_data(app_state, &mut page, TimeWindow::ThirtyMinutes).await;

                self.current_page = PageWrapper::TrendPage(Box::new(page));
            }
            PageId::WifiError => {
                let page = crate::pages::WifiErrorPage::new();
                self.current_page = PageWrapper::WifiError(Box::new(page));
            }
        }
        self.needs_redraw = true;
    }

    /// Load historical data for a trend page from storage
    /// This gets the appropriate rollups based on the time window and loads them into the page
    async fn load_trend_data<SD, DD, TD>(
        app_state: &'static AsyncMutex<CriticalSectionRawMutex, AppState<'static, SD, DD, TD>>,
        page: &mut crate::pages::TrendPage,
        window: TimeWindow,
    ) where
        SD: embedded_hal::spi::SpiDevice<u8>,
        DD: embedded_hal::delay::DelayNs,
        TD: embedded_sdmmc::TimeSource,
    {
        // Lock app state and get storage manager
        let state = app_state.lock().await;
        if let Some(storage) = state.storage_manager() {
            let tier = window.preferred_rollup_tier();

            // Get the current time from the latest rollup/sample
            let current_time = match tier {
                RollupTier::RawSample => {
                    let samples: alloc::vec::Vec<_> =
                        storage.get_raw_samples().iter().copied().collect();
                    let time = samples.last().map(|s| s.timestamp).unwrap_or(0);
                    page.load_historical_raw_samples(&samples, time);
                    debug!(
                        "Loaded {} raw samples, latest timestamp: {}",
                        samples.len(),
                        time
                    );
                    time
                }
                RollupTier::FiveMinute => {
                    let rollups: alloc::vec::Vec<_> =
                        storage.get_5m_rollups().iter().copied().collect();
                    let time = rollups.last().map(|r| r.start_ts + 300).unwrap_or(0);
                    page.load_historical_data(&rollups, time);
                    debug!(
                        "Loaded {} 5-minute rollups, latest timestamp: {}",
                        rollups.len(),
                        time
                    );
                    time
                }
                RollupTier::Hourly => {
                    let rollups: alloc::vec::Vec<_> =
                        storage.get_1h_rollups().iter().copied().collect();
                    let time = rollups.last().map(|r| r.start_ts + 3600).unwrap_or(0);
                    page.load_historical_data(&rollups, time);
                    debug!(
                        "Loaded {} hourly rollups, latest timestamp: {}",
                        rollups.len(),
                        time
                    );
                    time
                }
                RollupTier::Daily => {
                    let rollups: alloc::vec::Vec<_> =
                        storage.get_daily_rollups().iter().copied().collect();
                    let time = rollups.last().map(|r| r.start_ts + 86400).unwrap_or(0);
                    page.load_historical_data(&rollups, time);
                    debug!(
                        "Loaded {} daily rollups, latest timestamp: {}",
                        rollups.len(),
                        time
                    );
                    time
                }
            };

            debug!(
                "TrendPage stats after load - Current time: {}",
                current_time
            );
        }
    }

    /// Handle a touch event on the current page
    async fn handle_touch<SD, DD, TD>(
        &mut self,
        event: TouchEvent,
        app_state: &'static AsyncMutex<CriticalSectionRawMutex, AppState<'static, SD, DD, TD>>,
    ) where
        SD: embedded_hal::spi::SpiDevice<u8>,
        DD: embedded_hal::delay::DelayNs,
        TD: embedded_sdmmc::TimeSource,
    {
        debug!(" Received touch event: {:?}", event);
        if let Some(action) = Page::handle_touch(&mut self.current_page, event) {
            debug!(" Touch resulted in action: {:?}", action);
            match action {
                Action::NavigateToPage(page_id) => {
                    self.navigate_to(page_id, app_state).await;
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
            let current_page = &mut self.current_page;
            current_page.draw_page(&mut self.display)?;

            self.needs_redraw = false;
        }
        Ok(())
    }

    /// Process a display request
    async fn process_request<SD, DD, TD>(
        &mut self,
        request: DisplayRequest,
        app_state: &'static AsyncMutex<CriticalSectionRawMutex, AppState<'static, SD, DD, TD>>,
    ) -> Result<(), D::Error>
    where
        SD: embedded_hal::spi::SpiDevice<u8>,
        DD: embedded_hal::delay::DelayNs,
        TD: embedded_sdmmc::TimeSource,
    {
        debug!(" Processing display request: {:?}", request);
        match request {
            DisplayRequest::NavigateToPage(page_id) => {
                debug!(" -> NavigateToPage: {:?}", page_id);
                self.navigate_to(page_id, app_state).await;
            }
            DisplayRequest::Redraw => {
                debug!(" -> Redraw");
                self.needs_redraw = true;
            }
            DisplayRequest::HandleTouch(event) => {
                debug!(" -> HandleTouch: {:?}", event);
                self.handle_touch(event, app_state).await;
            }
            DisplayRequest::UpdateData(event) => {
                debug!(" -> UpdateData: {:?}", event);
                self.update_data(event);
            }
        }

        // Render if needed
        if self.needs_redraw {
            debug!(" Rendering page");
        }
        self.render()
    }

    /// Run the display manager task
    ///
    /// This async function processes display requests from the channel
    /// and updates the display accordingly.
    pub async fn run<SD, DD, TD>(
        &mut self,
        receiver: Receiver<'_, CriticalSectionRawMutex, DisplayRequest, PAGE_CHANGE_CAPACITY>,
        app_state: &'static AsyncMutex<CriticalSectionRawMutex, AppState<'static, SD, DD, TD>>,
    ) where
        SD: embedded_hal::spi::SpiDevice<u8>,
        DD: embedded_hal::delay::DelayNs,
        TD: embedded_sdmmc::TimeSource,
        <D as DrawTarget>::Error: core::fmt::Debug,
    {
        info!(" Display manager task started");

        // Initial render
        if let Err(e) = self.render() {
            error!(" Display render error: {:?}", e);
        }

        loop {
            // Wait for a display request
            debug!(" Display manager: Waiting for request...");
            let request = receiver.receive().await;
            debug!(" Display manager: Received request: {:?}", request);

            // Process the request
            if let Err(e) = self.process_request(request, app_state).await {
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
