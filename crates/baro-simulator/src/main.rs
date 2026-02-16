//! Desktop simulator for the baro-rs environmental instrumentation UI.
//!
//! Renders baro-core pages in an SDL2 window via `embedded-graphics-simulator`.
//! Generates synthetic sensor data so pages can be exercised without hardware.
//!
//! # Key bindings
//!
//! | Key | Action                       |
//! |-----|------------------------------|
//! | 1   | Home page                    |
//! | 2   | Temperature trend            |
//! | 3   | Humidity trend               |
//! | 4   | CO₂ trend                    |
//! | 5   | Settings page                |
//! | Q   | Quit                         |
//!
//! Mouse clicks are forwarded as touch events.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window, sdl2::Keycode,
};
use log::info;

use baro_core::pages::page::Page;
use baro_core::pages::{HomePage, PageWrapper, SettingsPage, TrendPage, WifiErrorPage};
use baro_core::sensors::SensorType;
use baro_core::storage::{RawSample, TimeWindow};
use baro_core::ui::{
    Action, DISPLAY_HEIGHT_PX, DISPLAY_WIDTH_PX, PageEvent, PageId, SensorData, TouchEvent,
    TouchPoint,
};

extern crate alloc;
use alloc::boxed::Box;

// ---------------------------------------------------------------------------
// Display constants
// ---------------------------------------------------------------------------

/// Pixel scale factor for the simulator window.
const WINDOW_SCALE: u32 = 2;

/// Target frame duration (~30 FPS).
const FRAME_DURATION: Duration = Duration::from_millis(33);

/// Interval between synthetic sensor samples.
const MOCK_SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

// ---------------------------------------------------------------------------
// Mock data generation
// ---------------------------------------------------------------------------

/// Generates synthetic sensor readings that vary over time.
struct MockSensorGenerator {
    /// Monotonic seconds counter used as the fake "epoch".
    elapsed_secs: f64,
}

impl MockSensorGenerator {
    fn new() -> Self {
        Self { elapsed_secs: 0.0 }
    }

    /// Advance the internal clock and return a new sample.
    fn next_sample(&mut self, dt_secs: f64) -> SensorData {
        self.elapsed_secs += dt_secs;
        let t = self.elapsed_secs;

        // Temperature: 20–26 °C sinusoidal with slow drift
        let temperature = 23.0 + 3.0 * (t / 120.0).sin() + 0.5 * (t / 37.0).cos();

        // Humidity: 40–60 % with different period
        let humidity = 50.0 + 10.0 * (t / 180.0).sin() + 2.0 * (t / 23.0).cos();

        // CO₂: 400–800 ppm with a longer cycle
        let co2 = 600.0 + 200.0 * (t / 300.0).sin() + 30.0 * (t / 41.0).cos();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        SensorData {
            temperature: Some(temperature as f32),
            humidity: Some(humidity as f32),
            co2: Some(co2 as f32),
            timestamp,
        }
    }

    /// Generate a batch of historical [`RawSample`]s for trend-page warm-up.
    ///
    /// Returns `count` samples spaced `interval_secs` apart, ending at `end_ts`.
    fn generate_history(
        &mut self,
        count: usize,
        interval_secs: u32,
        end_ts: u32,
    ) -> alloc::vec::Vec<RawSample> {
        let start_ts = end_ts.saturating_sub((count as u32) * interval_secs);
        (0..count)
            .map(|i| {
                let ts = start_ts + (i as u32) * interval_secs;
                let t = ts as f64;

                let temp_mc =
                    ((23.0 + 3.0 * (t / 120.0).sin() + 0.5 * (t / 37.0).cos()) * 1000.0) as i32;
                let hum_mp =
                    ((50.0 + 10.0 * (t / 180.0).sin() + 2.0 * (t / 23.0).cos()) * 1000.0) as i32;
                let co2_mp =
                    ((600.0 + 200.0 * (t / 300.0).sin() + 30.0 * (t / 41.0).cos()) * 1000.0) as i32;

                let mut sample = RawSample::default();
                sample.timestamp = ts;
                sample.values[baro_core::sensors::TEMPERATURE] = temp_mc;
                sample.values[baro_core::sensors::HUMIDITY] = hum_mp;
                sample.values[baro_core::sensors::CO2] = co2_mp;

                sample
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Page helpers
// ---------------------------------------------------------------------------

/// Full-screen bounding rectangle.
fn screen_bounds() -> Rectangle {
    Rectangle::new(
        Point::zero(),
        Size::new(DISPLAY_WIDTH_PX as u32, DISPLAY_HEIGHT_PX as u32),
    )
}

/// Create a new page of the given kind, optionally pre-loaded with history.
fn create_page(page_id: PageId, sensor_gen: &mut MockSensorGenerator) -> PageWrapper {
    let bounds = screen_bounds();

    match page_id {
        PageId::Home => {
            let mut page = HomePage::new(bounds);
            page.init();
            PageWrapper::Home(Box::new(page))
        }
        PageId::Settings => {
            let mut page = SettingsPage::new(bounds);
            page.init();
            PageWrapper::Settings(Box::new(page))
        }
        PageId::TrendTemperature => create_trend_page(
            bounds,
            SensorType::Temperature,
            TimeWindow::FiveMinutes,
            sensor_gen,
        ),
        PageId::TrendHumidity => create_trend_page(
            bounds,
            SensorType::Humidity,
            TimeWindow::OneHour,
            sensor_gen,
        ),
        PageId::TrendCo2 => create_trend_page(
            bounds,
            SensorType::Co2,
            TimeWindow::ThirtyMinutes,
            sensor_gen,
        ),
        PageId::WifiError => PageWrapper::WifiError(Box::new(WifiErrorPage::new())),
        // Fallback: show home for any unhandled page ID
        _ => {
            let mut page = HomePage::new(bounds);
            page.init();
            PageWrapper::Home(Box::new(page))
        }
    }
}

/// Create a [`TrendPage`] pre-loaded with synthetic historical data.
fn create_trend_page(
    bounds: Rectangle,
    sensor: SensorType,
    window: TimeWindow,
    sensor_gen: &mut MockSensorGenerator,
) -> PageWrapper {
    let mut page = TrendPage::new(bounds, sensor, window);

    let now_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    // Generate enough history to fill the requested window
    let sample_interval_secs: u32 = 10;
    let count = (window.duration_secs() / sample_interval_secs) as usize;
    let samples = sensor_gen.generate_history(count, sample_interval_secs, now_ts);

    page.load_historical_raw_samples(&samples, now_ts);
    PageWrapper::TrendPage(Box::new(page))
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

/// Map an SDL keycode to a page navigation request.
fn keycode_to_page(keycode: Keycode) -> Option<PageId> {
    match keycode {
        Keycode::Num1 | Keycode::Kp1 => Some(PageId::Home),
        Keycode::Num2 | Keycode::Kp2 => Some(PageId::TrendTemperature),
        Keycode::Num3 | Keycode::Kp3 => Some(PageId::TrendHumidity),
        Keycode::Num4 | Keycode::Kp4 => Some(PageId::TrendCo2),
        Keycode::Num5 | Keycode::Kp5 => Some(PageId::Settings),
        Keycode::Num6 | Keycode::Kp6 => Some(PageId::WifiError),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    env_logger::init();
    info!("Starting baro-rs simulator");
    info!(
        "Display: {}×{} (scale {}×)",
        DISPLAY_WIDTH_PX, DISPLAY_HEIGHT_PX, WINDOW_SCALE
    );
    info!("Keys: 1=Home  2=TempTrend  3=HumTrend  4=CO₂Trend  5=Settings  6=WifiErr  Q=Quit");

    // SDL2 display and window
    let mut display = SimulatorDisplay::<Rgb565>::new(Size::new(
        DISPLAY_WIDTH_PX as u32,
        DISPLAY_HEIGHT_PX as u32,
    ));

    let output_settings = OutputSettingsBuilder::new().scale(WINDOW_SCALE).build();
    let mut window = Window::new("Baro Simulator", &output_settings);

    // Sensor data generator
    let mut sensor_gen = MockSensorGenerator::new();

    // Start on the home page
    let mut current_page = create_page(PageId::Home, &mut sensor_gen);

    // Timing
    let mut last_sample = Instant::now();

    // The SDL window is lazily initialized on the first `update()` call.
    // We must call `update()` once before `events()` or it will panic.
    let _ = display.clear(Rgb565::BLACK);
    let _ = Page::draw_page(&mut current_page, &mut display);
    Page::mark_clean(&mut current_page);
    window.update(&display);
    let mut needs_redraw = false;

    // -----------------------------------------------------------------------
    // Main loop
    // -----------------------------------------------------------------------
    'running: loop {
        let frame_start = Instant::now();

        // --- SDL events ---------------------------------------------------
        for event in window.events() {
            match event {
                SimulatorEvent::Quit => break 'running,

                SimulatorEvent::KeyDown { keycode, .. } => {
                    if keycode == Keycode::Q || keycode == Keycode::Escape {
                        break 'running;
                    }

                    if let Some(target) = keycode_to_page(keycode) {
                        info!("Navigating to {:?}", target);
                        current_page = create_page(target, &mut sensor_gen);
                        needs_redraw = true;
                    }
                }

                SimulatorEvent::MouseButtonDown { point, .. } => {
                    let touch = TouchEvent::Press(TouchPoint::new(
                        point.x.max(0) as u16,
                        point.y.max(0) as u16,
                    ));

                    if let Some(action) = Page::handle_touch(&mut current_page, touch) {
                        match action {
                            Action::NavigateToPage(page_id) => {
                                info!("Touch → navigate to {:?}", page_id);
                                current_page = create_page(page_id, &mut sensor_gen);
                                needs_redraw = true;
                            }
                            other => {
                                info!("Touch → action {:?}", other);
                            }
                        }
                    }
                }

                _ => {}
            }
        }

        // --- Mock sensor data ---------------------------------------------
        if last_sample.elapsed() >= MOCK_SAMPLE_INTERVAL {
            let data = sensor_gen.next_sample(MOCK_SAMPLE_INTERVAL.as_secs_f64());
            let event = PageEvent::SensorUpdate(data);

            if Page::on_event(&mut current_page, &event) {
                needs_redraw = true;
            }
            last_sample = Instant::now();
        }

        // --- Page update tick ---------------------------------------------
        Page::update(&mut current_page);

        // --- Render -------------------------------------------------------
        if needs_redraw || Page::is_dirty(&current_page) {
            let _ = display.clear(Rgb565::BLACK);
            if let Err(e) = Page::draw_page(&mut current_page, &mut display) {
                log::error!("Draw error: {:?}", e);
            }
            Page::mark_clean(&mut current_page);
            needs_redraw = false;
        }

        window.update(&display);

        // --- Frame pacing -------------------------------------------------
        let elapsed = frame_start.elapsed();
        if elapsed < FRAME_DURATION {
            std::thread::sleep(FRAME_DURATION - elapsed);
        }
    }

    info!("Simulator exiting");
}
