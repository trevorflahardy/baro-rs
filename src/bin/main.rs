// cSpell: disable
#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::cell::RefCell;
use critical_section::Mutex as CsMutex;
use embassy_executor::Spawner;
use embassy_net::{Config as EmbassyNetConfig, StackResources};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex as AsyncMutex;
use embassy_time::{Duration, Timer};
use embedded_sdmmc::SdCard;
use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    i2c::master::Config as I2cConfig,
    spi::master::{Config as SpiConfig, Spi},
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_radio::wifi::ClientConfig;
use static_cell::StaticCell;

use rtt_target::rprintln;

use aw9523_embedded::r#async::Aw9523Async;
use axp2101_embedded::AsyncAxp2101;
use baro_rs::{
    async_i2c_bus::AsyncI2cDevice,
    dual_mode_pin::{DualModePin, DualModePinAsOutput, InputModeSpiDevice, OutputModeSpiDevice},
    sensors::{SHT40Indexed, SHT40Sensor},
    storage::MAX_SENSORS,
    wifi_secrets,
};
use embedded_hal_bus::spi::CriticalSectionDevice as SpiCriticalSectionDevice;
use ft6336u_driver::FT6336U;
use mipidsi::{
    Builder as MipidsiBuilder,
    interface::SpiInterface,
    models::ILI9342CRgb565,
    options::{ColorInversion, ColorOrder},
};

static NET_RESOURCES: StaticCell<StackResources<8>> = StaticCell::new();

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

    // === Radio Init ===
    rprintln!("Configuring SVC...");
    // esp_idf_svc::sys::link_patches();
    // esp_idf_svc::EspLogger::initialize_default();

    rprintln!("Configuring radio...");
    let radio_init = esp_radio::init().expect("Radio init failed");
    let (mut wifi, interfaces) =
        esp_radio::wifi::new(&radio_init, peripherals.WIFI, Default::default())
            .expect("WiFi init failed");

    rprintln!("Radio ready");

    // ==== Loading Wifi Credentials ====
    rprintln!("Connecting to WiFi SSID: {}", wifi_secrets::WIFI_SSID);

    let client_config = ClientConfig::default()
        .with_ssid(wifi_secrets::WIFI_SSID.into())
        .with_password(wifi_secrets::WIFI_PASSWORD.into());

    wifi.set_config(&esp_radio::wifi::ModeConfig::Client(client_config))
        .unwrap();

    // TODO: Later, need connection timeout management and retry logic here... ideally after the display has been set up so we can render connection status to the user.
    wifi.start_async().await.unwrap();
    wifi.connect_async().await.unwrap();

    rprintln!("Initializing SNTP...");
    // Open a new UDP socket for SNTP and pass it off
    let resources = NET_RESOURCES.init(StackResources::new());
    let net_config = EmbassyNetConfig::dhcpv4(Default::default());
    let (_stack, _runner) = embassy_net::new(interfaces.sta, net_config, resources, 1024);
    // spawner.spawn(runner).unwrap();

    rprintln!("WiFi connected");

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

    static I2C0_BUS: StaticCell<
        AsyncMutex<CriticalSectionRawMutex, esp_hal::i2c::master::I2c<'static, esp_hal::Async>>,
    > = StaticCell::new();
    let i2c0_bus = I2C0_BUS.init(AsyncMutex::new(i2c0));

    let i2c_for_axp = AsyncI2cDevice::new(i2c0_bus);
    let i2c_for_aw = AsyncI2cDevice::new(i2c0_bus);
    let i2c_for_touch = AsyncI2cDevice::new(i2c0_bus);
    let i2c_for_sht4x = AsyncI2cDevice::new(i2c0_bus);

    let mut power_mgmt_chip = AsyncAxp2101::new(i2c_for_axp);

    match power_mgmt_chip.init().await {
        Ok(_) => rprintln!("Power management ready"),
        Err(e) => rprintln!("Power init failed: {:?}", e),
    }

    power_mgmt_chip
        .set_charging_led_mode(axp2101_embedded::ChargeLedMode::On)
        .await
        .unwrap();

    // 0xBF = 0b10111111
    // Enable all ALDO and BLDO and DLDO except for
    // CPUSLDO
    power_mgmt_chip.enable_aldo1().await.unwrap();
    power_mgmt_chip.enable_aldo2().await.unwrap();
    power_mgmt_chip.enable_aldo3().await.unwrap();
    power_mgmt_chip.enable_aldo4().await.unwrap();
    power_mgmt_chip.enable_bldo1().await.unwrap();
    power_mgmt_chip.enable_bldo2().await.unwrap();
    power_mgmt_chip.enable_dldo1().await.unwrap();

    // aldo 4 voltage to 3.3V for display
    power_mgmt_chip.set_aldo4_voltage(3300).await.unwrap();

    // === GPIO Expander ===
    rprintln!("Configuring GPIO expander...");
    let mut gpio_expander = Aw9523Async::new(i2c_for_aw, 0x58);
    gpio_expander.init().await.unwrap();

    // Configure P1_2 (pin 10) as input for touch interrupt from FT6336U
    gpio_expander
        .pin_mode(10, aw9523_embedded::PinMode::Input)
        .await
        .unwrap();
    // Enable interrupt detection on P1_2 so it triggers the AW9523B's INTN pin
    gpio_expander.enable_interrupt(10, true).await.unwrap();

    rprintln!("GPIO expander ready (P1_2 configured for touch interrupt)");

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

    let spi_bus = CsMutex::new(RefCell::new(spi_bus_inner));

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

    rprintln!(
        "Touch controller ready (library version: 0x{:04X}, chip ID: 0x{:02X}, g_mode: 0x{:02X} [Polling])",
        library_version,
        chip_id,
        g_mode
    );

    rprintln!("=== Hardware initialization complete ===\n");

    // === Spawn Touch Polling Task ===
    rprintln!("Starting touch polling task...");
    spawner.spawn(touch_polling_task(touch_interface)).ok();
    spawner
        .spawn(background_sensor_reading_task(i2c_for_sht4x))
        .ok();

    use embedded_graphics::prelude::RgbColor;

    // === Main Loop ===
    rprintln!("Main loop running...\n");
    loop {
        // Draw black to the display to show it's alive and not have the display
        // render weird. Of course, we'll be changing this later.
        display
            .set_pixels(
                0,
                0,
                DISPLAY_WIDTH,
                DISPLAY_HEIGHT,
                [embedded_graphics::pixelcolor::Rgb565::BLACK;
                    (DISPLAY_HEIGHT as usize * DISPLAY_WIDTH as usize)],
            )
            .unwrap();

        Timer::after(Duration::from_secs(10)).await;
    }
}

#[embassy_executor::task]
async fn background_sensor_reading_task(
    sht4x_i2c: AsyncI2cDevice<'static, esp_hal::i2c::master::I2c<'static, esp_hal::Async>>,
) {
    rprintln!("Sensor reading task started");

    let mut _sht4x_sensor = SHT40Indexed::from(SHT40Sensor::new(sht4x_i2c));

    let mut _values = [0i32; MAX_SENSORS];

    loop {
        // ... TODO: On each iteration, read all sensors into the values array and then
        // process/store the data as needed ...
        Timer::after(Duration::from_millis(10)).await;
    }
}

/// Async task for polling touch input
#[embassy_executor::task]
async fn touch_polling_task(
    mut touch: FT6336U<AsyncI2cDevice<'static, esp_hal::i2c::master::I2c<'static, esp_hal::Async>>>,
) {
    rprintln!("Touch polling task started");

    loop {
        // Poll the touch controller
        match touch.scan().await {
            Ok(touch_data) => {
                if touch_data.touch_count > 0 {
                    for i in 0..touch_data.touch_count as usize {
                        let point = &touch_data.points[i];
                        rprintln!(
                            "🖐️ Touch {}: x={}, y={} (status: {:?})",
                            i,
                            point.x,
                            point.y,
                            point.status
                        );
                    }
                }
            }
            Err(e) => {
                rprintln!("Touch scan error: {:?}", e);
            }
        }

        Timer::after(Duration::from_millis(10)).await;
    }
}
