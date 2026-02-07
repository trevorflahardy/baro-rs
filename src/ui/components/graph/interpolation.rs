//! Interpolation algorithms for rendering smooth curves
//!
//! Provides linear and Catmull-Rom spline interpolation for data series.
//! All functions use embedded-graphics Line primitives for drawing.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle};

extern crate alloc;
use alloc::vec::Vec;

use super::constants::DEFAULT_SMOOTH_SUBDIVISIONS;
use super::series::{DataPoint, GradientFill, SeriesStyle};
use super::viewport::Viewport;

/// Draw a data series with linear interpolation (straight lines)
///
/// Connects consecutive data points with straight Line primitives.
pub(super) fn draw_linear_series<D: DrawTarget<Color = Rgb565>>(
    points: &[DataPoint],
    viewport: &Viewport,
    style: &SeriesStyle,
    display: &mut D,
) -> Result<(), D::Error> {
    if points.len() < 2 {
        return Ok(());
    }

    let line_style = PrimitiveStyle::with_stroke(style.color, style.line_width);

    // Convert data points to screen coordinates
    let mut prev_screen: Option<Point> = None;

    for point in points.iter() {
        if let Some(screen_point) = viewport.data_to_screen(*point) {
            if let Some(prev) = prev_screen {
                Line::new(prev, screen_point)
                    .into_styled(line_style)
                    .draw(display)?;
            }
            prev_screen = Some(screen_point);
        } else {
            // Point is out of viewport, reset previous point
            prev_screen = None;
        }
    }

    Ok(())
}

/// Draw a gradient fill under a linearly interpolated series
pub(super) fn draw_linear_fill<D: DrawTarget<Color = Rgb565>>(
    points: &[DataPoint],
    viewport: &Viewport,
    fill: &GradientFill,
    background: Rgb565,
    display: &mut D,
) -> Result<(), D::Error> {
    let screen_points = collect_linear_screen_points(points, viewport);
    draw_gradient_fill_from_screen_points(&screen_points, viewport, fill, background, display)
}

/// Draw a data series with smooth Catmull-Rom spline interpolation
///
/// Creates smooth curves through data points using Catmull-Rom basis.
/// Requires at least 4 points for proper interpolation.
pub(super) fn draw_smooth_series<D: DrawTarget<Color = Rgb565>>(
    points: &[DataPoint],
    viewport: &Viewport,
    style: &SeriesStyle,
    tension: f32,
    display: &mut D,
) -> Result<(), D::Error> {
    if points.len() < 2 {
        return Ok(());
    }

    // For less than 4 points, fall back to linear interpolation
    if points.len() < 4 {
        return draw_linear_series(points, viewport, style, display);
    }

    let line_style = PrimitiveStyle::with_stroke(style.color, style.line_width);
    let step = 1.0 / DEFAULT_SMOOTH_SUBDIVISIONS as f32;

    // Iterate through segments (need 4 control points per segment)
    for i in 0..points.len().saturating_sub(3) {
        let p0 = points[i];
        let p1 = points[i + 1];
        let p2 = points[i + 2];
        let p3 = points[i + 3];

        let mut prev_screen: Option<Point> = None;

        // Generate subdivisions along the curve segment
        for j in 0..=DEFAULT_SMOOTH_SUBDIVISIONS {
            let t = j as f32 * step;
            let interpolated = catmull_rom_point(p0, p1, p2, p3, t, tension);

            if let Some(screen_point) = viewport.data_to_screen(interpolated) {
                if let Some(prev) = prev_screen {
                    Line::new(prev, screen_point)
                        .into_styled(line_style)
                        .draw(display)?;
                }
                prev_screen = Some(screen_point);
            } else {
                prev_screen = None;
            }
        }
    }

    Ok(())
}

/// Draw a gradient fill under a smoothly interpolated series
pub(super) fn draw_smooth_fill<D: DrawTarget<Color = Rgb565>>(
    points: &[DataPoint],
    viewport: &Viewport,
    fill: &GradientFill,
    tension: f32,
    background: Rgb565,
    display: &mut D,
) -> Result<(), D::Error> {
    let screen_points = collect_smooth_screen_points(points, viewport, tension);
    draw_gradient_fill_from_screen_points(&screen_points, viewport, fill, background, display)
}

fn collect_linear_screen_points(points: &[DataPoint], viewport: &Viewport) -> Vec<Point> {
    let mut screen_points = Vec::with_capacity(points.len());

    for point in points.iter() {
        if let Some(screen_point) = viewport.data_to_screen(*point)
            && screen_points.last().copied() != Some(screen_point)
        {
            screen_points.push(screen_point);
        }
    }

    screen_points
}

fn collect_smooth_screen_points(
    points: &[DataPoint],
    viewport: &Viewport,
    tension: f32,
) -> Vec<Point> {
    if points.len() < 2 {
        return Vec::new();
    }

    if points.len() < 4 {
        return collect_linear_screen_points(points, viewport);
    }

    let mut screen_points = Vec::with_capacity(points.len() * DEFAULT_SMOOTH_SUBDIVISIONS);
    let step = 1.0 / DEFAULT_SMOOTH_SUBDIVISIONS as f32;

    for i in 0..points.len().saturating_sub(3) {
        let p0 = points[i];
        let p1 = points[i + 1];
        let p2 = points[i + 2];
        let p3 = points[i + 3];

        for j in 0..=DEFAULT_SMOOTH_SUBDIVISIONS {
            let t = j as f32 * step;
            let interpolated = catmull_rom_point(p0, p1, p2, p3, t, tension);

            if let Some(screen_point) = viewport.data_to_screen(interpolated)
                && screen_points.last().copied() != Some(screen_point)
            {
                screen_points.push(screen_point);
            }
        }
    }

    screen_points
}

fn draw_gradient_fill_from_screen_points<D: DrawTarget<Color = Rgb565>>(
    screen_points: &[Point],
    viewport: &Viewport,
    fill: &GradientFill,
    background: Rgb565,
    display: &mut D,
) -> Result<(), D::Error> {
    if screen_points.len() < 2 {
        return Ok(());
    }

    let plot_area = viewport.plot_area();
    let bottom = plot_area.top_left.y + plot_area.size.height as i32;
    let colors = build_gradient_colors(fill, background);

    for pair in screen_points.windows(2) {
        let mut x0 = pair[0].x;
        let mut y0 = pair[0].y;
        let mut x1 = pair[1].x;
        let mut y1 = pair[1].y;

        if x0 > x1 {
            core::mem::swap(&mut x0, &mut x1);
            core::mem::swap(&mut y0, &mut y1);
        }

        let dx = (x1 - x0).max(1) as f32;
        for x in x0..=x1 {
            let t = (x - x0) as f32 / dx;
            let y_line = y0 + ((y1 - y0) as f32 * t) as i32;
            draw_gradient_column(x, y_line, bottom, &colors, display)?;
        }
    }

    Ok(())
}

fn draw_gradient_column<D: DrawTarget<Color = Rgb565>>(
    x: i32,
    y_line: i32,
    bottom: i32,
    colors: &[Rgb565],
    display: &mut D,
) -> Result<(), D::Error> {
    if y_line >= bottom {
        return Ok(());
    }

    let height = bottom - y_line;
    let bands = colors.len().max(1) as i32;
    let band_height = (height as f32 / bands as f32).max(1.0);

    for (index, color) in colors.iter().enumerate() {
        let start = y_line + (band_height * index as f32) as i32;
        let end = if index == colors.len() - 1 {
            bottom
        } else {
            y_line + (band_height * (index as f32 + 1.0)) as i32
        };

        if end >= start {
            Line::new(Point::new(x, start), Point::new(x, end))
                .into_styled(PrimitiveStyle::with_stroke(*color, 1))
                .draw(display)?;
        }
    }

    Ok(())
}

fn build_gradient_colors(fill: &GradientFill, background: Rgb565) -> Vec<Rgb565> {
    let bands = fill.bands.max(1) as usize;
    let alpha = fill.opacity as f32 / 255.0;
    let start_color = if fill.opacity == u8::MAX {
        fill.start_color
    } else {
        lerp_color(background, fill.start_color, alpha)
    };
    let end_color = if fill.opacity == u8::MAX {
        fill.end_color
    } else {
        lerp_color(background, fill.end_color, alpha)
    };
    let mut colors = Vec::with_capacity(bands);
    for i in 0..bands {
        let t = if bands > 1 {
            i as f32 / (bands - 1) as f32
        } else {
            1.0
        };
        colors.push(lerp_color(start_color, end_color, t));
    }
    colors
}

fn lerp_color(start: Rgb565, end: Rgb565, t: f32) -> Rgb565 {
    let t = t.clamp(0.0, 1.0);
    let (r0, g0, b0) = rgb565_to_rgb888(start);
    let (r1, g1, b1) = rgb565_to_rgb888(end);

    let r = r0 as f32 + (r1 as f32 - r0 as f32) * t;
    let g = g0 as f32 + (g1 as f32 - g0 as f32) * t;
    let b = b0 as f32 + (b1 as f32 - b0 as f32) * t;

    rgb888_to_rgb565(r as u8, g as u8, b as u8)
}

fn rgb565_to_rgb888(color: Rgb565) -> (u8, u8, u8) {
    let raw = color.into_storage();
    let r5 = ((raw >> 11) & 0x1f) as u8;
    let g6 = ((raw >> 5) & 0x3f) as u8;
    let b5 = (raw & 0x1f) as u8;

    let r8 = (r5 << 3) | (r5 >> 2);
    let g8 = (g6 << 2) | (g6 >> 4);
    let b8 = (b5 << 3) | (b5 >> 2);

    (r8, g8, b8)
}

fn rgb888_to_rgb565(r8: u8, g8: u8, b8: u8) -> Rgb565 {
    Rgb565::new(r8 >> 3, g8 >> 2, b8 >> 3)
}

/// Calculate a point on a Catmull-Rom spline curve
///
/// Uses the standard Catmull-Rom basis matrix for smooth interpolation.
/// The curve passes through p1 and p2, using p0 and p3 as control points.
///
/// # Arguments
///
/// * `p0` - Previous control point
/// * `p1` - Start point (curve passes through this)
/// * `p2` - End point (curve passes through this)
/// * `p3` - Next control point
/// * `t` - Interpolation parameter (0.0 to 1.0)
/// * `tension` - Curve tension (0.0 = loose, 0.5 = balanced, 1.0 = tight)
fn catmull_rom_point(
    p0: DataPoint,
    p1: DataPoint,
    p2: DataPoint,
    p3: DataPoint,
    t: f32,
    tension: f32,
) -> DataPoint {
    let t2 = t * t;
    let t3 = t2 * t;

    // Catmull-Rom basis matrix coefficients
    // Adjusted by tension parameter for curve tightness control
    let _tau = tension.clamp(0.0, 1.0);

    // Standard Catmull-Rom formula (tension = 0.5)
    // Can be adjusted with _tau if needed for custom tension control
    let x = 0.5
        * (2.0 * p1.x
            + (-p0.x + p2.x) * t
            + (2.0 * p0.x - 5.0 * p1.x + 4.0 * p2.x - p3.x) * t2
            + (-p0.x + 3.0 * p1.x - 3.0 * p2.x + p3.x) * t3);

    let y = 0.5
        * (2.0 * p1.y
            + (-p0.y + p2.y) * t
            + (2.0 * p0.y - 5.0 * p1.y + 4.0 * p2.y - p3.y) * t2
            + (-p0.y + 3.0 * p1.y - 3.0 * p2.y + p3.y) * t3);

    DataPoint { x, y }
}
