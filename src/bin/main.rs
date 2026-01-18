#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::cell::RefCell;
use core::fmt::Write;
use critical_section::Mutex;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use embedded_sdmmc::SdCard;
use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    i2c::master::Config as I2cConfig,
    spi::master::{Config as SpiConfig, Spi},
    time::Rate,
    timer::timg::TimerGroup,
};
use heapless::String;

use rtt_target::rprintln;

use axp2101::{Axp2101, I2CPowerManagementInterface};
use baro_rs::{
    dual_mode_pin::{DualModePin, DualModePinAsOutput, InputModeSpiDevice, OutputModeSpiDevice},
    // ft6336u::FT6336U,
};
use embedded_hal_bus::{
    i2c::CriticalSectionDevice as I2cCriticalSectionDevice,
    spi::CriticalSectionDevice as SpiCriticalSectionDevice,
};
use mipidsi::{
    Builder as MipidsiBuilder,
    interface::SpiInterface,
    models::ILI9342CRgb565,
    options::{ColorInversion, ColorOrder},
};

const DISPLAY_WIDTH: u16 = 320;
const DISPLAY_HEIGHT: u16 = 240;

// Static dual-mode pin for GPIO35 (shared between SD card MISO and display DC)
static GPIO35_PIN: DualModePin<35> = DualModePin::new();

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    rtt_target::rprintln!("PANIC: {}", info);
    loop {}
}

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

#[allow(clippy::large_stack_frames)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // === Core System Init ===
    rtt_target::rtt_init_print!();
    let hal_config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(hal_config);
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    let timer_group = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timer_group.timer0);
    rprintln!("Core system initialized");

    // === Power Management ===
    // AXP2101 powers the display and other peripherals
    rprintln!("Configuring power management...");
    let i2c0 = esp_hal::i2c::master::I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO12)
    .with_scl(peripherals.GPIO11)
    .into_async();

    let i2c0_bus = Mutex::new(RefCell::new(i2c0));
    let i2c_for_axp = I2cCriticalSectionDevice::new(&i2c0_bus);
    let i2c_for_aw = I2cCriticalSectionDevice::new(&i2c0_bus);
    let __i2c_for_touch = I2cCriticalSectionDevice::new(&i2c0_bus);

    let power_mgmt_interface = I2CPowerManagementInterface::new(i2c_for_axp);
    let mut power_mgmt_chip = Axp2101::new(power_mgmt_interface);

    match power_mgmt_chip.init() {
        Ok(_) => rprintln!("Power management ready"),
        Err(e) => rprintln!("Power init failed: {:?}", e),
    }

    // === GPIO Expander ===
    rprintln!("Configuring GPIO expander...");
    let mut gpio_expander = aw9523_embedded::Aw9523::new(i2c_for_aw, 0x58);
    gpio_expander.init().unwrap();

    rprintln!("GPIO expander ready");

    // === Radio Init ===
    rprintln!("Configuring radio...");
    let radio_init = esp_radio::init().expect("Radio init failed");
    let (_wifi, _interfaces) =
        esp_radio::wifi::new(&radio_init, peripherals.WIFI, Default::default())
            .expect("WiFi init failed");
    rprintln!("Radio ready");

    // === Initialize the SPI devices (display and SD card) ===
    rprintln!("Configuring display...");
    let spi_bus_inner = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(40))
            .with_mode(esp_hal::spi::Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO36)
    .with_mosi(peripherals.GPIO37)
    .with_miso(peripherals.GPIO35)
    .into_async();

    let spi_bus = Mutex::new(RefCell::new(spi_bus_inner));

    let cs_display = Output::new(peripherals.GPIO3, Level::High, OutputConfig::default());
    let cs_sd_card = Output::new(peripherals.GPIO4, Level::High, OutputConfig::default());

    let display_spi_inner =
        SpiCriticalSectionDevice::new(&spi_bus, cs_display, esp_hal::delay::Delay::new()).unwrap();

    let sd_card_spi_inner =
        SpiCriticalSectionDevice::new(&spi_bus, cs_sd_card, esp_hal::delay::Delay::new()).unwrap();

    // Wrap SPI devices with dual-mode pin wrappers
    let display_spi = OutputModeSpiDevice::new(display_spi_inner, &GPIO35_PIN);
    let sd_card_spi = InputModeSpiDevice::new(sd_card_spi_inner, &GPIO35_PIN);

    let mut display_spi_buffer = [0u8; 512];
    // Use DualModePinAsOutput for display DC instead of direct Output
    let display_dc = DualModePinAsOutput::new(&GPIO35_PIN);
    let display_reset = Output::new(peripherals.GPIO15, Level::High, OutputConfig::default());

    let display_interface = SpiInterface::new(display_spi, display_dc, &mut display_spi_buffer);

    let mut display = MipidsiBuilder::new(ILI9342CRgb565, display_interface)
        .reset_pin(display_reset)
        .display_size(DISPLAY_WIDTH, DISPLAY_HEIGHT)
        .color_order(ColorOrder::Bgr)
        .invert_colors(ColorInversion::Inverted)
        .init(&mut embassy_time::Delay)
        .expect("Display init failed");

    rprintln!("Display ready");

    // Load up the SD card as well
    rprintln!("Configuring SD card...");

    let sd_card = SdCard::new(sd_card_spi, esp_hal::delay::Delay::new());
    let sd_card_size = match sd_card.num_bytes() {
        Ok(size) => size,
        Err(e) => {
            rprintln!("SD card init failed: {:?}", e);
            0
        }
    };
    rprintln!("SD card ready (size: {} bytes)", sd_card_size);

    // Load up the capacitive touch controller
    // Create I2C interface on the FT6336U@Capacitive touch, touch area pixels 320 x 280
    rprintln!("Configuring touch controller...");
    // let touch_interface = FT6336U::new(i2c_for_touch,)

    rprintln!("=== Hardware initialization complete ===\n");

    // === Application: Display Test ===
    draw_debug_screen(&mut display, sd_card_size);

    let _ = spawner;

    // === Main Loop ===
    loop {
        rprintln!("Running...");
        Timer::after(Duration::from_secs(1)).await;
    }
}

/// Draw test pattern on display
fn draw_debug_screen<D>(display: &mut D, sd_card_size: u64)
where
    D: embedded_graphics::draw_target::DrawTarget<Color = Rgb565>,
{
    // Clear screen with black background
    let _ = Rectangle::new(Point::new(0, 0), Size::new(320, 240))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
        .draw(display);

    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::GREEN);

    // Display title
    let _ = Text::new("M5Stack CoreS3", Point::new(60, 30), text_style).draw(display);

    // Format and display SD card size
    let mut buffer = String::<64>::new();
    if sd_card_size > 0 {
        let _ = write!(buffer, "SD: {} MB", sd_card_size / 1_000_000);
    } else {
        let _ = write!(buffer, "SD: Not detected");
    }
    let _ = Text::new(&buffer, Point::new(60, 120), text_style).draw(display);

    rprintln!("Display test complete");
}
