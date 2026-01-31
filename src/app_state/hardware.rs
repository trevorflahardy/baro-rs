//! Hardware initialization and management for the Baro device
//!
//! This module provides functions for setting up all hardware components
//! in the correct order, ensuring dependencies are properly initialized.

use axp2101_embedded::AsyncAxp2101;
use core::cell::RefCell;
use critical_section::Mutex as CsMutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex as AsyncMutex;
use embedded_hal_bus::spi::CriticalSectionDevice as SpiCriticalSectionDevice;
use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    i2c::master::Config as I2cConfig,
    spi::master::{Config as SpiConfig, Spi},
    time::Rate,
};
use ft6336u_driver::FT6336U;
use log::info;
use mipidsi::{
    Builder as MipidsiBuilder,
    interface::SpiInterface,
    models::ILI9342CRgb565,
    options::{ColorInversion, ColorOrder},
};
use static_cell::StaticCell;
use tca9548a_embedded::r#async::Tca9548aAsync;

use crate::async_i2c_bus::AsyncI2cDevice;
use crate::dual_mode_pin::{
    DualModePin, DualModePinAsOutput, InputModeSpiDevice, OutputModeSpiDevice,
};

pub type Tca9548SpiMultiplexer<'a> =
    Tca9548aAsync<AsyncI2cDevice<'a, esp_hal::i2c::master::I2c<'a, esp_hal::Async>>>;

/// Container for I2C-based hardware components
pub struct I2cHardware<'a> {
    pub power_mgmt: AsyncAxp2101<AsyncI2cDevice<'a, esp_hal::i2c::master::I2c<'a, esp_hal::Async>>>,
    pub gpio_expander: aw9523_embedded::r#async::Aw9523Async<
        embedded_hal::i2c::SevenBitAddress,
        AsyncI2cDevice<'a, esp_hal::i2c::master::I2c<'a, esp_hal::Async>>,
    >,
    pub touch_interface: FT6336U<AsyncI2cDevice<'a, esp_hal::i2c::master::I2c<'a, esp_hal::Async>>>,
}

/// Container for SPI-based hardware components
///
/// Uses concrete types for ESP32-S3 SPI peripherals
#[allow(clippy::type_complexity)]
pub struct SpiHardware {
    pub display: mipidsi::Display<
        SpiInterface<
            'static,
            OutputModeSpiDevice<
                SpiCriticalSectionDevice<
                    'static,
                    Spi<'static, esp_hal::Async>,
                    Output<'static>,
                    esp_hal::delay::Delay,
                >,
                35,
            >,
            DualModePinAsOutput<35>,
        >,
        ILI9342CRgb565,
        Output<'static>,
    >,
    pub sd_card: embedded_sdmmc::SdCard<
        InputModeSpiDevice<
            SpiCriticalSectionDevice<
                'static,
                Spi<'static, esp_hal::Async>,
                Output<'static>,
                esp_hal::delay::Delay,
            >,
            35,
        >,
        esp_hal::delay::Delay,
    >,
    pub sd_card_size: u64,
}

/// Initialize the I2C bus and all I2C-based peripherals
///
/// This function sets up:
/// - I2C bus (400 kHz)
/// - AXP2101 power management chip
/// - AW9523 GPIO expander
/// - FT6336U capacitive touch controller
/// - TCA9548A I2C multiplexer for sensors
///
/// # Returns
/// A tuple of (I2cHardware, Tca9548SpiMultiplexer)
pub async fn init_i2c_hardware(
    i2c0: esp_hal::i2c::master::I2c<'static, esp_hal::Async>,
) -> (I2cHardware<'static>, Tca9548SpiMultiplexer<'static>) {
    // The I2C bus is shared between the various sensors and devices using a Tca9548a
    // I2c multiplexer. Thus, each device is initialized using the same underlying I2C bus.
    // NOTE: This ONLY applies to devices on the I2C bus that externally connect to the ESP32.
    // NOTE: This does not apply to the internal I2C bus used by:
    //      - the display controller,
    //      - the GPIO expander, or
    //      - the touch controller
    info!("Configuring I2C devices...");

    static I2C0_BUS: StaticCell<
        AsyncMutex<CriticalSectionRawMutex, esp_hal::i2c::master::I2c<'static, esp_hal::Async>>,
    > = StaticCell::new();
    let i2c0_bus = I2C0_BUS.init(AsyncMutex::new(i2c0));

    // Create device wrappers
    let i2c_for_axp = AsyncI2cDevice::new(i2c0_bus);
    let i2c_for_aw = AsyncI2cDevice::new(i2c0_bus);
    let i2c_for_touch = AsyncI2cDevice::new(i2c0_bus);
    let i2c_inner_for_sensors = AsyncI2cDevice::new(i2c0_bus);

    let i2c_for_sensors =
        Tca9548aAsync::new(i2c_inner_for_sensors, tca9548a_embedded::SlaveAddr::Default);

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

    // Initialize touch controller
    info!("Configuring touch controller...");
    let mut touch_interface = FT6336U::new(i2c_for_touch);
    let library_version = touch_interface.read_library_version().await.unwrap_or(0);
    let chip_id = touch_interface.read_chip_id().await.unwrap();

    // Configure touch controller in Polling mode (INT stays LOW while touched)
    // This is better than Trigger mode for continuous touch detection
    touch_interface
        .write_g_mode(ft6336u_driver::GestureMode::Polling)
        .await
        .unwrap();
    let g_mode = touch_interface.read_g_mode().await.unwrap();

    info!(
        "Touch controller ready (library: 0x{:04X}, chip: 0x{:02X}, mode: 0x{:02X})",
        library_version, chip_id, g_mode
    );

    let hardware = I2cHardware {
        power_mgmt: power_mgmt_chip,
        gpio_expander,
        touch_interface,
    };

    (hardware, i2c_for_sensors)
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

/// Initialize all SPI-based peripherals including display and SD card
///
/// This function sets up:
/// - SPI bus (40 MHz)
/// - Display (ILI9342C with MIPIDSI)
/// - SD card with embedded-sdmmc
///
/// # Arguments
/// - `spi2_peripheral`: SPI2 peripheral
/// - `display_cs_pin`: Display CS pin (GPIO3)
/// - `sd_card_cs_pin`: SD card CS pin (GPIO4)
/// - `display_reset_pin`: Display reset pin (GPIO15)
/// - `dual_mode_pin`: Dual-mode pin for MISO/DC switching (GPIO35)
/// - `spi_sck_pin`: SPI SCK pin (GPIO36)
/// - `spi_mosi_pin`: SPI MOSI pin (GPIO37)
/// - `spi_miso_pin`: SPI MISO pin (GPIO35)
/// - `display_width`: Display width in pixels
/// - `display_height`: Display height in pixels
///
/// # Returns
/// A SpiHardware struct containing the initialized display and SD card
#[allow(clippy::too_many_arguments)]
pub fn init_spi_peripherals(
    spi2_peripheral: esp_hal::peripherals::SPI2<'static>,
    display_cs_pin: esp_hal::peripherals::GPIO3<'static>,
    sd_card_cs_pin: esp_hal::peripherals::GPIO4<'static>,
    display_reset_pin: esp_hal::peripherals::GPIO15<'static>,
    dual_mode_pin: &'static DualModePin<35>,
    spi_sck_pin: esp_hal::peripherals::GPIO36<'static>,
    spi_mosi_pin: esp_hal::peripherals::GPIO37<'static>,
    spi_miso_pin: esp_hal::peripherals::GPIO35<'static>,
    display_width: u16,
    display_height: u16,
) -> SpiHardware {
    info!("Configuring SPI devices...");

    // Create SPI bus
    let spi_bus_inner = Spi::new(
        spi2_peripheral,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(40))
            .with_mode(esp_hal::spi::Mode::_0),
    )
    .unwrap()
    .with_sck(spi_sck_pin)
    .with_mosi(spi_mosi_pin)
    .with_miso(spi_miso_pin)
    .into_async();

    static SPI_BUS: StaticCell<CsMutex<RefCell<Spi<'static, esp_hal::Async>>>> = StaticCell::new();
    let spi_bus = SPI_BUS.init(CsMutex::new(RefCell::new(spi_bus_inner)));

    // Create CS pins
    let cs_display = Output::new(display_cs_pin, Level::High, OutputConfig::default());
    let cs_sd_card = Output::new(sd_card_cs_pin, Level::High, OutputConfig::default());

    // Create SPI devices
    let display_spi_inner =
        SpiCriticalSectionDevice::new(spi_bus, cs_display, esp_hal::delay::Delay::new()).unwrap();
    let sd_card_spi_inner =
        SpiCriticalSectionDevice::new(spi_bus, cs_sd_card, esp_hal::delay::Delay::new()).unwrap();

    // Wrap SPI devices with dual-mode pin wrappers
    let display_spi = OutputModeSpiDevice::new(display_spi_inner, dual_mode_pin);
    let sd_card_spi = InputModeSpiDevice::new(sd_card_spi_inner, dual_mode_pin);

    // Initialize display
    static DISPLAY_SPI_BUFFER: StaticCell<[u8; 512]> = StaticCell::new();
    let display_spi_buffer = DISPLAY_SPI_BUFFER.init([0u8; 512]);
    let display_dc = DualModePinAsOutput::new(dual_mode_pin);
    let display_reset = Output::new(display_reset_pin, Level::High, OutputConfig::default());

    let display_interface = SpiInterface::new(display_spi, display_dc, display_spi_buffer);

    let display = MipidsiBuilder::new(ILI9342CRgb565, display_interface)
        .reset_pin(display_reset)
        .display_size(display_width, display_height)
        .color_order(ColorOrder::Bgr)
        .invert_colors(ColorInversion::Inverted)
        .init(&mut embassy_time::Delay)
        .expect("Display init failed");

    info!("Display ready");

    // Initialize SD card
    info!("Configuring SD card...");
    let sd_card = init_spi_hardware(sd_card_spi, esp_hal::delay::Delay::new());
    let sd_card_size = match sd_card.num_bytes() {
        Ok(size) => {
            info!("SD card ready (size: {} bytes)", size);
            size
        }
        Err(e) => {
            info!("SD card init failed: {:?}", e);
            0
        }
    };

    SpiHardware {
        display,
        sd_card,
        sd_card_size,
    }
}
