//! ESP32-S3 firmware-specific modules for baro-rs
//!
//! This crate contains hardware-specific code that cannot compile on desktop
//! targets: GPIO register manipulation, ESP32 peripheral initialization,
//! WiFi credential management, and concrete sensor state management.

#![no_std]

extern crate alloc;

pub mod app_state;
pub mod dual_mode_pin;
pub mod wifi_secrets;
