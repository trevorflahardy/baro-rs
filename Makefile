# baro-rs — Firmware + Simulator build commands
#
# The workspace contains both an ESP32-S3 firmware crate and a desktop
# simulator. Each needs a different target and toolchain configuration,
# so these Make targets wrap the correct cargo invocations.

ESP_TARGET  := xtensa-esp32s3-none-elf
BUILD_STD   := -Z build-std=alloc,core

# ---------------------------------------------------------------------------
# Firmware (ESP32-S3)
# ---------------------------------------------------------------------------
.PHONY: fw fw-check fw-release fw-clippy

fw:           ## Build firmware (debug)
	cargo build --target $(ESP_TARGET) $(BUILD_STD)

fw-check:     ## Type-check firmware
	cargo check --target $(ESP_TARGET) $(BUILD_STD)

fw-release:   ## Build firmware (release, LTO)
	cargo build --target $(ESP_TARGET) $(BUILD_STD) --release

fw-clippy:    ## Lint firmware
	cargo clippy --target $(ESP_TARGET) $(BUILD_STD) -- -D warnings

# ---------------------------------------------------------------------------
# Simulator (host / desktop)
# ---------------------------------------------------------------------------
.PHONY: sim sim-check sim-clippy

sim:          ## Run the desktop simulator
	cargo run -p baro-simulator

sim-check:    ## Type-check the simulator
	cargo check -p baro-simulator

sim-clippy:   ## Lint the simulator
	cargo clippy -p baro-simulator -- -D warnings

# ---------------------------------------------------------------------------
# Both
# ---------------------------------------------------------------------------
.PHONY: check-all clippy-all fmt fmt-check

check-all: fw-check sim-check   ## Type-check firmware + simulator

clippy-all: fw-clippy sim-clippy ## Lint everything

fmt:          ## Format all code
	cargo fmt --all

fmt-check:    ## Check formatting (CI)
	cargo fmt --all -- --check

# ---------------------------------------------------------------------------
# Help
# ---------------------------------------------------------------------------
.PHONY: help
help:         ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2}'

.DEFAULT_GOAL := help
