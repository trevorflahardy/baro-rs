//! Firmware-specific application state extensions
//!
//! Re-exports the hardware-independent app state from `baro_core` and
//! adds ESP32-specific hardware initialization and sensor state management.

mod hardware;
mod sensors_state;

pub use hardware::*;
pub use sensors_state::*;

// Re-export all shared app state types from baro-core
pub use baro_core::app_state::*;
