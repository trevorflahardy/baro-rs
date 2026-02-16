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

/// Maximum data points for the largest time window (1 hour at 10s interval)
pub(super) const MAX_DATA_POINTS: usize = 360;

/// Window growth chunk size for auto-zoom (seconds)
pub(super) const WINDOW_GROWTH_CHUNK_SECS: u32 = 300;

/// Gradient fill opacity (80% transparent)
pub(super) const GRADIENT_FILL_OPACITY: u8 = 51;

/// Data point for graphing: (timestamp, value)
pub(super) type DataPoint = (u32, i32);

// ============================================================================
// Layout Dimensions
// ============================================================================

/// Height of the trend page header section in pixels
pub(super) const HEADER_HEIGHT_PX: u32 = 40;

/// Height of the statistics bar at the bottom in pixels
pub(super) const STATS_HEIGHT_PX: u32 = 55;

// ============================================================================
// Header Layout
// ============================================================================

/// Left padding for header title text in pixels
pub(super) const HEADER_TITLE_PADDING_LEFT_PX: i32 = 5;

/// Horizontal padding around quality indicator text in pixels
pub(super) const QUALITY_INDICATOR_TEXT_PADDING_PX: u32 = 20;

/// Height of the quality indicator pill in pixels
pub(super) const QUALITY_INDICATOR_HEIGHT_PX: u32 = 20;

/// Right margin for quality indicator from header edge in pixels
pub(super) const QUALITY_INDICATOR_MARGIN_RIGHT_PX: i32 = 5;

/// Border width of the quality indicator pill in pixels
pub(super) const QUALITY_INDICATOR_BORDER_WIDTH_PX: u32 = 2;

/// Corner radius of the quality indicator pill in pixels
pub(super) const QUALITY_INDICATOR_CORNER_RADIUS_PX: u32 = 10;

/// Vertical padding inside quality indicator in pixels
pub(super) const QUALITY_INDICATOR_PADDING_VERTICAL_PX: u32 = 2;

/// Horizontal padding inside quality indicator in pixels
pub(super) const QUALITY_INDICATOR_PADDING_HORIZONTAL_PX: u32 = 4;

// ============================================================================
// Graph Styling
// ============================================================================

/// Line width for the main data series in pixels
pub(super) const SERIES_LINE_WIDTH_PX: u32 = 3;

/// Height of the gradient fill below the data line in pixels
pub(super) const GRADIENT_FILL_HEIGHT_PX: u8 = 12;

// ============================================================================
// Current Value Overlay
// ============================================================================

/// Horizontal offset for current value display from graph right edge in pixels
pub(super) const CURRENT_VALUE_OFFSET_X_PX: u32 = 10;

/// Vertical offset for current value display from graph top in pixels
pub(super) const CURRENT_VALUE_OFFSET_Y_PX: u32 = 30;
