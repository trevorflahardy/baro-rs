//! Hardware-independent core library for baro-rs
//!
//! This crate contains all platform-agnostic logic for the baro environmental
//! instrumentation device: UI rendering, page management, sensor trait
//! definitions, storage/rollup logic, and display management.
//!
//! It is `#![no_std]` with `extern crate alloc` so it compiles on both
//! embedded targets (ESP32-S3) and desktop hosts (for the simulator and tests).

#![no_std]

extern crate alloc;

pub mod app_state;
pub mod async_i2c_bus;
pub mod config;
pub mod display_manager;
pub mod framebuffer;
pub mod metrics;
pub mod pages;
pub mod sensors;
pub mod storage;
pub mod ui;
pub mod widgets;
