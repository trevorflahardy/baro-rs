#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

mod page;
mod sampling;
mod widgets;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    spi::master::{Config, Spi},
    time::Rate,
    timer::timg::TimerGroup,
};

use rtt_target::rprintln;

use aw9523::I2CGpioExpanderInterface;
use axp2101::{Axp2101, I2CPowerManagementInterface};
use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::{
    Builder as MipidsiBuilder,
    interface::SpiInterface,
    models::ILI9342CRgb565,
    options::{ColorInversion, ColorOrder},
};

const DISPLAY_WIDTH: u16 = 320;
const DISPLAY_HEIGHT: u16 = 240;

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
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);
    rprintln!("Core system initialized");

    // === Power Management ===
    // AXP2101 powers the display and other peripherals
    rprintln!("Configuring power management...");
    let i2c_bus = esp_hal::i2c::master::I2c::new(
        peripherals.I2C0,
        esp_hal::i2c::master::Config::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO12)
    .with_scl(peripherals.GPIO11);

    let axp_interface = I2CPowerManagementInterface::new(i2c_bus);
    let mut axp = Axp2101::new(axp_interface);

    match axp.init() {
        Ok(_) => rprintln!("Power management ready"),
        Err(e) => rprintln!("Power init failed: {:?}", e),
    }

    // === GPIO Expander ===
    rprintln!("Configuring GPIO expander...");
    let i2c_bus = axp.release_i2c();
    let aw_interface = I2CGpioExpanderInterface::new(i2c_bus);
    let mut aw = aw9523::Aw9523::new(aw_interface);
    aw.init().unwrap();
    rprintln!("GPIO expander ready");

    // === Radio Init ===
    rprintln!("Configuring radio...");
    let radio_init = esp_radio::init().expect("Radio init failed");
    let (_wifi, _interfaces) =
        esp_radio::wifi::new(&radio_init, peripherals.WIFI, Default::default())
            .expect("WiFi init failed");
    rprintln!("Radio ready");

    // === Display Init ===
    rprintln!("Configuring display...");
    let spi_bus = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(Rate::from_mhz(40))
            .with_mode(esp_hal::spi::Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO36)
    .with_mosi(peripherals.GPIO37);

    let cs = Output::new(peripherals.GPIO3, Level::High, OutputConfig::default());
    let dc = Output::new(peripherals.GPIO35, Level::Low, OutputConfig::default());
    let reset = Output::new(peripherals.GPIO15, Level::High, OutputConfig::default());

    let spi_device = ExclusiveDevice::new(spi_bus, cs, esp_hal::delay::Delay::new()).unwrap();
    let mut spi_buffer = [0u8; 512];
    let di = SpiInterface::new(spi_device, dc, &mut spi_buffer);

    let mut display = MipidsiBuilder::new(ILI9342CRgb565, di)
        .reset_pin(reset)
        .display_size(DISPLAY_WIDTH, DISPLAY_HEIGHT)
        .color_order(ColorOrder::Bgr)
        .invert_colors(ColorInversion::Inverted)
        .init(&mut embassy_time::Delay)
        .expect("Display init failed");

    rprintln!("Display ready");
    rprintln!("=== Hardware initialization complete ===\n");

    // === Application: Display Test ===
    draw_test_screen(&mut display);

    let _ = spawner;

    // === Main Loop ===
    loop {
        rprintln!("Running...");
        Timer::after(Duration::from_secs(1)).await;
    }
}

/// Draw test pattern on display
fn draw_test_screen<D>(display: &mut D)
where
    D: embedded_graphics::draw_target::DrawTarget<Color = Rgb565>,
{
    let _ = Rectangle::new(Point::new(0, 0), Size::new(320, 240))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
        .draw(display);

    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
    let _ = Text::new("balls", Point::new(60, 120), text_style).draw(display);

    rprintln!("Display test complete");
}
