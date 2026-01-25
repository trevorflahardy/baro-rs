# AGENTS.md

## Project Overview

**Project name:** `baro-rs`
**Purpose:**
`baro-rs` is a long-running, firmware-first environmental instrumentation device built in **Rust** for the **ESP32-S3** platform. The device continuously measures environmental, system, and derived metrics over long periods of time (months to years), logs them persistently, and presents meaningful summaries and visualizations on a small display.

This project is **not a demo** or throwaway firmware. It is designed to be:
- robust
- modular
- inspectable
- expandable over time
- pleasant to maintain

All contributors are expected to treat this as production-grade embedded Rust.

---

## Target Hardware

### Primary Board
- **M5Stack CoreS3 SE**
  - ESP32-S3 (dual-core Xtensa LX7)
  - SPI LCD display
  - SPI microSD card
  - Internal I²C bus (`intSDA = GPIO12`, `intSCL = GPIO11`)
  - Onboard peripherals (AW9523 GPIO expander, PMIC, etc.)

### Initial Sensors
- **SHT40** (I²C) – temperature & humidity

### Planned / Expandable Sensors
- Barometric pressure (BMP/BME series)
- Ambient light (BH1750 / VEML series)
- CO₂ (SCD4x)
- VOC / IAQ (SGP4x)
- Power / current monitoring
- Additional I²C or SPI peripherals as needed

### Key Hardware Constraints
- SPI bus is shared between LCD and SD card
- Some pins are multiplexed at the board level and must be handled carefully
- I²C is the primary expansion bus
- Firmware must serialize bus access safely

---

## Tooling Requirements (Mandatory)

All code **must** pass the following before being considered acceptable:

- `cargo check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo fmt`

CI and/or contributors should treat **clippy warnings as errors**.

### Formatting
- `rustfmt` is mandatory.
- Do not fight rustfmt.
- Do not commit unformatted code.

### Linting
- Clippy is not optional.
- If clippy complains, fix the code or explicitly justify the exception.

---

## Code Style & Philosophy

### General Principles

- Prefer **clarity over cleverness**.
- Prefer **explicitness over magic**.
- Prefer **simple, readable code** over micro-optimizations.
- This is firmware that may run for **years** — treat it accordingly.

### Error Handling and Robustness

It is very important that the device handles errors **gracefully**:
- anticipate and handle possible failure modes
- avoid panics in production code
- use `Result` and `Option` effectively
- log errors meaningfully

All errors stem from one centralized `AppError` struct. Expand on this
as needed to ensure meaningful error propagation.

---

### Naming & Constants

- **Do not use magic numbers**.
- Constants must have descriptive names.
- Include units in names when applicable.

Good:
- `const LOG_FLUSH_INTERVAL_SECS: u64 = 60;`
- `const DISPLAY_REFRESH_HZ: u32 = 5;`

Bad:
- `let timeout = 60;`
- `let rate = 5;`

---

### Documentation

Most code should be **self-documenting**. Add doc comments when:
- behavior is non-obvious
- hardware interactions are subtle
- invariants must be preserved
- public APIs need usage guidance

Avoid:
- redundant comments
- restating obvious code

Prefer:
- small functions with clear names
- meaningful types and modules

---

### Function & Module Size

- Keep functions **short**.
- If a function feels long, split it and extract helpers.
- Avoid “god” modules and “do-everything” files.

Guidelines:
- Functions should generally fit on one screen.
- Each module should have a single, clear responsibility.
- Optimize for maintainability over “one-file convenience”.

---

## Project Structure Expectations

The project should be **modular and expandable**.

Preferred structure:
- modules represent **conceptual subsystems**
- hardware quirks are isolated
- subsystem APIs are clear and testable

Suggested modules (example, not strict):
- `sensors/` (drivers + measurement normalization)
- `storage/` (log formats, rollups, SD I/O)
- `display/` (rendering, screens, UI flow)
- `net/` (Wi-Fi, SNTP, networking services)
- `time/` (time sync, timestamps, timezone/display conversions)
- `metrics/` (derived metrics, rollups, alert thresholds)
- `hal/` (board-specific pinouts, bus ownership, device bring-up)

Avoid:
- large, short-lived files
- scattered hardware manipulation
- mixing unrelated subsystems in one module

---

## Hardware Interaction Guidelines

- All hardware access should be **centralized and abstracted**.
- Shared buses (SPI, I²C) must be:
  - serialized
  - thread-safe
  - race-free by construction

Do not:
- scatter raw GPIO manipulation across the codebase
- duplicate bus-handling logic
- assume exclusive ownership of shared peripherals

If a peripheral has board-specific quirks, document them in the `hal/` module and keep the workaround localized.

---

## Time & Data Integrity

### Time
- Store timestamps as **UTC Unix epoch**.
- Convert to local time for **display only**.
- Use **SNTP** to synchronize time after Wi-Fi connects.
- Handle:
  - offline boots
  - delayed time sync
  - “approximate time” vs “synced time” states

### Storage
Persistent data must:
- be append-friendly
- survive power loss as well as reasonably possible
- prefer fixed-size records and versioned headers
- avoid write amplification

The device should remain functional even if:
- SD card is missing
- SD card is read-only
- SD card errors occur intermittently

---

## Testing & Validation

Even as embedded firmware, changes must be validated:
- logic should be testable on host when possible
- hardware-dependent code should be isolated
- error paths must be handled intentionally

Minimum expectation before PR/merge:
- `cargo check`
- `cargo fmt`
- `cargo clippy` with warnings denied
- basic runtime sanity on hardware for HAL-level changes

---

## Contribution Mindset

When contributing, ask:
- “Will this still make sense in 6 months?”
- “Can this be extended without rewriting it?”
- “Is the intent obvious to someone new to the codebase?”
- “Would I trust this to run unattended for a year?”

If not, refactor toward clarity and modularity.

---

## Summary

`baro-rs` is an instrumentation firmware project, not a toy.
Treat the hardware with respect, the data with care, and the codebase as something that should age well.

Clean code. Clear intent. Stable foundations.
