//! Application-wide state and error types for Baro

mod hardware;
mod sensors_state;

pub use hardware::*;
pub use sensors_state::*;

use core::str::FromStr;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex as AsyncMutex;
use embassy_sync::pubsub::PubSubChannel;
use thiserror_no_std::Error;

use crate::storage::{
    accumulator::{
        RollupAccumulator, RollupEvent, EVENT_CHANNEL_CAPACITY, EVENT_PUBLISHERS, EVENT_SUBSCRIBERS,
    },
    manager::StorageManager,
};

/// Global pub-sub channel for rollup events
/// This allows the accumulator to publish events that multiple subscribers can listen to
pub static ROLLUP_CHANNEL: PubSubChannel<
    CriticalSectionRawMutex,
    RollupEvent,
    EVENT_CHANNEL_CAPACITY,
    EVENT_SUBSCRIBERS,
    EVENT_PUBLISHERS,
> = PubSubChannel::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppRunState {
    Uninitialized,
    WifiConnecting,
    WifiConnected,
    TimeSyncing,
    TimeKnown,
    SensorsRunning,
    Error,
}

/// Main application state container
///
/// This struct holds all the major components and state of the application.
/// It provides methods for initialization and access to hardware and sensor components.
pub struct AppState<'a, S, D, T>
where
    S: embedded_hal::spi::SpiDevice<u8>,
    D: embedded_hal::delay::DelayNs,
    T: embedded_sdmmc::TimeSource,
{
    pub run_state: AppRunState,
    pub time_known: bool,
    pub wifi_connected: bool,
    pub accumulator: Option<RollupAccumulator<'a>>,
    pub storage_manager: Option<StorageManager<S, D, T>>,
}

impl<'a, S, D, T> AppState<'a, S, D, T>
where
    S: embedded_hal::spi::SpiDevice<u8>,
    D: embedded_hal::delay::DelayNs,
    T: embedded_sdmmc::TimeSource,
{
    /// Create a new uninitialized app state
    pub fn new() -> Self {
        Self {
            run_state: AppRunState::Uninitialized,
            time_known: false,
            wifi_connected: false,
            accumulator: None,
            storage_manager: None,
        }
    }

    /// Initialize the accumulator with a publisher from the global channel
    pub fn init_accumulator(&mut self) {
        let publisher = ROLLUP_CHANNEL
            .publisher()
            .expect("Failed to create publisher");
        self.accumulator = Some(RollupAccumulator::new(publisher));
    }

    /// Set the storage manager
    pub fn set_storage_manager(&mut self, storage_manager: StorageManager<S, D, T>) {
        self.storage_manager = Some(storage_manager);
    }

    /// Get a reference to the accumulator
    pub fn accumulator(&self) -> Option<&RollupAccumulator<'a>> {
        self.accumulator.as_ref()
    }

    /// Get a mutable reference to the accumulator
    pub fn accumulator_mut(&mut self) -> Option<&mut RollupAccumulator<'a>> {
        self.accumulator.as_mut()
    }

    /// Get a reference to the storage manager
    pub fn storage_manager(&self) -> Option<&StorageManager<S, D, T>> {
        self.storage_manager.as_ref()
    }

    /// Get a mutable reference to the storage manager
    pub fn storage_manager_mut(&mut self) -> Option<&mut StorageManager<S, D, T>> {
        self.storage_manager.as_mut()
    }
}

pub type GlobalStateType<'a, S, D, T> = AsyncMutex<CriticalSectionRawMutex, AppState<'a, S, D, T>>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("WiFi connection failed: {0}")]
    Wifi(heapless::String<64>),
    #[error("Time sync failed: {0}")]
    TimeSync(heapless::String<64>),
    #[error("SD card error: {0}")]
    Storage(heapless::String<64>),
    #[error("Sensor error: {0}")]
    Sensor(heapless::String<64>),
    #[error("Unknown error")]
    Unknown,
}

pub trait FromUnchecked<T> {
    fn from_unchecked(value: T) -> Self;
}

impl<'a, const N: usize> FromUnchecked<&'a str> for heapless::String<N> {
    fn from_unchecked(value: &'a str) -> Self {
        heapless::String::<N>::from_str(value).unwrap()
    }
}
