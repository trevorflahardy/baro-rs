// src/pages/settings.rs
//! Settings page with live sensor data and log feed.

use crate::pages::page_manager::Page;
use crate::ui::{
    Action, Alignment, Container, Direction, Drawable, Element, PageEvent, PageId, SizeConstraint,
    StorageEvent, TextSize, TouchEvent,
};
use embedded_graphics::Drawable as EgDrawable;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::Text;
use heapless::{String as HeaplessString, Vec};
use log::debug;

/// Log entry for the live feed.
#[derive(Clone)]
struct LogEntry {
    message: HeaplessString<64>,
}

pub struct SettingsPage {
    bounds: Rectangle,
    container: Container<6>,

    // Child indices (stable after init).
    title_idx: usize,
    sensor_header_idx: usize,
    temperature_idx: usize,
    humidity_idx: usize,
    log_header_idx: usize,
    log_area_idx: usize,

    // Log entries data (max 20).
    log_entries: Vec<LogEntry, 20>,

    // Current sensor values.
    last_temperature: Option<f32>,
    last_humidity: Option<f32>,

    dirty: bool,
}

impl SettingsPage {
    pub fn new(bounds: Rectangle) -> Self {
        let container = Container::new(bounds, Direction::Vertical)
            .with_alignment(Alignment::Stretch)
            .with_gap(5);

        // Temporary indices (replaced in init).
        Self {
            bounds,
            container,
            title_idx: 0,
            sensor_header_idx: 0,
            temperature_idx: 0,
            humidity_idx: 0,
            log_header_idx: 0,
            log_area_idx: 0,
            log_entries: Vec::new(),
            last_temperature: None,
            last_humidity: None,
            dirty: true,
        }
    }

    pub fn init(&mut self) {
        // Layout:
        // - Title: 30px
        // - Sensor header: 20px
        // - Temperature: 20px
        // - Humidity: 20px
        // - Log header: 20px
        // - Log display: grows
        let hint = Rectangle::new(Point::zero(), Size::new(self.bounds.size.width, 1));

        self.title_idx = self
            .container
            .add_child(
                Element::text(hint, "Settings & Monitor", TextSize::Large),
                SizeConstraint::Fixed(30),
            )
            .unwrap_or(0);

        self.sensor_header_idx = self
            .container
            .add_child(
                Element::text(hint, "Current Sensor Values:", TextSize::Medium),
                SizeConstraint::Fixed(20),
            )
            .unwrap_or(1);

        self.temperature_idx = self
            .container
            .add_child(
                Element::text(hint, "Temperature: --", TextSize::Medium),
                SizeConstraint::Fixed(20),
            )
            .unwrap_or(2);

        self.humidity_idx = self
            .container
            .add_child(
                Element::text(hint, "Humidity: --", TextSize::Medium),
                SizeConstraint::Fixed(20),
            )
            .unwrap_or(3);

        self.log_header_idx = self
            .container
            .add_child(
                Element::text(hint, "Live Data Feed:", TextSize::Medium),
                SizeConstraint::Fixed(20),
            )
            .unwrap_or(4);

        self.log_area_idx = self
            .container
            .add_child(Element::spacer(hint), SizeConstraint::Grow(1))
            .unwrap_or(5);

        self.dirty = true;
    }

    fn update_sensor_displays(&mut self) {
        // Temperature.
        if let Some(temp) = self.last_temperature {
            let mut text = HeaplessString::<64>::new();
            use core::fmt::Write;
            write!(&mut text, "Temperature: {:.1}°C", temp).ok();

            if let Some(Element::Text(t)) = self.container.child_mut(self.temperature_idx) {
                t.set_text(&text);
            }
        }

        // Humidity.
        if let Some(hum) = self.last_humidity {
            let mut text = HeaplessString::<64>::new();
            use core::fmt::Write;
            write!(&mut text, "Humidity: {:.1}%", hum).ok();

            if let Some(Element::Text(t)) = self.container.child_mut(self.humidity_idx) {
                t.set_text(&text);
            }
        }
    }

    fn add_log_entry(&mut self, message: &str, _timestamp: u64) {
        let mut entry_text = HeaplessString::<64>::new();
        entry_text.push_str(message).ok();

        if self.log_entries.len() >= 20 {
            // Shift left.
            for i in 0..19 {
                let next = self.log_entries.get(i + 1).cloned();
                if let (Some(dst), Some(next)) = (self.log_entries.get_mut(i), next) {
                    *dst = next;
                }
            }
            if let Some(last) = self.log_entries.get_mut(19) {
                last.message = entry_text;
            }
        } else {
            self.log_entries
                .push(LogEntry {
                    message: entry_text,
                })
                .ok();
        }
    }

    fn log_area_bounds(&self) -> Rectangle {
        self.container
            .child_bounds(self.log_area_idx)
            .unwrap_or(Rectangle::zero())
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

    fn update(&mut self) {}

    fn on_event(&mut self, event: &PageEvent) -> bool {
        debug!(" Received event: {:?}", event);
        match event {
            PageEvent::SensorUpdate(data) => {
                debug!(
                    " Processing sensor update - temp: {:?}, humidity: {:?}",
                    data.temperature, data.humidity
                );

                if let Some(temp) = data.temperature {
                    self.last_temperature = Some(temp);
                }
                if let Some(hum) = data.humidity {
                    self.last_humidity = Some(hum);
                }

                self.update_sensor_displays();
                debug!(" Sensor displays updated");

                // Log entry.
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
                        write!(&mut log_msg, "[Rollup] {}: {}", interval, count).ok();
                        self.add_log_entry(&log_msg, *timestamp);
                    }
                }
                self.dirty = true;
                true
            }
            _ => false,
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
        // Clear background.
        self.bounds
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
            .draw(display)?;

        // Layout-driven content.
        self.container.draw(display)?;

        // Log feed (drawn inside reserved log area bounds).
        let log_area = self.log_area_bounds();
        let style = PrimitiveStyleBuilder::new()
            .fill_color(Rgb565::BLACK)
            .stroke_color(Rgb565::WHITE)
            .stroke_width(1)
            .build();

        log_area.into_styled(style).draw(display)?;

        let text_style = MonoTextStyle::new(
            &embedded_graphics::mono_font::ascii::FONT_6X10,
            Rgb565::WHITE,
        );

        let mut y = log_area.top_left.y + 12;
        let max_y = log_area.top_left.y + log_area.size.height as i32 - 2;

        for entry in self.log_entries.iter().rev() {
            if y > max_y {
                break;
            }
            Text::new(
                entry.message.as_str(),
                Point::new(log_area.top_left.x + 4, y),
                text_style,
            )
            .draw(display)?;
            y += 12;
        }

        Ok(())
    }

    fn bounds(&self) -> Rectangle {
        self.bounds
    }

    fn is_dirty(&self) -> bool {
        self.dirty || self.container.is_dirty()
    }

    fn mark_clean(&mut self) {
        self.dirty = false;
        self.container.mark_clean();
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.container.mark_dirty();
    }
}
