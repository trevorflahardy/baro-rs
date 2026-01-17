# Embassy Async Task Guide

This project is now set up to use Embassy's async/await executor. The foundation is in place to add async tasks as needed.

## Current Setup

- ✅ Embassy executor initialized in `main()`
- ✅ `embassy-time` for async timers
- ✅ `embassy-sync` available for channels/mutexes
- ✅ Main loop runs asynchronously

## How to Add Async Tasks

### 1. Define Your Task Function

Tasks are async functions marked with `#[embassy_executor::task]`:

```rust
#[embassy_executor::task]
async fn sensor_task() {
    loop {
        // Read sensors
        let data = read_sensor().await;

        // Do something with data
        process(data);

        // Wait before next iteration
        Timer::after(Duration::from_secs(10)).await;
    }
}
```

### 2. Spawn the Task from Main

In `main()`, use the `spawner` to launch your task:

```rust
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // ... hardware init ...

    // Spawn your tasks
    spawner.spawn(sensor_task()).unwrap();
    spawner.spawn(storage_task()).unwrap();

    // Main loop continues independently
    loop {
        // ...
    }
}
```

## Inter-Task Communication

### Using Channels (Producer-Consumer)

For sending data between tasks:

```rust
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::Channel,
};

// Define a global channel
static DATA_CHANNEL: Channel<CriticalSectionRawMutex, SensorData, 4> = Channel::new();

// Producer task
#[embassy_executor::task]
async fn sensor_task() {
    loop {
        let data = read_sensor();
        DATA_CHANNEL.send(data).await;  // Blocks if full
        Timer::after(Duration::from_secs(10)).await;
    }
}

// Consumer task
#[embassy_executor::task]
async fn storage_task() {
    loop {
        let data = DATA_CHANNEL.receive().await;  // Blocks until data available
        write_to_sd(data);
    }
}
```

### Using Signals (Event Notification)

For simple event notifications:

```rust
use embassy_sync::signal::Signal;

static BUTTON_PRESSED: Signal<CriticalSectionRawMutex, ()> = Signal::new();

// Signaler
#[embassy_executor::task]
async fn button_task() {
    loop {
        wait_for_button_press().await;
        BUTTON_PRESSED.signal(());
    }
}

// Waiter
#[embassy_executor::task]
async fn display_task() {
    loop {
        BUTTON_PRESSED.wait().await;
        update_display();
    }
}
```

### Using Mutex (Shared State)

For protecting shared resources:

```rust
use embassy_sync::mutex::Mutex;

static DISPLAY: Mutex<CriticalSectionRawMutex, Option<Display>> = Mutex::new(None);

async fn use_display() {
    let mut display = DISPLAY.lock().await;
    if let Some(disp) = display.as_mut() {
        disp.draw_text("Hello");
    }
}
```

## Recommended Task Structure for Baro

### Sensor Task
- Reads sensors every 10 seconds
- Sends data to storage task via channel
- Sends display updates via channel
- Non-blocking

### Storage Task
- Receives sensor data from channel
- Writes to SD card (can be slow, won't block other tasks)
- Handles rollup calculations
- Manages ring buffer

### Display Task (or Main Loop)
- Receives display commands via channel
- Updates screen at 20-60 FPS
- Remains responsive even during SD writes
- Can be in main loop if display can't be moved

## Example: Complete Sensor + Storage Setup

```rust
// Data structure
#[derive(Clone, Copy)]
struct SensorReading {
    timestamp: u32,
    temperature: i32,
    pressure: i32,
    humidity: i32,
}

// Channel
static SENSOR_DATA: Channel<CriticalSectionRawMutex, SensorReading, 4> = Channel::new();

// Sensor task
#[embassy_executor::task]
async fn sensor_task() {
    loop {
        let reading = SensorReading {
            timestamp: get_timestamp(),
            temperature: read_temp_sensor(),
            pressure: read_pressure_sensor(),
            humidity: read_humidity_sensor(),
        };

        SENSOR_DATA.send(reading).await;
        Timer::after(Duration::from_secs(10)).await;
    }
}

// Storage task
#[embassy_executor::task]
async fn storage_task() {
    let mut samples = 0;
    loop {
        let reading = SENSOR_DATA.receive().await;

        // Write to ring buffer
        write_raw_sample(&reading).await;
        samples += 1;

        // Every 30 samples (5 min), create rollup
        if samples >= 30 {
            create_5min_rollup().await;
            samples = 0;
        }
    }
}

// Main
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // Init hardware...

    spawner.spawn(sensor_task()).unwrap();
    spawner.spawn(storage_task()).unwrap();

    // Main loop handles display
    loop {
        update_display();
        Timer::after(Duration::from_millis(50)).await;
    }
}
```

## Benefits of This Architecture

1. ✅ **Display stays responsive**: SD writes don't block UI
2. ✅ **Clean separation**: Each task has single responsibility
3. ✅ **Type-safe**: Channels prevent race conditions
4. ✅ **Scalable**: Easy to add more tasks (WiFi, battery monitor, etc.)
5. ✅ **Power efficient**: Executor sleeps when all tasks are waiting

## Next Steps

When you're ready to implement:
1. Define your data structures (SensorReading, etc.)
2. Create channels for inter-task communication
3. Implement task functions with `#[embassy_executor::task]`
4. Spawn tasks from `main()`
5. Use `Timer::after()` for delays instead of blocking
