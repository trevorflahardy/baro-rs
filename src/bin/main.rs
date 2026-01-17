#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_graphics::mono_font::{MonoTextStyle, ascii::FONT_10X20};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::Text;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::timer::timg::TimerGroup;
use rtt_target::rprintln;

// Display-LCD panel specific imports
use aw9523::I2CGpioExpanderInterface;
use axp2101::{Axp2101, I2CPowerManagementInterface};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;
use mipidsi::interface::SpiInterface;
use mipidsi::options::{ColorInversion, ColorOrder};
use mipidsi::{Builder as MipidsiBuilder, models::ILI9342CRgb565};

const DISPLAY_WIDTH: u16 = 320;
const DISPLAY_HEIGHT: u16 = 240;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    rtt_target::rprintln!("PANIC: {}", info);
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.2.0

    rtt_target::rtt_init_print!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    rprintln!("Embassy initialized!");

    // CRITICAL: Initialize I2C for AXP2101 power management (REQUIRED for CoreS3 display!)
    rprintln!("Initializing I2C for power management...");
    let i2c_bus = esp_hal::i2c::master::I2c::new(
        peripherals.I2C0,
        esp_hal::i2c::master::Config::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO12)
    .with_scl(peripherals.GPIO11);

    rprintln!("Initializing AXP2101 power management...");
    // Initialize AXP2101 - this powers the display!
    let axp_interface = I2CPowerManagementInterface::new(i2c_bus);
    let mut axp = Axp2101::new(axp_interface);
    match axp.init() {
        Ok(_) => rprintln!("AXP2101 initialized successfully - display now has power!"),
        Err(e) => rprintln!("AXP2101 initialization failed: {:?}", e),
    }

    rprintln!("Initializing AW9523 GPIO expander...");
    // Get the I2C bus back and initialize AW9523
    let i2c_bus = axp.release_i2c();
    let aw_interface = I2CGpioExpanderInterface::new(i2c_bus);
    let mut aw = aw9523::Aw9523::new(aw_interface);
    aw.init().unwrap();
    rprintln!("AW9523 initialized!");

    let radio_init = esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller");
    let (mut _wifi_controller, _interfaces) =
        esp_radio::wifi::new(&radio_init, peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");

    // Configure and initialize the display

    // 1. Configure SPI bus with 40MHz frequency
    let spi_bus = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(40))
            .with_mode(esp_hal::spi::Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO36)
    .with_mosi(peripherals.GPIO37);

    // 2. Create the CS (Chip Select) pin - GPIO3 for CoreS3
    let cs = Output::new(peripherals.GPIO3, Level::High, OutputConfig::default());

    // 3. Wrap the SPI bus as a SPI device (required by embedded-hal traits)
    let spi_delay = esp_hal::delay::Delay::new();
    let spi_device = ExclusiveDevice::new(spi_bus, cs, spi_delay).unwrap();

    // 4. Set up DC (Data/Command) pin - GPIO35 for CoreS3
    let dc = Output::new(peripherals.GPIO35, Level::Low, OutputConfig::default());

    // 5. Set up Reset pin - GPIO15 for CoreS3
    let reset = Output::new(peripherals.GPIO15, Level::High, OutputConfig::default());

    // 6. Create a buffer for SPI batching (larger = faster, uses more RAM)
    let mut spi_buffer = [0u8; 512];

    // 7. Create display interface
    let di = SpiInterface::new(spi_device, dc, &mut spi_buffer);

    // 8. Build and initialize the display driver with CoreS3-specific settings
    let mut display = MipidsiBuilder::new(ILI9342CRgb565, di)
        .reset_pin(reset)
        .display_size(DISPLAY_WIDTH, DISPLAY_HEIGHT)
        .color_order(ColorOrder::Bgr) // Critical for CoreS3!
        .invert_colors(ColorInversion::Inverted) // Critical for CoreS3!
        .init(&mut embassy_time::Delay)
        .expect("Failed to initialize display");

    rprintln!("Display initialized!");

    // Draw a filled red rectangle covering the entire display
    Rectangle::new(Point::new(0, 0), Size::new(320, 240))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
        .draw(&mut display)
        .unwrap();
    rprintln!("Drew red rectangle!");

    // Draw white text on top
    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
    Text::new("Hello CoreS3!", Point::new(60, 120), text_style)
        .draw(&mut display)
        .unwrap();
    rprintln!("Drew text!");

    // TODO: Spawn some tasks
    let _ = spawner;

    loop {
        rprintln!("Hello world!");
        Timer::after(Duration::from_secs(1)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples
}
