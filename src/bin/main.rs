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
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::timer::timg::TimerGroup;
use rtt_target::rprintln;

// Display-LCD panel specific imports
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::spi::master::{Config, Spi};
use mipidsi::interface::SpiInterface;
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

    let radio_init = esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller");
    let (mut _wifi_controller, _interfaces) =
        esp_radio::wifi::new(&radio_init, peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");

    // Configure and initialize the display

    // 1. Configure SPI bus
    let spi_bus = Spi::new(peripherals.SPI2, Config::default())
        .unwrap()
        .with_sck(peripherals.GPIO36)
        .with_mosi(peripherals.GPIO37);

    // 2. Create a dummy CS pin (we don't use hardware CS for this display)
    let cs = Output::new(peripherals.GPIO35, Level::High, OutputConfig::default());

    // 3. Wrap the SPI bus as a SPI device (required by embedded-hal traits)
    let spi_device = ExclusiveDevice::new_no_delay(spi_bus, cs).unwrap();

    // 4. Set up DC (Data/Command) pin
    let dc = Output::new(peripherals.GPIO34, Level::Low, OutputConfig::default());

    // 5. Create a buffer for SPI batching (larger = faster, uses more RAM)
    let mut spi_buffer = [0u8; 64];

    // 6. Create display interface
    let di = SpiInterface::new(spi_device, dc, &mut spi_buffer);

    // 7. Build and initialize the display driver
    let mut display = MipidsiBuilder::new(ILI9342CRgb565, di)
        .display_size(DISPLAY_WIDTH, DISPLAY_HEIGHT)
        .init(&mut embassy_time::Delay)
        .expect("Failed to initialize display");

    rprintln!("Display initialized!");

    // TODO: Spawn some tasks
    let _ = spawner;

    loop {
        rprintln!("Hello world!");
        Timer::after(Duration::from_secs(1)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples
}
