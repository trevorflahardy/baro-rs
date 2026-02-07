# CLAUDE.md

## Build & Development

```bash
# Build (targets xtensa-esp32s3-none-elf via .cargo/config.toml)
cargo build --release

# Check / lint / format (all three must pass, clippy warnings = errors)
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all

# Flash and monitor
espflash flash --monitor target/xtensa-esp32s3-none-elf/release/baro-rs
```

**Toolchain:** `esp` channel (see `rust-toolchain.toml`). Uses `build-std = ["alloc", "core"]`.

**WiFi secrets:** Copy `.env.example` to `.env` and set `WIFI_SSID` / `WIFI_PASSWORD`. The build script (`build.rs`) bakes them into the binary at compile time via `env!()`.

## Code Standards

These are the non-obvious rules from AGENTS.md that are easy to violate:

- **No magic numbers** — constants must have descriptive names with units (e.g. `LOG_FLUSH_INTERVAL_SECS`, not `60`)
- **No panics in production code** — use `Result`/`Option`; all errors flow through the centralized `AppError` type
- **Imports at top of file** — never inside functions unless absolutely necessary
- **Clippy warnings are errors** — fix or explicitly justify any exception
- **`rustfmt` is mandatory** — do not fight it
- **Clarity over cleverness** — this firmware runs for years unattended; prefer simple, readable code

See [AGENTS.md](AGENTS.md) for the full philosophy.

## Architecture Overview

### Async Task Model
Embassy executor on ESP32-S3 (dual-core Xtensa LX7). The main entry point (`src/bin/main.rs`) spawns long-lived async tasks for sensing, storage, display, and networking.

### Data Flow
```
Sensors → Accumulator → PubSub (RollupEvent) → Storage Manager (SD card)
                                              → Display Manager (UI)
```

- **Sensors** read every 10s into a shared `[i32; MAX_SENSORS]` values array
- **Accumulator** (`src/storage/accumulator.rs`) buffers samples in RAM and generates rollups (5m, 1h, daily) when thresholds are met
- **PubSub** — `ROLLUP_CHANNEL` (embassy `PubSubChannel`) distributes `RollupEvent` variants to 2 subscribers: storage and UI
- **Storage** writes tiered data to SD card (raw ring buffer + append-only rollup files). See [STORAGE.md](STORAGE.md)

### UI Framework
Custom component-based UI in `src/ui/` (core primitives, components, layouts, styling). Pages live in `src/pages/` (home, trend, settings, wifi_error). Display requests flow through a `Channel` to `DisplayManager`.

### Type-Safe Sensor System
Sensors use const generics to guarantee correct array indexing at compile time:

```rust
// IndexedSensor<S, START, COUNT, MUX_CHANNEL>
type SHT40Indexed<I> = IndexedSensor<SHT40Sensor<I>, 0, 2, 0>;  // temp+humidity at [0..2], mux ch 0
type SCD41Indexed<I> = IndexedSensor<SCD41Sensor<I>, 2, 1, 1>;  // CO2 at [2], mux ch 1
```

Named index constants (`TEMPERATURE = 0`, `HUMIDITY = 1`, `CO2 = 2`) in `sensors::indices`. Sensor features are gated (`sensor-sht40`, `sensor-scd41`).

### Dual-Mode Pin
`DualModePin<const PIN: u8>` (`src/dual_mode_pin.rs`) uses raw register manipulation to switch a GPIO between input/output modes at runtime. Used because GPIO35 serves as both SPI MISO (input for SD card) and DC signal (output for LCD) on the shared SPI bus.

## Key Hardware Constraints

- **Shared SPI bus** — LCD (ILI9342C) and SD card share a single SPI peripheral; bus access must be serialized
- **GPIO35 dual-mode** — shared between MISO (SD) and DC (LCD); wrapped with `OutputModeSpiDevice` / `InputModeSpiDevice`
- **I2C mux** — TCA9548A multiplexer on internal I2C bus (GPIO12 SDA, GPIO11 SCL) routes to sensor channels
- **Board** — M5Stack CoreS3 SE with AXP2101 PMIC, AW9523 GPIO expander, FT6336U touch, 8MB PSRAM
