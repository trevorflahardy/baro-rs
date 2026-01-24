// cSpell: disable
use crate::storage::sd_card::{ROLLUP_FILE_1H, ROLLUP_FILE_5M, ROLLUP_FILE_DAILY, SdCardManager};

use super::{LifetimeStats, RawSample, Rollup, accumulator::RollupEvent};
use log::{debug, error, info};

extern crate alloc;
use alloc::collections::VecDeque;

/// Storage manager that maintains ring buffers in RAM and handles SD card persistence
///
/// This task subscribes to rollup events and:
/// 1. Stores data in RAM ring buffers for fast UI access
/// 2. Writes data to SD card for long-term persistence
///
/// ## Memory Usage
///
/// - Raw samples: 360 × 96 bytes = 34.5 KB (1 hour)
/// - 5-min rollups: 2,016 × 256 bytes = 516 KB (7 days)
/// - Hourly rollups: 720 × 256 bytes = 180 KB (30 days)
/// - Daily rollups: 365 × 256 bytes = 91 KB (1 year)
/// - **Total: ~822 KB** (allocated from PSRAM heap, not static memory)
pub struct StorageManager<S, D, T>
where
    S: embedded_hal::spi::SpiDevice<u8>,
    D: embedded_hal::delay::DelayNs,
    T: embedded_sdmmc::TimeSource,
{
    /// Ring buffer for raw samples (last 1 hour for 5m and 1h graphs)
    raw_samples: VecDeque<RawSample>,
    /// Ring buffer for 5-minute rollups (last 7 days for 24h and 7d graphs)
    rollups_5m: VecDeque<Rollup>,
    /// Ring buffer for hourly rollups (last 30 days for 1-month graphs)
    rollups_1h: VecDeque<Rollup>,
    /// Ring buffer for daily rollups (last 1 year for all-time graphs)
    rollups_daily: VecDeque<Rollup>,
    /// Lifetime statistics
    lifetime_stats: LifetimeStats,
    /// SD Card storage
    sd_card_manager: SdCardManager<S, D, T>,
}

impl<S, D, T> StorageManager<S, D, T>
where
    S: embedded_hal::spi::SpiDevice<u8>,
    D: embedded_hal::delay::DelayNs,
    T: embedded_sdmmc::TimeSource,
{
    pub fn new(sd_card_manager: SdCardManager<S, D, T>) -> Self {
        Self {
            raw_samples: VecDeque::with_capacity(360),
            rollups_5m: VecDeque::with_capacity(2016),
            rollups_1h: VecDeque::with_capacity(720),
            rollups_daily: VecDeque::with_capacity(365),
            lifetime_stats: LifetimeStats::default(),
            sd_card_manager,
        }
    }

    pub fn init(&mut self) {
        // Load lifetime stats and propagate to ring buffers.
    }

    /// Process a rollup event (store in RAM and write to SD card)
    pub async fn process_event(&mut self, event: RollupEvent) {
        match event {
            RollupEvent::RawSample(sample) => {
                // Add to ring buffer (oldest is automatically dropped when full)
                if self.raw_samples.len() >= 360 {
                    self.raw_samples.pop_front();
                }
                self.raw_samples.push_back(sample);

                // Update lifetime stats
                self.lifetime_stats.update(&sample);
                debug!(" Recalculated lifetime stats: {:?}", self.lifetime_stats);
            }
            RollupEvent::Rollup5m(rollup) => {
                if self.rollups_5m.len() >= 2016 {
                    self.rollups_5m.pop_front();
                }
                self.rollups_5m.push_back(rollup);

                // Append to rollup_5m.bin on SD card
                if let Err(e) = self
                    .sd_card_manager
                    .append_rollup_data(ROLLUP_FILE_5M, &rollup)
                {
                    error!(" Failed to write 5m rollup to SD: {:?}", e);
                } else {
                    info!(" Updating rollup file 5m.");
                }
            }
            RollupEvent::Rollup1h(rollup) => {
                if self.rollups_1h.len() >= 720 {
                    self.rollups_1h.pop_front();
                }
                self.rollups_1h.push_back(rollup);

                // Append to rollup_1h.bin on SD card
                if let Err(e) = self
                    .sd_card_manager
                    .append_rollup_data(ROLLUP_FILE_1H, &rollup)
                {
                    error!(" Failed to write 1h rollup to SD: {:?}", e);
                } else {
                    info!(" Updating rollup file 1h.");
                }
            }
            RollupEvent::RollupDaily(rollup) => {
                if self.rollups_daily.len() >= 365 {
                    self.rollups_daily.pop_front();
                }
                self.rollups_daily.push_back(rollup);

                // Append to rollup_daily.bin on SD card
                if let Err(e) = self
                    .sd_card_manager
                    .append_rollup_data(ROLLUP_FILE_DAILY, &rollup)
                {
                    error!(" Failed to write daily rollup to SD: {:?}", e);
                } else {
                    info!(" Updating rollup file 24h.");
                }
            }
        }
    }

    // Get raw samples for graph rendering (non-consuming, read-only access)
    pub fn get_raw_samples(&self) -> &VecDeque<RawSample> {
        &self.raw_samples
    }

    /// Get 5-minute rollups for graph rendering
    pub fn get_5m_rollups(&self) -> &VecDeque<Rollup> {
        &self.rollups_5m
    }

    /// Get hourly rollups for graph rendering
    pub fn get_1h_rollups(&self) -> &VecDeque<Rollup> {
        &self.rollups_1h
    }

    /// Get daily rollups for graph rendering
    pub fn get_daily_rollups(&self) -> &VecDeque<Rollup> {
        &self.rollups_daily
    }

    /// Get lifetime statistics
    pub fn get_lifetime_stats(&self) -> &LifetimeStats {
        &self.lifetime_stats
    }
}
