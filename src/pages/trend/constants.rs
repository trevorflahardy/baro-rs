//! Constants for the trend page module

use embedded_graphics::pixelcolor::Rgb565;

// Color constants from styling
// RGB565 format: R(5 bits), G(6 bits), B(5 bits)
// Convert from 8-bit RGB: R>>3, G>>2, B>>3
pub(super) const COLOR_FOREGROUND: Rgb565 = Rgb565::new(26 >> 3, 32 >> 2, 33 >> 3);
pub(super) const _COLOR_STROKE: Rgb565 = Rgb565::new(43 >> 3, 55 >> 2, 57 >> 3);
pub(super) const LIGHT_GRAY: Rgb565 = Rgb565::new(21, 42, 21);

/// Very faint gray for grid lines (less visible than LIGHT_GRAY)
pub(super) const FAINT_GRAY: Rgb565 = Rgb565::new(10, 20, 10);

/// Maximum data points for the largest time window (limited by embedded_charts)
pub(super) const MAX_DATA_POINTS: usize = 256;

/// Data point for graphing: (timestamp, value)
pub(super) type DataPoint = (u32, i32);
