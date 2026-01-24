// src/pages/settings.rs
//! Settings page with live sensor data and log feed

use crate::pages::page_manager::Page;
use crate::ui::{
    Action, Button, ButtonVariant, ColorPalette, Drawable, MultiLineText, PageEvent, PageId,
    StorageEvent, TextComponent, TextSize, TouchEvent, TouchResult, Touchable,
};
use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::mono_font::{MonoTextStyle, ascii::FONT_10X20};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::Text;
use heapless::{String as HeaplessString, Vec};

/// Log entry for the live feed
#[derive(Clone)]
struct LogEntry {
    message: HeaplessString<64>,
    // timestamp: u64,
}

pub struct SettingsPage {
    bounds: Rectangle,
    // Sensor data displays
    temperature_text: TextComponent,
    humidity_text: TextComponent,
    pressure_text: TextComponent,
    // Live log feed
    log_entries: Vec<LogEntry, 20>,
    log_display: MultiLineText,
    // Back button
    back_button: Button,
    // Current sensor values
    last_temperature: Option<f32>,
    last_humidity: Option<f32>,
    last_pressure: Option<f32>,
    dirty: bool,
}

impl SettingsPage {
    pub fn new(bounds: Rectangle) -> Self {
        let palette = ColorPalette::default();

        // Create sensor value displays
        let temperature_text = TextComponent::new(
            Rectangle::new(Point::new(20, 50), Size::new(280, 20)),
            "Temperature: --",
            TextSize::Medium,
        );

        let humidity_text = TextComponent::new(
            Rectangle::new(Point::new(20, 80), Size::new(280, 20)),
            "Humidity: --",
            TextSize::Medium,
        );

        let pressure_text = TextComponent::new(
            Rectangle::new(Point::new(20, 110), Size::new(280, 20)),
            "Pressure: --",
            TextSize::Medium,
        );

        // Live log feed area
        let log_display = MultiLineText::new(
            Rectangle::new(Point::new(10, 150), Size::new(300, 100)),
            "Waiting for data...",
            TextSize::Small,
        );

        // Back button
        let back_button = Button::new(
            Rectangle::new(
                Point::new(20, bounds.size.height as i32 - 60),
                Size::new(120, 40),
            ),
            "Back",
            Action::GoBack,
        )
        .with_palette(palette)
        .with_variant(ButtonVariant::Outline);

        Self {
            bounds,
            temperature_text,
            humidity_text,
            pressure_text,
            log_entries: Vec::new(),
            log_display,
            back_button,
            last_temperature: None,
            last_humidity: None,
            last_pressure: None,
            dirty: true,
        }
    }

    fn update_sensor_displays(&mut self) {
        // Update temperature display
        if let Some(temp) = self.last_temperature {
            let mut text = HeaplessString::<64>::new();
            use core::fmt::Write;
            write!(&mut text, "Temperature: {:.1}°C", temp).ok();
            self.temperature_text.set_text(&text);
        }

        // Update humidity display
        if let Some(hum) = self.last_humidity {
            let mut text = HeaplessString::<64>::new();
            use core::fmt::Write;
            write!(&mut text, "Humidity: {:.1}%", hum).ok();
            self.humidity_text.set_text(&text);
        }

        // Update pressure display
        if let Some(press) = self.last_pressure {
            let mut text = HeaplessString::<64>::new();
            use core::fmt::Write;
            write!(&mut text, "Pressure: {:.1} hPa", press).ok();
            self.pressure_text.set_text(&text);
        }
    }

    fn add_log_entry(&mut self, message: &str, _timestamp: u64) {
        let mut entry_text = HeaplessString::<64>::new();
        entry_text.push_str(message).ok();

        let entry = LogEntry {
            message: entry_text,
            // timestamp,
        };

        // Keep only the last 20 entries
        if self.log_entries.len() >= 20 {
            self.log_entries.remove(0);
        }

        self.log_entries.push(entry).ok();

        // Update log display
        self.update_log_display();
    }

    fn update_log_display(&mut self) {
        // Build the display text from recent log entries
        let mut display_text = HeaplessString::<512>::new();

        for (i, entry) in self.log_entries.iter().enumerate().rev().take(10) {
            if i > 0 {
                display_text.push('\n').ok();
            }
            display_text.push_str(&entry.message).ok();
        }

        if display_text.is_empty() {
            display_text.push_str("No data yet...").ok();
        }

        self.log_display.set_text(&display_text);
    }
}

impl Page for SettingsPage {
    fn id(&self) -> PageId {
        PageId::Settings
    }

    fn title(&self) -> &str {
        "Settings & Monitor"
    }

    fn on_activate(&mut self) {
        self.dirty = true;
    }

    fn handle_touch(&mut self, event: TouchEvent) -> Option<Action> {
        match self.back_button.handle_touch(event) {
            TouchResult::Action(action) => Some(action),
            TouchResult::Handled => None,
            TouchResult::NotHandled => None,
        }
    }

    fn update(&mut self) {
        // Update page state if needed
    }

    fn on_event(&mut self, event: &PageEvent) -> bool {
        match event {
            PageEvent::SensorUpdate(data) => {
                // Update sensor values
                if let Some(temp) = data.temperature {
                    self.last_temperature = Some(temp);
                }
                if let Some(hum) = data.humidity {
                    self.last_humidity = Some(hum);
                }
                if let Some(press) = data.pressure {
                    self.last_pressure = Some(press);
                }

                self.update_sensor_displays();

                // Add log entry
                let mut log_msg = HeaplessString::<64>::new();
                use core::fmt::Write;
                if let Some(temp) = data.temperature {
                    write!(&mut log_msg, "[Sensor] T:{:.1}C", temp).ok();
                } else if let Some(hum) = data.humidity {
                    write!(&mut log_msg, "[Sensor] H:{:.1}%", hum).ok();
                } else if let Some(press) = data.pressure {
                    write!(&mut log_msg, "[Sensor] P:{:.1}hPa", press).ok();
                }
                self.add_log_entry(&log_msg, data.timestamp);

                self.dirty = true;
                true
            }
            PageEvent::StorageEvent(storage_event) => {
                match storage_event {
                    StorageEvent::RawSample {
                        sensor,
                        value,
                        timestamp,
                    } => {
                        let mut log_msg = HeaplessString::<64>::new();
                        use core::fmt::Write;
                        write!(&mut log_msg, "[Raw] {}: {:.2}", sensor, value).ok();
                        self.add_log_entry(&log_msg, *timestamp);
                    }
                    StorageEvent::Rollup {
                        interval,
                        count,
                        timestamp,
                    } => {
                        let mut log_msg = HeaplessString::<64>::new();
                        use core::fmt::Write;
                        write!(&mut log_msg, "[Rollup] {}: {} samples", interval, count).ok();
                        self.add_log_entry(&log_msg, *timestamp);
                    }
                }
                self.dirty = true;
                true
            }
            PageEvent::SystemEvent(_) => {
                // Handle system events if needed
                false
            }
        }
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

impl Drawable for SettingsPage {
    fn draw<D: DrawTarget<Color = Rgb565>>(&self, display: &mut D) -> Result<(), D::Error> {
        // Clear background
        self.bounds
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(display)?;

        // Draw title
        let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
        Text::new("Settings & Monitor", Point::new(20, 25), text_style).draw(display)?;

        // Draw sensor values section header
        let section_style = MonoTextStyle::new(
            &embedded_graphics::mono_font::ascii::FONT_6X10,
            Rgb565::CSS_LIGHT_GRAY,
        );
        Text::new("Current Sensor Values:", Point::new(20, 45), section_style).draw(display)?;

        // Draw sensor data
        self.temperature_text.draw(display)?;
        self.humidity_text.draw(display)?;
        self.pressure_text.draw(display)?;

        // Draw log section
        let log_header_bounds = Rectangle::new(Point::new(10, 135), Size::new(300, 10));
        log_header_bounds
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(Rgb565::CSS_DARK_GRAY)
                    .build(),
            )
            .draw(display)?;

        Text::new("Live Data Feed:", Point::new(15, 145), section_style).draw(display)?;

        // Draw log box
        let log_box = Rectangle::new(Point::new(10, 150), Size::new(300, 100));
        log_box
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(Rgb565::new(0x08, 0x08, 0x10))
                    .stroke_color(Rgb565::CSS_GRAY)
                    .stroke_width(1)
                    .build(),
            )
            .draw(display)?;

        self.log_display.draw(display)?;

        // Draw back button
        self.back_button.draw(display)?;

        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    fn is_dirty(&self) -> bool {
        self.dirty
            || self.temperature_text.is_dirty()
            || self.humidity_text.is_dirty()
            || self.pressure_text.is_dirty()
            || self.log_display.is_dirty()
            || self.back_button.is_dirty()
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
        self.temperature_text.mark_clean();
        self.humidity_text.mark_clean();
        self.pressure_text.mark_clean();
        self.log_display.mark_clean();
        self.back_button.mark_clean();
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}
