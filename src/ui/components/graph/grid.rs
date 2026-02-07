//! Grid line rendering for graph backgrounds
//!
//! Provides configurable vertical and horizontal grid lines with
//! support for solid and dashed line styles.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle};

use super::constants::{
    DEFAULT_GRID_COLOR, DEFAULT_GRID_LINE_WIDTH_PX, DEFAULT_VERTICAL_GRID_COUNT,
};
use super::viewport::Viewport;

/// Line style for grid rendering
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineStyle {
    /// Solid continuous line
    Solid,
    /// Dashed line with specified dash and gap lengths
    Dashed {
        /// Length of each dash in pixels
        dash_length: u32,
        /// Length of gap between dashes in pixels
        gap_length: u32,
    },
}

/// Configuration for vertical grid lines
#[derive(Debug, Clone, Copy)]
pub struct VerticalGridLines {
    /// Number of vertical grid lines
    pub count: usize,
    /// Line color
    pub color: Rgb565,
    /// Line width in pixels
    pub width: u32,
    /// Line style (solid or dashed)
    pub style: LineStyle,
}

impl Default for VerticalGridLines {
    fn default() -> Self {
        Self {
            count: DEFAULT_VERTICAL_GRID_COUNT,
            color: DEFAULT_GRID_COLOR,
            width: DEFAULT_GRID_LINE_WIDTH_PX,
            style: LineStyle::Solid,
        }
    }
}

/// Configuration for horizontal grid lines
#[derive(Debug, Clone, Copy)]
pub struct HorizontalGridLines {
    /// Number of horizontal grid lines
    pub count: usize,
    /// Line color
    pub color: Rgb565,
    /// Line width in pixels
    pub width: u32,
    /// Line style (solid or dashed)
    pub style: LineStyle,
}

impl Default for HorizontalGridLines {
    fn default() -> Self {
        Self {
            count: DEFAULT_VERTICAL_GRID_COUNT,
            color: DEFAULT_GRID_COLOR,
            width: DEFAULT_GRID_LINE_WIDTH_PX,
            style: LineStyle::Solid,
        }
    }
}

/// Complete grid configuration
#[derive(Debug, Clone, Copy)]
pub struct GridConfig {
    /// Vertical grid line configuration (None = no vertical lines)
    pub vertical_lines: Option<VerticalGridLines>,
    /// Horizontal grid line configuration (None = no horizontal lines)
    pub horizontal_lines: Option<HorizontalGridLines>,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            vertical_lines: Some(VerticalGridLines::default()),
            horizontal_lines: None,
        }
    }
}

/// Draw grid lines on the graph
///
/// Renders vertical and horizontal grid lines according to configuration.
pub(super) fn draw_grid<D: DrawTarget<Color = Rgb565>>(
    config: &GridConfig,
    viewport: &Viewport,
    display: &mut D,
) -> Result<(), D::Error> {
    let plot_area = viewport.plot_area();

    // Draw vertical grid lines
    if let Some(ref vlines) = config.vertical_lines
        && vlines.count > 0
    {
        let spacing = plot_area.size.width / (vlines.count + 1) as u32;

        for i in 1..=vlines.count {
            let x = plot_area.top_left.x + (spacing * i as u32) as i32;
            let start = Point::new(x, plot_area.top_left.y);
            let end = Point::new(x, plot_area.top_left.y + plot_area.size.height as i32);

            draw_line(
                start,
                end,
                vlines.color,
                vlines.width,
                vlines.style,
                display,
            )?;
        }
    }

    // Draw horizontal grid lines
    if let Some(ref hlines) = config.horizontal_lines
        && hlines.count > 0
    {
        let spacing = plot_area.size.height / (hlines.count + 1) as u32;

        for i in 1..=hlines.count {
            let y = plot_area.top_left.y + (spacing * i as u32) as i32;
            let start = Point::new(plot_area.top_left.x, y);
            let end = Point::new(plot_area.top_left.x + plot_area.size.width as i32, y);

            draw_line(
                start,
                end,
                hlines.color,
                hlines.width,
                hlines.style,
                display,
            )?;
        }
    }

    Ok(())
}

/// Draw a single line with specified style
fn draw_line<D: DrawTarget<Color = Rgb565>>(
    start: Point,
    end: Point,
    color: Rgb565,
    width: u32,
    style: LineStyle,
    display: &mut D,
) -> Result<(), D::Error> {
    match style {
        LineStyle::Solid => {
            Line::new(start, end)
                .into_styled(PrimitiveStyle::with_stroke(color, width))
                .draw(display)?;
        }
        LineStyle::Dashed {
            dash_length,
            gap_length,
        } => {
            draw_dashed_line(start, end, color, width, dash_length, gap_length, display)?;
        }
    }

    Ok(())
}

/// Simple square root approximation using Newton-Raphson method
fn sqrt_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }

    // Initial guess
    let mut guess = x / 2.0;

    // Newton-Raphson iterations (3 iterations give good accuracy)
    for _ in 0..3 {
        guess = (guess + x / guess) / 2.0;
    }

    guess
}

/// Draw a dashed line by rendering individual dash segments
fn draw_dashed_line<D: DrawTarget<Color = Rgb565>>(
    start: Point,
    end: Point,
    color: Rgb565,
    width: u32,
    dash_length: u32,
    gap_length: u32,
    display: &mut D,
) -> Result<(), D::Error> {
    let dx = (end.x - start.x) as f32;
    let dy = (end.y - start.y) as f32;
    let total_length = sqrt_approx(dx * dx + dy * dy);

    if total_length < 0.1 {
        return Ok(());
    }

    let pattern_length = (dash_length + gap_length) as f32;
    let mut distance = 0.0;

    let line_style = PrimitiveStyle::with_stroke(color, width);

    while distance < total_length {
        let t_start = distance / total_length;
        let t_end = ((distance + dash_length as f32).min(total_length)) / total_length;

        let dash_start = Point::new(
            start.x + (dx * t_start) as i32,
            start.y + (dy * t_start) as i32,
        );

        let dash_end = Point::new(start.x + (dx * t_end) as i32, start.y + (dy * t_end) as i32);

        Line::new(dash_start, dash_end)
            .into_styled(line_style)
            .draw(display)?;

        distance += pattern_length;
    }

    Ok(())
}
