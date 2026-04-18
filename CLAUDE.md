# CLAUDE.md

Guidance for Claude Code working in this repository. Read [AGENTS.md](AGENTS.md) for the full project philosophy; this file captures the non-obvious, operational rules.

## Build & Development

Three-crate workspace: `baro-core` (platform-agnostic library), `baro-firmware` (ESP32-S3 binary), `baro-simulator` (desktop SDL2 simulator). The firmware is the workspace `default-members` target, but it requires `--target xtensa-esp32s3-none-elf -Z build-std=alloc,core` — so bare `cargo build` / `cargo check` will **not** work. Always go through the Makefile.

```bash
# Firmware (ESP32-S3)
make fw              # Debug build
make fw-release      # Release build (LTO "fat", opt-level "s", single codegen unit)
make fw-check        # Type-check
make fw-clippy       # Lint (-D warnings)
make run / run-release  # cargo run on device

# Simulator (desktop, SDL2)
make sim
make sim-check
make sim-clippy

# Cross-cutting
make check-all       # Type-check firmware + simulator
make clippy-all      # Lint everything
make fmt             # cargo fmt --all
make fmt-check       # cargo fmt --all -- --check

# Flash & monitor
espflash flash --monitor target/xtensa-esp32s3-none-elf/release/baro-firmware
```

**Toolchain:** `esp` channel pinned in `rust-toolchain.toml`. `build-std = ["alloc", "core"]` for firmware only. Rust edition 2024, MSRV 1.88.

**WiFi secrets:** copy `.env.example` → `.env` and set `WIFI_SSID` / `WIFI_PASSWORD`. `baro-firmware/build.rs` bakes them in via `env!()` at compile time.

**Simulator prereq:** SDL2 (`brew install sdl2` on macOS). `.cargo/config.toml` points `aarch64-apple-darwin` rustflags at `/opt/homebrew/lib`.

## Code Quality Rules

These are mandatory — CI and review enforce them.

### Verification before returning a task
After any code change, run **`make fmt-check`** and **`make clippy-all`** (or at minimum `make sim-clippy` if the ESP toolchain isn't available locally). Fix every failure before reporting completion. Clippy warnings are treated as errors (`-D warnings`).

### Style
- **No magic numbers.** Constants must have descriptive names with units: `LOG_FLUSH_INTERVAL_SECS`, `DISPLAY_WIDTH_PX`, `DISPLAY_REFRESH_HZ`. No bare literals in logic.
- **No panics in production code.** Use `Result` / `Option`. All errors flow through the centralized `AppError` (core) and `SensorError` (sensor layer) types — expand these rather than introducing ad-hoc error types.
- **Imports at top of file.** Never inside functions unless there's a concrete reason (feature-gated cycle, etc.).
- **`rustfmt` is mandatory.** Do not fight it. Don't commit unformatted code.
- **Clarity over cleverness.** This firmware runs unattended for months/years. Prefer short functions, meaningful names, and obvious control flow over micro-optimizations.
- **Self-documenting code first.** Add doc comments only when behavior is non-obvious, hardware is subtle, or invariants must be preserved. Don't restate the code.
- **Small modules, single responsibility.** If a file is sprawling, split it. The `pages/`, `ui/`, `sensors/`, and `storage/` trees already follow this pattern.
- **Hardware access is centralized.** Shared buses (SPI, I²C) must be serialized and race-free by construction. Don't scatter raw GPIO/bus manipulation outside the `hal` / firmware layer.
- **Timestamps are UTC Unix epoch.** Convert to local time for display only.

## Workspace Structure

```
baro-rs/
├── crates/
│   ├── baro-core/        # #![no_std] + alloc, platform-agnostic
│   ├── baro-firmware/    # ESP32-S3 binary (Embassy, esp-hal)
│   └── baro-simulator/   # Desktop simulator (embedded-graphics-simulator + SDL2)
├── Makefile              # Canonical build commands
├── .cargo/config.toml    # Per-target rustflags (no global target)
├── rust-toolchain.toml   # esp channel
├── AGENTS.md             # Full project philosophy / coding creed
└── STORAGE.md            # On-SD persistence format details
```

### `baro-core` layout

```
crates/baro-core/src/
├── lib.rs                # #![no_std], pub modules below
├── app_state.rs          # Shared application state containers
├── async_i2c_bus.rs      # Shared async I2C bus + TCA9548A mux routing
├── config.rs             # Runtime configuration / settings persistence
├── display_manager.rs    # Frame-pacing + page-draw orchestration
├── framebuffer.rs        # PSRAM-backed framebuffer abstraction
├── metrics.rs            # Derived metrics & summary calculations
├── sensor_store.rs       # Cross-task sensor value storage
├── widgets.rs            # Small reusable drawables (not full UI components)
├── sensors/              # bh1750, bmp388, scd41, sht40 drivers + IndexedSensor
├── storage/              # accumulator, manager, rollup_storage, sd_card
├── pages/
│   ├── home/             # grid.rs (HomeGridPage), outdoor.rs (HomePage)
│   ├── settings/         # list.rs (SettingsPage), display.rs (DisplaySettingsPage)
│   ├── trend/            # page, data, stats, constants
│   ├── monitor.rs        # Live sensor log
│   ├── wifi_status.rs    # WifiState / WifiStatusPage
│   ├── page.rs           # Page trait + PageWrapper enum
│   ├── page_manager.rs   # Transitions
│   └── constants.rs
└── ui/
    ├── core.rs           # Action, PageEvent, PageId, SensorData, TouchEvent
    ├── components/       # button, text, graph/, option_selector
    ├── layouts/          # container, scrollable
    ├── styling/          # colors, layout, style, theme
    └── elements.rs
```

### `baro-firmware` layout

```
crates/baro-firmware/src/
├── bin/main.rs           # Embassy entry point; spawns long-lived tasks
├── app_state/            # hardware.rs (peripheral bring-up), sensors_state.rs
├── dual_mode_pin.rs      # Runtime input/output register flipping for GPIO35
├── wifi_secrets.rs       # env!() WIFI_SSID / WIFI_PASSWORD
└── lib.rs
```

## Architecture Overview

### Async task model
Embassy executor on the ESP32-S3 (dual-core Xtensa LX7). `bin/main.rs` spawns long-lived tasks for sensing, storage, display, and networking. All cross-task communication is via `embassy-sync` primitives (`PubSubChannel`, `Signal`, `Mutex`).

### Data flow
```
Sensors → shared [i32; MAX_SENSORS] values array
        → Accumulator  → PubSub(RollupEvent) ─┬─→ Storage Manager (SD card)
                                              └─→ Display Manager (UI)
```

- **Sensors** read on a periodic schedule into a shared `[i32; MAX_SENSORS]` (`MAX_SENSORS = 20`).
- **Accumulator** (`storage/accumulator.rs`) buffers samples and emits rollups when tier thresholds are met.
- **Rollup tiers:** `RawSample`, `FiveMinute`, `Hourly`, `Daily`.
- **Time windows:** 1m, 5m, 30m, 1h, 6h, 1d, 1w.
- **ROLLUP_CHANNEL** (`PubSubChannel`) fans events out to storage and UI subscribers.
- **Storage** persists a raw ring buffer plus append-only rollup files. See [STORAGE.md](STORAGE.md).

### Type-safe sensor indexing

Sensors use const generics to guarantee correct array indices at compile time. Read the comment block in `sensors/mod.rs` — **there is no runtime check** that a sensor's readings land at the right indices, only the type-level guarantee through `IndexedSensor<S, START, COUNT, MUX_CHANNEL>`.

```rust
type SHT40Indexed<I>  = IndexedSensor<SHT40Sensor<I>,  0, 2, 0>;  // temp+humidity @ [0..2], mux ch 0
type SCD41Indexed<I>  = IndexedSensor<SCD41Sensor<I>,  2, 1, 1>;  // CO2           @ [2],    mux ch 1
type BH1750Indexed<I> = IndexedSensor<BH1750Sensor<I>, 3, 1, 2>;  // lux           @ [3],    mux ch 2
type BMP388Indexed<I> = IndexedSensor<BMP388Sensor<I>, 4, 1, 3>;  // pressure      @ [4],    mux ch 3
```

| Constant      | Index | Sensor | I2C Mux Ch | Unit     |
|---------------|-------|--------|------------|----------|
| `TEMPERATURE` | 0     | SHT40  | 0          | °C       |
| `HUMIDITY`    | 1     | SHT40  | 0          | %        |
| `CO2`         | 2     | SCD41  | 1          | ppm      |
| `LUX`         | 3     | BH1750 | 2          | lux      |
| `PRESSURE`    | 4     | BMP388 | 3          | hPa      |

`SensorType` (in `sensors/mod.rs`) is the canonical enum used by UI surfaces. Its `ALL` slice drives anything that enumerates sensors (e.g. `MonitorPage`) — extend `ALL` when adding a new sensor so those surfaces pick it up automatically.

Feature flags (all enabled by default):
`sensor-sht40`, `sensor-scd41`, `sensor-bh1750`, `sensor-bmp388`.

### UI framework

Custom component UI in `ui/`. Display is **320×240** (`DISPLAY_WIDTH_PX`, `DISPLAY_HEIGHT_PX`).

- **Core** (`ui/core.rs`) — `Action`, `PageEvent`, `PageId`, `SensorData`, `TouchEvent`.
- **Components** — `TextComponent`, `Button`, `Graph` (series/axis/grid/viewport/interpolation), `OptionSelector`.
- **Layouts** — `Container<N>` (flex-like alignment + spacing), `ScrollableContainer`.
- **Styling** — `Theme`, `Style`, palette, font constants, layout constants.

### Pages

All pages implement the `Page` trait (`draw_page`, `handle_touch`, `on_event`, `update`, `is_dirty` / `mark_clean`). `PageWrapper` is the enum the manager dispatches over.

| Page                   | Module                       | Purpose                                           |
|------------------------|------------------------------|---------------------------------------------------|
| `HomePage`             | `pages/home/outdoor.rs`      | Outdoor dashboard view                            |
| `HomeGridPage`         | `pages/home/grid.rs`         | Grid-style home dashboard                         |
| `TrendPage`            | `pages/trend/page.rs`        | Time-series graphs (switchable windows + stats)   |
| `MonitorPage`          | `pages/monitor.rs`           | Live sensor log using ASCII-safe formatting       |
| `SettingsPage`         | `pages/settings/list.rs`     | Settings index                                    |
| `DisplaySettingsPage`  | `pages/settings/display.rs`  | Temperature unit, home-page mode, etc.            |
| `WifiStatusPage`       | `pages/wifi_status.rs`       | Connecting / Connected / Error                    |

### Networking & time
- DHCP via `embassy-net`.
- NTP over UDP (pool.ntp.org, time.google.com fallback).
- `SimpleTimeSource` converts Unix timestamps to FAT format for the SD layer.

### Hardware quirks worth remembering
- **Shared SPI bus** — LCD (ILI9342C) and microSD share one SPI peripheral; access must be serialized.
- **GPIO35 dual-mode** — also acts as LCD DC while sharing the SPI MISO line. `DualModePin<const PIN: u8>` uses raw register writes to flip direction at runtime; wrapped by `OutputModeSpiDevice` / `InputModeSpiDevice`.
- **I²C mux** — TCA9548A on the internal I²C bus (GPIO12 SDA, GPIO11 SCL) routes to sensor channels. `async_i2c_bus` owns the routing.
- **Board** — M5Stack CoreS3 SE: AXP2101 PMIC, AW9523 GPIO expander, FT6336U touch, 8 MB PSRAM.
- **PSRAM** — 8 MB, used for framebuffer allocation.

## Desktop simulator

`baro-simulator` (`crates/baro-simulator/src/main.rs`) renders the exact same `baro-core` UI on desktop via SDL2. Synthetic sensor data via `MockSensorGenerator`, keyboard page navigation (digits + `Q`), mouse clicks forwarded as touch events, ~30 FPS pacing. This is the primary place to iterate on UI changes without flashing.
