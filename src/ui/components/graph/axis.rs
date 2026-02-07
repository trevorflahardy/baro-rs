//! Axis configuration and label rendering
//!
//! Provides axis configuration, label formatting, and rendering for graph axes.

use embedded_graphics::mono_font::{MonoTextStyle, ascii::FONT_6X10};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Text};
use heapless::String;

use crate::ui::styling::LIGHT_GRAY;

use super::constants::{DEFAULT_X_AXIS_LABEL_COUNT, MAX_AXIS_LABEL_LENGTH};
use super::viewport::Viewport;

/// Label formatter for axis values
#[derive(Debug, Clone, Copy)]
pub enum LabelFormatter {
    /// Format as time offset (e.g., "-48H", "-24H", "NOW")
    TimeOffset {
        /// Label for the current time point
        now_label: &'static str,
    },
    /// Format as numeric value with optional unit
    Numeric {
        /// Number of decimal places
        precision: usize,
        /// Unit suffix (e.g., "°C", "%", "ppm")
        unit: &'static str,
    },
    /// Custom formatter using function pointer
    Custom(fn(f32) -> String<MAX_AXIS_LABEL_LENGTH>),
}

/// X-axis configuration
#[derive(Clone, Copy)]
pub struct XAxisConfig {
    /// Number of labels to display
    pub label_count: usize,
    /// Label formatter
    pub label_formatter: LabelFormatter,
    /// Text style for labels
    pub label_style: MonoTextStyle<'static, Rgb565>,
    /// Whether to show the axis line
    pub show_axis_line: bool,
}

impl Default for XAxisConfig {
    fn default() -> Self {
        Self {
            label_count: DEFAULT_X_AXIS_LABEL_COUNT,
            label_formatter: LabelFormatter::Numeric {
                precision: 1,
                unit: "",
            },
            label_style: MonoTextStyle::new(&FONT_6X10, LIGHT_GRAY),
            show_axis_line: false,
        }
    }
}

/// Y-axis configuration
#[derive(Clone, Copy)]
pub struct YAxisConfig {
    /// Number of labels to display
    pub label_count: usize,
    /// Label formatter
    pub label_formatter: LabelFormatter,
    /// Text style for labels
    pub label_style: MonoTextStyle<'static, Rgb565>,
    /// Whether to show the axis line
    pub show_axis_line: bool,
}

impl Default for YAxisConfig {
    fn default() -> Self {
        Self {
            label_count: DEFAULT_X_AXIS_LABEL_COUNT,
            label_formatter: LabelFormatter::Numeric {
                precision: 1,
                unit: "",
            },
            label_style: MonoTextStyle::new(&FONT_6X10, LIGHT_GRAY),
            show_axis_line: false,
        }
    }
}

/// Complete axis configuration
#[derive(Clone, Copy)]
pub struct AxisConfig {
    /// X-axis configuration
    pub x_axis: Option<XAxisConfig>,
    /// Y-axis configuration (placeholder for future use)
    pub y_axis: Option<YAxisConfig>,
}

impl Default for AxisConfig {
    fn default() -> Self {
        Self {
            x_axis: Some(XAxisConfig::default()),
            y_axis: None,
        }
    }
}

/// Draw X-axis labels
///
/// Renders labels along the bottom of the plot area according to configuration.
pub(super) fn draw_x_axis_labels<D: DrawTarget<Color = Rgb565>>(
    config: &XAxisConfig,
    viewport: &Viewport,
    display: &mut D,
) -> Result<(), D::Error> {
    if config.label_count == 0 {
        return Ok(());
    }

    let plot_area = viewport.plot_area();
    let data_bounds = viewport.data_bounds();
    let data_range = data_bounds.x_range();

    // Calculate label positions
    let spacing = plot_area.size.width / (config.label_count.saturating_sub(1).max(1)) as u32;
    let label_y = plot_area.top_left.y + plot_area.size.height as i32 + 15;

    for i in 0..config.label_count {
        // Calculate data value for this position
        let t = if config.label_count > 1 {
            i as f32 / (config.label_count - 1) as f32
        } else {
            0.5
        };

        let data_x = data_bounds.x_min + (data_bounds.x_max - data_bounds.x_min) * t;

        // Format label
        let label_text = format_label(
            data_x,
            data_bounds.x_max,
            data_range,
            &config.label_formatter,
        );

        // Calculate screen position
        let label_x = if i == 0 {
            plot_area.top_left.x
        } else if i == config.label_count - 1 {
            plot_area.top_left.x + plot_area.size.width as i32
        } else {
            plot_area.top_left.x + (spacing * i as u32) as i32
        };

        // Determine alignment based on position
        let alignment = if i == 0 {
            Alignment::Left
        } else if i == config.label_count - 1 {
            Alignment::Right
        } else {
            Alignment::Center
        };

        // Draw label
        Text::with_alignment(
            label_text.as_str(),
            Point::new(label_x, label_y),
            config.label_style,
            alignment,
        )
        .draw(display)?;
    }

    Ok(())
}

/// Draw Y-axis labels
///
/// Renders labels along the left side of the plot area according to configuration.
pub(super) fn draw_y_axis_labels<D: DrawTarget<Color = Rgb565>>(
    config: &YAxisConfig,
    viewport: &Viewport,
    display: &mut D,
) -> Result<(), D::Error> {
    if config.label_count == 0 {
        return Ok(());
    }

    let plot_area = viewport.plot_area();
    let data_bounds = viewport.data_bounds();
    let data_range = data_bounds.y_range();

    // Calculate label positions
    let spacing = plot_area.size.height / (config.label_count.saturating_sub(1).max(1)) as u32;
    let label_x = plot_area.top_left.x - 5; // Left of plot area

    for i in 0..config.label_count {
        // Calculate data value for this position
        let t = if config.label_count > 1 {
            i as f32 / (config.label_count - 1) as f32
        } else {
            0.5
        };

        // Note: Y-axis goes from bottom (min) to top (max), so we invert t
        let data_y = data_bounds.y_min + (data_bounds.y_max - data_bounds.y_min) * (1.0 - t);

        // Format label
        let label_text = format_label(
            data_y,
            data_bounds.y_max,
            data_range,
            &config.label_formatter,
        );

        // Calculate screen position
        let label_y = if i == 0 {
            plot_area.top_left.y
        } else if i == config.label_count - 1 {
            plot_area.top_left.y + plot_area.size.height as i32
        } else {
            plot_area.top_left.y + (spacing * i as u32) as i32
        };

        // Draw label (right-aligned to sit next to the plot area)
        Text::with_alignment(
            label_text.as_str(),
            Point::new(label_x, label_y + 5), // +5 for vertical centering
            config.label_style,
            Alignment::Right,
        )
        .draw(display)?;
    }

    Ok(())
}

/// Format a label value according to the formatter configuration
///
/// Uses a fixed-capacity heapless String to avoid heap allocations during rendering.
/// This reduces memory fragmentation on embedded devices.
fn format_label(
    value: f32,
    max_value: f32,
    data_range: f32,
    formatter: &LabelFormatter,
) -> String<MAX_AXIS_LABEL_LENGTH> {
    match formatter {
        LabelFormatter::TimeOffset { now_label } => {
            let threshold = (data_range.abs() * 0.02).max(1.0);

            // Check if this is the "now" point (within 2% of range or 1s)
            if (value - max_value).abs() <= threshold {
                let mut s = String::new();
                let _ = core::fmt::write(&mut s, format_args!("{}", now_label));
                s
            } else {
                // Calculate time offset in seconds
                let offset_seconds = (value - max_value) as i32;
                let mut s = String::new();

                // Format adaptively based on magnitude
                let abs_offset = offset_seconds.abs();

                if abs_offset >= 86400 {
                    // >= 1 day: show days
                    let days = offset_seconds / 86400;
                    let _ = core::fmt::write(&mut s, format_args!("{}D", days));
                } else if abs_offset >= 3600 {
                    // >= 1 hour: show hours
                    let hours = offset_seconds / 3600;
                    let _ = core::fmt::write(&mut s, format_args!("{}H", hours));
                } else if abs_offset >= 60 {
                    // >= 1 minute: show minutes
                    let minutes = offset_seconds / 60;
                    let _ = core::fmt::write(&mut s, format_args!("{}M", minutes));
                } else {
                    // < 1 minute: show seconds
                    let _ = core::fmt::write(&mut s, format_args!("{}S", offset_seconds));
                }

                s
            }
        }
        LabelFormatter::Numeric { precision, unit } => {
            let mut s = String::new();
            match precision {
                0 => {
                    let _ = core::fmt::write(&mut s, format_args!("{:.0}{}", value, unit));
                }
                1 => {
                    let _ = core::fmt::write(&mut s, format_args!("{:.1}{}", value, unit));
                }
                2 => {
                    let _ = core::fmt::write(&mut s, format_args!("{:.2}{}", value, unit));
                }
                _ => {
                    let _ = core::fmt::write(&mut s, format_args!("{:.1}{}", value, unit));
                }
            }
            s
        }
        LabelFormatter::Custom(func) => func(value),
    }
}
