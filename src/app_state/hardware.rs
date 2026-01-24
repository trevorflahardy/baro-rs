//! Hardware initialization and management for the Baro device
//!
//! This module provides functions for setting up all hardware components
//! in the correct order, ensuring dependencies are properly initialized.

use axp2101_embedded::AsyncAxp2101;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex as AsyncMutex;
use esp_hal::{i2c::master::Config as I2cConfig, time::Rate};
use log::info;
use static_cell::StaticCell;

use crate::async_i2c_bus::AsyncI2cDevice;

/// Container for I2C-based hardware components
pub struct I2cHardware<'a> {
    pub power_mgmt: AsyncAxp2101<AsyncI2cDevice<'a, esp_hal::i2c::master::I2c<'a, esp_hal::Async>>>,
    pub gpio_expander: aw9523_embedded::r#async::Aw9523Async<
        embedded_hal::i2c::SevenBitAddress,
        AsyncI2cDevice<'a, esp_hal::i2c::master::I2c<'a, esp_hal::Async>>,
    >,
}

/// Initialize the I2C bus and all I2C-based peripherals
///
/// This function sets up:
/// - I2C bus (400 kHz)
/// - AXP2101 power management chip
/// - AW9523 GPIO expander
///
/// # Returns
/// A tuple of (I2cHardware, AsyncI2cDevice for touch, AsyncI2cDevice for sensors)
pub async fn init_i2c_hardware(
    i2c0: esp_hal::i2c::master::I2c<'static, esp_hal::Async>,
) -> (
    I2cHardware<'static>,
    AsyncI2cDevice<'static, esp_hal::i2c::master::I2c<'static, esp_hal::Async>>,
    AsyncI2cDevice<'static, esp_hal::i2c::master::I2c<'static, esp_hal::Async>>,
) {
    // Create shared I2C bus
    static I2C0_BUS: StaticCell<
        AsyncMutex<CriticalSectionRawMutex, esp_hal::i2c::master::I2c<'static, esp_hal::Async>>,
    > = StaticCell::new();
    let i2c0_bus = I2C0_BUS.init(AsyncMutex::new(i2c0));

    // Create device wrappers
    let i2c_for_axp = AsyncI2cDevice::new(i2c0_bus);
    let i2c_for_aw = AsyncI2cDevice::new(i2c0_bus);
    let i2c_for_touch = AsyncI2cDevice::new(i2c0_bus);
    let i2c_for_sensors = AsyncI2cDevice::new(i2c0_bus);

    // Initialize power management
    info!("Configuring power management");
    let mut power_mgmt_chip = AsyncAxp2101::new(i2c_for_axp);

    match power_mgmt_chip.init().await {
        Ok(_) => info!("Power management ready"),
        Err(e) => info!("Power init failed: {:?}", e),
    }

    power_mgmt_chip
        .set_charging_led_mode(axp2101_embedded::ChargeLedMode::On)
        .await
        .unwrap();

    // Enable all LDOs
    power_mgmt_chip.enable_aldo1().await.unwrap();
    power_mgmt_chip.enable_aldo2().await.unwrap();
    power_mgmt_chip.enable_aldo3().await.unwrap();
    power_mgmt_chip.enable_aldo4().await.unwrap();
    power_mgmt_chip.enable_bldo1().await.unwrap();
    power_mgmt_chip.enable_bldo2().await.unwrap();
    power_mgmt_chip.enable_dldo1().await.unwrap();

    // Set ALDO4 voltage to 3.3V for display
    power_mgmt_chip.set_aldo4_voltage(3300).await.unwrap();

    // Initialize GPIO expander
    info!("Configuring GPIO expander...");
    let mut gpio_expander = aw9523_embedded::r#async::Aw9523Async::new(i2c_for_aw, 0x58);
    gpio_expander.init().await.unwrap();

    // Configure P1_2 (pin 10) as input for touch interrupt
    gpio_expander
        .pin_mode(10, aw9523_embedded::PinMode::Input)
        .await
        .unwrap();
    gpio_expander.enable_interrupt(10, true).await.unwrap();

    info!("GPIO expander ready (P1_2 configured for touch interrupt)");

    let hardware = I2cHardware {
        power_mgmt: power_mgmt_chip,
        gpio_expander,
    };

    (hardware, i2c_for_touch, i2c_for_sensors)
}

/// Initialize the I2C bus hardware
///
/// Creates the I2C peripheral with proper configuration
pub fn create_i2c_bus(
    i2c0: esp_hal::peripherals::I2C0<'static>,
    sda: esp_hal::peripherals::GPIO12<'static>,
    scl: esp_hal::peripherals::GPIO11<'static>,
) -> esp_hal::i2c::master::I2c<'static, esp_hal::Async> {
    esp_hal::i2c::master::I2c::new(
        i2c0,
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(sda)
    .with_scl(scl)
    .into_async()
}

/// Initialize SPI hardware for the SD card
///
/// This function takes an SPI device and wraps it with a delay implementation
/// to create an SD card instance ready for use.
///
/// # Type Parameters
/// - `S`: The SPI device type for the SD card
/// - `D`: The delay implementation type
///
/// # Returns
/// An SdCard instance with the provided SPI device and delay
pub fn init_spi_hardware<S, D>(sd_card_spi: S, delay: D) -> embedded_sdmmc::SdCard<S, D>
where
    S: embedded_hal::spi::SpiDevice<u8>,
    D: embedded_hal::delay::DelayNs,
{
    embedded_sdmmc::SdCard::new(sd_card_spi, delay)
}
