// cSpell: disable
#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
// Note: large_stack_frames warnings are expected in async embassy tasks
// due to Future state machines. These are monitored but not denied.

use alloc::boxed::Box;
use baro_rs::app_state::{
    AppError, AppRunState, AppState, FromUnchecked, GlobalStateType, ROLLUP_CHANNEL, SensorsState,
    create_i2c_bus, init_i2c_hardware, init_spi_peripherals,
};
use baro_rs::display_manager::{
    DisplayManager, DisplayRequest, get_display_receiver, get_display_sender,
};
use baro_rs::storage::{MAX_SENSORS, manager::StorageManager, sd_card::SdCardManager};
use baro_rs::ui::core::PageId;
use embassy_executor::Spawner;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{Config as EmbassyNetConfig, IpListenEndpoint, Runner, StackResources};
use embassy_net::{IpAddress, IpEndpoint};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex as AsyncMutex;
use embassy_time::{Duration, Timer};
use esp_hal::{clock::CpuClock, gpio::Output, spi::master::Spi, timer::timg::TimerGroup};
use esp_radio::Controller;
use esp_radio::wifi::{ClientConfig, WifiController, WifiDevice};
use heapless::String;
use static_cell::StaticCell;

use log::{debug, error, info};

use baro_rs::{
    dual_mode_pin::{DualModePin, DualModePinAsOutput, InputModeSpiDevice, OutputModeSpiDevice},
    wifi_secrets,
};
use embedded_hal_bus::spi::CriticalSectionDevice as SpiCriticalSectionDevice;
use ft6336u_driver::{FT6336U, TouchStatus};
use mipidsi::{interface::SpiInterface, models::ILI9342CRgb565};

// ====== Concrete Type Definitions for App State ======
// These concrete types are required because embassy tasks cannot use generics or `impl Trait`

/// Type alias for the SPI device used by the SD card
/// InputModeSpiDevice wraps a CriticalSectionDevice for the SD card CS pin
type SdCardSpiDevice = InputModeSpiDevice<
    SpiCriticalSectionDevice<
        'static,
        Spi<'static, esp_hal::Async>,
        Output<'static>,
        esp_hal::delay::Delay,
    >,
    35,
>;

/// Type alias for the delay implementation used throughout the app
type DelayImpl = esp_hal::delay::Delay;

/// Type alias for the time source used by embedded-sdmmc
type TimeSourceImpl = SimpleTimeSource;

/// Type alias for the concrete global state type
type ConcreteGlobalStateType = GlobalStateType<'static, SdCardSpiDevice, DelayImpl, TimeSourceImpl>;

/// Type alias for the SPI device used by the display
/// OutputModeSpiDevice wraps a CriticalSectionDevice for the display CS pin
type DisplaySpiDevice = OutputModeSpiDevice<
    SpiCriticalSectionDevice<
        'static,
        Spi<'static, esp_hal::Async>,
        Output<'static>,
        esp_hal::delay::Delay,
    >,
    35,
>;

/// Type alias for the display interface (SPI + DC pin)
type DisplayInterface<'a> = SpiInterface<'a, DisplaySpiDevice, DualModePinAsOutput<35>>;

/// Type alias for the complete display type used throughout the application
type DisplayType = mipidsi::Display<DisplayInterface<'static>, ILI9342CRgb565, Output<'static>>;

static NET_RESOURCES: StaticCell<StackResources<8>> = StaticCell::new();
static WIFI_CONTROLLER: StaticCell<WifiController<'static>> = StaticCell::new();
static RADIO_INIT: StaticCell<Controller<'static>> = StaticCell::new();

const DISPLAY_WIDTH: u16 = 320;
const DISPLAY_HEIGHT: u16 = 240;

// Static dual-mode pin for GPIO35 (shared between SD card MISO and display DC)
static GPIO35_PIN: DualModePin<35> = DualModePin::new();

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error!("PANIC: {}", info);
    loop {}
}

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

/// Synchronize time with an NTP server using UDP
///
/// This function sends an NTP request and parses the response to get the current
/// Unix timestamp. The time can then be used to set the system clock for accurate
/// timestamping of sensor data and rollups.
#[allow(clippy::large_stack_frames)]
async fn udp_time_sync(stack: &embassy_net::Stack<'static>) -> Result<u32, AppError> {
    use embassy_time::with_timeout;

    // Wait for network to be configured
    stack.wait_config_up().await;

    info!("Network configured, starting NTP time sync");

    // Print our IP address for debugging
    if let Some(config) = stack.config_v4() {
        info!("Our IP: {}", config.address.address());
        info!("Gateway: {:?}", config.gateway);
        info!("DNS: {:?}", config.dns_servers);
    } else {
        error!("WARNING: No IPv4 config available yet");
    }

    // NTP servers to try (pool.ntp.org and time.google.com)
    let ntp_servers = [
        IpEndpoint::new(IpAddress::v4(162, 159, 200, 1), 123), // pool.ntp.org
        IpEndpoint::new(IpAddress::v4(216, 239, 35, 0), 123),  // time.google.com
        IpEndpoint::new(IpAddress::v4(216, 239, 35, 4), 123),  // time.google.com
    ];

    // Try each server
    for (i, &ntp_server) in ntp_servers.iter().enumerate() {
        info!("Trying NTP server #{}: {}", i + 1, ntp_server);

        // UDP socket buffers
        let mut rx_meta: [PacketMetadata; 4] = [PacketMetadata::EMPTY; 4];
        let mut rx_buf: [u8; 128] = [0; 128];
        let mut tx_meta: [PacketMetadata; 4] = [PacketMetadata::EMPTY; 4];
        let mut tx_buf: [u8; 128] = [0; 128];

        let mut socket =
            UdpSocket::new(*stack, &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf);

        // Bind to any port (let OS choose)
        if let Err(e) = socket.bind(IpListenEndpoint {
            addr: None, // Changed from Some to None - let stack choose
            port: 0,    // Changed from fixed port to 0 - let OS assign
        }) {
            info!("UDP bind failed: {:?}", e);
            continue;
        }

        info!("Socket bound successfully");

        // NTP request packet (48 bytes, first byte 0x1B)
        // 0x1B = LI=0 (no warning), VN=3 (version 3), Mode=3 (client)
        let mut ntp_packet = [0u8; 48];
        ntp_packet[0] = 0x1B;

        info!("Sending NTP request to {}", ntp_server);

        if let Err(e) = socket.send_to(&ntp_packet, ntp_server).await {
            error!("UDP send failed: {:?}", e);
            continue;
        }

        info!("NTP request sent successfully, waiting for response...");
        // Add timeout to recv_from (5 seconds)
        let mut recv_buf = [0u8; 64];
        let recv_result =
            with_timeout(Duration::from_secs(5), socket.recv_from(&mut recv_buf)).await;

        match recv_result {
            Ok(Ok((len, endpoint))) => {
                info!("NTP response received from {} ({} bytes)", endpoint, len);

                if len < 48 {
                    info!("NTP response too short: {} bytes", len);
                    continue;
                }

                // Parse NTP response (Transmit Timestamp: bytes 40..44)
                let secs =
                    u32::from_be_bytes([recv_buf[40], recv_buf[41], recv_buf[42], recv_buf[43]]);
                // NTP epoch starts in 1900, Unix in 1970
                let unix_time = secs.wrapping_sub(2_208_988_800);
                info!("NTP time: {} (unix)", unix_time);

                return Ok(unix_time);
            }
            Ok(Err(e)) => {
                error!("UDP recv failed: {:?}", e);
                continue;
            }
            Err(_) => {
                error!("NTP request timed out after 5 seconds");
                continue;
            }
        }
    }

    Err(AppError::TimeSync(String::from_unchecked(
        "All NTP servers failed",
    )))
}

/// Simple time source for embedded-sdmmc that uses actual Unix time
struct SimpleTimeSource {
    /// Unix timestamp (seconds since 1970-01-01)
    unix_time: core::cell::RefCell<u32>,
}

impl SimpleTimeSource {
    fn new(initial_time: u32) -> Self {
        Self {
            unix_time: core::cell::RefCell::new(initial_time),
        }
    }

    /// Update the current Unix time
    #[allow(dead_code)]
    fn set_time(&self, unix_time: u32) {
        *self.unix_time.borrow_mut() = unix_time;
    }

    /// Get current Unix time
    #[allow(dead_code)]
    fn get_unix_time(&self) -> u32 {
        *self.unix_time.borrow()
    }
}

impl embedded_sdmmc::TimeSource for SimpleTimeSource {
    fn get_timestamp(&self) -> embedded_sdmmc::Timestamp {
        let unix_time = *self.unix_time.borrow();

        // Convert Unix timestamp to FAT timestamp
        // This is a simplified conversion - for production use a proper datetime library
        const SECONDS_PER_DAY: u32 = 86400;
        const SECONDS_PER_HOUR: u32 = 3600;
        const SECONDS_PER_MINUTE: u32 = 60;

        // Days since Unix epoch (1970-01-01)
        let days_since_epoch = unix_time / SECONDS_PER_DAY;
        let seconds_today = unix_time % SECONDS_PER_DAY;

        // Approximate year calculation (ignoring leap years for simplicity)
        let years_since_1970 = (days_since_epoch / 365).min(255) as u8;
        let days_this_year = days_since_epoch % 365;
        let month = (days_this_year / 30).min(11) as u8;
        let day = (days_this_year % 30) as u8;

        let hours = (seconds_today / SECONDS_PER_HOUR) as u8;
        let minutes = ((seconds_today % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE) as u8;
        let seconds = (seconds_today % SECONDS_PER_MINUTE) as u8;

        embedded_sdmmc::Timestamp {
            year_since_1970: years_since_1970,
            zero_indexed_month: month,
            zero_indexed_day: day,
            hours,
            minutes,
            seconds,
        }
    }
}
/// Initialize and connect WiFi
///
/// This function:
/// - Initializes the radio and WiFi peripheral
/// - Configures WiFi client with SSID and password
/// - Attempts to connect to the network
///
/// # Returns
/// A tuple of (interfaces, wifi_connected) where:
/// - interfaces: Network interfaces
/// - wifi_connected: Whether connection was successful
#[allow(clippy::large_stack_frames)]
async fn setup_wifi(
    radio_init: &'static mut Controller<'static>,
    wifi_peripheral: esp_hal::peripherals::WIFI<'static>,
) -> (esp_radio::wifi::Interfaces<'static>, bool) {
    info!("Configuring radio...");
    let (wifi, interfaces) = esp_radio::wifi::new(radio_init, wifi_peripheral, Default::default())
        .expect("WiFi init failed");
    let wifi = WIFI_CONTROLLER.init(wifi);

    info!("Radio ready");
    info!("Connecting to WiFi SSID: {}", wifi_secrets::WIFI_SSID);

    let client_config = ClientConfig::default()
        .with_ssid(wifi_secrets::WIFI_SSID.into())
        .with_password(wifi_secrets::WIFI_PASSWORD.into());

    wifi.set_config(&esp_radio::wifi::ModeConfig::Client(client_config))
        .unwrap();
    wifi.start_async().await.unwrap();

    let wifi_result = wifi.connect_async().await;
    let wifi_connected = wifi_result.is_ok();

    if wifi_connected {
        info!("WiFi connected");
    } else {
        error!("WiFi connection failed: {:?}", wifi_result.err());
    }

    (interfaces, wifi_connected)
}

/// Setup network stack and wait for configuration
///
/// This function:
/// - Initializes the embassy-net stack with DHCP
/// - Spawns the network runner task
/// - Waits for link up and DHCP configuration
///
/// # Returns
/// Static reference to the network stack
async fn setup_network_stack(
    interfaces: esp_radio::wifi::Interfaces<'static>,
    spawner: &Spawner,
) -> &'static embassy_net::Stack<'static> {
    let resources = NET_RESOURCES.init(StackResources::new());
    let net_config = EmbassyNetConfig::dhcpv4(Default::default());
    let (stack, runner) = embassy_net::new(interfaces.sta, net_config, resources, 1024);

    static STACK: StaticCell<embassy_net::Stack<'static>> = StaticCell::new();
    let stack_ref = STACK.init(stack);

    // Spawn network runner task
    spawner.spawn(task_wifi_runner(runner)).unwrap();

    // Wait for link up
    loop {
        if stack_ref.is_link_up() {
            break;
        }
        info!("Waiting for network link...");
        Timer::after(Duration::from_secs(1)).await;
    }

    info!("Network link is up!");
    info!("Waiting for network configuration (DHCP)...");
    stack_ref.wait_config_up().await;

    // Give the network stack a moment to stabilize
    Timer::after(Duration::from_millis(500)).await;
    info!("Network fully configured and ready");

    stack_ref
}

/// Perform time synchronization via NTP
///
/// # Returns
/// Optional Unix timestamp if sync was successful
#[allow(clippy::large_stack_frames)]
#[allow(clippy::large_stack_frames)]
async fn sync_time(stack: &embassy_net::Stack<'static>) -> Option<u32> {
    info!("Performing time sync...");
    match udp_time_sync(stack).await {
        Ok(timestamp) => {
            info!("Time sync successful: {}", timestamp);
            Some(timestamp)
        }
        Err(e) => {
            error!("Time sync failed: {:?}", e);
            None
        }
    }
}
/// Initialize application state with storage manager
///
/// This function sets up the application state including:
/// - SimpleTimeSource with synced time
/// - SD card manager with time source
/// - Storage manager with rollup loading
/// - App state with WiFi and time status
///
/// # Arguments
/// - `sd_card`: The SD card instance
/// - `time`: Optional Unix timestamp from NTP sync
/// - `wifi_connected`: Whether WiFi connection was successful
///
/// # Returns
/// A tuple of (app_state_ref, initial_time) where:
/// - app_state_ref: Static reference to the app state wrapped in AsyncMutex
/// - initial_time: The Unix timestamp to use for sensor readings (0 if no sync)
async fn setup_app_state(
    sd_card: embedded_sdmmc::SdCard<SdCardSpiDevice, DelayImpl>,
    time: Option<u32>,
    wifi_connected: bool,
) -> (
    &'static AsyncMutex<
        CriticalSectionRawMutex,
        AppState<'static, SdCardSpiDevice, DelayImpl, TimeSourceImpl>,
    >,
    u32,
) {
    let initial_time = time.unwrap_or(0);
    let time_source = SimpleTimeSource::new(initial_time);
    let sd_card_manager = SdCardManager::new(sd_card, time_source);
    let mut storage_manager = StorageManager::new(sd_card_manager);

    if let Some(t) = time {
        info!("Initializing storage manager with synced time: {}", t);
        match storage_manager.init(t).await {
            Ok(_) => info!("Storage manager initialized successfully"),
            Err(e) => error!("Storage manager initialization failed: {:?}", e),
        }
    } else {
        error!("Storage manager initialized without time sync (using fallback)");
    }

    static APP_STATE: StaticCell<ConcreteGlobalStateType> = StaticCell::new();
    let mut app_state = AppState::new();
    app_state.wifi_connected = wifi_connected;
    app_state.time_known = time.is_some();
    app_state.run_state = if wifi_connected {
        AppRunState::WifiConnected
    } else {
        AppRunState::Error
    };
    app_state.init_accumulator();
    app_state.set_storage_manager(storage_manager);

    let app_state_ref = APP_STATE.init(AsyncMutex::new(app_state));

    (app_state_ref, initial_time)
}

#[allow(clippy::large_stack_frames)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // === Core System Init ===
    rtt_target::rtt_init_log!(log::LevelFilter::Debug);

    // Initialize logger with Info level
    info!("Logger initialized");

    let hal_config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(hal_config);

    // 225KB for heap in internal RAM
    esp_alloc::heap_allocator!(size: 74_000);
    esp_alloc::psram_allocator!(&peripherals.PSRAM, esp_hal::psram);

    info!("PSRAM global allocator initialized (8MB)");
    info!(
        "Heap allocation completed: {} bytes used / {} bytes free",
        esp_alloc::HEAP.used(),
        esp_alloc::HEAP.free()
    );

    let timer_group = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timer_group.timer0);
    info!("Core system initialized");

    // === Initialize Radio ===
    let radio_init = RADIO_INIT.init(esp_radio::init().expect("Radio init failed"));

    // === Concurrent Initialization: WiFi + Hardware ===
    // Run WiFi setup and hardware initialization in parallel to speed up boot time
    info!("Starting concurrent WiFi and hardware initialization...");

    // WiFi setup future
    let wifi_future = setup_wifi(radio_init, peripherals.WIFI);

    // Hardware initialization future
    let hardware_future = async {
        // 1. I2C hardware (power management, GPIO expander, touch controller)
        let i2c0 = create_i2c_bus(peripherals.I2C0, peripherals.GPIO12, peripherals.GPIO11);

        let (i2c_hardware, i2c_for_sensors) = init_i2c_hardware(i2c0).await;

        // 2. SPI hardware (display and SD card)
        let spi_hardware = init_spi_peripherals(
            peripherals.SPI2,
            peripherals.GPIO3,  // display_cs_pin
            peripherals.GPIO4,  // sd_card_cs_pin
            peripherals.GPIO15, // display_reset_pin
            &GPIO35_PIN,        // dual_mode_pin
            peripherals.GPIO36, // spi_sck_pin
            peripherals.GPIO37, // spi_mosi_pin
            peripherals.GPIO35, // spi_miso_pin
            DISPLAY_WIDTH,
            DISPLAY_HEIGHT,
        );

        return (i2c_hardware, i2c_for_sensors, spi_hardware);
    };

    info!("Spawning concurrent initialization tasks...");

    // Both futures should complete around the same time
    let ((interfaces, wifi_connected), (i2c_hardware, i2c_mux, spi_hardware)) =
        embassy_futures::join::join(wifi_future, hardware_future).await;

    info!("=== Concurrent initialization complete ===\n");

    let touch_interface = i2c_hardware.touch_interface;
    let display = spi_hardware.display;
    let sd_card = spi_hardware.sd_card;
    #[cfg(any(feature = "sensor-sht40", feature = "sensor-scd41"))]
    let sd_card_size = spi_hardware.sd_card_size;
    #[cfg(not(any(feature = "sensor-sht40", feature = "sensor-scd41")))]
    let _sd_card_size = spi_hardware.sd_card_size;

    // === Network Stack Setup (only if WiFi connected) ===
    let (_stack_ref, time) = if wifi_connected {
        let stack_ref = setup_network_stack(interfaces, &spawner).await;
        let time = sync_time(stack_ref).await;
        (Some(stack_ref), time)
    } else {
        (None, None)
    };

    // === Application State Setup ===
    #[cfg(any(feature = "sensor-sht40", feature = "sensor-scd41"))]
    let (app_state_ref, initial_time) = setup_app_state(sd_card, time, wifi_connected).await;

    #[cfg(not(any(feature = "sensor-sht40", feature = "sensor-scd41")))]
    let (_app_state_ref, _initial_time) = setup_app_state(sd_card, time, wifi_connected).await;

    // === Spawn Background Tasks ===

    // Start touch polling task
    spawner.spawn(touch_polling_task(touch_interface)).ok();

    // Start display manager task
    let display_manager = DisplayManager::new(display);
    spawner.spawn(display_manager_task(display_manager)).ok();

    // Navigate to appropriate page based on WiFi status
    if !wifi_connected {
        info!("Navigating to WiFi error page");
        let display_sender = get_display_sender();
        display_sender
            .send(DisplayRequest::NavigateToPage(PageId::WifiError))
            .await;
    }

    // Only start sensor tasks if WiFi connected successfully and sensors are enabled
    #[cfg(any(feature = "sensor-sht40", feature = "sensor-scd41"))]
    if wifi_connected && sd_card_size > 0 {
        info!("Starting sensor and storage tasks...");

        // Create sensors state
        let sensors = { SensorsState::new(i2c_mux) };

        spawner
            .spawn(background_sensor_reading_task(
                sensors,
                app_state_ref,
                initial_time,
            ))
            .ok();

        // Start storage event processing task
        spawner
            .spawn(storage_event_processing_task(app_state_ref))
            .ok();

        info!("Sensor and storage tasks started");
    } else {
        info!("Skipping sensor tasks - WiFi not connected or SD card unavailable");
    }

    #[cfg(not(any(feature = "sensor-sht40", feature = "sensor-scd41")))]
    info!("No sensors enabled - sensor tasks will not start");

    info!("All tasks spawned\n");

    // === Main Loop ===
    info!("Main loop running...\n");
    loop {
        Timer::after(Duration::from_secs(10)).await;
    }
}

#[embassy_executor::task]
async fn task_wifi_runner(mut runner: Runner<'static, WifiDevice<'static>>) {
    info!("WiFi runner task started");
    runner.run().await
}

/// Background task for reading sensors and publishing rollup events
///
/// This task:
/// 1. Reads all sensors every 10 seconds
/// 2. Creates a RawSample with the current timestamp
/// 3. Dispatches the sample to the accumulator via the app state
#[allow(clippy::large_stack_frames)]
#[embassy_executor::task]
async fn background_sensor_reading_task(
    mut sensors: SensorsState<'static>,
    app_state: &'static ConcreteGlobalStateType,
    initial_unix_time: u32,
) {
    info!(
        "Sensor reading task started with initial time: {}",
        initial_unix_time
    );

    let mut timestamp: u32 = initial_unix_time;

    loop {
        // Read all sensors
        let values = match sensors.read_all().await {
            Ok(v) => v,
            Err(e) => {
                error!("Sensor read error: {:?}", e);
                Timer::after(Duration::from_secs(10)).await;
                continue;
            }
        };

        debug!(
            "Sensor readings at {} (unix time): {:?}",
            timestamp,
            &values[..MAX_SENSORS]
        );

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
#[allow(clippy::large_stack_frames)]
#[embassy_executor::task]
async fn storage_event_processing_task(app_state: &'static ConcreteGlobalStateType) {
    info!("Storage event processing task started");

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
        let _ = display_sender.try_send(DisplayRequest::UpdateData(Box::new(event)));
    }
}

/// Async task for polling touch input
#[allow(clippy::large_stack_frames)]
#[embassy_executor::task]
async fn touch_polling_task(
    mut touch: FT6336U<
        baro_rs::async_i2c_bus::AsyncI2cDevice<
            'static,
            esp_hal::i2c::master::I2c<'static, esp_hal::Async>,
        >,
    >,
) {
    info!("Touch polling task started");

    loop {
        match touch.scan().await {
            Ok(touch_data) => {
                if touch_data.touch_count > 0 {
                    for i in 0..touch_data.touch_count as usize {
                        let point = &touch_data.points[i];

                        // Convert touch to our TouchEvent and send to display
                        let touch_point = baro_rs::ui::TouchPoint {
                            x: point.x,
                            y: point.y,
                        };

                        // TODO: Handle Release events properly
                        // For now, always send a Press event
                        let event = match point.status {
                            TouchStatus::Touch => baro_rs::ui::TouchEvent::Press(touch_point),
                            TouchStatus::Stream => baro_rs::ui::TouchEvent::Drag(touch_point),
                            _ => baro_rs::ui::TouchEvent::Press(touch_point), // <- Release does not ever be fired (?)
                        };

                        let display_sender = baro_rs::display_manager::get_display_sender();
                        let _ = display_sender.try_send(DisplayRequest::HandleTouch(event));
                    }
                }
            }
            Err(e) => {
                error!("Touch scan error: {:?}", e);
            }
        }

        Timer::after(Duration::from_millis(5)).await;
    }
}

/// Display manager task for rendering pages
#[embassy_executor::task]
async fn display_manager_task(mut display_manager: DisplayManager<DisplayType>) {
    let receiver = get_display_receiver();
    display_manager.run(receiver).await;
}
