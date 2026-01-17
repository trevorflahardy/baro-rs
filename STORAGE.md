# Baro Storage Plan (Optimized, Long-Lived, Sensor-Agnostic)

This storage design is optimized for:
- **Multiple sensors (assume up to 20 values per sample)**
- **Multiple time scales**: 5m, 1h, 24h, 7d, 1m, all-time
- **Very long lifetime** (many years)
- **Small screen constraints**
- **16 GB SD card**
- **Embedded reliability** (power loss, SD wear)

This is *not* a database-in-firmware. It’s a **time-series instrument design**.

---

## Design Principles

1. **Never store raw data forever**
2. **Pre-aggregate aggressively**
3. **Append-only writes**
4. **Fixed-size records**
5. **Versioned formats**
6. **O(1) access for graphs**
7. **Explainable math**

---

## Sensor Assumptions

- Up to **20 sensor values**
- Each sensor stored as **fixed-point i32** (e.g. milli-units)
- Timestamp always present
- No strings, no floats on disk

---

## Sampling Strategy

### Raw sampling rate
- **Every 10 seconds**
- Good visual smoothness
- Low write pressure

Samples per day:
8640

---

## Data Tiers

### Tier 1 — Raw Samples (Short-Term)
Used for:
- 5 minute graph
- 1 hour graph

Retention:
- **24 hours only**

Record format:
```rust
struct RawSample {
    timestamp: u32,      // seconds since boot or epoch
    values: [i32; 20],   // sensor readings
}
```

Size:
4 + (20 × 4) = 84 bytes
padded to 96 bytes

Storage/day:
```
8640 × 96 ≈ 830 KB/day
```
This is a ring buffer file, overwritten daily.

### Tier 2 — 5-Minute Rollups
Used for:
- 24 hour graph
- 7 day graph

Aggregation window:
5 minutes = 30 samples

Rollup record:
```rust
struct Rollup5m {
    start_ts: u32,
    avg: [i32; 20],
    min: [i32; 20],
    max: [i32; 20],
}
```

Size:
- timestamp: 4
- avg/min/max: 20 × 3 × 4 = 240
- total ≈ 256 bytes

Records/day:
```
288
```

Storage/day:
```
288 × 256 ≈ 74 KB/day
```

Storage/year:
```
≈ 27 MB/year
```

### Tier 3 - Hourly Rollups

Used for:
- 1 month graph
- long-term trends

Records/day:
```
24
```

Storage/year:
```
24 × 365 × 256 ≈ 2.2 MB/year
```

### Tier 4 — Daily Rollups
Used for:
- All-time stats
- Monthly heatmaps
- Long-term averages
Records/year:
```
365
```

Storage/year:
```
365 × 256 ≈ 94 KB/year
```

### Tier 5 — Lifetime Counters
Used for:
- uptime
- total samples
- exposure metrics
- max/min ever seen

Single struct, rewritten occasionally:
```rust
struct LifetimeStats {
    boot_time: u32,
    total_samples: u64,
    sensor_integrals: [i64; 20],
    sensor_max: [i32; 20],
    sensor_min: [i32; 20],
}
```

Size:
```
< 256 bytes
```

## Storage Summary (Per Year)

| Tier           | Size                |
| -------------- | ------------------- |
| Raw samples    | 0.8 MB              |
| 5-min rollups  | 27 MB               |
| Hourly rollups | 2.2 MB              |
| Daily rollups  | 0.09 MB             |
| **Total**      | **≈ 30 MB / year** |


### 16 GB Card Lifetime Estimate

Usable storage (safe):
```
≈ 14,000 MB
```

Years supported:
```
14,000 / 30 ≈ 467 years
```

## File Structure

One binary file per tier:

```
/
├── raw_samples.bin      (ring buffer, fixed 829,440 bytes)
├── rollup_5m.bin        (append-only)
├── rollup_1h.bin        (append-only)
├── rollup_daily.bin     (append-only)
└── lifetime.bin         (single record, 256 bytes)
```

### Why This Structure?

- **Simple append operations**: Each tier has fixed record size
- **O(1) seeking**: `offset = record_number × record_size`
- **Fast graph reads**: Read last N records from end of file
- **SD card friendly**: Fewer files, less FAT overhead, no fragmentation
- **Power-loss resilient**: Lose at most 1 record on failure
- **Easy validation**: `file_size % record_size == 0`

---

## Data Retention Policy

| Tier           | Retention    | Storage Strategy           | Size/Year |
| -------------- | ------------ | -------------------------- | --------- |
| Raw samples    | 24 hours     | Ring buffer (overwrite)    | 0.8 MB    |
| 5-min rollups  | **Forever**  | Append-only                | 27 MB     |
| Hourly rollups | **Forever**  | Append-only                | 2.2 MB    |
| Daily rollups  | **Forever**  | Append-only                | 0.09 MB   |
| Lifetime stats | **Forever**  | Single record (overwrite)  | 256 bytes |

### Why Keep All Rollups?

At 30 MB/year, a 16 GB card lasts **467 years**. Even after 10 years of continuous operation, only **300 MB** (~2%) is used. No pruning needed.

---

## Write Patterns

### Every 10 seconds:
1. Write 1 raw sample to ring buffer
2. Position wraps after 8,640 samples (24 hours)

```rust
let pos = (sample_count % 8640) * 96;
file.seek(SeekFrom::Start(pos as u64))?;
file.write_all(&raw_sample_bytes)?;
```

### Every 5 minutes (30 raw samples):
1. Calculate avg/min/max from last 30 raw samples
2. Append 1 record to `rollup_5m.bin`

```rust
file.seek(SeekFrom::End(0))?;
file.write_all(&rollup_bytes)?;
```

### Every 1 hour (12 five-minute rollups):
1. Calculate avg/min/max from last 12 five-minute rollups
2. Append 1 record to `rollup_1h.bin`

### Every 24 hours (24 hourly rollups):
1. Calculate avg/min/max from last 24 hourly rollups
2. Append 1 record to `rollup_daily.bin`

---

## Graph Coverage (Guaranteed)
| Timeframe | Data Source                    | Records Read |
| --------- | ------------------------------ | ------------ |
| 5 min     | raw samples                    | 30           |
| 1 hour    | raw samples                    | 360          |
| 24 hour   | 5-min rollups                  | 288          |
| 7 days    | 5-min rollups                  | 2,016        |
| 1 month   | hourly rollups                 | 720          |
| All-time  | daily rollups + lifetime stats | all records  |

---

## Binary Format

All records use **little-endian** encoding for multi-byte values.

### Raw Sample (96 bytes)
```rust
#[repr(C)]
struct RawSample {
    timestamp: u32,      // seconds since epoch
    values: [i32; 20],   // sensor readings (fixed-point)
    _padding: [u8; 12],  // pad to 96 bytes
}
```

### Rollup Record (256 bytes)
```rust
#[repr(C)]
struct Rollup {
    start_ts: u32,       // window start timestamp
    avg: [i32; 20],      // averages
    min: [i32; 20],      // minimums
    max: [i32; 20],      // maximums
    _padding: [u8; 12],  // pad to 256 bytes
}
```

### Lifetime Stats (256 bytes)
```rust
#[repr(C)]
struct LifetimeStats {
    boot_time: u32,
    total_samples: u64,
    sensor_integrals: [i64; 20],
    sensor_max: [i32; 20],
    sensor_min: [i32; 20],
    _padding: [u8; 24],  // pad to 256 bytes
}
```
