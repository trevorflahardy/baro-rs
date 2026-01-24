use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, pubsub::Publisher};

extern crate alloc;
use alloc::vec::Vec;

use super::{MAX_SENSORS, RawSample, Rollup};

/// Channel capacity for pub-sub events
/// Set to 8 to handle bursts without blocking the sensor task
pub const EVENT_CHANNEL_CAPACITY: usize = 8;

/// Number of subscribers that can listen to rollup events
/// - Subscriber 0: StorageManager (SD card writer + RAM buffers)
/// - Subscriber 1: UI rendering task
pub const EVENT_SUBSCRIBERS: usize = 2;

/// Number of publishers (just the sensor task)
pub const EVENT_PUBLISHERS: usize = 1;

/// Events published by the accumulator to notify subscribers of new data
#[derive(Debug, Clone, Copy)]
pub enum RollupEvent {
    /// A new raw sample was recorded
    RawSample(RawSample),
    /// A 5-minute rollup was completed
    Rollup5m(Rollup),
    /// An hourly rollup was completed
    Rollup1h(Rollup),
    /// A daily rollup was completed
    RollupDaily(Rollup),
}

/// In-memory accumulator for generating rollups from raw samples
///
/// This struct maintains rolling buffers of samples and generates higher-tier
/// rollups when accumulation thresholds are met. It publishes events to a
/// PubSubChannel for consumption by storage and UI tasks.
///
/// ## Accumulation Windows
///
/// - **5-minute rollups**: 30 raw samples (10s × 30 = 5 minutes)
/// - **Hourly rollups**: 12 five-minute rollups (5m × 12 = 1 hour)
/// - **Daily rollups**: 24 hourly rollups (1h × 24 = 24 hours)
///
/// ## Usage
///
/// ```rust,ignore
/// // Create a global pub-sub channel
/// static ROLLUP_CHANNEL: PubSubChannel<...> = PubSubChannel::new();
///
/// let publisher = ROLLUP_CHANNEL.publisher().unwrap();
/// let mut accumulator = RollupAccumulator::new(publisher);
///
/// // Add samples every 10 seconds
/// accumulator.add_sample(timestamp, &sensor_values).await;
/// ```
pub struct RollupAccumulator<'a> {
    /// Buffer for raw samples (up to 30 for 5-minute rollup)
    raw_buffer: Vec<RawSample>,
    /// Buffer for 5-minute rollups (up to 12 for hourly rollup)
    rollup_5m_buffer: Vec<Rollup>,
    /// Buffer for hourly rollups (up to 24 for daily rollup)
    rollup_1h_buffer: Vec<Rollup>,
    /// Publisher for sending rollup events
    publisher: Publisher<
        'a,
        CriticalSectionRawMutex,
        RollupEvent,
        EVENT_CHANNEL_CAPACITY,
        EVENT_SUBSCRIBERS,
        EVENT_PUBLISHERS,
    >,
}

impl<'a> RollupAccumulator<'a> {
    /// Create a new rollup accumulator with a publisher
    pub fn new(
        publisher: Publisher<
            'a,
            CriticalSectionRawMutex,
            RollupEvent,
            EVENT_CHANNEL_CAPACITY,
            EVENT_SUBSCRIBERS,
            EVENT_PUBLISHERS,
        >,
    ) -> Self {
        Self {
            raw_buffer: Vec::with_capacity(30),
            rollup_5m_buffer: Vec::with_capacity(12),
            rollup_1h_buffer: Vec::with_capacity(24),
            publisher,
        }
    }

    fn compute_rollup(rollup: &[RawSample]) -> Rollup {
        let mut avg = [0i32; MAX_SENSORS];
        let mut min = [i32::MAX; MAX_SENSORS];
        let mut max = [i32::MIN; MAX_SENSORS];

        for r in rollup.iter() {
            for i in 0..MAX_SENSORS {
                avg[i] += r.values[i];
                if r.values[i] < min[i] {
                    min[i] = r.values[i];
                }
                if r.values[i] > max[i] {
                    max[i] = r.values[i];
                }
            }
        }

        let count = rollup.len() as i32;
        for i in 0..MAX_SENSORS {
            avg[i] /= count;
        }

        Rollup::new(rollup[0].timestamp, &avg, &min, &max)
    }

    fn compute_rollup_from_rollups(rollup: &[Rollup]) -> Rollup {
        let mut avg = [0i32; MAX_SENSORS];
        let mut min = [i32::MAX; MAX_SENSORS];
        let mut max = [i32::MIN; MAX_SENSORS];

        for r in rollup.iter() {
            for i in 0..MAX_SENSORS {
                avg[i] += r.avg[i];
                if r.min[i] < min[i] {
                    min[i] = r.min[i];
                }
                if r.max[i] > max[i] {
                    max[i] = r.max[i];
                }
            }
        }

        let count = rollup.len() as i32;
        for i in 0..MAX_SENSORS {
            avg[i] /= count;
        }

        Rollup::new(rollup[0].start_ts, &avg, &min, &max)
    }

    /// Add a new raw sample to the accumulator
    ///
    /// This should be called every 10 seconds with fresh sensor readings.
    /// When 30 samples accumulate, a 5-minute rollup is automatically generated.
    /// All events are published to subscribers (storage manager, UI tasks, etc.)
    pub async fn add_sample(&mut self, timestamp: u32, values: &[i32; MAX_SENSORS]) {
        let sample = RawSample::new(timestamp, values);

        // Publish raw sample event
        self.publisher.publish(RollupEvent::RawSample(sample)).await;

        // Try to add to buffer; if full, generate rollup
        if self.raw_buffer.len() < 30 {
            self.raw_buffer.push(sample);
        } else {
            // Buffer is full (30 samples), generate 5-minute rollup
            self.generate_5m_rollup().await;
            // Clear buffer and add current sample
            self.raw_buffer.clear();
            self.raw_buffer.push(sample);
        }
    }

    /// Generate a 5-minute rollup from accumulated raw samples
    async fn generate_5m_rollup(&mut self) {
        if self.raw_buffer.is_empty() {
            return;
        }

        let rollup = Self::compute_rollup(&self.raw_buffer);

        // Publish 5-minute rollup event
        self.publisher.publish(RollupEvent::Rollup5m(rollup)).await;

        // Add to hourly buffer
        if self.rollup_5m_buffer.len() < 12 {
            self.rollup_5m_buffer.push(rollup);
        } else {
            // Buffer is full (12 rollups), generate hourly rollup
            self.generate_1h_rollup().await;
            self.rollup_5m_buffer.clear();
            self.rollup_5m_buffer.push(rollup);
        }
    }

    /// Generate an hourly rollup from accumulated 5-minute rollups
    async fn generate_1h_rollup(&mut self) {
        if self.rollup_5m_buffer.is_empty() {
            return;
        }

        let rollup = Self::compute_rollup_from_rollups(&self.rollup_5m_buffer);

        // Publish hourly rollup event
        self.publisher.publish(RollupEvent::Rollup1h(rollup)).await;

        // Add to daily buffer
        if self.rollup_1h_buffer.len() < 24 {
            self.rollup_1h_buffer.push(rollup);
        } else {
            // Buffer is full (24 rollups), generate daily rollup
            self.generate_daily_rollup().await;
            self.rollup_1h_buffer.clear();
            self.rollup_1h_buffer.push(rollup);
        }
    }

    /// Generate a daily rollup from accumulated hourly rollups
    async fn generate_daily_rollup(&mut self) {
        if self.rollup_1h_buffer.is_empty() {
            return;
        }

        let rollup = Self::compute_rollup_from_rollups(&self.rollup_1h_buffer);

        // Publish daily rollup event
        self.publisher
            .publish(RollupEvent::RollupDaily(rollup))
            .await;
    }
}
