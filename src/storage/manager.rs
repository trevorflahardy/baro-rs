use crate::storage::sd_card::SdCardManager;

use super::{LifetimeStats, RawSample, Rollup, accumulator::RollupEvent};

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
/// - **Total: ~822 KB**
pub struct StorageManager<S, D, T>
where
    S: embedded_hal::spi::SpiDevice<u8>,
    D: embedded_hal::delay::DelayNs,
    T: embedded_sdmmc::TimeSource,
{
    /// Ring buffer for raw samples (last 1 hour for 5m and 1h graphs)
    raw_samples: heapless::Deque<RawSample, 360>,
    /// Ring buffer for 5-minute rollups (last 7 days for 24h and 7d graphs)
    rollups_5m: heapless::Deque<Rollup, 2016>,
    /// Ring buffer for hourly rollups (last 30 days for 1-month graphs)
    rollups_1h: heapless::Deque<Rollup, 720>,
    /// Ring buffer for daily rollups (last 1 year for all-time graphs)
    rollups_daily: heapless::Deque<Rollup, 365>,
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
    fn new(sd_card_manager: SdCardManager<S, D, T>) -> Self {
        Self {
            raw_samples: heapless::Deque::new(),
            rollups_5m: heapless::Deque::new(),
            rollups_1h: heapless::Deque::new(),
            rollups_daily: heapless::Deque::new(),
            lifetime_stats: LifetimeStats::default(),
            sd_card_manager,
        }
    }

    fn init(&mut self) -> () {
        // Load lifetime stats and propagate to ring buffers.
    }

    /// Process a rollup event (store in RAM and write to SD card)
    pub async fn process_event(&mut self, event: RollupEvent) {
        match event {
            RollupEvent::RawSample(sample) => {
                // Add to ring buffer (oldest is automatically dropped when full)
                if self.raw_samples.push_back(sample).is_err() {
                    // Buffer full, remove oldest
                    self.raw_samples.pop_front();
                    let _ = self.raw_samples.push_back(sample);
                }

                // Update lifetime stats
                self.lifetime_stats.update(&sample);
            }
            RollupEvent::Rollup5m(rollup) => {
                if self.rollups_5m.push_back(rollup).is_err() {
                    self.rollups_5m.pop_front();
                    let _ = self.rollups_5m.push_back(rollup);
                }

                // TODO: Append to rollup_5m.bin on SD card
            }
            RollupEvent::Rollup1h(rollup) => {
                if self.rollups_1h.push_back(rollup).is_err() {
                    self.rollups_1h.pop_front();
                    let _ = self.rollups_1h.push_back(rollup);
                }

                // TODO: Append to rollup_1h.bin on SD card
            }
            RollupEvent::RollupDaily(rollup) => {
                if self.rollups_daily.push_back(rollup).is_err() {
                    self.rollups_daily.pop_front();
                    let _ = self.rollups_daily.push_back(rollup);
                }

                // TODO: Append to rollup_daily.bin on SD card
            }
        }
    }

    // Get raw samples for graph rendering (non-consuming, read-only access)
    pub fn get_raw_samples(&self) -> &heapless::Deque<RawSample, 360> {
        &self.raw_samples
    }

    /// Get 5-minute rollups for graph rendering
    pub fn get_5m_rollups(&self) -> &heapless::Deque<Rollup, 2016> {
        &self.rollups_5m
    }

    /// Get hourly rollups for graph rendering
    pub fn get_1h_rollups(&self) -> &heapless::Deque<Rollup, 720> {
        &self.rollups_1h
    }

    /// Get daily rollups for graph rendering
    pub fn get_daily_rollups(&self) -> &heapless::Deque<Rollup, 365> {
        &self.rollups_daily
    }

    /// Get lifetime statistics
    pub fn get_lifetime_stats(&self) -> &LifetimeStats {
        &self.lifetime_stats
    }
}
