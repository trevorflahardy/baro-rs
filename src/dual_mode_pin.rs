//! Dual-mode GPIO pin implementation for ESP32-S3
//!
//! This module provides raw register-level control to switch any GPIO pin between
//! input mode and output mode dynamically, bypassing Rust's ownership system.
//!
//! Useful when a pin needs to serve multiple functions (e.g., SPI MISO and DC signal).

use core::ptr::write_volatile;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::{ErrorType, Operation, SpiDevice};

// ESP32-S3 GPIO register addresses for GPIO 0-31 (low bank)
const GPIO_OUT_W1TS_REG: u32 = 0x6000_4008; // Set output bits
const GPIO_OUT_W1TC_REG: u32 = 0x6000_400C; // Clear output bits
const GPIO_ENABLE_W1TS_REG: u32 = 0x6000_4020; // Enable output mode
const GPIO_ENABLE_W1TC_REG: u32 = 0x6000_4024; // Disable output mode (enable input)

// ESP32-S3 GPIO register addresses for GPIO 32-48 (high bank)
const GPIO_OUT1_W1TS_REG: u32 = 0x6000_4014; // Set output bits
const GPIO_OUT1_W1TC_REG: u32 = 0x6000_4018; // Clear output bits
const GPIO_ENABLE1_W1TS_REG: u32 = 0x6000_4030; // Enable output mode
const GPIO_ENABLE1_W1TC_REG: u32 = 0x6000_4034; // Disable output mode (enable input)

/// A GPIO pin that can be dynamically switched between input and output modes
/// using raw register manipulation.
///
/// The const generic `PIN` parameter specifies the GPIO number (0-48 for ESP32-S3).
///
/// This bypasses Rust's ownership system to allow a single pin to serve multiple
/// functions by switching modes before each use.
///
/// # Example
/// ```no_run
/// // Create a dual-mode pin for GPIO35
/// static GPIO35_PIN: DualModePin<35> = DualModePin::new();
/// ```
pub struct DualModePin<const PIN: u8> {
    _private: (),
}

impl<const PIN: u8> DualModePin<PIN> {
    /// Creates a new DualModePin for the specified GPIO number.
    ///
    /// # Safety
    /// This is safe because we're using critical sections for all register access.
    /// However, the caller must ensure the pin is not simultaneously used elsewhere
    /// in a way that would cause conflicts.
    pub const fn new() -> Self {
        Self { _private: () }
    }

    /// Returns the register addresses and bit mask for this pin
    #[inline]
    const fn registers(&self) -> (u32, u32, u32, u32, u32) {
        if PIN < 32 {
            // Low bank (GPIO 0-31)
            let bit = 1u32 << PIN;
            (
                GPIO_OUT_W1TS_REG,
                GPIO_OUT_W1TC_REG,
                GPIO_ENABLE_W1TS_REG,
                GPIO_ENABLE_W1TC_REG,
                bit,
            )
        } else {
            // High bank (GPIO 32-48)
            let bit = 1u32 << (PIN - 32);
            (
                GPIO_OUT1_W1TS_REG,
                GPIO_OUT1_W1TC_REG,
                GPIO_ENABLE1_W1TS_REG,
                GPIO_ENABLE1_W1TC_REG,
                bit,
            )
        }
    }

    /// Switches the pin to input mode
    pub fn set_as_input(&self) {
        let (_, _, _, enable_clr, bit) = self.registers();
        critical_section::with(|_| {
            unsafe {
                // Disable output mode (enable input mode)
                write_volatile(enable_clr as *mut u32, bit);
            }
        });
    }

    /// Switches the pin to output mode
    pub fn set_as_output(&self) {
        let (_, _, enable_set, _, bit) = self.registers();
        critical_section::with(|_| {
            unsafe {
                // Enable output mode
                write_volatile(enable_set as *mut u32, bit);
            }
        });
    }

    /// Sets the pin output high (only effective when in output mode)
    pub fn set_high(&self) {
        let (out_set, _, _, _, bit) = self.registers();
        critical_section::with(|_| unsafe {
            write_volatile(out_set as *mut u32, bit);
        });
    }

    /// Sets the pin output low (only effective when in output mode)
    pub fn set_low(&self) {
        let (_, out_clr, _, _, bit) = self.registers();
        critical_section::with(|_| unsafe {
            write_volatile(out_clr as *mut u32, bit);
        });
    }
}

/// Wrapper type that implements OutputPin for GPIO control
pub struct DualModePinAsOutput<const PIN: u8> {
    pin: &'static DualModePin<PIN>,
}

impl<const PIN: u8> DualModePinAsOutput<PIN> {
    /// Creates a new OutputPin wrapper around the DualModePin
    pub const fn new(pin: &'static DualModePin<PIN>) -> Self {
        Self { pin }
    }
}

impl<const PIN: u8> OutputPin for DualModePinAsOutput<PIN> {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.pin.set_low();
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.pin.set_high();
        Ok(())
    }
}

impl<const PIN: u8> embedded_hal::digital::ErrorType for DualModePinAsOutput<PIN> {
    type Error = core::convert::Infallible;
}

/// SPI device wrapper that automatically sets a pin to output mode before each transaction
pub struct DisplaySpiDevice<T, const PIN: u8> {
    device: T,
    pin: &'static DualModePin<PIN>,
}

impl<T, const PIN: u8> DisplaySpiDevice<T, PIN> {
    /// Creates a new DisplaySpiDevice that wraps an existing SPI device
    pub const fn new(device: T, pin: &'static DualModePin<PIN>) -> Self {
        Self { device, pin }
    }
}

impl<T: ErrorType, const PIN: u8> ErrorType for DisplaySpiDevice<T, PIN> {
    type Error = T::Error;
}

impl<T: SpiDevice<u8>, const PIN: u8> SpiDevice<u8> for DisplaySpiDevice<T, PIN> {
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
        // Switch pin to output mode
        self.pin.set_as_output();
        // Perform the SPI transaction
        self.device.transaction(operations)
    }
}

/// SPI device wrapper that automatically sets a pin to input mode before each transaction
pub struct SdCardSpiDevice<T, const PIN: u8> {
    device: T,
    pin: &'static DualModePin<PIN>,
}

impl<T, const PIN: u8> SdCardSpiDevice<T, PIN> {
    /// Creates a new SdCardSpiDevice that wraps an existing SPI device
    pub const fn new(device: T, pin: &'static DualModePin<PIN>) -> Self {
        Self { device, pin }
    }
}

impl<T: ErrorType, const PIN: u8> ErrorType for SdCardSpiDevice<T, PIN> {
    type Error = T::Error;
}

impl<T: SpiDevice<u8>, const PIN: u8> SpiDevice<u8> for SdCardSpiDevice<T, PIN> {
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
        // Switch pin to input mode
        self.pin.set_as_input();
        // Perform the SPI transaction
        self.device.transaction(operations)
    }
}
