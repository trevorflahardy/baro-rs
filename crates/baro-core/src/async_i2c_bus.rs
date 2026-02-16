//! Async I2C bus sharing implementation
//!
//! Provides an async-aware version of `CriticalSectionDevice` that properly
//! awaits async I2C operations instead of blocking.

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_hal_async::i2c::{ErrorType, I2c, Operation};

/// Async I2C bus sharing device using Embassy's async Mutex.
///
/// This allows sharing an async I2C bus across multiple devices, with each device
/// getting its own instance. Unlike the blocking `CriticalSectionDevice`, this
/// properly awaits async I2C operations and yields control to the executor.
///
/// # Why use Embassy Mutex?
///
/// Embassy's `Mutex` provides true async locking that can be held across await points.
/// It uses `CriticalSectionRawMutex` for interrupt-safe locking while still allowing
/// async operations to yield to the executor during I2C transactions.
///
/// # Example
///
/// ```no_run
/// use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
/// use embassy_sync::mutex::Mutex;
/// use static_cell::StaticCell;
///
/// static I2C_BUS: StaticCell<Mutex<CriticalSectionRawMutex, esp_hal::i2c::master::I2c<'static, esp_hal::Async>>> = StaticCell::new();
///
/// let i2c = /* ... create async I2C ... */;
/// let i2c_bus = I2C_BUS.init(Mutex::new(i2c));
///
/// let device1 = AsyncI2cDevice::new(i2c_bus);
/// let device2 = AsyncI2cDevice::new(i2c_bus);
/// ```
pub struct AsyncI2cDevice<'a, T> {
    bus: &'a Mutex<CriticalSectionRawMutex, T>,
}

impl<'a, T> AsyncI2cDevice<'a, T> {
    /// Create a new `AsyncI2cDevice`.
    #[inline]
    pub const fn new(bus: &'a Mutex<CriticalSectionRawMutex, T>) -> Self {
        Self { bus }
    }
}

impl<T> ErrorType for AsyncI2cDevice<'_, T>
where
    T: ErrorType,
{
    type Error = T::Error;
}

impl<T> I2c for AsyncI2cDevice<'_, T>
where
    T: I2c,
{
    /// Reads bytes from the I2C bus asynchronously.
    ///
    /// This properly awaits the async I2C operation, yielding control to the
    /// executor while the I2C transaction is in progress.
    #[inline]
    async fn read(&mut self, address: u8, read: &mut [u8]) -> Result<(), Self::Error> {
        let mut bus = self.bus.lock().await;
        bus.read(address, read).await
    }

    /// Writes bytes to the I2C bus asynchronously.
    ///
    /// This properly awaits the async I2C operation, yielding control to the
    /// executor while the I2C transaction is in progress.
    #[inline]
    async fn write(&mut self, address: u8, write: &[u8]) -> Result<(), Self::Error> {
        let mut bus = self.bus.lock().await;
        bus.write(address, write).await
    }

    /// Performs a write-read transaction asynchronously.
    ///
    /// This properly awaits the async I2C operation, yielding control to the
    /// executor while the I2C transaction is in progress.
    #[inline]
    async fn write_read(
        &mut self,
        address: u8,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), Self::Error> {
        let mut bus = self.bus.lock().await;
        bus.write_read(address, write, read).await
    }

    /// Executes multiple I2C operations as a single transaction asynchronously.
    ///
    /// This properly awaits the async I2C operation, yielding control to the
    /// executor while the I2C transaction is in progress.
    #[inline]
    async fn transaction(
        &mut self,
        address: u8,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        let mut bus = self.bus.lock().await;
        bus.transaction(address, operations).await
    }
}

// Safety: AsyncI2cDevice can be sent across thread boundaries if the underlying
// bus type is Send. The Mutex ensures exclusive access.
unsafe impl<T: Send> Send for AsyncI2cDevice<'_, T> {}
