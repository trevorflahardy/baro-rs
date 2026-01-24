//! Application-wide state and error types for Baro

use core::str::FromStr;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex as AsyncMutex;
use thiserror_no_std::Error;

use crate::storage::{accumulator::RollupAccumulator, manager::StorageManager};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppRunState {
    Uninitialized,
    WifiConnecting,
    WifiConnected,
    TimeSyncing,
    TimeKnown,
    Error,
}

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
    pub fn new() -> Self {
        Self {
            run_state: AppRunState::Uninitialized,
            time_known: false,
            wifi_connected: false,
            accumulator: None,
            storage_manager: None,
        }
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
