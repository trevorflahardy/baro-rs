# CLAUDE.md

## Build & Development

This is a **three-crate workspace** (firmware + core library + desktop simulator). There is no global build target — use the Makefile or pass explicit flags.

```bash
# ── Firmware (ESP32-S3) ──────────────────────────────────────────────
make fw              # Debug build
make fw-release      # Release build (LTO, opt-level "s")
make fw-check        # Type-check only
make fw-clippy       # Lint (clippy warnings = errors)

# ── Simulator (desktop, SDL2) ───────────────────────────────────────
make sim             # Run the desktop simulator
make sim-check       # Type-check only
make sim-clippy      # Lint

# ── Both ─────────────────────────────────────────────────────────────
make check-all       # Type-check firmware + simulator
make clippy-all      # Lint everything
make fmt             # Format all code
make fmt-check       # Check formatting (CI)

# ── Flash and monitor ────────────────────────────────────────────────
espflash flash --monitor target/xtensa-esp32s3-none-elf/release/baro-firmware
```

**Important:** Do NOT use bare `cargo build` / `cargo check` — the workspace has no default target. The firmware requires `--target xtensa-esp32s3-none-elf -Z build-std=alloc,core` (the Makefile handles this).

**Toolchain:** `esp` channel (see `rust-toolchain.toml`). Uses `build-std = ["alloc", "core"]` for firmware only.

**WiFi secrets:** Copy `.env.example` to `.env` and set `WIFI_SSID` / `WIFI_PASSWORD`. The build script (`build.rs`) bakes them into the binary at compile time via `env!()`.

**Simulator prereq:** SDL2 must be installed (`brew install sdl2` on macOS). The `.cargo/config.toml` points `aarch64-apple-darwin` rustflags at `/opt/homebrew/lib`.

## Code Standards

These are the non-obvious rules from AGENTS.md that are easy to violate:

- **No magic numbers** — constants must have descriptive names with units (e.g. `LOG_FLUSH_INTERVAL_SECS`, not `60`)
- **No panics in production code** — use `Result`/`Option`; all errors flow through the centralized `AppError` type
- **Imports at top of file** — never inside functions unless absolutely necessary
- **Clippy warnings are errors** — fix or explicitly justify any exception
- **`rustfmt` is mandatory** — do not fight it
- **Clarity over cleverness** — this firmware runs for years unattended; prefer simple, readable code

See [AGENTS.md](AGENTS.md) for the full philosophy.

## Workspace Structure

```
baro-rs/
├── crates/
│   ├── baro-core/        # Platform-agnostic core library (#![no_std] + alloc)
│   │                     # UI, pages, sensors, storage, display — anything
│   │                     # that doesn't touch real hardware
│   ├── baro-firmware/    # ESP32-S3 binary, hardware init, WiFi, GPIO, main.rs
│   └── baro-simulator/   # Desktop simulator (SDL2 via embedded-graphics-simulator)
├── Makefile              # Canonical build commands
├── .cargo/config.toml    # Per-target rustflags (no global target)
└── rust-toolchain.toml   # esp channel
```

**Design principle:** All UI, page, sensor, and storage logic lives in `baro-core` so it can compile on both the ESP32 target and the host (for the simulator). `baro-firmware` only contains hardware-specific code — pin setup, I2C/SPI bus init, WiFi, and the async task entry points.

## Architecture Overview

### Async Task Model
Embassy executor on ESP32-S3 (dual-core Xtensa LX7). The main entry point (`crates/baro-firmware/src/bin/main.rs`) spawns long-lived async tasks for sensing, storage, display, and networking.

### Data Flow
```
Sensors → Accumulator → PubSub (RollupEvent) → Storage Manager (SD card)
                                              → Display Manager (UI)
```

- **Sensors** read every 10s into a shared `[i32; MAX_SENSORS]` values array (`MAX_SENSORS = 20`)
- **Accumulator** (`baro-core/src/storage/accumulator.rs`) buffers samples in RAM and generates rollups when thresholds are met
- **Rollup tiers:** `RawSample`, `FiveMinute`, `Hourly`, `Daily`
- **Time windows:** 1m, 5m, 30m, 1h, 6h, 1d, 1w
- **PubSub** — `ROLLUP_CHANNEL` (embassy `PubSubChannel`) distributes `RollupEvent` variants to 2 subscribers: storage and UI
- **Storage** writes tiered data to SD card (raw ring buffer + append-only rollup files). See [STORAGE.md](STORAGE.md)

### Type-Safe Sensor System

Sensors use const generics to guarantee correct array indexing at compile time. Defined in `baro-core/src/sensors/`:

```rust
// IndexedSensor<S, START, COUNT, MUX_CHANNEL>
type SHT40Indexed<I>  = IndexedSensor<SHT40Sensor<I>,  0, 2, 0>;  // temp+humidity at [0..2], mux ch 0
type SCD41Indexed<I>  = IndexedSensor<SCD41Sensor<I>,  2, 1, 1>;  // CO2 at [2],             mux ch 1
type BH1750Indexed<I> = IndexedSensor<BH1750Sensor<I>, 3, 1, 2>;  // lux at [3],             mux ch 2
```

Named index constants in `sensors::indices`:
| Constant      | Index | Sensor | I2C Mux Ch |
|---------------|-------|--------|------------|
| `TEMPERATURE` | 0     | SHT40  | 0          |
| `HUMIDITY`    | 1     | SHT40  | 0          |
| `CO2`         | 2     | SCD41  | 1          |
| `LUX`         | 3     | BH1750 | 2          |

Sensors are feature-gated (all enabled by default):
- `sensor-sht40` → `sht4x` crate
- `sensor-scd41` → `scd41-embedded` (git, async)
- `sensor-bh1750` → `bh1750-embedded` (git, async)

### UI Framework

Custom component-based UI in `baro-core/src/ui/`:

- **Core** (`core.rs`) — `Action`, `PageEvent`, `PageId`, `SensorData`, `TouchEvent`
- **Components** — `TextComponent`, `Button`, `Graph` (with series, axis, grid, viewport, interpolation)
- **Layouts** — `Container<N>` (flex-like with alignment/spacing), `ScrollableContainer`
- **Styling** — `Theme`, `Style`, color palette, font constants
- **Display** — 320×240 pixels (`DISPLAY_WIDTH_PX`, `DISPLAY_HEIGHT_PX`)

### Pages

All pages in `baro-core/src/pages/`, implementing the `Page` trait (`draw_page`, `handle_touch`, `on_event`, `update`, `is_dirty`/`mark_clean`):

| Page             | File                | Purpose                                  |
|------------------|---------------------|------------------------------------------|
| `HomePage`       | `home.rs`           | Dashboard with current sensor readings   |
| `TrendPage`      | `trend/page.rs`     | Time-series graphs (switchable windows)  |
| `SettingsPage`   | `settings.rs`       | Device settings                          |
| `WifiStatusPage` | `wifi_status.rs`    | WiFi connection status (Connecting/Error)|

`PageManager` handles page transitions. `PageWrapper` enum wraps all page types.

### Networking & Time Sync

- WiFi credentials baked at compile time from `.env`
- DHCP-based network configuration via `embassy-net`
- NTP time sync via UDP (pool.ntp.org, time.google.com fallbacks)
- `SimpleTimeSource` converts Unix timestamps to FAT format for SD card

### Dual-Mode Pin
`DualModePin<const PIN: u8>` (`baro-firmware/src/dual_mode_pin.rs`) uses raw register manipulation to switch a GPIO between input/output modes at runtime. Used because GPIO35 serves as both SPI MISO (input for SD card) and DC signal (output for LCD) on the shared SPI bus. Wrapped in `OutputModeSpiDevice` / `InputModeSpiDevice`.

### Desktop Simulator

`baro-simulator` (`crates/baro-simulator/src/main.rs`) renders the same `baro-core` UI on desktop via SDL2:

- `MockSensorGenerator` produces synthetic sinusoidal sensor data
- Keyboard navigation: keys 1–6 switch pages, Q quits
- Mouse clicks forwarded as touch events
- ~30 FPS frame rate pacing

## Key Hardware Constraints

- **Shared SPI bus** — LCD (ILI9342C) and SD card share a single SPI peripheral; bus access must be serialized
- **GPIO35 dual-mode** — shared between MISO (SD) and DC (LCD); wrapped with `OutputModeSpiDevice` / `InputModeSpiDevice`
- **I2C mux** — TCA9548A multiplexer on internal I2C bus (GPIO12 SDA, GPIO11 SCL) routes to sensor channels
- **Board** — M5Stack CoreS3 SE with AXP2101 PMIC, AW9523 GPIO expander, FT6336U touch, 8MB PSRAM
- **PSRAM** — 8MB available; used for framebuffer allocation
