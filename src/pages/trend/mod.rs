//! Trend page for displaying time-series sensor data with graphs
//!
//! This page provides a generic interface for visualizing any sensor's data
//! over configurable time windows, with quality assessment and statistics.

mod constants;
mod data;
mod page;
mod stats;

pub use page::TrendPage;
