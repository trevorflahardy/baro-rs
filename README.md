# baro-rs

[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-ESP32--S3-blue.svg)](https://www.espressif.com/en/products/socs/esp32-s3)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green.svg)](LICENSE)

**baro-rs** is a production-grade, long-term environmental monitoring system written entirely in Rust for the ESP32-S3 platform. Designed to run continuously for months or years, it measures, logs, and visualizes environmental data with exceptional reliability and extensibility.

This is **not a prototype**—it's firmware built to last, with clean architecture, robust error handling, and a modular design that welcomes expansion.

---

## 🎯 Project Vision

### Current State
A fully functional environmental monitoring device that:
- Continuously samples temperature and humidity (SHT40 sensor)
- Stores data persistently on microSD with intelligent rollups
- Displays real-time readings and historical trends on a color LCD
- Synchronizes time via SNTP over Wi-Fi
- Handles errors gracefully (missing SD card, sensor failures, network issues)
- Runs efficiently on battery or USB power

### Long-Term Goal
**A comprehensive, single-PCB environmental monitoring platform with 20+ sensors**, providing:
- Complete environmental profiling (temperature, humidity, pressure, light, air quality, CO₂, VOC, etc.)
- Multi-year data retention with smart aggregation
- Beautiful, customizable UI with multiple visualization modes
- Wi-Fi connectivity for remote monitoring and OTA updates
- Open, documented APIs for integration with other systems
- Community-driven sensor and feature expansion
- Plug-and-play architecture for easy hardware customization

---

## ✨ Features

### Core Capabilities

#### 📊 Data Collection & Storage
- **Continuous sampling** at 10-second intervals
- **Intelligent data rollups** across multiple time scales (5m, 1h, 24h, 7d, 1m, all-time)
- **Fixed-size, versioned records** for reliability and predictable wear
- **Append-only architecture** optimized for SD card longevity
- **Graceful degradation** when SD card is missing or fails
- Supports up to **20 sensor values per sample**

#### 📈 Visualization & Display
- Real-time sensor readings with customizable home screen
- **Historical trend graphs** with smooth curves and animations
- Multiple time windows (1m, 5m, 30m, 1h, 12h, 1d, 1w)
- Touch-based navigation (FT6336U capacitive touch)
- Clean, modern UI with consistent theming
- Settings page for configuration

#### 🌐 Connectivity
- **Wi-Fi support** for network time synchronization
- **SNTP time sync** for accurate timestamps
- UTC-based time storage with local display conversion
- Handles offline operation gracefully

#### 🔧 System Design
- **Modular, extensible architecture**
- Centralized error handling through `AppError`
- Safe I²C/SPI bus sharing with async coordination
- Embassy-based async runtime for efficient multitasking
- Comprehensive logging via RTT

### Current Hardware Support

#### Primary Board
- **M5Stack CoreS3 SE** (ESP32-S3 dual-core Xtensa LX7)
  - 320×240 ILI9342C RGB565 LCD (SPI)
  - MicroSD card slot (SPI)
  - Internal I²C bus (GPIO12 SDA, GPIO11 SCL)
  - Capacitive touch screen (FT6336U)
  - AXP2101 PMIC for power management
  - AW9523 GPIO expander
  - 8MB PSRAM

#### Active Sensors
- **SHT40** (I²C) — temperature and humidity with high accuracy

#### Supported Peripherals
- **TCA9548A** I²C multiplexer for sensor expansion (up to 8 channels)

---

## 🚀 Planned Features & Expansion

### Near-Term (Next 6 Months)
- [ ] Barometric pressure sensor (BMP390 / BME680)
- [ ] Ambient light sensor (BH1750 / VEML7700)
- [ ] Additional time windows and rollup statistics
- [ ] Settings persistence on SD card
- [ ] Battery level monitoring and display
- [ ] Sleep mode for extended battery life

### Medium-Term (6-12 Months)
- [ ] CO₂ sensor (SCD40/SCD41)
- [ ] VOC/IAQ sensor (SGP40 / BME688)
- [ ] Advanced metrics: dew point, heat index, air quality indices
- [ ] Configurable alerts and thresholds
- [ ] Data export formats (CSV, JSON)
- [ ] Web server for local data access
- [ ] Multiple screen themes

### Long-Term Vision
- [ ] **Custom PCB with 20+ integrated sensors**
  - Temperature, humidity, pressure (multiple sensors)
  - CO₂, VOC, formaldehyde, particulate matter (PM2.5/PM10)
  - UV index, ambient light, color temperature
  - Sound level monitoring
  - Current/power monitoring
  - Magnetic field sensing
  - Motion/vibration detection
- [ ] **OTA firmware updates** via Wi-Fi
- [ ] **Remote monitoring** via MQTT/HTTP API
- [ ] **Cloud integration** (optional, privacy-first)
- [ ] **Multi-device synchronization** for distributed sensing
- [ ] **Plugin system** for community sensor drivers
- [ ] **Configurable data retention policies**
- [ ] **E-ink display option** for ultra-low-power deployments
- [ ] **Solar charging support**
- [ ] **User-scriptable alerts** (threshold, rate-of-change, anomaly detection)

---

## 🏗️ Architecture

### Project Structure

```
baro-rs/
├── src/
│   ├── bin/
│   │   └── main.rs              # Application entry point & task spawning
│   ├── app_state/               # Global state management
│   │   ├── hardware.rs          # Hardware initialization & abstraction
│   │   ├── sensors_state.rs     # Sensor state & coordination
│   │   └── mod.rs               # State types & error handling
│   ├── sensors/                 # Sensor drivers & abstractions
│   │   ├── sht40.rs             # SHT40 temperature/humidity
│   │   └── mod.rs               # Sensor trait & indexed sensor system
│   ├── storage/                 # Data persistence & rollups
│   │   ├── sd_card.rs           # SD card interface
│   │   ├── rollup_storage.rs    # Time-series rollup engine
│   │   ├── accumulator.rs       # In-memory sample buffering
│   │   └── manager.rs           # Storage coordination
│   ├── display_manager.rs       # Display task & rendering coordination
│   ├── pages/                   # UI screens
│   │   ├── home.rs              # Main sensor display
│   │   ├── trend.rs             # Historical graphs
│   │   ├── settings.rs          # Configuration UI
│   │   └── wifi_error.rs        # Network error handling
│   ├── ui/                      # UI framework
│   │   ├── core.rs              # Base UI primitives
│   │   ├── components/          # Reusable UI components
│   │   ├── layouts/             # Layout containers
│   │   └── styling/             # Theme & styling system
│   ├── widgets.rs               # Specialized display widgets
│   ├── metrics.rs               # Derived metric calculations
│   ├── async_i2c_bus.rs         # Async I²C bus multiplexing
│   ├── dual_mode_pin.rs         # GPIO pin state management
│   └── config.rs                # Configuration types
├── AGENTS.md                    # Development philosophy & standards
├── STORAGE.md                   # Detailed storage architecture
├── Cargo.toml                   # Dependencies & build config
└── rust-toolchain.toml          # Rust version pinning
```

### Key Design Principles

1. **Modularity First** — Each subsystem is isolated with clear boundaries
2. **Type-Safe Hardware** — Compile-time guarantees for sensor indexing and bus access
3. **Error Handling** — Centralized `AppError` type, no panics in production code
4. **Async-First** — Embassy executor for efficient multitasking
5. **Zero Magic** — Explicit, readable code over clever abstractions
6. **Production-Grade** — Designed for years of unattended operation

### Storage Architecture

The storage system is optimized for **long-term operation** on SD cards:

- **Raw samples** (10s interval): 24-hour ring buffer
- **5-minute rollups**: 30-day retention (avg/min/max)
- **1-hour rollups**: 1-year retention
- **Daily rollups**: Multi-year retention
- **All-time statistics**: Lifetime min/max/avg

All data structures use **fixed-size records** with **version headers** for forward compatibility. See [STORAGE.md](STORAGE.md) for detailed design.

---

## 🛠️ Getting Started

### Prerequisites

1. **Rust toolchain** (1.88+) with `xtensa` target support
2. **espflash** for flashing firmware
3. **M5Stack CoreS3 SE** board
4. MicroSD card (16GB recommended, FAT32 formatted)

### Building

```bash
# Clone the repository
git clone https://github.com/trevorflahardy/baro-rs.git
cd baro-rs

# Check code (runs automatically in CI)
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check

# Build for ESP32-S3
cargo build --release

# Flash to device
espflash flash --monitor target/xtensa-esp32s3-none-elf/release/baro-rs
```

### Configuration

Edit `src/wifi_secrets.rs` to configure Wi-Fi credentials:

```rust
pub const WIFI_SSID: &str = "YourNetworkName";
pub const WIFI_PASSWORD: &str = "YourPassword";
```

---

## 📚 Documentation

- **[AGENTS.md](AGENTS.md)** — Code philosophy, style guide, and contribution requirements
- **[STORAGE.md](STORAGE.md)** — Detailed storage architecture and data lifecycle
- **Module docs** — Run `cargo doc --open` for inline documentation

---

## 🤝 Contributing

We welcome contributions that align with the project's philosophy:

### Before Contributing
1. Read [AGENTS.md](AGENTS.md) thoroughly
2. Ensure your code passes:
   - `cargo check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo fmt`
3. Test on hardware when modifying HAL or peripheral code

### What We're Looking For
- New sensor drivers (with type-safe integration)
- UI improvements and new visualizations
- Storage optimizations
- Documentation improvements
- Bug fixes and error handling improvements

### Contribution Standards
- **Clarity over cleverness** — readable code wins
- **No magic numbers** — use named constants with units
- **Robust error handling** — anticipate failures
- **Modular design** — keep functions small and focused
- **Test your changes** — verify on hardware when applicable

---

## 🧪 Testing

```bash
# Run clippy (all warnings treated as errors)
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt --all

# Build for host (limited functionality)
cargo check

# Build for ESP32-S3
cargo build --target xtensa-esp32s3-none-elf
```

---

## 📊 Performance Characteristics

- **Boot time**: ~2-3 seconds to first sensor reading
- **Sampling rate**: 10 seconds (configurable)
- **Display refresh**: 200ms (5 Hz)
- **SD write frequency**: ~1 minute (batched for wear leveling)
- **Memory usage**: ~80KB heap, extensive PSRAM utilization
- **Power consumption**: TBD (active development)

---

## 🔒 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

---

## 🙏 Acknowledgments

Built with:
- [esp-hal](https://github.com/esp-rs/esp-hal) — ESP32 hardware abstraction
- [embassy](https://embassy.dev/) — Async runtime for embedded
- [embedded-graphics](https://github.com/embedded-graphics/embedded-graphics) — Display graphics
- [embedded-sdmmc](https://github.com/rust-embedded-community/embedded-sdmmc-rs) — SD card support
- [mipidsi](https://github.com/almindor/mipidsi) — Display driver

---

## 📬 Contact & Support

- **Issues**: [GitHub Issues](https://github.com/trevorflahardy/baro-rs/issues)
- **Discussions**: [GitHub Discussions](https://github.com/trevorflahardy/baro-rs/discussions)

---

## 🎯 Project Status

**Active Development** — Core functionality complete, actively expanding features and sensor support.

**Hardware tested**: M5Stack CoreS3 SE
**Current sensor count**: 1 (SHT40)
**Target sensor count**: 20+

---

**Built with care. Designed to last. Open to all.**
