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
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use log::{debug, error, info};

use crate::app_state::AppState;
use crate::config::{HomePageMode, TemperatureUnit};
use crate::framebuffer::FrameBuffer;
use crate::metrics::QualityLevel;
use crate::pages::home::grid::HomeGridPage;
use crate::pages::home::outdoor::HomePage;
use crate::pages::monitor::MonitorPage;
use crate::pages::page::{Page, PageWrapper};
use crate::pages::settings::DisplaySettingsPage;
use crate::pages::settings::SettingsPage;
use crate::pages::wifi_status::{WifiState, WifiStatusPage};
use crate::sensor_store::SensorDataStore;
use crate::sensors::SensorType;
use crate::sensors::{
    CO2 as SENSOR_CO2_INDEX, HUMIDITY as SENSOR_HUMIDITY_INDEX, LUX as SENSOR_LUX_INDEX,
    TEMPERATURE as SENSOR_TEMPERATURE_INDEX,
};
use crate::storage::accumulator::RollupEvent;
use crate::storage::{RollupTier, TimeWindow};
use crate::ui::{
    Action, DISPLAY_HEIGHT_PX, DISPLAY_WIDTH_PX, PageEvent, PageId, SensorData, TouchEvent,
};

extern crate alloc;
use alloc::boxed::Box;

/// Channel capacity for page change requests
const PAGE_CHANGE_CAPACITY: usize = 4;

/// Auto-cycle interval in seconds (Home grid mode only)
const AUTO_CYCLE_INTERVAL_SECS: u64 = 15;

/// Sensors to cycle through in auto-cycle mode
const AUTO_CYCLE_PAGES: [PageId; 4] = [
    PageId::TrendTemperature,
    PageId::TrendHumidity,
    PageId::TrendCo2,
    PageId::TrendLux,
];

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
    framebuffer: FrameBuffer,
    current_page: PageWrapper,
    bounds: Rectangle,
    needs_redraw: bool,
    /// Current home page mode (loaded from device config)
    home_page_mode: HomePageMode,
    /// Current temperature display unit (loaded from device config)
    temperature_unit: TemperatureUnit,
    /// Whether auto-cycling is currently active (Home grid mode)
    auto_cycle_enabled: bool,
    /// Timestamp of the last auto-cycle page switch
    auto_cycle_last_switch: u64,
    /// Index into AUTO_CYCLE_PAGES for the next page to show
    auto_cycle_index: usize,
    /// Last known sensor quality (true = all sensors Good/Excellent)
    all_sensors_healthy: bool,
    /// Last known timestamp from sensor data
    last_sensor_timestamp: u64,
    /// Centralized sensor data store — survives page navigation
    sensor_store: SensorDataStore,
    /// Touch debounce: skip the next Press event when true.
    ///
    /// Set after a touch that caused a page state change (dirty transition)
    /// to prevent a single physical press from triggering two logical actions
    /// (e.g. dismiss alert → tap underlying element).
    skip_next_press: bool,
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

        // Start on the WiFi connecting page — the firmware will navigate
        // to Home once WiFi is up, or to WifiStatus(Error) on failure.
        let wifi_page = WifiStatusPage::new(WifiState::Connecting);

        Self {
            display,
            framebuffer: FrameBuffer::new(),
            current_page: PageWrapper::WifiStatus(Box::new(wifi_page)),
            bounds,
            needs_redraw: true,
            home_page_mode: HomePageMode::default(),
            temperature_unit: TemperatureUnit::default(),
            auto_cycle_enabled: false,
            auto_cycle_last_switch: 0,
            auto_cycle_index: 0,
            all_sensors_healthy: true,
            last_sensor_timestamp: 0,
            sensor_store: SensorDataStore::new(),
            skip_next_press: false,
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
                // Navigate to the correct home page based on current mode
                match self.home_page_mode {
                    HomePageMode::Outdoor => {
                        let mut page = HomePage::new(self.bounds);
                        page.init();
                        page.load_from_store(&self.sensor_store);
                        self.current_page = PageWrapper::Home(Box::new(page));
                        self.auto_cycle_enabled = false;
                    }
                    HomePageMode::Home => {
                        let mut page = HomeGridPage::new(self.bounds);
                        page.load_from_store(&self.sensor_store);
                        self.current_page = PageWrapper::HomeGrid(Box::new(page));
                        self.auto_cycle_enabled = true;
                        self.auto_cycle_last_switch = self.last_sensor_timestamp;
                        self.auto_cycle_index = 0;
                    }
                }
            }
            PageId::HomeGrid => {
                let mut page = HomeGridPage::new(self.bounds);
                page.load_from_store(&self.sensor_store);
                self.current_page = PageWrapper::HomeGrid(Box::new(page));
                self.auto_cycle_enabled = true;
                self.auto_cycle_last_switch = self.last_sensor_timestamp;
                self.auto_cycle_index = 0;
            }
            PageId::Settings => {
                let mut page = SettingsPage::new(self.bounds);
                page.init();
                self.current_page = PageWrapper::Settings(Box::new(page));
                self.auto_cycle_enabled = false;
            }
            PageId::DisplaySettings => {
                let page = DisplaySettingsPage::new(
                    self.bounds,
                    self.home_page_mode,
                    self.temperature_unit,
                );
                self.current_page = PageWrapper::DisplaySettings(Box::new(page));
                self.auto_cycle_enabled = false;
            }
            PageId::Monitor => {
                let mut page = MonitorPage::new(self.bounds);
                page.init();
                page.load_from_store(&self.sensor_store);
                self.current_page = PageWrapper::Monitor(Box::new(page));
                self.auto_cycle_enabled = false;
            }
            PageId::Graphs => {
                debug!(" Graphs page not yet implemented");
            }
            PageId::TrendPage => {
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
            PageId::TrendLux => {
                debug!(" Creating TrendLux page with historical data");
                let mut page = crate::pages::TrendPage::new(
                    self.bounds,
                    SensorType::Lux,
                    TimeWindow::ThirtyMinutes,
                );

                Self::load_trend_data(app_state, &mut page, TimeWindow::ThirtyMinutes).await;

                self.current_page = PageWrapper::TrendPage(Box::new(page));
            }
            PageId::WifiStatus => {
                let page = WifiStatusPage::new(WifiState::Error);
                self.current_page = PageWrapper::WifiStatus(Box::new(page));
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

        // Touch debounce: skip this Press if the previous touch caused a
        // page state change (prevents dismiss-then-tap-through on alerts).
        if matches!(event, TouchEvent::Press(_)) && self.skip_next_press {
            debug!(" Skipping press (debounce)");
            self.skip_next_press = false;
            return;
        }

        // Any manual touch interaction disables auto-cycle
        // (it will re-enable when navigating back to HomeGrid)
        if self.auto_cycle_enabled {
            self.auto_cycle_enabled = false;
        }

        // Snapshot dirty state before touch so we can detect state changes
        let was_dirty = Page::is_dirty(&self.current_page);

        if let Some(action) = Page::handle_touch(&mut self.current_page, event) {
            debug!(" Touch resulted in action: {:?}", action);
            match action {
                Action::NavigateToPage(page_id) => {
                    self.navigate_to(page_id, app_state).await;
                }
                Action::GoBack => {
                    // Context-aware back navigation
                    let current_id = Page::id(&self.current_page);
                    match current_id {
                        // Sub-settings pages go back to Settings
                        PageId::DisplaySettings | PageId::Monitor => {
                            self.navigate_to(PageId::Settings, app_state).await;
                        }
                        // Trend pages go back to Home
                        PageId::TrendTemperature
                        | PageId::TrendHumidity
                        | PageId::TrendCo2
                        | PageId::TrendLux
                        | PageId::TrendPage => {
                            self.navigate_to(PageId::Home, app_state).await;
                        }
                        // Default: go to Home
                        _ => {
                            self.navigate_to(PageId::Home, app_state).await;
                        }
                    }
                }
                Action::UpdateHomePageMode(mode) => {
                    info!(" Updating home page mode to {:?}", mode);
                    self.home_page_mode = mode;

                    // Update device config in app state
                    {
                        let mut state = app_state.lock().await;
                        state.device_config.home_page_mode = mode;
                    }

                    // Navigate to the correct home page
                    self.navigate_to(PageId::Home, app_state).await;
                }
                Action::UpdateTemperatureUnit(unit) => {
                    info!(" Updating temperature unit to {:?}", unit);
                    self.temperature_unit = unit;

                    // Update device config in app state
                    {
                        let mut state = app_state.lock().await;
                        state.device_config.temperature_unit = unit;
                    }
                }
                _ => {
                    debug!(" Unhandled action: {:?}", action);
                }
            }
        } else {
            debug!(" Touch event not handled by page");
        }

        // If this press caused the page to change state (became dirty when it
        // wasn't before, or triggered navigation), arm the debounce so the
        // next press is ignored. This prevents a single physical tap from
        // triggering two separate logical actions.
        if matches!(event, TouchEvent::Press(_)) {
            let is_dirty_now = Page::is_dirty(&self.current_page);
            if !was_dirty && is_dirty_now {
                self.skip_next_press = true;
            }
        }
    }

    /// Check if all sensor values indicate Good or Excellent quality.
    fn check_all_healthy(temp: f32, humidity: f32, co2: f32, lux: f32) -> bool {
        let qualities = [
            QualityLevel::assess(SensorType::Temperature, temp),
            QualityLevel::assess(SensorType::Humidity, humidity),
            QualityLevel::assess(SensorType::Co2, co2),
            QualityLevel::assess(SensorType::Lux, lux),
        ];
        qualities
            .iter()
            .all(|q| matches!(q, QualityLevel::Good | QualityLevel::Excellent))
    }

    /// Set the home page mode (called during boot after loading config)
    pub fn set_home_page_mode(&mut self, mode: HomePageMode) {
        self.home_page_mode = mode;
    }

    /// Set the temperature display unit (called during boot after loading config)
    pub fn set_temperature_unit(&mut self, unit: TemperatureUnit) {
        self.temperature_unit = unit;
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
                let lux_ml = sample.values[SENSOR_LUX_INDEX];

                // Convert to float values (divide by 1000)
                let temp_c = temperature_mc as f32 / 1000.0;
                let humidity_pct = humidity_mp as f32 / 1000.0;
                let co2_ppm = co2_mp as f32 / 1000.0;
                let lux_val = lux_ml as f32 / 1000.0;

                debug!("{}", sample);

                // Track health for auto-cycle
                self.all_sensors_healthy =
                    Self::check_all_healthy(temp_c, humidity_pct, co2_ppm, lux_val);
                self.last_sensor_timestamp = sample.timestamp as u64;

                let sensor_data = SensorData {
                    temperature: Some(temp_c),
                    humidity: Some(humidity_pct),
                    co2: Some(co2_ppm),
                    lux: Some(lux_val),
                    timestamp: sample.timestamp as u64,
                };

                // Persist into the centralized store so future page
                // navigations start with current data.
                self.sensor_store.push(&sensor_data);

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
                let lux_ml = rollup.avg[SENSOR_LUX_INDEX];

                let temp_c = temperature_mc as f32 / 1000.0;
                let humidity_pct = humidity_mp as f32 / 1000.0;
                let co2_ppm = co2_mp as f32 / 1000.0;
                let lux_val = lux_ml as f32 / 1000.0;

                debug!("{}", rollup);

                let sensor_data = SensorData {
                    temperature: Some(temp_c),
                    humidity: Some(humidity_pct),
                    co2: Some(co2_ppm),
                    lux: Some(lux_val),
                    timestamp: rollup.start_ts as u64,
                };

                // Persist into the centralized store
                self.sensor_store.push(&sensor_data);

                let page_event = PageEvent::SensorUpdate(sensor_data);
                let needs_redraw = Page::on_event(&mut self.current_page, &page_event);

                if needs_redraw || needs_redraw_rollup {
                    debug!(" Page marked for redraw after rollup update");
                    self.needs_redraw = true;
                }
            }
        }
    }

    /// Render the current page if needed.
    ///
    /// Drawing targets the PSRAM framebuffer first. After the page finishes,
    /// only the bounding rectangle of pixels that actually changed is flushed
    /// to the hardware display over SPI — eliminating the black-flash flicker
    /// that previously occurred when the full screen was cleared each frame.
    fn render(&mut self) -> Result<(), D::Error> {
        if self.needs_redraw {
            debug!(" Rendering page to framebuffer");

            // Clear the framebuffer (only pixels that differ will be marked dirty)
            let _ = self.framebuffer.clear(Rgb565::BLACK);

            // Draw the current page into the RAM framebuffer (infallible)
            let _ = self.current_page.draw_page(&mut self.framebuffer);

            // Flush only the changed region to the hardware display
            self.framebuffer.flush(&mut self.display)?;

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

        // Auto-cycle logic (Home grid mode only)
        if self.auto_cycle_enabled
            && self.last_sensor_timestamp > 0
            && self
                .last_sensor_timestamp
                .saturating_sub(self.auto_cycle_last_switch)
                >= AUTO_CYCLE_INTERVAL_SECS
        {
            if self.all_sensors_healthy {
                // Cycle to next trend page
                let target = AUTO_CYCLE_PAGES[self.auto_cycle_index % AUTO_CYCLE_PAGES.len()];
                self.auto_cycle_index = (self.auto_cycle_index + 1) % AUTO_CYCLE_PAGES.len();
                self.auto_cycle_last_switch = self.last_sensor_timestamp;
                debug!(" Auto-cycle: navigating to {:?}", target);
                self.navigate_to(target, app_state).await;
                // Keep auto_cycle_enabled true so we continue cycling
                self.auto_cycle_enabled = true;
            } else {
                // A sensor is unhealthy — return to HomeGrid
                debug!(" Auto-cycle: sensor unhealthy, returning to HomeGrid");
                self.auto_cycle_last_switch = self.last_sensor_timestamp;
                self.navigate_to(PageId::HomeGrid, app_state).await;
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
