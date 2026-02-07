//! Interpolation algorithms for rendering smooth curves
//!
//! Provides linear and Catmull-Rom spline interpolation for data series.
//! All functions use embedded-graphics Line primitives for drawing.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle};

use super::constants::DEFAULT_SMOOTH_SUBDIVISIONS;
use super::series::{DataPoint, SeriesStyle};
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
