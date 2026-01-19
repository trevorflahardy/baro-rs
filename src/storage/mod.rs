//! Storage backend for time-series sensor data.
//!
//! This module implements the storage design from STORAGE.md, providing:
//! - Raw sample storage (24-hour ring buffer)
//! - Multi-tier rollup storage (5m, 1h, daily)
//! - Lifetime statistics
//!
//! All structures use fixed-size binary representations optimized for SD card storage.

mod rollup;

pub use rollup::{LifetimeStats, RawSample, Rollup, MAX_SENSORS};
