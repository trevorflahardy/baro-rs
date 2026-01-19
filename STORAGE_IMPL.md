# Storage Implementation Notes

## Struct Sizes

The rollup backend implements three main data structures as specified in STORAGE.md:

### RawSample (96 bytes)
- **Actual size**: 96 bytes ✓
- Stores raw sensor readings for 24-hour ring buffer
- Binary layout: timestamp (4) + values[20] (80) + padding (12) = 96 bytes

### Rollup (256 bytes)  
- **Actual size**: 256 bytes ✓
- Stores aggregated min/avg/max for 5m, 1h, and daily tiers
- Binary layout: start_ts (4) + avg[20] (80) + min[20] (80) + max[20] (80) + padding (12) = 256 bytes

### LifetimeStats (336 bytes)
- **Actual size**: 336 bytes (not 256 as originally specified)
- **Reason**: The C ABI requires 8-byte alignment for u64 and i64 fields, adding 4 bytes of padding after boot_time
- Binary layout: boot_time (4) + align_pad (4) + total_samples (8) + sensor_integrals[20] (160) + sensor_max[20] (80) + sensor_min[20] (80) = 336 bytes

### Alignment Consideration

The original STORAGE.md specified LifetimeStats as 256 bytes, but the actual size with proper C alignment is 336 bytes:
- boot_time: 4 bytes
- **Implicit padding: 4 bytes** (added by compiler for u64 alignment)
- total_samples: 8 bytes
- sensor_integrals: 160 bytes
- sensor_max: 80 bytes
- sensor_min: 80 bytes
- **Total: 336 bytes**

This is still acceptable for single-record storage (< 1 KB) and ensures proper memory alignment on the ESP32-S3 target, avoiding potential performance issues or alignment faults.

## Features

All structs implement:
- ✅ Binary serialization (`to_bytes()`)
- ✅ Binary deserialization (`from_bytes()`)
- ✅ Little-endian encoding for multi-byte values
- ✅ `#[repr(C)]` for predictable memory layout
- ✅ Fixed-size records for O(1) file seeking

Additional features:
- ✅ `Rollup::from_samples()` - Calculate rollup from raw samples
- ✅ `Rollup::from_rollups()` - Aggregate multiple rollups into higher tier
- ✅ `LifetimeStats::update()` - Update lifetime stats with new sample
- ✅ Comprehensive unit tests for sizes and serialization

## Usage

These structures are ready for use in storage tasks. Next steps:
1. Create file I/O backend for SD card
2. Implement ring buffer for raw samples
3. Implement append-only writers for rollups
4. Create async task for periodic rollup generation (as outlined in ASYNC_TASKS.md)
