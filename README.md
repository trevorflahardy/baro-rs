# baro-rs

[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-ESP32--S3-blue.svg)](https://www.espressif.com/en/products/socs/esp32-s3)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green.svg)](LICENSE)

**baro-rs** is a production-grade, long-term environmental monitoring system written entirely in Rust for the ESP32-S3 platform. Designed to run continuously for months or years, it measures, logs, and visualizes environmental data with exceptional reliability and extensibility.

To date, it would be very hard for you, as a user, to reproduce this project on your own. The hardware is complex, the software is non-trivial, and the design is optimized for long-term operation rather than ease of replication.

Have no fear! With enough time, I hope for this project to evolve out of its breadboard phase and into a completely polished, single-PCB design with 20+ integrated sensors. At that point, it will be much easier for anyone to build (or buy) their own device, use it, and even contribute to the project.

<img width="414" height="356" alt="image" src="https://github.com/user-attachments/assets/3aa8cdf3-0e02-4e15-ad62-3fa980b84ee4" />
<img width="414" height="356" alt="image" src="https://github.com/user-attachments/assets/095a26be-16b5-4113-8366-6ff56dbe6902" />


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
- [x] CO₂ sensor (SCD40/SCD41)
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

### Workspace Structure

The project is organized as a Cargo workspace with three crates:

```
crates/
├── baro-core/        # Hardware-independent core library
│   └── src/
│       ├── app_state.rs       # Generic application state
│       ├── display_manager.rs # Page rendering pipeline
│       ├── framebuffer.rs     # Framebuffer with dirty tracking
│       ├── pages/             # UI pages (Home, Trend, Settings, …)
│       ├── sensors/           # Sensor traits and driver wrappers
│       ├── storage/           # Rollup engine, SD card abstraction
│       └── ui/                # Layout system, components, styling
├── baro-firmware/    # ESP32-S3 binary (production firmware)
│   └── src/
│       ├── app_state/         # Hardware init, concrete sensor state
│       ├── dual_mode_pin.rs   # GPIO register manipulation
│       └── bin/main.rs        # Firmware entry point
└── baro-simulator/   # Desktop simulator (SDL2 window)
    └── src/
        └── main.rs            # SDL2 event loop with mock sensors
```

- **baro-core** — `#![no_std]` library containing all platform-agnostic logic.
  Compiles on both Xtensa and standard Rust toolchains.
- **baro-firmware** — ESP32-S3 binary that wires up real hardware peripherals.
- **baro-simulator** — Desktop binary that renders the same pages in an SDL2
  window with synthetic sensor data, no hardware required.

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

## �️ Desktop Simulator

Develop and test UI pages without physical hardware. The simulator renders the
same `baro-core` pages inside an SDL2 window using
[`embedded-graphics-simulator`](https://docs.rs/embedded-graphics-simulator).

### Running

```bash
# Install SDL2 (macOS)
brew install sdl2

# Run the simulator
make sim
# or: cargo run -p baro-simulator
```

### Key Bindings

| Key | Page              |
|-----|-------------------|
| 1   | Home              |
| 2   | Temperature trend |
| 3   | Humidity trend    |
| 4   | CO₂ trend         |
| 5   | Settings          |
| 6   | Wi-Fi error       |
| Q   | Quit              |

Mouse clicks are forwarded as touch events.

### What It Simulates

- Page rendering at 320×240 Rgb565 (scaled 2× on-screen)
- Sinusoidal mock sensor data (temperature, humidity, CO₂)
- Trend pages pre-loaded with synthetic history
- Touch-based navigation between pages

### What It Does Not Simulate

- SD card I/O and storage persistence
- Real Wi-Fi, SNTP, or networking
- Embassy async runtime (uses a simple synchronous loop)
- Power management and sleep modes

---

## �🛠️ Getting Started

### Prerequisites

1. **Rust toolchain** (1.88+) with `xtensa` target support (ESP channel)
2. **espflash** for flashing firmware
3. **SDL2** for the desktop simulator (`brew install sdl2`)
4. **M5Stack CoreS3 SE** board (for firmware deployment)
5. MicroSD card (16GB recommended, FAT32 formatted)

### Building

```bash
# Clone the repository
git clone https://github.com/trevorflahardy/baro-rs.git
cd baro-rs

# --- Firmware (ESP32-S3) ---
make fw-check          # Type-check firmware
make fw                # Build firmware (debug)
make fw-release        # Build firmware (release, LTO)
make fw-clippy         # Lint firmware

# --- Simulator (desktop) ---
make sim-check         # Type-check simulator
make sim               # Run the desktop simulator
make sim-clippy        # Lint simulator

# --- Both ---
make check-all         # Type-check everything
make clippy-all        # Lint everything
make fmt               # Format all code
make fmt-check         # Check formatting (CI)

# --- Flash to device ---
espflash flash --monitor target/xtensa-esp32s3-none-elf/release/baro-firmware
```

Without `make`, use the underlying Cargo commands directly:

```bash
# Firmware
cargo build --target xtensa-esp32s3-none-elf -Z build-std=alloc,core
cargo clippy --target xtensa-esp32s3-none-elf -Z build-std=alloc,core -- -D warnings

# Simulator
cargo run -p baro-simulator
cargo clippy -p baro-simulator -- -D warnings
```

---

## 📚 Documentation

- **[AGENTS.md](AGENTS.md)** — Code philosophy, style guide, and contribution requirements
- **[STORAGE.md](STORAGE.md)** — Detailed storage architecture and data lifecycle
- **Simulator** — `make sim` to run; see the [Desktop Simulator](#-desktop-simulator) section above
- **Module docs** — Run `cargo doc --open -p baro-core` for inline documentation

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
# Lint everything (warnings = errors)
make clippy-all

# Format all code
make fmt

# Type-check firmware + simulator
make check-all

# Run the simulator for visual/interactive testing
make sim
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
