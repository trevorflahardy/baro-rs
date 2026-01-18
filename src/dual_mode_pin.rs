//! Dual-mode GPIO pin implementation for ESP32-S3 GPIO35
//!
//! This module provides raw register-level control to switch GPIO35 between
//! input mode (for SPI MISO) and output mode (for display DC signal).
//!
//! Hardware constraint: On M5Stack CoreS3, GPIO35 is physically shared between
//! the SD card's MISO line and the display's DC (Data/Command) signal.

use core::ptr::write_volatile;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::{ErrorType, Operation, SpiDevice};

// ESP32-S3 GPIO register addresses for GPIO 32-48 (high bank)
const GPIO_OUT1_W1TS_REG: u32 = 0x6000_4014; // Set output bits
const GPIO_OUT1_W1TC_REG: u32 = 0x6000_4018; // Clear output bits
const GPIO_ENABLE1_W1TS_REG: u32 = 0x6000_4030; // Enable output mode
const GPIO_ENABLE1_W1TC_REG: u32 = 0x6000_4034; // Disable output mode (enable input)

// GPIO35 is bit 3 in the high register bank (GPIO 32-48)
const GPIO35_BIT: u32 = 1 << 3;

/// A GPIO pin that can be dynamically switched between input and output modes
/// using raw register manipulation.
///
/// This bypasses Rust's ownership system to allow GPIO35 to be used for both
/// SPI MISO (input) and display DC (output) by switching modes before each use.
pub struct DualModePin {
    _private: (),
}

impl DualModePin {
    /// Creates a new DualModePin for GPIO35.
    ///
    /// # Safety
    /// This is safe because we're using critical sections for all register access
    /// and the pin is physically dedicated to this dual-mode usage.
    pub const fn new() -> Self {
        Self { _private: () }
    }

    /// Switches GPIO35 to input mode (for SPI MISO operations)
    pub fn set_as_input(&self) {
        critical_section::with(|_| {
            unsafe {
                // Disable output mode (enable input mode)
                write_volatile(GPIO_ENABLE1_W1TC_REG as *mut u32, GPIO35_BIT);
            }
        });
    }

    /// Switches GPIO35 to output mode (for display DC signal)
    pub fn set_as_output(&self) {
        critical_section::with(|_| {
            unsafe {
                // Enable output mode
                write_volatile(GPIO_ENABLE1_W1TS_REG as *mut u32, GPIO35_BIT);
            }
        });
    }

    /// Sets GPIO35 output high (only effective when in output mode)
    pub fn set_high(&self) {
        critical_section::with(|_| unsafe {
            write_volatile(GPIO_OUT1_W1TS_REG as *mut u32, GPIO35_BIT);
        });
    }

    /// Sets GPIO35 output low (only effective when in output mode)
    pub fn set_low(&self) {
        critical_section::with(|_| unsafe {
            write_volatile(GPIO_OUT1_W1TC_REG as *mut u32, GPIO35_BIT);
        });
    }
}

/// Wrapper type that implements OutputPin for display DC signal control
pub struct DualModePinAsOutput {
    pin: &'static DualModePin,
}

impl DualModePinAsOutput {
    /// Creates a new OutputPin wrapper around the DualModePin
    pub const fn new(pin: &'static DualModePin) -> Self {
        Self { pin }
    }
}

impl OutputPin for DualModePinAsOutput {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.pin.set_low();
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.pin.set_high();
        Ok(())
    }
}

impl embedded_hal::digital::ErrorType for DualModePinAsOutput {
    type Error = core::convert::Infallible;
}

/// SPI device wrapper for the display that automatically sets GPIO35 to output mode
/// before each transaction
pub struct DisplaySpiDevice<T> {
    device: T,
    gpio35: &'static DualModePin,
}

impl<T> DisplaySpiDevice<T> {
    /// Creates a new DisplaySpiDevice that wraps an existing SPI device
    pub const fn new(device: T, gpio35: &'static DualModePin) -> Self {
        Self { device, gpio35 }
    }
}

impl<T: ErrorType> ErrorType for DisplaySpiDevice<T> {
    type Error = T::Error;
}

impl<T: SpiDevice<u8>> SpiDevice<u8> for DisplaySpiDevice<T> {
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
        // Switch GPIO35 to output mode for display DC signal
        self.gpio35.set_as_output();
        // Perform the SPI transaction
        self.device.transaction(operations)
    }
}

/// SPI device wrapper for the SD card that automatically sets GPIO35 to input mode
/// before each transaction
pub struct SdCardSpiDevice<T> {
    device: T,
    gpio35: &'static DualModePin,
}

impl<T> SdCardSpiDevice<T> {
    /// Creates a new SdCardSpiDevice that wraps an existing SPI device
    pub const fn new(device: T, gpio35: &'static DualModePin) -> Self {
        Self { device, gpio35 }
    }
}

impl<T: ErrorType> ErrorType for SdCardSpiDevice<T> {
    type Error = T::Error;
}

impl<T: SpiDevice<u8>> SpiDevice<u8> for SdCardSpiDevice<T> {
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
        // Switch GPIO35 to input mode for SPI MISO
        self.gpio35.set_as_input();
        // Perform the SPI transaction
        self.device.transaction(operations)
    }
}
