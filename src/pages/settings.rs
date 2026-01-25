// src/pages/settings.rs
//! Settings page with live sensor data and log feed

use crate::pages::page_manager::Page;
use crate::ui::{
    Action, Alignment, Container, Direction, Drawable, PageEvent, PageId, SizeConstraint,
    StorageEvent, TextComponent, TextSize, TouchEvent,
};
use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::Text;
use heapless::{String as HeaplessString, Vec};
use log::debug;

/// Log entry for the live feed
#[derive(Clone)]
struct LogEntry {
    message: HeaplessString<64>,
}

pub struct SettingsPage {
    bounds: Rectangle,
    container: Container<6>,
    // Title
    title: TextComponent,
    // Section header
    sensor_header: TextComponent,
    // Sensor data displays
    temperature_text: TextComponent,
    humidity_text: TextComponent,
    // Log section
    log_header: TextComponent,
    log_area_bounds: Rectangle,
    // Log entries data (max 20)
    log_entries: Vec<LogEntry, 20>,
    // Current sensor values
    last_temperature: Option<f32>,
    last_humidity: Option<f32>,
    dirty: bool,
}

impl SettingsPage {
    pub fn new(bounds: Rectangle) -> Self {
        // Create vertical container with stretch alignment
        let container = Container::new(bounds, Direction::Vertical)
            .with_alignment(Alignment::Stretch)
            .with_spacing(5);

        // Create all text components with zero bounds (will be set during init)
        let title = TextComponent::new(Rectangle::zero(), "Settings & Monitor", TextSize::Large);

        let sensor_header = TextComponent::new(
            Rectangle::zero(),
            "Current Sensor Values:",
            TextSize::Medium,
        );

        let temperature_text =
            TextComponent::new(Rectangle::zero(), "Temperature: --", TextSize::Medium);

        let humidity_text = TextComponent::new(Rectangle::zero(), "Humidity: --", TextSize::Medium);

        let log_header = TextComponent::new(Rectangle::zero(), "Live Data Feed:", TextSize::Medium);

        Self {
            bounds,
            container,
            title,
            sensor_header,
            temperature_text,
            humidity_text,
            log_header,
            log_area_bounds: Rectangle::zero(),
            log_entries: Vec::new(),
            last_temperature: None,
            last_humidity: None,
            dirty: true,
        }
    }

    pub fn init(&mut self) {
        // Build the layout dynamically:
        // - Title: 30px
        // - Sensor header: 20px
        // - Temperature: 20px
        // - Humidity: 20px
        // - Log header: 20px
        // - Log display: Expands to fill remaining space

        // Add title
        self.container
            .add_child(
                Size::new(self.bounds.size.width, 30),
                SizeConstraint::Fixed(30),
            )
            .ok();

        // Add sensor header
        self.container
            .add_child(
                Size::new(self.bounds.size.width, 20),
                SizeConstraint::Fixed(20),
            )
            .ok();

        // Add temperature
        self.container
            .add_child(
                Size::new(self.bounds.size.width, 20),
                SizeConstraint::Fixed(20),
            )
            .ok();

        // Add humidity
        self.container
            .add_child(
                Size::new(self.bounds.size.width, 20),
                SizeConstraint::Fixed(20),
            )
            .ok();

        // Add log header
        self.container
            .add_child(
                Size::new(self.bounds.size.width, 20),
                SizeConstraint::Fixed(20),
            )
            .ok();

        // Add log display that expands
        self.container
            .add_child(Size::new(self.bounds.size.width, 0), SizeConstraint::Expand)
            .ok();

        // Update all component bounds from container
        if let Some(bounds) = self.container.child_bounds(0) {
            self.title.set_bounds(bounds);
        }
        if let Some(bounds) = self.container.child_bounds(1) {
            self.sensor_header.set_bounds(bounds);
        }
        if let Some(bounds) = self.container.child_bounds(2) {
            self.temperature_text.set_bounds(bounds);
        }
        if let Some(bounds) = self.container.child_bounds(3) {
            self.humidity_text.set_bounds(bounds);
        }
        if let Some(bounds) = self.container.child_bounds(4) {
            self.log_header.set_bounds(bounds);
        }
        if let Some(bounds) = self.container.child_bounds(5) {
            self.log_area_bounds = bounds;
        }

        self.dirty = true;
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
    }

    fn add_log_entry(&mut self, message: &str, _timestamp: u64) {
        let mut entry_text = HeaplessString::<64>::new();
        entry_text.push_str(message).ok();

        let entry = LogEntry {
            message: entry_text,
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
        // Just mark as dirty - rendering will handle showing the log entries
        self.dirty = true;
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

    fn handle_touch(&mut self, _event: TouchEvent) -> Option<Action> {
        None
    }

    fn update(&mut self) {
        // Update page state if needed
    }

    fn on_event(&mut self, event: &PageEvent) -> bool {
        debug!(" Received event: {:?}", event);
        match event {
            PageEvent::SensorUpdate(data) => {
                debug!(
                    " Processing sensor update - temp: {:?}, humidity: {:?}",
                    data.temperature, data.humidity
                );
                // Update sensor values
                if let Some(temp) = data.temperature {
                    self.last_temperature = Some(temp);
                }
                if let Some(hum) = data.humidity {
                    self.last_humidity = Some(hum);
                }

                self.update_sensor_displays();
                debug!(" Sensor displays updated");

                // Add log entry
                let mut log_msg = HeaplessString::<64>::new();
                use core::fmt::Write;
                if let Some(temp) = data.temperature {
                    write!(&mut log_msg, "[Sensor] T:{:.1}C", temp).ok();
                } else if let Some(hum) = data.humidity {
                    write!(&mut log_msg, "[Sensor] H:{:.1}%", hum).ok();
                }
                self.add_log_entry(&log_msg, data.timestamp);
                debug!(" Log entry added: {}", log_msg.as_str());

                self.dirty = true;
                true
            }
            PageEvent::StorageEvent(storage_event) => {
                debug!(" Processing storage event: {:?}", storage_event);
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
            PageEvent::RollupEvent(_) => {
                // Settings page doesn't need to handle rollup events directly
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

        // Draw all components
        self.title.draw(display)?;
        self.sensor_header.draw(display)?;
        self.temperature_text.draw(display)?;
        self.humidity_text.draw(display)?;
        self.log_header.draw(display)?;

        // Draw log box background
        if self.log_area_bounds != Rectangle::zero() {
            self.log_area_bounds
                .into_styled(
                    PrimitiveStyleBuilder::new()
                        .fill_color(Rgb565::new(0x08, 0x08, 0x10))
                        .stroke_color(Rgb565::CSS_DARK_BLUE)
                        .stroke_width(1)
                        .build(),
                )
                .draw(display)?;

            // Draw log entries (most recent first, up to what fits)
            let font = embedded_graphics::mono_font::ascii::FONT_5X8;
            let line_height = font.character_size.height + 2;
            let text_style = MonoTextStyle::new(&font, Rgb565::WHITE);

            let content_x = self.log_area_bounds.top_left.x + 4;
            let mut y = self.log_area_bounds.top_left.y + line_height as i32;

            let max_lines = (self.log_area_bounds.size.height / line_height).min(20) as usize;

            if self.log_entries.is_empty() {
                // Show placeholder
                Text::new("Waiting for data...", Point::new(content_x, y), text_style)
                    .draw(display)?;
            } else {
                // Show most recent entries (reversed)
                for entry in self.log_entries.iter().rev().take(max_lines) {
                    if y + line_height as i32
                        > self.log_area_bounds.top_left.y + self.log_area_bounds.size.height as i32
                    {
                        break;
                    }
                    Text::new(entry.message.as_str(), Point::new(content_x, y), text_style)
                        .draw(display)?;
                    y += line_height as i32;
                }
            }
        }

        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    fn is_dirty(&self) -> bool {
        self.dirty
            || self.title.is_dirty()
            || self.sensor_header.is_dirty()
            || self.temperature_text.is_dirty()
            || self.humidity_text.is_dirty()
            || self.log_header.is_dirty()
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
        self.title.mark_clean();
        self.sensor_header.mark_clean();
        self.temperature_text.mark_clean();
        self.humidity_text.mark_clean();
        self.log_header.mark_clean();
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}
