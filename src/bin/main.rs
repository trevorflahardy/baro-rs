// cSpell: disable
#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use baro_rs::app_state::{
    AppError, AppRunState, AppState, FromUnchecked, GlobalStateType, ROLLUP_CHANNEL, SensorsState,
    create_i2c_bus, init_i2c_hardware,
};
use baro_rs::display_manager::{DisplayManager, DisplayRequest, get_display_receiver};
use baro_rs::storage::{
    MAX_SENSORS, accumulator::RollupEvent, manager::StorageManager, sd_card::SdCardManager,
};
use core::cell::RefCell;
use critical_section::Mutex as CsMutex;
use embassy_executor::Spawner;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{Config as EmbassyNetConfig, IpListenEndpoint, Runner, StackResources};
use embassy_net::{IpAddress, IpEndpoint, Ipv4Address};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex as AsyncMutex;
use embassy_time::{Duration, Timer};
use embedded_sdmmc::SdCard;
use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    spi::master::{Config as SpiConfig, Spi},
    time::Rate,
    timer::timg::TimerGroup,
};
use esp_radio::Controller;
use esp_radio::wifi::{ClientConfig, WifiDevice};
use heapless::String;
use static_cell::StaticCell;

use rtt_target::rprintln;

use baro_rs::{
    dual_mode_pin::{DualModePin, DualModePinAsOutput, InputModeSpiDevice, OutputModeSpiDevice},
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
static RADIO_INIT: StaticCell<Controller<'static>> = StaticCell::new();

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

/// Synchronize time with an NTP server using UDP
///
/// This function sends an NTP request and parses the response to get the current
/// Unix timestamp. The time can then be used to set the system clock for accurate
/// timestamping of sensor data and rollups.
async fn udp_time_sync(stack: &embassy_net::Stack<'static>) -> Result<u32, AppError> {
    // Wait for network to be configured
    stack.wait_config_up().await;

    // NTP server: pool.ntp.org
    let ntp_server = IpEndpoint::new(IpAddress::v4(162, 159, 200, 1), 123);
    let local_port = 12345;

    // UDP socket buffers
    let mut rx_meta: [PacketMetadata; 4] = [PacketMetadata::EMPTY; 4];
    let mut rx_buf: [u8; 128] = [0; 128];
    let mut tx_meta: [PacketMetadata; 4] = [PacketMetadata::EMPTY; 4];
    let mut tx_buf: [u8; 128] = [0; 128];

    let mut socket = UdpSocket::new(*stack, &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf);

    socket
        .bind(IpListenEndpoint {
            addr: Some(IpAddress::v4(0, 0, 0, 0)),
            port: local_port,
        })
        .map_err(|_| AppError::TimeSync(String::from_unchecked("UDP bind failed")))?;

    // NTP request packet (48 bytes, first byte 0x1B)
    let mut ntp_packet = [0u8; 48];
    ntp_packet[0] = 0x1B;

    socket
        .send_to(&ntp_packet, ntp_server)
        .await
        .map_err(|_| AppError::TimeSync(String::from_unchecked("UDP send failed")))?;

    let mut recv_buf = [0u8; 64];
    let (len, _endpoint) = socket
        .recv_from(&mut recv_buf)
        .await
        .map_err(|_| AppError::TimeSync(String::from_unchecked("UDP recv failed")))?;

    if len < 48 {
        return Err(AppError::TimeSync(String::from_unchecked(
            "NTP response too short",
        )));
    }

    // Parse NTP response (Transmit Timestamp: bytes 40..44)
    let secs = u32::from_be_bytes([recv_buf[40], recv_buf[41], recv_buf[42], recv_buf[43]]);
    // NTP epoch starts in 1900, Unix in 1970
    let unix_time = secs.wrapping_sub(2_208_988_800);
    rprintln!("NTP time: {} (unix)", unix_time);

    Ok(unix_time)
}

/// Simple time source for embedded-sdmmc that uses a counter
/// In production, this should use the actual RTC or system time from NTP
struct SimpleTimeSource {
    counter: core::cell::RefCell<u32>,
}

impl SimpleTimeSource {
    fn new() -> Self {
        Self {
            counter: core::cell::RefCell::new(0),
        }
    }
}

impl embedded_sdmmc::TimeSource for SimpleTimeSource {
    fn get_timestamp(&self) -> embedded_sdmmc::Timestamp {
        let count = *self.counter.borrow();
        *self.counter.borrow_mut() = count.wrapping_add(1);

        // Return a default timestamp (2024-01-01 00:00:00)
        embedded_sdmmc::Timestamp {
            year_since_1970: 54,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

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
    rprintln!("Configuring radio...");
    let radio_init = RADIO_INIT.init(esp_radio::init().expect("Radio init failed"));
    let (mut wifi, interfaces) =
        esp_radio::wifi::new(radio_init, peripherals.WIFI, Default::default())
            .expect("WiFi init failed");

    rprintln!("Radio ready");

    // ==== Loading Wifi Credentials ====
    rprintln!("Connecting to WiFi SSID: {}", wifi_secrets::WIFI_SSID);

    let client_config = ClientConfig::default()
        .with_ssid(wifi_secrets::WIFI_SSID.into())
        .with_password(wifi_secrets::WIFI_PASSWORD.into());

    wifi.set_config(&esp_radio::wifi::ModeConfig::Client(client_config))
        .unwrap();
    wifi.start_async().await.unwrap();

    let wifi_result = wifi.connect_async().await;
    let wifi_connected = wifi_result.is_ok();

    if wifi_connected {
        rprintln!("WiFi connected");
    } else {
        rprintln!("WiFi connection failed: {:?}", wifi_result.err());
    }

    // === Network Stack Setup ===
    let resources = NET_RESOURCES.init(StackResources::new());
    let net_config = EmbassyNetConfig::dhcpv4(Default::default());
    let (stack, runner) = embassy_net::new(interfaces.sta, net_config, resources, 1024);

    static STACK: StaticCell<embassy_net::Stack<'static>> = StaticCell::new();
    let stack_ref = STACK.init(stack);

    spawner.spawn(task_wifi_runner(runner)).unwrap();

    // === Time Synchronization ===
    let mut time_known = false;
    if wifi_connected {
        rprintln!("Performing time sync...");
        match udp_time_sync(stack_ref).await {
            Ok(timestamp) => {
                rprintln!("Time sync successful: {}", timestamp);
                time_known = true;
            }
            Err(e) => {
                rprintln!("Time sync failed: {:?}", e);
            }
        }
    }

    // === Hardware Initialization (in correct order) ===

    // 1. I2C and power management (CRITICAL - must be first)
    let i2c0 = create_i2c_bus(peripherals.I2C0, peripherals.GPIO12, peripherals.GPIO11);
    let (_i2c_hardware, i2c_for_touch, i2c_for_sensors) = init_i2c_hardware(i2c0).await;

    // 2. SPI devices (display and SD card)
    rprintln!("Configuring SPI devices...");
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
        "Touch controller ready (library: 0x{:04X}, chip: 0x{:02X}, mode: 0x{:02X})",
        library_version,
        chip_id,
        g_mode
    );

    rprintln!("=== Hardware initialization complete ===\n");

    // === Application State Setup ===
    let time_source = SimpleTimeSource::new();
    let sd_card_manager = SdCardManager::new(sd_card, time_source);
    let storage_manager = StorageManager::new(sd_card_manager);

    static APP_STATE = StaticCell::new();
    let mut app_state = AppState::new();
    app_state.wifi_connected = wifi_connected;
    app_state.time_known = time_known;
    app_state.run_state = if wifi_connected {
        AppRunState::WifiConnected
    } else {
        AppRunState::Error
    };
    app_state.init_accumulator();
    app_state.set_storage_manager(storage_manager);

    let app_state_ref = APP_STATE.init(AsyncMutex::new(app_state));

    // === Spawn Background Tasks ===

    // Only start sensor tasks if WiFi connected successfully
    if wifi_connected && sd_card_size > 0 {
        rprintln!("Starting sensor and storage tasks...");

        // Create sensors state
        let sensors = SensorsState::new(i2c_for_sensors);
        spawner
            .spawn(background_sensor_reading_task(sensors, app_state_ref))
            .ok();

        // Start storage event processing task
        spawner
            .spawn(storage_event_processing_task(app_state_ref))
            .ok();

        rprintln!("Sensor and storage tasks started");
    } else {
        rprintln!("Skipping sensor tasks - WiFi not connected or SD card unavailable");
    }

    // Start touch polling task
    spawner.spawn(touch_polling_task(touch_interface)).ok();

    // Start display manager task
    let display_manager = DisplayManager::new(display);
    spawner.spawn(display_manager_task(display_manager)).ok();

    rprintln!("All tasks spawned\n");

    // === Main Loop ===
    rprintln!("Main loop running...\n");
    loop {
        Timer::after(Duration::from_secs(10)).await;
    }
}

#[embassy_executor::task]
async fn task_wifi_runner(mut runner: Runner<'static, WifiDevice<'static>>) {
    rprintln!("WiFi runner task started");
    runner.run().await;
}

/// Background task for reading sensors and publishing rollup events
///
/// This task:
/// 1. Reads all sensors every 10 seconds
/// 2. Creates a RawSample with the current timestamp
/// 3. Dispatches the sample to the accumulator via the app state
#[embassy_executor::task]
async fn background_sensor_reading_task(
    mut sensors: SensorsState<'static>,
    app_state: &'static GlobalStateType<
        'static,
        impl embedded_hal::spi::SpiDevice<u8>,
        impl embedded_hal::delay::DelayNs,
        impl embedded_sdmmc::TimeSource,
    >,
)
{
    rprintln!("Sensor reading task started");

    let mut timestamp: u32 = 0;

    loop {
        let mut values = [0i32; MAX_SENSORS];

        // Read all sensors
        sensors.read_all(&mut values).await;

        // Add sample to accumulator via app state
        {
            let mut state = app_state.lock().await;
            if let Some(accumulator) = state.accumulator_mut() {
                accumulator.add_sample(timestamp, &values).await;
            }
        }

        timestamp = timestamp.wrapping_add(10);
        Timer::after(Duration::from_secs(10)).await;
    }
}

/// Background task for processing rollup events and storing them
///
/// This task:
/// 1. Subscribes to the rollup event channel
/// 2. Receives events from the accumulator
/// 3. Passes events to the storage manager for persistence
#[embassy_executor::task]
async fn storage_event_processing_task(
    app_state: &'static GlobalStateType<
        'static,
        impl embedded_hal::spi::SpiDevice<u8>,
        impl embedded_hal::delay::DelayNs,
        impl embedded_sdmmc::TimeSource,
    >,
) {
    rprintln!("Storage event processing task started");

    // Subscribe to rollup events
    let mut subscriber = ROLLUP_CHANNEL.subscriber().unwrap();

    loop {
        // Wait for next rollup event
        let event = subscriber.next_message_pure().await;

        // Process event through storage manager
        {
            let mut state = app_state.lock().await;
            if let Some(storage) = state.storage_manager_mut() {
                storage.process_event(event).await;
            }
        }

        // Also send to display for updates
        let display_sender = baro_rs::display_manager::get_display_sender();
        let _ = display_sender.try_send(DisplayRequest::UpdateData(event));
    }
}

/// Async task for polling touch input
#[embassy_executor::task]
async fn touch_polling_task(
    mut touch: FT6336U<
        baro_rs::async_i2c_bus::AsyncI2cDevice<
            'static,
            esp_hal::i2c::master::I2c<'static, esp_hal::Async>,
        >,
    >,
) {
    rprintln!("Touch polling task started");

    loop {
        match touch.scan().await {
            Ok(touch_data) => {
                if touch_data.touch_count > 0 {
                    for i in 0..touch_data.touch_count as usize {
                        let point = &touch_data.points[i];

                        // Convert touch to our TouchEvent and send to display
                        let touch_point = baro_rs::ui_core::TouchPoint {
                            x: point.x,
                            y: point.y,
                        };

                        let event = match point.status {
                            ft6336u_driver::TouchStatus::Contact => {
                                baro_rs::ui_core::TouchEvent::Press(touch_point)
                            }
                            ft6336u_driver::TouchStatus::Lift => {
                                baro_rs::ui_core::TouchEvent::Release(touch_point)
                            }
                        };

                        let display_sender = baro_rs::display_manager::get_display_sender();
                        let _ = display_sender.try_send(DisplayRequest::HandleTouch(event));
                    }
                }
            }
            Err(e) => {
                rprintln!("Touch scan error: {:?}", e);
            }
        }

        Timer::after(Duration::from_millis(50)).await;
    }
}

/// Display manager task for rendering pages
#[embassy_executor::task]
async fn display_manager_task(
    mut display_manager: DisplayManager<
        impl embedded_graphics::prelude::DrawTarget<Color = embedded_graphics::pixelcolor::Rgb565>,
    >,
) {
    let receiver = get_display_receiver();
    display_manager.run(receiver).await;
}
