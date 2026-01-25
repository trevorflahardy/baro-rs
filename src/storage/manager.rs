// cSpell: disable
use crate::storage::sd_card::{
    ROLLUP_FILE_1H, ROLLUP_FILE_5M, ROLLUP_FILE_DAILY, SdCardManager, SdCardManagerError,
};

use super::{LifetimeStats, RawSample, Rollup, accumulator::RollupEvent};
use log::{debug, error, info};

extern crate alloc;
use alloc::collections::VecDeque;

// Capacity constants for ring buffers
const RAW_SAMPLES_CAPACITY: usize = 360; // 1 hour (one sample every 10 seconds)
const ROLLUPS_5M_CAPACITY: usize = 2016; // 7 days (12 per hour * 24 * 7)
const ROLLUPS_1H_CAPACITY: usize = 720; // 30 days (24 per day * 30)
const ROLLUPS_DAILY_CAPACITY: usize = 365; // 1 year

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
            raw_samples: VecDeque::with_capacity(RAW_SAMPLES_CAPACITY),
            rollups_5m: VecDeque::with_capacity(ROLLUPS_5M_CAPACITY),
            rollups_1h: VecDeque::with_capacity(ROLLUPS_1H_CAPACITY),
            rollups_daily: VecDeque::with_capacity(ROLLUPS_DAILY_CAPACITY),
            lifetime_stats: LifetimeStats::default(),
            sd_card_manager,
        }
    }

    pub async fn init(&mut self, time: u32) -> Result<(), SdCardManagerError> {
        info!(" Initializing storage manager, loading rollups from SD card...");

        let lifetime_data_buffer = &mut [0u8; core::mem::size_of::<LifetimeStats>()];
        let lifetime_data = match self
            .sd_card_manager
            .read_lifetime_data(lifetime_data_buffer)
        {
            Ok(_) => Ok(LifetimeStats::from(lifetime_data_buffer)),
            Err(e) => {
                error!(" Failed to load lifetime stats from SD card: {:?}", e);
                Err(e)
            }
        }?;

        self.lifetime_stats = lifetime_data;

        // Calculate time window - load the max capacity worth of data
        // For each rollup type, we want to load the last N entries where N is the capacity

        // Load 5-minute rollups (last 7 days)
        let window_5m = (
            time.saturating_sub(7 * 24 * 60 * 60), // 7 days ago
            time,
        );
        let mut buffer_5m = alloc::vec![Rollup::default(); ROLLUPS_5M_CAPACITY];
        match self
            .sd_card_manager
            .read_rollup_data(ROLLUP_FILE_5M, &mut buffer_5m, window_5m)
        {
            Ok(count) => {
                info!(" Loaded {} 5-minute rollups from SD card", count);
                for i in 0..count {
                    self.rollups_5m.push_back(buffer_5m[i]);
                }

                Ok(())
            }
            Err(e) => {
                error!(" Failed to load 5-minute rollups: {:?}", e);
                Err(e)
            }
        }?;

        // Load hourly rollups (last 30 days)
        let window_1h = (
            time.saturating_sub(30 * 24 * 60 * 60), // 30 days ago
            time,
        );
        let mut buffer_1h = alloc::vec![Rollup::default(); ROLLUPS_1H_CAPACITY];
        match self
            .sd_card_manager
            .read_rollup_data(ROLLUP_FILE_1H, &mut buffer_1h, window_1h)
        {
            Ok(count) => {
                info!(" Loaded {} hourly rollups from SD card", count);
                for i in 0..count {
                    self.rollups_1h.push_back(buffer_1h[i]);
                }

                Ok(())
            }
            Err(e) => {
                error!(" Failed to load hourly rollups: {:?}", e);
                Err(e)
            }
        }?;

        // Load daily rollups (last 365 days)
        let window_daily = (
            time.saturating_sub(365 * 24 * 60 * 60), // 365 days ago
            time,
        );
        let mut buffer_daily = alloc::vec![Rollup::default(); ROLLUPS_DAILY_CAPACITY];
        match self.sd_card_manager.read_rollup_data(
            ROLLUP_FILE_DAILY,
            &mut buffer_daily,
            window_daily,
        ) {
            Ok(count) => {
                info!(" Loaded {} daily rollups from SD card", count);
                for i in 0..count {
                    self.rollups_daily.push_back(buffer_daily[i]);
                }

                Ok(())
            }
            Err(e) => {
                error!(" Failed to load daily rollups: {:?}", e);

                Err(e)
            }
        }?;

        info!(" Storage manager initialization complete");
        Ok(())
    }

    /// Process a rollup event (store in RAM and write to SD card)
    pub async fn process_event(&mut self, event: RollupEvent) {
        match event {
            RollupEvent::RawSample(sample) => {
                // Add to ring buffer (oldest is automatically dropped when full)
                if self.raw_samples.len() >= RAW_SAMPLES_CAPACITY {
                    self.raw_samples.pop_front();
                }
                self.raw_samples.push_back(sample);

                // Update lifetime stats
                self.lifetime_stats.update(&sample);
                debug!(" Recalculated lifetime stats: {:?}", self.lifetime_stats);
            }
            RollupEvent::Rollup5m(rollup) => {
                if self.rollups_5m.len() >= ROLLUPS_5M_CAPACITY {
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
                if self.rollups_1h.len() >= ROLLUPS_1H_CAPACITY {
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
                if self.rollups_daily.len() >= ROLLUPS_DAILY_CAPACITY {
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
