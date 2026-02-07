//! Constants for graph rendering
//!
//! All magic numbers are defined here with descriptive names and units.
//! This ensures maintainability and follows the project's code standards.

use crate::ui::styling::DARK_GRAY;
use embedded_graphics::pixelcolor::Rgb565;

/// Number of subdivisions per segment for smooth curve interpolation
///
/// Higher values produce smoother curves but increase rendering time.
/// Value of 5 provides good balance between quality and performance.
pub const DEFAULT_SMOOTH_SUBDIVISIONS: usize = 5;

/// Default number of vertical grid lines
pub const DEFAULT_VERTICAL_GRID_COUNT: usize = 5;

/// Default grid line color (subtle dark gray)
pub const DEFAULT_GRID_COLOR: Rgb565 = DARK_GRAY;

/// Default grid line width in pixels
pub const DEFAULT_GRID_LINE_WIDTH_PX: u32 = 1;

/// Default number of X-axis labels
pub const DEFAULT_X_AXIS_LABEL_COUNT: usize = 3;

/// Maximum length of formatted axis labels (characters)
pub const MAX_AXIS_LABEL_LENGTH: usize = 16;

/// Default viewport padding for top edge in pixels
pub const DEFAULT_VIEWPORT_PADDING_TOP_PX: u32 = 5;

/// Default viewport padding for right edge in pixels
pub const DEFAULT_VIEWPORT_PADDING_RIGHT_PX: u32 = 10;

/// Default viewport padding for bottom edge in pixels
pub const DEFAULT_VIEWPORT_PADDING_BOTTOM_PX: u32 = 20;

/// Default viewport padding for left edge in pixels
pub const DEFAULT_VIEWPORT_PADDING_LEFT_PX: u32 = 5;

/// Minimum data range for auto-scaling (prevents division by zero)
pub const MIN_DATA_RANGE: f32 = 0.001;

/// Default Catmull-Rom spline tension
///
/// 0.0 = loose curve, 0.5 = balanced, 1.0 = tight through points
pub const DEFAULT_CURVE_TENSION: f32 = 0.5;

/// Margin factor for auto-scaling bounds (10% padding)
pub const AUTO_SCALE_MARGIN_FACTOR: f32 = 0.1;

/// Default series line width in pixels
pub const DEFAULT_SERIES_LINE_WIDTH_PX: u32 = 2;
